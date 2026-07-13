use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::application::user_service;
use crate::domain::user::{GithubUser, UserList};
use crate::errors::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ImportUserBody {
    token: String,
}

#[derive(Deserialize, Default)]
pub struct ListQuery {
    page: Option<u32>,
    limit: Option<u32>,
}

pub async fn import_user(
    State(state): State<AppState>,
    Json(body): Json<ImportUserBody>,
) -> Result<(StatusCode, Json<GithubUser>), AppError> {
    let user = user_service::import(&state, &body.token).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<UserList>, AppError> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    if page == 0 || !(1..=100).contains(&limit) {
        return Err(AppError::Validation(
            "page 必须大于等于 1，limit 必须在 1 到 100 之间".into(),
        ));
    }
    Ok(Json(user_service::list(&state, page, limit).await?))
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GithubUser>, AppError> {
    Ok(Json(user_service::get(&state, parse_id(&id)?).await?))
}

pub async fn refresh_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GithubUser>, AppError> {
    Ok(Json(user_service::refresh(&state, parse_id(&id)?).await?))
}

fn parse_id(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError::Validation("用户 id 必须是有效 UUID".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_uuid_is_validation_error() {
        assert!(matches!(parse_id("nope"), Err(AppError::Validation(_))));
    }
}
