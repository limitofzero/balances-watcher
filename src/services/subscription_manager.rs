use crate::domain::{BalanceEvent, SubscriptionKey};
use crate::services::errors::SubscriptionError;
use alloy::primitives::Address;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

struct SubWithCounter {
    pub clients: u32,
    pub subscription: Arc<Subscription>,
}

pub struct Subscription {
    pub sender: broadcast::Sender<BalanceEvent>,
    pub balances_snapshot: RwLock<HashMap<Address, String>>,
    pub cancel_token: tokio_util::sync::CancellationToken,
}

pub struct SubscriptionManager {
    subscriptions: RwLock<HashMap<SubscriptionKey, SubWithCounter>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
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
                .ok_or_else(|| SubscriptionError::TooManyClients)?;
            let receiver = existing.subscription.sender.subscribe();
            return Ok((receiver, false, Arc::clone(&existing.subscription)));
        }

        let (sender, receiver) = broadcast::channel::<BalanceEvent>(256);
        let subscription = Arc::new(Subscription {
            sender,
            balances_snapshot: RwLock::new(HashMap::new()),
            cancel_token: tokio_util::sync::CancellationToken::new(),
        });
        let sub_with_counter = SubWithCounter {
            clients: 1,
            subscription: Arc::clone(&subscription),
        };

        subs.insert(key, sub_with_counter);
        Ok((receiver, true, Arc::clone(&subscription)))
    }

    // true - if it was the last client
    pub async fn unsubscribe(&self, key: &SubscriptionKey) -> Result<bool, SubscriptionError> {
        let mut subs = self.subscriptions.write().await;

        if let Some(existing) = subs.get_mut(&key) {
            existing.clients = existing
                .clients
                .checked_sub(1)
                .ok_or_else(|| SubscriptionError::ThereIsNoClients)?;
            if existing.clients == 0 {
                existing.subscription.cancel_token.cancel();
                subs.remove(&key);
                return Ok(true);
            }

            return Ok(false);
        }

        Err(SubscriptionError::ThereIsNoClients)
    }
}
