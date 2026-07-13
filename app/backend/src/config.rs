/// 应用配置，全部来自环境变量。
///
#[derive(Debug, Clone)]
pub struct Config {
    /// 本地开发 HTTP 端口（Lambda 模式下不使用）
    pub port: u16,
    /// PostgreSQL 连接串（如 postgres://user:pass@host:5432/db）
    pub database_url: String,
    /// 连接池最大连接数（Lambda 单实例并发为 1，保持小值避免打满 RDS）
    pub database_max_connections: u32,
    /// Token AES-256-GCM 加密密钥（32 字节 hex，共 64 个 hex 字符）。
    pub token_encryption_key: String,
    /// 允许访问 API 的前端 Origin。默认仅用于本地开发。
    pub allowed_origin: String,
}

impl Config {
    /// 缺少必填配置时直接 panic（启动即失败，避免带病运行）。
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);
        let database_url =
            std::env::var("DATABASE_URL").expect("environment variable DATABASE_URL must be set");
        let database_max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let token_encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
            .expect("environment variable TOKEN_ENCRYPTION_KEY must be set (32-byte hex)");
        let allowed_origin =
            std::env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".into());
        Self {
            port,
            database_url,
            database_max_connections,
            token_encryption_key,
            allowed_origin,
        }
    }
}
