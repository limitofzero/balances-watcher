use crate::evm::erc20::ERC20;
use alloy::eips::BlockId;
use alloy::{
    primitives::{Address, U256},
    providers::{DynProvider, Provider},
    rpc::types::{Filter, Log, Topic},
    sol_types::SolEvent,
};
use futures::StreamExt;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::RwLockWriteGuard;
use tokio::time::interval;

use crate::services::balances::BalanceCallCtx;
use crate::{
    domain::{BalanceEvent, EvmNetwork},
    evm::wrapped::WrappedToken,
    services::{balances, subscription_manager::Subscription},
};

enum WethEvents {
    Deposit(Option<BlockId>),
    Withdrawal(Option<BlockId>),
}

#[derive(Error, Debug, Clone)]
pub enum WatcherError {
    #[error("ws rpc subscription on erc20 TRANSFER for network({0}) event connection error for owner: {1}")]
    Erc20WsSubscription(EvmNetwork, Address),

    #[error("ws rpc subscription on Weth({0}:{1}) DEPOSIT/TRANFSER event connection error for owner: {2}")]
    WethEventsSubscription(EvmNetwork, Address, Address),

    #[error("unable to get balance for owner{0} in network{1}: {2}")]
    GettingBalance(Address, EvmNetwork, String),

    #[error("Parse log error for network: {1}, owner: {2}: {0}")]
    ParseLog(EvmNetwork, Address, String),
}

#[derive(Error, Debug, Clone)]
pub enum ParseWeb3LogsError {
    #[error("log.topic0() is none")]
    Topic0IsNone,

    #[error("event HASH_SIGNATURE is not expected")]
    UnexpectedHashSignature,
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

    // create all necessary watchers to sync balances
    // spawn_erc20_transfer_listeners - spawn listener for erc20 transfer events
    // spawn_wrapped_events_listener - spawn listener for wrapped token events (deposit/withdrawal)
    // spawn_snapshot_updater - spawn listener for snapshot update (every interval_secs)
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

    // watcher to request balances via multicall every interval_secs to have an actual state
    // it update the whole state of balances and then send event to clients
    // could be removed if we check more ws subscriptions for updates
    async fn spawn_snapshot_updater(&self, interval_secs: usize) {
        let sub = Arc::clone(&self.sub);
        let ctx = Arc::clone(&self.ctx);
        let cancel = sub.cancel_token.clone();

        let (balance_call_ctx, tokens) = {
            let tokens = sub.tokens.read().await;
            let tokens: Vec<Address> = tokens.iter().copied().collect();

            let balance_call_ctx = BalanceCallCtx {
                owner: ctx.owner,
                multicall3: ctx.multicall3,
                provider: Arc::new(ctx.provider.clone()),
                network: ctx.network,
            };

            (Arc::new(balance_call_ctx), tokens)
        };

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs as u64));

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => { break; }
                    _ = interval.tick() => {
                        Self::fetch_balances_and_broadcast(Arc::clone(&balance_call_ctx), &tokens, Arc::clone(&sub)).await;
                    }
                }
            }
        });
    }

    // request all balances for a list of watched tokens via multicall and broadcast them to clients
    async fn fetch_balances_and_broadcast(
        ctx: Arc<BalanceCallCtx>,
        tokens: &[Address],
        sub: Arc<Subscription>,
    ) {
        let owner = ctx.owner;
        let result = Self::get_tokens_balance(ctx, tokens, None).await;

        let event = match result {
            Ok(balances) => {
                {
                    let mut balances_snapshot = sub.balances_snapshot.write().await;
                    balances_snapshot.extend(balances.clone());
                }

                // TODO better to add comparing with previous snapshot and only send updated balances
                let balances: HashMap<Address, String> = balances
                    .into_iter()
                    .map(|(address, balance)| (address, balance.to_string()))
                    .collect();
                BalanceEvent::BalanceUpdate(balances)
            }
            Err(e) => {
                tracing::error!("Failed to get balances for {}: {}", owner, e);
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

    // request balances via multicall for a list of tokens and map error
    async fn get_tokens_balance(
        ctx: Arc<BalanceCallCtx>,
        tokens: &[Address],
        block_id: Option<BlockId>,
    ) -> Result<HashMap<Address, U256>, WatcherError> {
        let owner = ctx.owner;
        let network = ctx.network;
        balances::get_balances(ctx, tokens, block_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get balances for {}: {}", owner, e);
                WatcherError::GettingBalance(owner, network, e.to_string())
            })
    }

    /**
     * Listen Deposit/Withdrawal events
     *
     * Need to sync wrap/unwrap txs to handle wrapped token balance
     */
    async fn spawn_wrapped_events_listener(&self) -> Result<(), WatcherError> {
        let ctx = Arc::clone(&self.ctx);

        let event_signatures = vec![
            WrappedToken::Deposit::SIGNATURE_HASH,
            WrappedToken::Withdrawal::SIGNATURE_HASH,
        ];
        let filter = Filter::new()
            .address(ctx.weth_address)
            .event_signature(event_signatures)
            .topic1(Topic::from(ctx.owner));

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

                WatcherError::WethEventsSubscription(ctx.network, ctx.weth_address, ctx.owner)
            })?
            .into_stream();

        let sub: Arc<Subscription> = Arc::clone(&self.sub);
        let cancel = sub.cancel_token.clone();

        let provider = Arc::new(self.ctx.provider.clone());
        let balance_call_ctx = {
            let ctx = BalanceCallCtx {
                owner: ctx.owner,
                network: ctx.network,
                provider,
                multicall3: ctx.multicall3,
            };

            Arc::new(ctx)
        };

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    },
                    Some(log) = stream.next() => {
                        let ctx = Arc::clone(&balance_call_ctx);
                        let event = match Self::parse_weth_logs_and_fetch_balance(ctx, &log).await {
                            Ok(balances) => {
                                let balance_snapshot = sub.balances_snapshot.write().await;
                                let diff = Self::update_balances_and_take_diff(balance_snapshot, balances);
                                BalanceEvent::BalanceUpdate(diff)
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

    // this function is requesting balance per token + eth balance via multicall
    // the main reason to take both of them - rpc providers usually take the same compute units for balanceOf
    // and for multicall3 (depends on chunks, but for both tokens it would be 1 chunk)
    // so we can get both balances in one request (in the future it would be great to have a list of
    // frequently used tokens to sync their balances more often
    async fn fetch_erc20_and_eth_balance(
        ctx: Arc<BalanceCallCtx>,
        token: Address,
        block_id: Option<BlockId>,
    ) -> Result<HashMap<Address, U256>, WatcherError> {
        let network = ctx.network;
        let owner = ctx.owner;
        let native_address = network.native_token_address();
        let tokens = vec![token, network.native_token_address()];

        balances::get_balances(
            ctx,
            &tokens,
            block_id,
        ).await.map_err(|err| {
            tracing::error!(
                error = %err,
                "error when get balance for tokens: {token}, {native_address}, for network: {network}"
            );
            WatcherError::GettingBalance(owner, network, err.to_string())
        })
    }

    async fn parse_weth_logs_and_fetch_balance(
        ctx: Arc<BalanceCallCtx>,
        log: &Log,
    ) -> Result<HashMap<Address, U256>, WatcherError> {
        let parsed_log = Self::parse_weth_logs(log)
            .map_err(|err| WatcherError::ParseLog(ctx.network, ctx.owner, err.to_string()))?;

        let block_id = match parsed_log {
            Some(WethEvents::Deposit(block_id)) => block_id,
            Some(WethEvents::Withdrawal(block_id)) => block_id,
            _ => None,
        };

        let weth_address = ctx.network.native_token_address();
        Self::fetch_erc20_and_eth_balance(ctx, weth_address, block_id).await
    }

    // parse WETH logs, search DEPOSIT/WITHDRAWAL events
    // if there is no DEPOSIT/WITHDRAWAL event signature in a log - return Error
    // otherwise return parsed event data
    fn parse_weth_logs(log: &Log) -> Result<Option<WethEvents>, ParseWeb3LogsError> {
        let topic0 = match log.topic0() {
            Some(topic0) => topic0,
            None => {
                tracing::error!("topic0 is None for log(WETH event): {:#?}", log);
                return Err(ParseWeb3LogsError::Topic0IsNone);
            }
        };

        let block_number = log.block_number.or_else(|| {
            tracing::error!("block_number is None for log(WETH event): {:#?}", log);
            None
        });

        let block_id = block_number.map(BlockId::from);

        if *topic0 == WrappedToken::Deposit::SIGNATURE_HASH {
            let result = log
                .log_decode::<WrappedToken::Deposit>()
                .inspect_err(|err| {
                    tracing::error!(
                        error = %err,
                        "error when decode DEPOSIT event"
                    );
                })
                .map(|log| {
                    let data = log.inner.data;
                    tracing::info!("Deposit event dst={}, wad={}", data.dst, data.wad);

                    WethEvents::Deposit(block_id)
                })
                .ok();

            return Ok(result);
        }

        if *topic0 == WrappedToken::Withdrawal::SIGNATURE_HASH {
            let result = log
                .log_decode::<WrappedToken::Withdrawal>()
                .inspect_err(|err| {
                    tracing::error!(
                        error = %err,
                        "error when decode Withdrawal event"
                    );
                })
                .map(|log| {
                    let data = log.inner.data;
                    tracing::info!("Withdrawal event: src={}, wad={}", data.src, data.wad);
                    WethEvents::Withdrawal(block_id)
                })
                .ok();

            return Ok(result);
        };

        tracing::error!("unexpected topic0(WETH event): {:#?}", topic0);
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

    // listent to erc20 transfer events for owner (in/out)
    // if an event is received - get balance for token(+ eth balance) and send it to clients
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

                WatcherError::Erc20WsSubscription(ctx.network, ctx.owner)
            })?
            .into_stream();

        let sub = Arc::clone(&self.sub);
        let cancel = sub.cancel_token.clone();

        let balance_call_ctx = {
            let ctx = BalanceCallCtx {
                owner: ctx.owner,
                network: ctx.network,
                provider: Arc::new(ctx.provider.clone()),
                multicall3: ctx.multicall3,
            };

            Arc::new(ctx)
        };

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    },
                    // TODO handle None case / reconnect
                    Some(log) = stream.next() => {
                        // TODO check address in map before request
                        let token_balance = Self::parse_transfer_event_and_get_balance(
                            Arc::clone(&balance_call_ctx),
                            &log
                        ).await;
                        let event = match token_balance {
                            Some(token_balance) => {
                                let balance_snapshot = sub.balances_snapshot.write().await;
                                let diff = Self::update_balances_and_take_diff(balance_snapshot, token_balance);
                                BalanceEvent::BalanceUpdate(diff)
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

    fn update_balances_and_take_diff(
        mut snapshot: RwLockWriteGuard<HashMap<Address, U256>>,
        new_balances: HashMap<Address, U256>,
    ) -> HashMap<Address, String> {
        let mut diff: HashMap<Address, String> = HashMap::new();
        if new_balances.is_empty() {
            tracing::warn!("balances is empty, nothing to update");
            return diff;
        }

        for (address, new_balance) in new_balances {
            let current_balance = snapshot.get_mut(&address);
            if let Some(current_balance) = current_balance {
                if *current_balance != new_balance {
                    diff.insert(address, new_balance.to_string());
                    *current_balance = new_balance;
                }
            }
        }

        diff
    }

    async fn parse_transfer_event_and_get_balance(
        ctx: Arc<BalanceCallCtx>,
        log: &Log,
    ) -> Option<HashMap<Address, U256>> {
        let Some(block_number) = log.block_number else {
            tracing::warn!("block number is undefined for network {}", ctx.network);
            return None;
        };

        let decoded_log: Log<ERC20::Transfer> = match log.log_decode() {
            Ok(log) => log,
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "error when parsing log for network {}",
                    ctx.network,
                );
                return None;
            }
        };

        Self::fetch_erc20_and_eth_balance(
            ctx,
            decoded_log.address(),
            Some(BlockId::from(block_number)),
        )
        .await
        .ok()
    }
}
