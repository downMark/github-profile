use crate::errors::InfraError;

/// 顶层错误类型，聚合所有领域/基础设施错误。
///
/// 新增领域时在此追加一个 `#[from]` 变体，并在 `http_error.rs` 补充映射。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Infra(#[from] InfraError),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("GitHub Token 无效或权限不足")]
    InvalidGithubToken,

    #[error("not found")]
    NotFound,

    #[error("unauthorized")]
    Unauthorized,

    #[error("authentication service unavailable")]
    AuthUnavailable,
}
