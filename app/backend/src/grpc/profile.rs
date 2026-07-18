use std::net::SocketAddr;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::profile_v1::profile_service_server::{ProfileService, ProfileServiceServer};
use super::profile_v1::{
    AuthorizeGithubUserRequest, AuthorizeGithubUserResponse, GetUserRequest, GetUserResponse,
    UserSummary,
};
use crate::domain::user::GithubUser;
use crate::infrastructure::user_repository;
use crate::state::AppState;

pub struct ProfileGrpcService {
    state: AppState,
}

impl ProfileGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    async fn authorize<T>(&self, request: &Request<T>) -> Result<uuid::Uuid, Status> {
        let value = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Status::unauthenticated("missing bearer token"))?;
        self.state
            .auth
            .authenticate(value)
            .await
            .map(|v| v.account_id)
            .map_err(|error| match error {
                crate::errors::AppError::AuthUnavailable => {
                    Status::unavailable("authentication service unavailable")
                }
                _ => Status::unauthenticated("invalid bearer token"),
            })
    }

    async fn find_user(&self, owner: Uuid, id: &str) -> Result<GithubUser, Status> {
        let id = parse_user_id(id)?;
        user_repository::find(&self.state.db, owner, id)
            .await
            .map_err(|error| {
                tracing::error!(%error, "profile gRPC database error");
                Status::internal("internal server error")
            })?
            .ok_or_else(|| Status::permission_denied("github user is not owned by this account"))
    }
}

#[tonic::async_trait]
impl ProfileService for ProfileGrpcService {
    async fn authorize_github_user(
        &self,
        request: Request<AuthorizeGithubUserRequest>,
    ) -> Result<Response<AuthorizeGithubUserResponse>, Status> {
        let owner = self.authorize(&request).await?;
        let user = self.find_user(owner, &request.into_inner().user_id).await?;
        Ok(Response::new(AuthorizeGithubUserResponse {
            user: Some(user_summary(user)),
        }))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let owner = self.authorize(&request).await?;
        let user = self.find_user(owner, &request.into_inner().user_id).await?;
        Ok(Response::new(GetUserResponse {
            user: Some(user_summary(user)),
        }))
    }
}

pub async fn serve(address: SocketAddr, state: AppState) -> Result<(), tonic::transport::Error> {
    tracing::info!(%address, "profile gRPC listening");
    tonic::transport::Server::builder()
        .add_service(ProfileServiceServer::new(ProfileGrpcService::new(state)))
        .serve(address)
        .await
}

fn parse_user_id(value: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value).map_err(|_| Status::invalid_argument("user id must be a valid UUID"))
}

fn user_summary(user: GithubUser) -> UserSummary {
    UserSummary {
        id: user.id.to_string(),
        github_id: user.github_id,
        login: user.login,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn invalid_uuid_is_invalid_argument() {
        let status = parse_user_id("invalid").unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn summary_contains_only_public_identity() {
        let user = GithubUser {
            id: Uuid::new_v4(),
            github_id: 42,
            login: "octocat".into(),
            name: None,
            bio: None,
            avatar_url: None,
            html_url: None,
            public_repos: 0,
            followers: 0,
            following: 0,
            company: None,
            blog: None,
            location: None,
            github_created_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let summary = user_summary(user);
        assert_eq!(summary.github_id, 42);
        assert_eq!(summary.login, "octocat");
    }
}
