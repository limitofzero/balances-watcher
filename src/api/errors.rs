use serde::Serialize;
use axum::response::{IntoResponse, Response};
use alloy::transports::http::reqwest::StatusCode;
use axum::Json;

#[derive(Serialize)]
pub struct StreamError {
    pub code: u16,
    pub message: String,
}

impl IntoResponse for StreamError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Json(self)).into_response()
    }
}