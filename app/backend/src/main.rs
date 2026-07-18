mod api;
mod application;
mod config;
mod domain;
mod errors;
mod grpc;
mod infrastructure;
mod state;

use config::Config;
use infrastructure::auth::AuthVerifier;
use infrastructure::crypto::TokenCipher;
use infrastructure::github::GithubClient;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        // Lambda 环境下 CloudWatch 已带时间戳
        .without_time()
        .init();

    let config = Config::from_env();

    if config.database_schema_action.as_deref() == Some("drop") {
        infrastructure::db::drop_schema(&config).await?;
        return Ok(());
    }

    let pool = infrastructure::db::create_pool(&config).await?;
    infrastructure::db::run_migrations(&pool).await?;
    tracing::info!("database connected, migrations up to date");

    let cipher = TokenCipher::from_hex_key(&config.token_encryption_key)?;
    let auth = AuthVerifier::new(
        &config.auth_issuer,
        &config.auth_audience,
        &config.auth_jwks_url,
    );
    let state = AppState::new(
        pool,
        cipher,
        GithubClient::new(),
        auth,
        config.deploy_environment.clone(),
        config.service_revision.clone(),
    );
    let app = api::router::build(state.clone(), &config.allowed_origin, &config.api_base_path)?;

    // 在 Lambda 运行时中由 API Gateway 事件驱动；本地开发直接起 HTTP 服务
    if std::env::var("AWS_LAMBDA_RUNTIME_API").is_ok() {
        lambda_http::run(app).await
    } else {
        let addr = format!("0.0.0.0:{}", config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        tracing::info!("listening on http://{addr}");
        let grpc_addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
        let http_server = axum::serve(listener, app);
        let grpc_server = grpc::profile::serve(grpc_addr, state);
        tokio::select! {
            result = http_server => result?,
            result = grpc_server => result?,
        }
        Ok(())
    }
}
