use crate::config::constants::BROADCAST_CHANNEL_CAPACITY;
use crate::domain::{BalanceEvent, SubscriptionKey};
use crate::services::errors::SubscriptionError;
use alloy::primitives::Address;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};

struct SubWithCounter {
    pub clients: u32,
    pub subscription: Arc<Subscription>,
    pub idle_since: Option<Instant>,
}

pub struct Subscription {
    pub sender: broadcast::Sender<BalanceEvent>,
    pub balances_snapshot: RwLock<HashMap<Address, String>>,
    pub cancel_token: tokio_util::sync::CancellationToken,
    pub tokens: RwLock<HashSet<Address>>,
}

pub struct SubscriptionManager {
    subscriptions: RwLock<HashMap<SubscriptionKey, SubWithCounter>>,
}

const SESSION_TTL: Duration = Duration::from_secs(60);

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_or_update(
        &self,
        key: SubscriptionKey,
        tokens: HashSet<Address>,
    ) -> Arc<Subscription> {
        let mut subs = self.subscriptions.write().await;
        if let Some(existing) = subs.get_mut(&key) {
            let mut watchet_tokens = existing.subscription.tokens.write().await;
            watchet_tokens.extend(tokens);

            return Arc::clone(&existing.subscription);
        }

        let (sender, _) = broadcast::channel::<BalanceEvent>(BROADCAST_CHANNEL_CAPACITY);

        let tokens_len = tokens.len();
        let subscription = Arc::new(Subscription {
            sender,
            balances_snapshot: RwLock::new(HashMap::new()),
            cancel_token: tokio_util::sync::CancellationToken::new(),
            tokens: RwLock::new(tokens),
        });

        let sub_with_counter = SubWithCounter {
            clients: 0,
            subscription: Arc::clone(&subscription),
            idle_since: Some(Instant::now()),
        };

        subs.insert(key, sub_with_counter);

        tracing::info!("session was created with token len: {}", tokens_len);

        Arc::clone(&subscription)
    }

    pub async fn get_subscription(&self, key: SubscriptionKey) -> Option<Arc<Subscription>> {
        let subs = self.subscriptions.read().await;
        subs.get(&key).map(|sub| Arc::clone(&sub.subscription))
    }

    pub async fn subscribe(
        &self,
        key: SubscriptionKey,
    ) -> Result<(broadcast::Receiver<BalanceEvent>, bool, Arc<Subscription>), SubscriptionError>
    {
        let mut subs = self.subscriptions.write().await;

        if let Some(existing) = subs.get_mut(&key) {
            existing.clients = existing
                .clients
                .checked_add(1)
                .ok_or(SubscriptionError::TooManyClients)?;
            existing.idle_since = None;
            let receiver = existing.subscription.sender.subscribe();
            let is_first = existing.clients == 1;
            return Ok((receiver, is_first, Arc::clone(&existing.subscription)));
        }

        Err(SubscriptionError::NoSession)
    }

    // true - if it was the last client
    pub async fn unsubscribe(&self, key: &SubscriptionKey) -> Result<bool, SubscriptionError> {
        let mut subs = self.subscriptions.write().await;

        if let Some(existing) = subs.get_mut(key) {
            existing.clients = existing
                .clients
                .checked_sub(1)
                .ok_or(SubscriptionError::ThereIsNoClients)?;
            if existing.clients == 0 {
                existing.idle_since = Some(Instant::now());
                return Ok(true);
            }

            return Ok(false);
        }

        Err(SubscriptionError::ThereIsNoClients)
    }

    pub fn spawn_cleanup(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(SESSION_TTL);
            loop {
                interval.tick().await;
                self.cleanup_subs().await;
            }
        });
    }

    async fn cleanup_subs(&self) {
        let mut subs = self.subscriptions.write().await;

        let now = Instant::now();

        subs.retain(|key, sub| {
            let should_remove = if sub.clients == 0 {
                match sub.idle_since {
                    Some(idle_since) => now.duration_since(idle_since) > SESSION_TTL,
                    None => false,
                }
            } else {
                false
            };

            if should_remove {
                sub.subscription.cancel_token.cancel();
                tracing::info!(?key, "cleanup session");
            }

            !should_remove
        })
    }
}
