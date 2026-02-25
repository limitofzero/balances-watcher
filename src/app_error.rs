use alloy::primitives::Address;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

use crate::domain::EvmNetwork;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Provider is not defined for network {0}")]
    ProviderIsNotDefined(EvmNetwork),

    #[error("No session with network({0}) and owner({1})")]
    NoSession(EvmNetwork, Address),

    #[error("Token limit exceeded")]
    TokenLimitExceeded,

    #[error("WETH address is not defined for network: {0}")]
    WethAddressIsNotDefined(EvmNetwork),
}

#[derive(Serialize)]
pub struct ErrorBody {
    code: u16,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            AppError::ProviderIsNotDefined(_) => (StatusCode::NOT_FOUND, &self.to_string()),
            AppError::NoSession(_, _) => (StatusCode::NOT_FOUND, &self.to_string()),
            AppError::TokenLimitExceeded => (StatusCode::BAD_REQUEST, &self.to_string()),
            AppError::WethAddressIsNotDefined(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, &self.to_string())
            }
        };

        (
            status,
            Json(ErrorBody {
                code: status.as_u16(),
                message: message.clone(),
            }),
        )
            .into_response()
    }
}
