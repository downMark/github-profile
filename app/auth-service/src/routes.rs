use axum::extract::State;
use axum::http::header::{AUTHORIZATION, COOKIE, ORIGIN, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::config::Config;
use crate::error::AppError;
use crate::model::Account;
use crate::security::JwtKeys;
use crate::{repository, security};

const REFRESH_COOKIE: &str = "refresh_token";

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt: JwtKeys,
    pub config: Config,
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: i64,
    account: Account,
}

pub fn router(state: AppState) -> Result<Router, AppError> {
    let origin = HeaderValue::from_str(&state.config.allowed_origin)
        .map_err(|_| AppError::Config("invalid ALLOWED_ORIGIN".into()))?;
    let routes = Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .route(
            "/health/auth",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .route("/.well-known/jwks.json", get(jwks))
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me));
    let app = if state.config.api_base_path.is_empty() {
        routes
    } else {
        Router::new().nest(&state.config.api_base_path, routes)
    };
    Ok(app
        .layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_credentials(true)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE, AUTHORIZATION]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> Result<Response, AppError> {
    let (username, normalized) = validate_credentials(&body)?;
    let hash = security::hash_password(&body.password)?;
    let account = repository::create_account(&state.pool, &username, &normalized, &hash).await?;
    session_response(&state, account, StatusCode::CREATED).await
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<Credentials>,
) -> Result<Response, AppError> {
    let normalized = body.username.trim().to_lowercase();
    let credential = repository::find_credential(&state.pool, &normalized)
        .await?
        .ok_or(AppError::InvalidCredentials)?;
    if credential.status != "active"
        || !security::verify_password(&body.password, &credential.password_hash)
    {
        return Err(AppError::InvalidCredentials);
    }
    session_response(&state, credential.into(), StatusCode::OK).await
}

async fn refresh(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    require_origin(&headers, &state.config.allowed_origin)?;
    let old = cookie(&headers, REFRESH_COOKIE).ok_or(AppError::Unauthorized)?;
    let new = security::new_refresh_token();
    let account_id = repository::rotate_session(
        &state.pool,
        &security::token_hash(&old),
        &security::token_hash(&new),
        state.config.refresh_ttl_seconds,
    )
    .await?;
    let account = repository::find_account(&state.pool, account_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    auth_response(&state, account, new, StatusCode::OK).await
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    require_origin(&headers, &state.config.allowed_origin)?;
    if let Some(token) = cookie(&headers, REFRESH_COOKIE) {
        repository::revoke_session(&state.pool, &security::token_hash(&token)).await?;
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, clear_cookie(&state.config)?);
    Ok(response)
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Account>, AppError> {
    let account_id = authenticated_account(&state.jwt, &headers)?;
    Ok(Json(
        repository::find_account(&state.pool, account_id)
            .await?
            .ok_or(AppError::Unauthorized)?,
    ))
}

async fn jwks(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(state.jwt.jwks())
}

async fn discovery(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"issuer":state.jwt.issuer(),"jwks_uri":format!("{}/.well-known/jwks.json",state.jwt.issuer()),"id_token_signing_alg_values_supported":["RS256"]}),
    )
}

async fn session_response(
    state: &AppState,
    account: Account,
    status: StatusCode,
) -> Result<Response, AppError> {
    let refresh = security::new_refresh_token();
    repository::create_session(
        &state.pool,
        account.id,
        Uuid::new_v4(),
        &security::token_hash(&refresh),
        state.config.refresh_ttl_seconds,
    )
    .await?;
    auth_response(state, account, refresh, status).await
}

async fn auth_response(
    state: &AppState,
    account: Account,
    refresh: String,
    status: StatusCode,
) -> Result<Response, AppError> {
    let body = AuthResponse {
        access_token: state.jwt.issue(account.id).await?,
        token_type: "Bearer",
        expires_in: state.jwt.ttl(),
        account,
    };
    let mut response = (status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, refresh_cookie(&refresh, &state.config)?);
    Ok(response)
}

fn validate_credentials(body: &Credentials) -> Result<(String, String), AppError> {
    let username = body.username.trim();
    if !(3..=32).contains(&username.len())
        || !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(AppError::Validation(
            "username must be 3-32 characters using letters, numbers, dot, underscore or hyphen"
                .into(),
        ));
    }
    if !(7..=128).contains(&body.password.chars().count()) {
        return Err(AppError::Validation(
            "password must be 7-128 characters".into(),
        ));
    }
    Ok((username.into(), username.to_lowercase()))
}

fn authenticated_account(jwt: &JwtKeys, headers: &HeaderMap) -> Result<Uuid, AppError> {
    let value = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;
    Uuid::parse_str(&jwt.validate(value)?.sub).map_err(|_| AppError::Unauthorized)
}

fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then(|| value.to_string())
        })
}

fn require_origin(headers: &HeaderMap, allowed_origin: &str) -> Result<(), AppError> {
    let origin = headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    if origin != allowed_origin {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

fn refresh_cookie(token: &str, config: &Config) -> Result<HeaderValue, AppError> {
    let secure = if config.cookie_secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{REFRESH_COOKIE}={token}; HttpOnly; SameSite={}; Path={}; Max-Age={}{}",
        config.cookie_same_site, config.cookie_path, config.refresh_ttl_seconds, secure
    ))
    .map_err(|_| AppError::Internal)
}

fn clear_cookie(config: &Config) -> Result<HeaderValue, AppError> {
    let secure = if config.cookie_secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{REFRESH_COOKIE}=; HttpOnly; SameSite={}; Path={}; Max-Age=0{secure}",
        config.cookie_same_site, config.cookie_path
    ))
    .map_err(|_| AppError::Internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_validation() {
        assert!(
            validate_credentials(&Credentials {
                username: "user.one".into(),
                password: "1234567".into()
            })
            .is_ok()
        );
        assert!(
            validate_credentials(&Credentials {
                username: "user.one".into(),
                password: "123456".into()
            })
            .is_err()
        );
        assert!(
            validate_credentials(&Credentials {
                username: "x".into(),
                password: "long-password".into()
            })
            .is_err()
        );
    }
}
