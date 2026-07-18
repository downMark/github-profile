use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};

use crate::config::{Config, DatabaseConnection};
use crate::errors::InfraError;

/// 创建 PostgreSQL 连接池。
///
/// - Lambda 单实例并发为 1，`max_connections` 保持小值（默认 5，可用
///   `DATABASE_MAX_CONNECTIONS` 覆盖），避免高并发扩容时打满 RDS 连接数
/// - `acquire_timeout` 收紧到 5s，DB 不可达时快速失败而不是拖满请求超时
pub async fn create_pool(config: &Config) -> Result<PgPool, InfraError> {
    let mut options = connect_options(config)?;

    if let Some(schema) = &config.database_schema {
        ensure_schema(&options, schema).await?;
        options = options.options([("search_path", schema)]);
    }

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        // Lambda 冻结期间连接可能被 RDS 侧回收，定期回收空闲连接
        .idle_timeout(Duration::from_secs(300))
        .connect_with(options)
        .await?;
    Ok(pool)
}

/// 删除 PR 独立 schema。schema 名在配置层已严格限制为 `pr_<number>`。
pub async fn drop_schema(config: &Config) -> Result<(), InfraError> {
    let Some(schema) = &config.database_schema else {
        return Ok(());
    };

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(connect_options(config)?)
        .await?;

    sqlx::query(AssertSqlSafe(format!(
        "DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"
    )))
    .execute(&pool)
    .await?;
    tracing::info!(schema, "database schema dropped");
    Ok(())
}

fn connect_options(config: &Config) -> Result<PgConnectOptions, InfraError> {
    match &config.database {
        DatabaseConnection::Url(url) => Ok(PgConnectOptions::from_str(url)?),
        DatabaseConnection::Components {
            host,
            port,
            database,
            username,
            password,
            ssl_mode,
        } => Ok(PgConnectOptions::new_without_pgpass()
            .host(host)
            .port(*port)
            .database(database)
            .username(username)
            .password(password)
            .ssl_mode(ssl_mode.parse()?)),
    }
}

async fn ensure_schema(options: &PgConnectOptions, schema: &str) -> Result<(), InfraError> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options.clone())
        .await?;

    sqlx::query(AssertSqlSafe(format!(
        "CREATE SCHEMA IF NOT EXISTS \"{schema}\""
    )))
    .execute(&pool)
    .await?;
    Ok(())
}

/// 执行 `app/backend/migrations/` 下的全部待执行 migration（编译期嵌入二进制）。
pub async fn run_migrations(pool: &PgPool) -> Result<(), InfraError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| InfraError::Database(sqlx::Error::Migrate(Box::new(e))))?;
    Ok(())
}
