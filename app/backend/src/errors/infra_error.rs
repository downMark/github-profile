/// 基础设施层错误（DB / 外部服务），细节不暴露给客户端。
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // 骨架阶段尚未使用，T-002/T-004 启用
pub enum InfraError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("external service error: {0}")]
    External(String),

    /// Token 加解密错误（密钥配置错误 / 密文损坏），映射为 500，细节仅进日志。
    #[error("crypto error: {0}")]
    Crypto(String),
}
