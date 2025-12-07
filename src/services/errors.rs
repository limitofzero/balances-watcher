use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ServiceError {
    #[error("Error getting balances from multicall")]
    BalancesMultiCallError(String),
}