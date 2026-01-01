use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ServiceError {
    #[error("Error getting balances from multicall")]
    BalancesMultiCallError(String),
}

#[derive(Debug, Clone, Error)]
pub enum SubscriptionError {
    #[error("Too many clients")]
    TooManyClients,

    #[error("There is no more clients")]
    ThereIsNoClients,
}