use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub status: String,
    #[serde(skip_serializing)]
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing)]
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct AccountCredential {
    pub id: Uuid,
    pub username: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub password_hash: String,
}

impl From<AccountCredential> for Account {
    fn from(value: AccountCredential) -> Self {
        Self {
            id: value.id,
            username: value.username,
            status: value.status,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct RefreshSession {
    pub id: Uuid,
    pub family_id: Uuid,
    pub account_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
