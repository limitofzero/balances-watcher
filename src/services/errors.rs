use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ServiceError {
    #[error("Error getting balances from multicall")]
    BalancesMultiCallError(String),

    #[error("Unable to load list")]
    UnableToLoadList(String),
}

#[derive(Debug, Clone, Error)]
pub enum SubscriptionError {
    #[error("There is no session for provided key")]
    NoSession,

    #[error("Too many clients")]
    TooManyClients,

    #[error("There is no more clients")]
    ThereIsNoClients,
}
