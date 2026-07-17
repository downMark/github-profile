use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::http::{HeaderMap, header};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Clone)]
pub struct AuthVerifier {
    inner: Arc<Inner>,
}

struct Inner {
    client: reqwest::Client,
    issuer: String,
    audience: String,
    jwks_url: String,
    keys: RwLock<HashMap<String, DecodingKey>>,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub account_id: Uuid,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
    kty: String,
    alg: String,
}

impl AuthVerifier {
    pub fn new(issuer: &str, audience: &str, jwks_url: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                client: reqwest::Client::builder()
                    .timeout(Duration::from_secs(2))
                    .build()
                    .expect("valid authentication HTTP client"),
                issuer: issuer.into(),
                audience: audience.into(),
                jwks_url: jwks_url.into(),
                keys: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub async fn authenticate_headers(&self, headers: &HeaderMap) -> Result<AuthContext, AppError> {
        let value = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        self.authenticate(value).await
    }

    pub async fn authenticate(&self, authorization: &str) -> Result<AuthContext, AppError> {
        let token = authorization
            .strip_prefix("Bearer ")
            .filter(|v| !v.is_empty())
            .ok_or(AppError::Unauthorized)?;
        let header = decode_header(token).map_err(|_| AppError::Unauthorized)?;
        if header.alg != Algorithm::RS256 {
            return Err(AppError::Unauthorized);
        }
        let kid = header.kid.ok_or(AppError::Unauthorized)?;
        // Drop the read guard before a cache miss triggers a JWKS refresh. Keeping
        // the temporary guard alive across the match would deadlock refresh_keys
        // while it waits for the write lock.
        let cached_key = {
            let keys = self.inner.keys.read().await;
            keys.get(&kid).cloned()
        };
        let key = match cached_key {
            Some(key) => key,
            None => {
                self.refresh_keys().await?;
                self.inner
                    .keys
                    .read()
                    .await
                    .get(&kid)
                    .cloned()
                    .ok_or(AppError::Unauthorized)?
            }
        };
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.inner.issuer]);
        validation.set_audience(&[&self.inner.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        let claims = decode::<Claims>(token, &key, &validation)
            .map_err(|_| AppError::Unauthorized)?
            .claims;
        let account_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
        Ok(AuthContext { account_id })
    }

    async fn refresh_keys(&self) -> Result<(), AppError> {
        let jwks = self
            .inner
            .client
            .get(&self.inner.jwks_url)
            .send()
            .await
            .map_err(|_| AppError::AuthUnavailable)?
            .error_for_status()
            .map_err(|_| AppError::AuthUnavailable)?
            .json::<Jwks>()
            .await
            .map_err(|_| AppError::AuthUnavailable)?;
        let mut keys = HashMap::new();
        for jwk in jwks.keys {
            if jwk.kty == "RSA" && jwk.alg == "RS256" {
                if let Ok(key) = DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                    keys.insert(jwk.kid, key);
                }
            }
        }
        if keys.is_empty() {
            return Err(AppError::AuthUnavailable);
        }
        *self.inner.keys.write().await = keys;
        Ok(())
    }
}
