use std::{sync::Arc, time::Duration};

use crate::evm::erc20::ERC20;
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
    evm::wrapped::WrappedToken,
    services::{balances, subscription_manager::Subscription},
};

struct TokenBalance {
    address: Address,
    balance: U256,
}

enum WethEvents {
    Deposit(U256),
    Withdrawal(U256),
}

#[derive(Error, Debug, Clone)]
pub enum WatcherError {
    #[error("ws rpc subscription on erc20 TRANSFER for network({0}) event connection error for owner: {1}")]
    Erc20WsSubscriptionError(EvmNetwork, Address),

    #[error("ws rpc subscription on Weth({0}:{1}) DEPOSIT/TRANFSER event connection error for owner: {2}")]
    WethEventsSubscriptionError(EvmNetwork, Address, Address),
}

#[derive(Error, Debug, Clone)]
pub enum ParseWeb3LogsError {
    #[error("parse weth {0} log error, details: {1}")]
    ParseWethLogError(String, String),

    #[error("log.topic0() is none")]
    Topic0IsNone,

    #[error("event HASH_SIGNATURE is not expected")]
    UnexpectedHashSignature,
}

#[derive(Error, Debug, Clone)]
pub enum UpdateBalanceError {
    #[error("unable to parse balance from snapshot string: token: {0}, value: {1}")]
    UnableToParseBalanceString(Address, String),

    #[error("overflow happened: token {token}, operation: {operation}, initial_value: {initial_value}, operand: {operand}")]
    OverflowError {
        token: Address,
        operation: char,
        initial_value: U256,
        operand: U256,
    },

    #[error("attempt to sub from zero balance: token: {0}, value to sub: {1}")]
    AttemptToSubFromZero(Address, U256),
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

        let sub: Arc<Subscription> = Arc::clone(&self.sub);
        let cancel = sub.cancel_token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    },
                    Some(log) = stream.next() => {
                        let event = match Self::parse_weth_logs(&log) {
                            Ok(parsed_event_data) => {
                                match Self::update_weth_balance_in_snapshot(Arc::clone(&sub), ctx.weth_address, parsed_event_data).await {
                                    Ok(value) => BalanceEvent::TokenBalanceUpdated { address: ctx.weth_address, balance: value.to_string() },
                                    Err(err) => BalanceEvent::Error { code: 500, message: err.to_string() },
                                }
                            },
                            Err(err) => {
                                BalanceEvent::Error {
                                    code: 500,
                                    message: err.to_string(),
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
                    // TODO: handle None - disconnect
                }
            }
        });

        Ok(())
    }

    async fn update_weth_balance_in_snapshot(
        sub: Arc<Subscription>,
        token: Address,
        event_data: WethEvents,
    ) -> Result<U256, UpdateBalanceError> {
        // cases that need to handle
        // * there is no balance in the snapshot
        // ** got a withdrawal -> error
        // ** got a deposit -> insert balance
        // * there is balance in the snapshot
        // ** got a withdrawal -> checked_sub
        // *** overflow -> error
        // *** sucsess -> update balance
        // ** got a deposit -> checked_add
        // *** overflow -> error
        // *** success -> updated the snapshot

        let mut snapshot = sub.balances_snapshot.write().await;

        let new_balance = match snapshot.get(&token) {
            Some(balance_as_string) => {
                let curr_balance = balance_as_string.parse::<U256>().map_err(|err| {
                    tracing::error!(
                        error = %err,
                        "error when parse balance string to U256 from snapshot"
                    );
                    UpdateBalanceError::UnableToParseBalanceString(token, balance_as_string.clone())
                })?;

                match event_data {
                    WethEvents::Deposit(value) => {
                        curr_balance
                            .checked_add(value)
                            // TODO add tracing
                            .ok_or(UpdateBalanceError::OverflowError {
                                token,
                                operation: '+',
                                initial_value: curr_balance,
                                operand: value,
                            })?
                    }
                    WethEvents::Withdrawal(value) => {
                        curr_balance
                            .checked_sub(value)
                            // TODO add tracing
                            .ok_or(UpdateBalanceError::OverflowError {
                                token,
                                operation: '-',
                                initial_value: curr_balance,
                                operand: value,
                            })?
                    }
                }
            }
            None => match event_data {
                WethEvents::Deposit(value) => value,
                WethEvents::Withdrawal(value) => {
                    return Err(UpdateBalanceError::AttemptToSubFromZero(token, value));
                }
            },
        };

        snapshot.insert(token, new_balance.to_string());
        Ok(new_balance)
    }

    fn parse_weth_logs(log: &Log) -> Result<WethEvents, ParseWeb3LogsError> {
        let topic0 = log.topic0().ok_or(ParseWeb3LogsError::Topic0IsNone)?;

        if *topic0 == WrappedToken::Deposit::SIGNATURE_HASH {
            let data = log
                .log_decode::<WrappedToken::Deposit>()
                .map_err(|err| {
                    tracing::error!(
                        error = %err,
                        "error when decode DEPOSIT event"
                    );

                    ParseWeb3LogsError::ParseWethLogError("Deposit".into(), err.to_string())
                })?
                .inner
                .data;

            tracing::info!("Deposit event dst={}, wad={}", data.dst, data.wad);
            return Ok(WethEvents::Deposit(data.wad));
        }

        if *topic0 == WrappedToken::Withdrawal::SIGNATURE_HASH {
            let data = log
                .log_decode::<WrappedToken::Withdrawal>()
                .map_err(|err| {
                    tracing::error!(
                        error = %err,
                        "error when decode Withdrawal event"
                    );

                    ParseWeb3LogsError::ParseWethLogError("Withdrawal".into(), err.to_string())
                })?
                .inner
                .data;

            tracing::info!("Withdrawal event: src={}, wad={}", data.src, data.wad);
            return Ok(WethEvents::Withdrawal(data.wad));
        }

        Err(ParseWeb3LogsError::UnexpectedHashSignature)
    }

    async fn spawn_erc20_transfer_listeners(&self) -> Result<(), WatcherError> {
        let ctx = Arc::clone(&self.ctx);
        let base = Filter::new().event_signature(ERC20::Transfer::SIGNATURE_HASH);
        let from = base.clone().topic1(Topic::from(ctx.owner));
        let to = base.clone().topic2(Topic::from(ctx.owner));

        self.spawn_erc20_transfer_listener_with_filter(from).await?;
        self.spawn_erc20_transfer_listener_with_filter(to).await?;
        self.spawn_wrapped_events_listener().await?;

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
