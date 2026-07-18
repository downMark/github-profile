use uuid::Uuid;

use crate::domain::user::{GithubUser, UserList};
use crate::errors::AppError;
use crate::infrastructure::user_repository;
use crate::state::AppState;

pub async fn import(state: &AppState, owner: Uuid, token: &str) -> Result<GithubUser, AppError> {
    let token = token.trim();
    if token.is_empty() || token.len() > 512 {
        return Err(AppError::Validation(
            "token 不能为空且不能超过 512 个字符".into(),
        ));
    }
    let profile = state.github.current_user(token).await?;
    let encrypted = state.cipher.encrypt(token)?;
    Ok(user_repository::upsert(&state.db, owner, &profile, &encrypted).await?)
}

pub async fn list(
    state: &AppState,
    owner: Uuid,
    page: u32,
    limit: u32,
) -> Result<UserList, AppError> {
    Ok(user_repository::list(&state.db, owner, page, limit).await?)
}

pub async fn get(state: &AppState, owner: Uuid, id: Uuid) -> Result<GithubUser, AppError> {
    user_repository::find(&state.db, owner, id)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn refresh(state: &AppState, owner: Uuid, id: Uuid) -> Result<GithubUser, AppError> {
    let stored = user_repository::find_stored(&state.db, owner, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let token = state.cipher.decrypt(&stored.encrypted_token)?;
    let profile = state.github.current_user(&token).await?;
    Ok(user_repository::update_profile(&state.db, stored.id, owner, &profile).await?)
}
