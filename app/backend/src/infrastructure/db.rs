use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::config::Config;
use crate::errors::InfraError;

/// 创建 PostgreSQL 连接池。
///
/// - Lambda 单实例并发为 1，`max_connections` 保持小值（默认 5，可用
///   `DATABASE_MAX_CONNECTIONS` 覆盖），避免高并发扩容时打满 RDS 连接数
/// - `acquire_timeout` 收紧到 5s，DB 不可达时快速失败而不是拖满 API Gateway 超时
pub async fn create_pool(config: &Config) -> Result<PgPool, InfraError> {
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        // Lambda 冻结期间连接可能被 RDS 侧回收，定期回收空闲连接
        .idle_timeout(Duration::from_secs(300))
        .connect(&config.database_url)
        .await?;
    Ok(pool)
}

/// 执行 `app/backend/migrations/` 下的全部待执行 migration（编译期嵌入二进制）。
pub async fn run_migrations(pool: &PgPool) -> Result<(), InfraError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| InfraError::Database(sqlx::Error::Migrate(Box::new(e))))?;
    Ok(())
}
