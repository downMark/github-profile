/// 数据库连接配置。
#[derive(Debug, Clone)]
pub enum DatabaseConnection {
    /// 本地开发等场景继续支持完整连接串。
    Url(String),
    /// ECS 从 RDS 托管 Secret 分别注入用户名和密码，避免凭证经过 CI。
    Components {
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        ssl_mode: String,
    },
}

/// 应用配置，全部来自环境变量。
#[derive(Debug, Clone)]
pub struct Config {
    /// 本地开发 HTTP 端口（Lambda 模式下不使用）
    pub port: u16,
    /// ECS/本地内部 gRPC 端口；Lambda 模式不监听。
    pub grpc_port: u16,
    /// PostgreSQL 连接信息。
    pub database: DatabaseConnection,
    /// PR 环境使用的独立 schema；本地未设置时使用数据库默认 search_path。
    pub database_schema: Option<String>,
    /// 一次性清理任务使用 `drop`；常规 API 任务不设置。
    pub database_schema_action: Option<String>,
    /// 连接池最大连接数（Lambda 单实例并发为 1，保持小值避免打满 RDS）
    pub database_max_connections: u32,
    /// Token AES-256-GCM 加密密钥（32 字节 hex，共 64 个 hex 字符）。
    pub token_encryption_key: String,
    /// 允许访问 API 的前端 Origin。默认仅用于本地开发。
    pub allowed_origin: String,
    /// ALB 为每个 PR 分配的路径前缀，如 `/pr-123`。
    pub api_base_path: String,
    pub auth_issuer: String,
    pub auth_audience: String,
    pub auth_jwks_url: String,
    pub deploy_environment: String,
    pub service_revision: String,
}

impl Config {
    /// 缺少必填配置时直接 panic（启动即失败，避免带病运行）。
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);
        let grpc_port = std::env::var("GRPC_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50051);
        let database = match std::env::var("DATABASE_URL") {
            Ok(url) => DatabaseConnection::Url(url),
            Err(_) => DatabaseConnection::Components {
                host: required_env("DB_HOST"),
                port: std::env::var("DB_PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5432),
                database: std::env::var("DB_NAME").unwrap_or_else(|_| "postgres".into()),
                username: required_env("DB_USERNAME"),
                password: required_env("DB_PASSWORD"),
                ssl_mode: std::env::var("DB_SSL_MODE").unwrap_or_else(|_| "require".into()),
            },
        };
        let database_schema = std::env::var("DB_SCHEMA").ok();
        if let Some(schema) = &database_schema {
            assert!(
                valid_schema(schema),
                "DB_SCHEMA must be prod, staging or pr_<number>"
            );
        }
        let database_schema_action = std::env::var("DB_SCHEMA_ACTION").ok();
        if database_schema_action.as_deref() == Some("drop") {
            assert!(
                database_schema.as_deref().is_some_and(valid_pr_schema),
                "DB_SCHEMA_ACTION=drop is only allowed for pr_<number>"
            );
        }
        let database_max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let token_encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
            .expect("environment variable TOKEN_ENCRYPTION_KEY must be set (32-byte hex)");
        let allowed_origin =
            std::env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".into());
        let api_base_path = std::env::var("API_BASE_PATH").unwrap_or_default();
        let auth_issuer = required_env("AUTH_ISSUER");
        let auth_audience = required_env("AUTH_AUDIENCE");
        let auth_jwks_url = required_env("AUTH_JWKS_URL");
        let deploy_environment =
            std::env::var("DEPLOY_ENVIRONMENT").unwrap_or_else(|_| "local".into());
        let service_revision =
            std::env::var("SERVICE_REVISION").unwrap_or_else(|_| "development".into());
        assert!(
            api_base_path.is_empty()
                || (api_base_path.starts_with('/') && !api_base_path.ends_with('/')),
            "API_BASE_PATH must be empty or start with / and must not end with /"
        );
        Self {
            port,
            grpc_port,
            database,
            database_schema,
            database_schema_action,
            database_max_connections,
            token_encryption_key,
            allowed_origin,
            api_base_path,
            auth_issuer,
            auth_audience,
            auth_jwks_url,
            deploy_environment,
            service_revision,
        }
    }
}

fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("environment variable {name} must be set"))
}

fn valid_pr_schema(schema: &str) -> bool {
    schema
        .strip_prefix("pr_")
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

fn valid_schema(schema: &str) -> bool {
    matches!(schema, "prod" | "staging") || valid_pr_schema(schema)
}

#[cfg(test)]
mod tests {
    use super::{valid_pr_schema, valid_schema};

    #[test]
    fn validates_pr_schema_names() {
        assert!(valid_pr_schema("pr_123"));
        assert!(!valid_pr_schema("pr_"));
        assert!(!valid_pr_schema("public"));
        assert!(!valid_pr_schema("pr_1;drop schema public"));
        assert!(valid_schema("prod"));
        assert!(valid_schema("staging"));
    }
}
