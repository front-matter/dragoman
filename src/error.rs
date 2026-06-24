use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("PID not found: {0}")]
    NotFound(String),
    #[error("Failed to fetch metadata: {0}")]
    FetchError(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::UnsupportedFormat(_) => (StatusCode::NOT_ACCEPTABLE, self.to_string()),
            AppError::FetchError(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, body).into_response()
    }
}
