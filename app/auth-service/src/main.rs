mod config;
mod error;
mod model;
mod repository;
mod routes;
mod security;

use std::net::{Ipv4Addr, SocketAddr};

use config::Config;
use error::AppError;
use routes::AppState;
use security::JwtKeys;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();
    let config = Config::from_env()?;
    let mut options = PgConnectOptions::from_str(&config.database_url)?;
    if let Some(schema) = &config.database_schema {
        let bootstrap = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(options.clone())
            .await?;
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema}\""))
            .execute(&bootstrap)
            .await?;
        bootstrap.close().await;
        options = options.options([("search_path", schema)]);
    }
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect_with(options)
        .await?;
    repository::migrate(&pool).await?;
    let jwt = JwtKeys::load(
        config.jwt_issuer.clone(),
        config.jwt_audience.clone(),
        config.access_ttl_seconds,
        config.kms_signing_key_id.as_deref(),
    )
    .await?;
    let app = routes::router(AppState {
        pool,
        jwt,
        config: config.clone(),
    })?;
    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));
    tracing::info!(%address, "auth service listening");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|_| AppError::Internal)?;
    axum::serve(listener, app)
        .await
        .map_err(|_| AppError::Internal)
}
