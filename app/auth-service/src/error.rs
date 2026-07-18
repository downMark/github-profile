use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Validation(String),
    #[error("username already exists")]
    Conflict,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::Validation(message) => {
                (StatusCode::BAD_REQUEST, "invalid_argument", message.clone())
            }
            Self::Conflict => (StatusCode::CONFLICT, "username_conflict", self.to_string()),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                self.to_string(),
            ),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string()),
            Self::Config(_) | Self::Database(_) | Self::Internal => {
                tracing::error!(error = %self, "auth request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error".into(),
                )
            }
        };
        (status, Json(ErrorBody { code, message })).into_response()
    }
}
