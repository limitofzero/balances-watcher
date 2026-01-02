use crate::domain::EvmNetworks;
use alloy::primitives::Address;
use serde::Serialize;
use std::collections::HashMap;

/// Unique key to identify a subscription (owner + network)
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SubscriptionKey {
    pub owner: Address,
    pub network: EvmNetworks,
}

/// Events sent to SSE clients
#[derive(Debug, Clone, Serialize)]
pub enum BalanceEvent {
    /// Full balance snapshot (all tokens)
    FullSnapshot(HashMap<Address, String>),
    /// Single token balance update
    TokenBalanceUpdated { address: Address, balance: String },
    /// Error event
    Error { code: u16, message: String },
}
