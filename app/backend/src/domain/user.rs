use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct GithubUser {
    pub id: Uuid,
    pub github_id: i64,
    pub login: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
    pub public_repos: i32,
    pub followers: i32,
    pub following: i32,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub github_created_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct StoredGithubUser {
    pub id: Uuid,
    pub encrypted_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubProfile {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
    pub public_repos: i32,
    pub followers: i32,
    pub following: i32,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserList {
    pub items: Vec<GithubUser>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_deserializes_github_created_at() {
        let profile: GithubProfile = serde_json::from_value(serde_json::json!({
            "id": 1, "login": "octocat", "name": null, "bio": null,
            "avatar_url": null, "html_url": null, "public_repos": 8,
            "followers": 9, "following": 1, "company": null, "blog": null,
            "location": null, "created_at": "2011-01-25T18:44:36Z"
        }))
        .unwrap();
        assert_eq!(profile.created_at.to_rfc3339(), "2011-01-25T18:44:36+00:00");
    }
}
