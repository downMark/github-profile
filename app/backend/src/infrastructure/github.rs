use reqwest::{Client, StatusCode};

use crate::domain::user::GithubProfile;
use crate::errors::{AppError, InfraError};

#[derive(Clone, Debug)]
pub struct GithubClient {
    client: Client,
    base_url: String,
}

impl GithubClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("github-profile-manager")
                .build()
                .expect("valid GitHub HTTP client"),
            base_url: "https://api.github.com".into(),
        }
    }

    pub async fn current_user(&self, token: &str) -> Result<GithubProfile, AppError> {
        let response = self
            .client
            .get(format!("{}/user", self.base_url))
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| InfraError::External(e.to_string()))?;

        if matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            return Err(AppError::InvalidGithubToken);
        }
        if !response.status().is_success() {
            return Err(InfraError::External(format!(
                "GitHub API returned status {}",
                response.status()
            ))
            .into());
        }
        response
            .json()
            .await
            .map_err(|e| InfraError::External(e.to_string()).into())
    }
}
