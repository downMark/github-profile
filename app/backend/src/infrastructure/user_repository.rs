use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::user::{GithubProfile, GithubUser, StoredGithubUser, UserList};
use crate::errors::InfraError;

pub async fn upsert(
    pool: &PgPool,
    owner_account_id: Uuid,
    profile: &GithubProfile,
    encrypted_token: &str,
) -> Result<GithubUser, InfraError> {
    sqlx::query_as::<_, GithubUser>(
        "INSERT INTO github_users (owner_account_id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, encrypted_token, github_created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
         ON CONFLICT (owner_account_id, github_id) DO UPDATE SET login=EXCLUDED.login, name=EXCLUDED.name, bio=EXCLUDED.bio, avatar_url=EXCLUDED.avatar_url, html_url=EXCLUDED.html_url, public_repos=EXCLUDED.public_repos, followers=EXCLUDED.followers, following=EXCLUDED.following, company=EXCLUDED.company, blog=EXCLUDED.blog, location=EXCLUDED.location, encrypted_token=EXCLUDED.encrypted_token, github_created_at=EXCLUDED.github_created_at, updated_at=NOW()
         RETURNING id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at",
    )
        .bind(owner_account_id)
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
    owner_account_id: Uuid,
    profile: &GithubProfile,
) -> Result<GithubUser, InfraError> {
    sqlx::query_as::<_, GithubUser>(
        "UPDATE github_users SET github_id=$3, login=$4, name=$5, bio=$6, avatar_url=$7, html_url=$8, public_repos=$9, followers=$10, following=$11, company=$12, blog=$13, location=$14, github_created_at=$15, updated_at=NOW() WHERE id=$1 AND owner_account_id=$2 RETURNING id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at",
    )
        .bind(id)
        .bind(owner_account_id)
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

pub async fn list(
    pool: &PgPool,
    owner_account_id: Uuid,
    page: u32,
    limit: u32,
) -> Result<UserList, InfraError> {
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM github_users WHERE owner_account_id=$1")
            .bind(owner_account_id)
            .fetch_one(pool)
            .await?;
    let items = sqlx::query_as::<_, GithubUser>(
        "SELECT id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at FROM github_users WHERE owner_account_id=$1 ORDER BY updated_at DESC, id DESC LIMIT $2 OFFSET $3",
    )
        .bind(owner_account_id)
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

pub async fn find(
    pool: &PgPool,
    owner_account_id: Uuid,
    id: Uuid,
) -> Result<Option<GithubUser>, InfraError> {
    sqlx::query_as::<_, GithubUser>(
        "SELECT id, github_id, login, name, bio, avatar_url, html_url, public_repos, followers, following, company, blog, location, github_created_at, created_at, updated_at FROM github_users WHERE id=$1 AND owner_account_id=$2",
    )
        .bind(id)
        .bind(owner_account_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn find_stored(
    pool: &PgPool,
    owner_account_id: Uuid,
    id: Uuid,
) -> Result<Option<StoredGithubUser>, InfraError> {
    sqlx::query_as(
        "SELECT id, encrypted_token FROM github_users WHERE id=$1 AND owner_account_id=$2",
    )
    .bind(id)
    .bind(owner_account_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}
