use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AlexandriaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("FFprobe error: {0}")]
    Ffprobe(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AlexandriaError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AlexandriaError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AlexandriaError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {}", self),
            ),
        };

        let body = Json(json!({
            "error": message,
            "kind": format!("{:?}", self),
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AlexandriaError>;
