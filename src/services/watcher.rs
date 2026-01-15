use std::{sync::Arc, time::Duration};

use alloy::{
    primitives::{Address, U256},
    providers::{DynProvider, Provider},
    rpc::types::{Filter, Log, Topic},
    sol_types::SolEvent,
};
use futures::StreamExt;
use thiserror::Error;
use tokio::time::interval;

use crate::{
    domain::{BalanceEvent, EvmNetwork},
    evm::{erc20::ERC20, wrapped::WrappedToken},
    services::{balances, subscription_manager::Subscription},
};

struct TokenBalance {
    address: Address,
    balance: U256,
}

#[derive(Error, Debug, Clone)]
pub enum WatcherError {
    #[error("ws rpc subscription on erc20 TRANSFER for network({0}) event connection error for owner: {1}")]
    Erc20WsSubscriptionError(EvmNetwork, Address),

    #[error("ws rpc subscription on Weth({0}:{1}) DEPOSIT/TRANFSER event connection error for owner: {2}")]
    WethEventsSubscriptionError(EvmNetwork, Address, Address),
}

pub struct WatcherContext {
    pub owner: Address,
    pub provider: DynProvider,
    pub network: EvmNetwork,
    pub multicall3: Address,
    pub ws_provider: DynProvider,
    pub weth_address: Address,
}

pub struct Watcher {
    ctx: Arc<WatcherContext>,
    sub: Arc<Subscription>,
}

impl Watcher {
    pub fn new(ctx: WatcherContext, subscription: Arc<Subscription>) -> Self {
        Self {
            ctx: Arc::new(ctx),
            sub: subscription,
        }
    }

    pub async fn spawn_watchers(&self, interval_secs: usize) {
        self.spawn_snapshot_updater(interval_secs).await;

        match self.spawn_erc20_transfer_listeners().await {
            Ok(()) => {}
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "error when spawn erc20 listeners"
                );
            }
        }

        match self.spawn_wrapped_events_listener().await {
            Ok(()) => {}
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "error when spawn weth listeners for {}",
                    self.ctx.weth_address,
                );
            }
        }
    }

    // request balances via multicall every interval_secs to have an actual
    async fn spawn_snapshot_updater(&self, interval_secs: usize) {
        let sub = Arc::clone(&self.sub);
        let ctx = Arc::clone(&self.ctx);
        let cancel = sub.cancel_token.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs as u64));

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => { break; }
                    _ = interval.tick() => {
                        Self::fetch_balances_and_broadcast(Arc::clone(&ctx), Arc::clone(&sub)).await;
                    }
                }
            }
        });
    }

    async fn fetch_balances_and_broadcast(ctx: Arc<WatcherContext>, sub: Arc<Subscription>) {
        let result = {
            let tokens = sub.tokens.read().await;

            balances::get_balances(
                &tokens,
                &ctx.provider,
                ctx.owner,
                ctx.network,
                ctx.multicall3,
            )
            .await
        };

        let event = match result {
            Ok(balances) => {
                let mut balances_snapshot = sub.balances_snapshot.write().await;
                *balances_snapshot = balances.clone();
                BalanceEvent::FullSnapshot(balances)
            }
            Err(e) => {
                tracing::error!("Failed to get balances for {}: {}", ctx.owner, e);
                BalanceEvent::Error {
                    code: 500,
                    message: "Error when make multicall3 request".to_string(),
                }
            }
        };

        let _ = sub.sender.send(event).inspect_err(|err| {
            tracing::info!("unable to send update_snapshot event to clients: {err}");
        });
    }

    /**
     * Listen Deposit/Withdrawal events
     *
     * Need to sync wrap/unwrap txs to handle wrapped token balance
     */
    // TODO
    async fn spawn_wrapped_events_listener(&self) -> Result<(), WatcherError> {
        let ctx = Arc::clone(&self.ctx);

        let event_signatures = vec![
            WrappedToken::Deposit::SIGNATURE_HASH,
            WrappedToken::Withdrawal::SIGNATURE_HASH,
        ];
        let filter = Filter::new()
            .address(ctx.weth_address)
            .event_signature(event_signatures);

        let mut stream = self
            .ctx
            .ws_provider
            .clone()
            .subscribe_logs(&filter)
            .await
            .map_err(|err| {
                tracing::error!(
                    error = %err,
                    "error to subscribe on Weth({}:{}) TRANSFER/DEPOSIT events for owner: {}",
                    ctx.network,
                    ctx.weth_address,
                    ctx.owner,
                );

                WatcherError::WethEventsSubscriptionError(ctx.network, ctx.weth_address, ctx.owner)
            })?
            .into_stream();

        let sub = Arc::clone(&self.sub);
        let cancel = sub.cancel_token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    },
                    Some(log) = stream.next() => {
                        match log.topic0() {
                            Some(hash) if *hash == WrappedToken::Deposit::SIGNATURE_HASH => {
                                match log.log_decode::<WrappedToken::Deposit>() {
                                    Ok(decoded) => {
                                        let data = decoded.inner.data;
                                        tracing::info!("catch Deposit event, receiver: {}, value: {}", data.dst, data.wad);
                                    },
                                    // TODO handle error
                                    Err(_) => {},
                                };

                            },
                            Some(hash) if *hash == WrappedToken::Withdrawal::SIGNATURE_HASH => {
                                match log.log_decode::<WrappedToken::Withdrawal>() {
                                    Ok(decoded) => {
                                        let data = decoded.inner.data;
                                        tracing::info!("catch Withdrawal event, receiver: {}, value: {}", data.src, data.wad);
                                    },
                                    Err(_) => {},
                                }
                            }
                            _ => {}
                            // TODO handle None
                        };
                    }
                    // TODO: handle None - disconnect
                }
            }
        });

        Ok(())
    }

    async fn spawn_erc20_transfer_listeners(&self) -> Result<(), WatcherError> {
        let ctx = Arc::clone(&self.ctx);
        let base = Filter::new().event_signature(ERC20::Transfer::SIGNATURE_HASH);
        let from = base.clone().topic1(Topic::from(ctx.owner));
        let to = base.clone().topic2(Topic::from(ctx.owner));

        self.spawn_erc20_transfer_listener_with_filter(from).await?;
        self.spawn_erc20_transfer_listener_with_filter(to).await?;

        Ok(())
    }

    async fn spawn_erc20_transfer_listener_with_filter(
        &self,
        filter: Filter,
    ) -> Result<(), WatcherError> {
        let ctx = Arc::clone(&self.ctx);

        // TODO check limit for address list - maybe it can be better to use filter by addresses than general one
        let mut stream = self
            .ctx
            .ws_provider
            .clone()
            .subscribe_logs(&filter)
            .await
            .map_err(|err| {
                tracing::error!(
                    error = %err,
                    "error to subscribe on erc20 transfer event {} for network {}",
                    ctx.owner,
                    ctx.network,
                );

                WatcherError::Erc20WsSubscriptionError(ctx.network, ctx.owner)
            })?
            .into_stream();

        let sub = Arc::clone(&self.sub);
        let provider = Arc::new(ctx.provider.clone());
        let owner = ctx.owner;
        let network = ctx.network;
        let cancel = sub.cancel_token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    },
                    // TODO handle None case / reconnect
                    Some(log) = stream.next() => {
                        // TODO check address in map before request
                        let token_balance = Self::parse_transfer_and_get_balance(Arc::clone(&provider), owner, network, &log).await;
                        let event = match token_balance {
                            Some(token_balance) => {
                                // parse value from balance_of and update snapshot
                                // then send update_balance event with new balance for token to clients
                                let balance_as_string = token_balance.balance.to_string();
                                let mut balance_snapshot = sub.balances_snapshot.write().await;
                                balance_snapshot.insert(token_balance.address, balance_as_string.clone());
                                BalanceEvent::TokenBalanceUpdated { address: token_balance.address, balance: balance_as_string }
                            },
                            None => {
                                BalanceEvent::Error {
                                    code: 500,
                                    message: "unable to parse erc20 tranfer event".to_string(),
                                }
                            }
                        };

                        let _ = sub.sender.send(event).inspect_err(|err| {
                            tracing::info!(
                                error = %err,
                                "unable to send update_balance event: {err}"
                            );
                        });
                    }
                }
            }
        });

        Ok(())
    }

    async fn parse_transfer_and_get_balance(
        provider: Arc<DynProvider>,
        owner: Address,
        network: EvmNetwork,
        log: &Log,
    ) -> Option<TokenBalance> {
        let Some(block_number) = log.block_number else {
            tracing::warn!("block number is undefined for network {}", network,);
            return None;
        };

        let decoded_log: Log<ERC20::Transfer> = match log.log_decode() {
            Ok(log) => log,
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "error when parsing log for network {}",
                    network,
                );
                return None;
            }
        };

        let erc20 = ERC20::new(decoded_log.address(), &provider);

        match erc20
            .balanceOf(owner)
            .block(block_number.into())
            .call()
            .await
        {
            Ok(balance) => Some(TokenBalance {
                address: decoded_log.address(),
                balance,
            }),
            Err(e) => {
                tracing::error!(
                    "failed to get balance for {} at block {}: {:?}",
                    decoded_log.address(),
                    block_number,
                    e
                );
                None
            }
        }
    }
}
