use sqlx::PgPool;

use crate::infrastructure::crypto::TokenCipher;
use crate::infrastructure::github::GithubClient;

/// 全局共享状态，通过 axum `State` 注入到 handler。
///
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL 连接池（sqlx::PgPool 内部是 Arc，Clone 开销极小）
    pub db: PgPool,
    pub cipher: TokenCipher,
    pub github: GithubClient,
}

impl AppState {
    pub fn new(db: PgPool, cipher: TokenCipher, github: GithubClient) -> Self {
        Self { db, cipher, github }
    }
}
