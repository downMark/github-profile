use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::errors::AppError;

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            AppError::InvalidGithubToken => (StatusCode::BAD_REQUEST, "INVALID_GITHUB_TOKEN"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Infra(err) => {
                tracing::error!(error = %err, "infrastructure error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR")
            }
        };

        let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
            // 不向客户端暴露基础设施细节
            "internal server error".to_string()
        } else {
            self.to_string()
        };

        (status, Json(ErrorBody { code, message })).into_response()
    }
}
