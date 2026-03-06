use crate::domain::EvmNetwork;
use alloy::primitives::Address;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;

/// Unique key to identify a subscription (owner + network)
#[derive(Clone, Debug, Eq, Hash, PartialEq, Copy)]
pub struct SubscriptionKey {
    pub owner: Address,
    pub network: EvmNetwork,
}

impl Display for SubscriptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.owner, self.network)
    }
}

/// Events sent to SSE clients
#[derive(Debug, Clone, Serialize)]
pub enum BalanceEvent {
    /// Full balance snapshot (all tokens)
    BalanceUpdate(HashMap<Address, String>),
    /// Error event
    Error { code: u16, message: String },
}
