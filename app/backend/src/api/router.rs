use axum::http::{HeaderValue, Method};
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use super::users;
use crate::errors::AppError;
use crate::state::AppState;

/// 组装应用路由。
///
/// 业务路由（T-004 ~ T-007）在此注册：
/// - POST /api/users
/// - GET  /api/users
/// - GET  /api/users/{id}
/// - POST /api/users/{id}/refresh
pub fn build(
    state: AppState,
    allowed_origin: &str,
    api_base_path: &str,
) -> Result<Router, axum::http::header::InvalidHeaderValue> {
    let origin = HeaderValue::from_str(allowed_origin)?;
    let routes = Router::new()
        .route("/health", get(health))
        .route(
            "/api/users",
            post(users::import_user).get(users::list_users),
        )
        .route("/api/users/{id}", get(users::get_user))
        .route("/api/users/{id}/refresh", post(users::refresh_user))
        .fallback(fallback);
    let app = if api_base_path.is_empty() {
        routes
    } else {
        Router::new().nest(api_base_path, routes)
    };

    Ok(app
        .layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn fallback() -> AppError {
    AppError::NotFound
}
