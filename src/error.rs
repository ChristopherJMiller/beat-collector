use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("External API error: {0}")]
    ExternalApi(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::Database(ref e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred")
            }
            Self::Redis(ref e) => {
                tracing::error!("Redis error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Cache error occurred")
            }
            Self::HttpRequest(ref e) => {
                tracing::error!("HTTP request error: {}", e);
                (StatusCode::BAD_GATEWAY, "External service request failed")
            }
            Self::Serialization(ref e) => {
                tracing::error!("Serialization error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Data processing error")
            }
            Self::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            Self::Authentication(ref msg) => (StatusCode::UNAUTHORIZED, msg.as_str()),
            Self::ExternalApi(ref msg) => (StatusCode::BAD_GATEWAY, msg.as_str()),
            Self::Configuration(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str()),
            Self::Internal(ref msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str())
            }
            Self::Other(ref e) => {
                tracing::error!("Unexpected error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "An unexpected error occurred")
            }
        };

        let body = Json(json!({
            "error": error_message,
            "details": self.to_string(),
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
