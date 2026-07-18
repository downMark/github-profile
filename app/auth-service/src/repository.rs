use chrono::{Duration, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::AppError;
use crate::model::{Account, AccountCredential, RefreshSession};

pub async fn migrate(pool: &PgPool) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS auth_schema_migrations (version BIGINT PRIMARY KEY, applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW())").execute(&mut *tx).await?;
    let applied: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_schema_migrations WHERE version=1)")
            .fetch_one(&mut *tx)
            .await?;
    if !applied {
        sqlx::raw_sql(include_str!("../migrations/000001_create_auth_tables.sql"))
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO auth_schema_migrations(version) VALUES(1)")
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn create_account(
    pool: &PgPool,
    username: &str,
    normalized: &str,
    password_hash: &str,
) -> Result<Account, AppError> {
    let mut tx = pool.begin().await?;
    let id = Uuid::new_v4();
    let account = sqlx::query_as::<_, Account>("INSERT INTO accounts(id,username,username_normalized) VALUES($1,$2,$3) RETURNING id,username,status,created_at,updated_at")
        .bind(id).bind(username).bind(normalized).fetch_one(&mut *tx).await.map_err(map_unique)?;
    sqlx::query("INSERT INTO password_credentials(account_id,password_hash) VALUES($1,$2)")
        .bind(id)
        .bind(password_hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(account)
}

pub async fn find_credential(
    pool: &PgPool,
    normalized: &str,
) -> Result<Option<AccountCredential>, AppError> {
    Ok(sqlx::query_as("SELECT a.id,a.username,a.status,a.created_at,a.updated_at,p.password_hash FROM accounts a JOIN password_credentials p ON p.account_id=a.id WHERE a.username_normalized=$1")
        .bind(normalized).fetch_optional(pool).await?)
}

pub async fn find_account(pool: &PgPool, id: Uuid) -> Result<Option<Account>, AppError> {
    Ok(sqlx::query_as("SELECT id,username,status,created_at,updated_at FROM accounts WHERE id=$1 AND status='active'").bind(id).fetch_optional(pool).await?)
}

pub async fn create_session(
    pool: &PgPool,
    account_id: Uuid,
    family_id: Uuid,
    token_hash: &str,
    ttl_seconds: i64,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO refresh_sessions(id,family_id,account_id,token_hash,expires_at) VALUES($1,$2,$3,$4,$5)")
        .bind(Uuid::new_v4()).bind(family_id).bind(account_id).bind(token_hash).bind(Utc::now()+Duration::seconds(ttl_seconds)).execute(pool).await?;
    Ok(())
}

pub async fn rotate_session(
    pool: &PgPool,
    old_hash: &str,
    new_hash: &str,
    ttl_seconds: i64,
) -> Result<Uuid, AppError> {
    let mut tx = pool.begin().await?;
    let old: Option<RefreshSession> = sqlx::query_as("SELECT id,family_id,account_id,expires_at,revoked_at FROM refresh_sessions WHERE token_hash=$1 FOR UPDATE").bind(old_hash).fetch_optional(&mut *tx).await?;
    let Some(old) = old else {
        return Err(AppError::Unauthorized);
    };
    if old.revoked_at.is_some() {
        sqlx::query(
            "UPDATE refresh_sessions SET revoked_at=COALESCE(revoked_at,NOW()) WHERE family_id=$1",
        )
        .bind(old.family_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Err(AppError::Unauthorized);
    }
    if old.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }
    let new_id = Uuid::new_v4();
    sqlx::query("INSERT INTO refresh_sessions(id,family_id,account_id,token_hash,expires_at) VALUES($1,$2,$3,$4,$5)")
        .bind(new_id).bind(old.family_id).bind(old.account_id).bind(new_hash).bind(Utc::now()+Duration::seconds(ttl_seconds)).execute(&mut *tx).await?;
    sqlx::query("UPDATE refresh_sessions SET revoked_at=NOW(),replaced_by=$2 WHERE id=$1")
        .bind(old.id)
        .bind(new_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(old.account_id)
}

pub async fn revoke_session(pool: &PgPool, hash: &str) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE refresh_sessions SET revoked_at=COALESCE(revoked_at,NOW()) WHERE token_hash=$1",
    )
    .bind(hash)
    .execute(pool)
    .await?;
    Ok(())
}

fn map_unique(error: sqlx::Error) -> AppError {
    if matches!(&error, sqlx::Error::Database(db) if db.code().as_deref()==Some("23505")) {
        AppError::Conflict
    } else {
        AppError::Database(error)
    }
}

#[allow(dead_code)]
async fn _use_transaction(_: &mut Transaction<'_, Postgres>) {}
