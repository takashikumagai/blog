use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

pub enum AppError {
    Unauthorized,
    SyncNotConfigured,
    NotFound,
    Conflict,
    InvalidInput,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "missing or invalid bearer token"),
            Self::SyncNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "blog sync is not configured",
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "post not found"),
            Self::Conflict => (StatusCode::CONFLICT, "post already exists"),
            Self::InvalidInput => (
                StatusCode::BAD_REQUEST,
                "invalid post ID, title, or content",
            ),
            Self::Database(error) => {
                tracing::error!(error = %error, "database request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "database request failed")
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
