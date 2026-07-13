use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::user::{GithubProfile, GithubUser, StoredGithubUser, UserList};
use crate::errors::InfraError;

pub async fn upsert(
    pool: &PgPool,
    profile: &GithubProfile,
    encrypted_token: &str,
) -> Result<GithubUser, InfraError> {
    sqlx::query_as::<_, GithubUser>(
        "INSERT INTO github_users (github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, encrypted_token, github_created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
         ON CONFLICT (github_id) DO UPDATE SET login=EXCLUDED.login, name=EXCLUDED.name, bio=EXCLUDED.bio, avatar_url=EXCLUDED.avatar_url, html_url=EXCLUDED.html_url, public_repos=EXCLUDED.public_repos, followers=EXCLUDED.followers, following=EXCLUDED.following, company=EXCLUDED.company, blog=EXCLUDED.blog, location=EXCLUDED.location, encrypted_token=EXCLUDED.encrypted_token, github_created_at=EXCLUDED.github_created_at, updated_at=NOW()
         RETURNING id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at",
    )
        .bind(profile.id)
        .bind(&profile.login)
        .bind(&profile.name)
        .bind(&profile.bio)
        .bind(&profile.avatar_url)
        .bind(&profile.html_url)
        .bind(profile.public_repos)
        .bind(profile.followers)
        .bind(profile.following)
        .bind(&profile.company)
        .bind(&profile.blog)
        .bind(&profile.location)
        .bind(encrypted_token)
        .bind(profile.created_at)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
}

pub async fn update_profile(
    pool: &PgPool,
    id: Uuid,
    profile: &GithubProfile,
) -> Result<GithubUser, InfraError> {
    sqlx::query_as::<_, GithubUser>(
        "UPDATE github_users SET github_id=$2, login=$3, name=$4, bio=$5, avatar_url=$6, html_url=$7, public_repos=$8, followers=$9, following=$10, company=$11, blog=$12, location=$13, github_created_at=$14, updated_at=NOW() WHERE id=$1 RETURNING id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at",
    )
        .bind(id)
        .bind(profile.id)
        .bind(&profile.login)
        .bind(&profile.name)
        .bind(&profile.bio)
        .bind(&profile.avatar_url)
        .bind(&profile.html_url)
        .bind(profile.public_repos)
        .bind(profile.followers)
        .bind(profile.following)
        .bind(&profile.company)
        .bind(&profile.blog)
        .bind(&profile.location)
        .bind(profile.created_at)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
}

pub async fn list(pool: &PgPool, page: u32, limit: u32) -> Result<UserList, InfraError> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM github_users")
        .fetch_one(pool)
        .await?;
    let items = sqlx::query_as::<_, GithubUser>(
        "SELECT id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at FROM github_users ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
    )
        .bind(i64::from(limit))
        .bind((i64::from(page) - 1) * i64::from(limit))
        .fetch_all(pool)
        .await?;
    Ok(UserList {
        items,
        total,
        page,
        limit,
    })
}

pub async fn find(pool: &PgPool, id: Uuid) -> Result<Option<GithubUser>, InfraError> {
    sqlx::query_as::<_, GithubUser>(
        "SELECT id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at FROM github_users WHERE id=$1",
    )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn find_stored(pool: &PgPool, id: Uuid) -> Result<Option<StoredGithubUser>, InfraError> {
    sqlx::query_as("SELECT id, encrypted_token FROM github_users WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}
