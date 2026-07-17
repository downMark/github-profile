use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use aws_config::BehaviorVersion;
use aws_sdk_kms::Client as KmsClient;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::{MessageType, SigningAlgorithmSpec};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use jsonwebtoken::{
    Algorithm as JwtAlgorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
};
use rand::{RngCore, rngs::OsRng};
use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use rsa::pkcs8::{DecodePublicKey, EncodePrivateKey, LineEnding};
use rsa::traits::PublicKeyParts;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub iat: usize,
    pub nbf: usize,
    pub exp: usize,
    pub jti: String,
}

#[derive(Clone)]
pub struct JwtKeys {
    encoding: Option<EncodingKey>,
    kms: Option<KmsSigner>,
    decoding: DecodingKey,
    kid: String,
    n: String,
    e: String,
    issuer: String,
    audience: String,
    ttl_seconds: i64,
}

#[derive(Clone)]
struct KmsSigner {
    client: KmsClient,
    key_id: String,
}

impl JwtKeys {
    pub async fn load(
        issuer: String,
        audience: String,
        ttl_seconds: i64,
        kms_key_id: Option<&str>,
    ) -> Result<Self, AppError> {
        match kms_key_id {
            Some(key_id) => Self::from_kms(issuer, audience, ttl_seconds, key_id).await,
            None => Self::generate(issuer, audience, ttl_seconds),
        }
    }

    pub fn generate(issuer: String, audience: String, ttl_seconds: i64) -> Result<Self, AppError> {
        let private = RsaPrivateKey::new(&mut OsRng, 2048).map_err(|_| AppError::Internal)?;
        let public = private.to_public_key();
        let pem = private
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|_| AppError::Internal)?;
        let n = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
        Ok(Self {
            encoding: Some(
                EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|_| AppError::Internal)?,
            ),
            kms: None,
            decoding: DecodingKey::from_rsa_components(&n, &e).map_err(|_| AppError::Internal)?,
            kid: Uuid::new_v4().to_string(),
            n,
            e,
            issuer,
            audience,
            ttl_seconds,
        })
    }

    async fn from_kms(
        issuer: String,
        audience: String,
        ttl_seconds: i64,
        key_id: &str,
    ) -> Result<Self, AppError> {
        let sdk = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = KmsClient::new(&sdk);
        let response = client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(%error, "failed to load KMS public key");
                AppError::Internal
            })?;
        let der = response.public_key().ok_or(AppError::Internal)?.as_ref();
        let public = RsaPublicKey::from_public_key_der(der).map_err(|error| {
            tracing::error!(%error, "KMS public key is not a valid RSA public key");
            AppError::Internal
        })?;
        let n = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
        let kid_hash = URL_SAFE_NO_PAD.encode(Sha256::digest(key_id.as_bytes()));
        Ok(Self {
            encoding: None,
            kms: Some(KmsSigner {
                client,
                key_id: key_id.to_string(),
            }),
            decoding: DecodingKey::from_rsa_components(&n, &e).map_err(|_| AppError::Internal)?,
            kid: kid_hash[..16].to_string(),
            n,
            e,
            issuer,
            audience,
            ttl_seconds,
        })
    }

    pub async fn issue(&self, account_id: Uuid) -> Result<String, AppError> {
        let now = Utc::now().timestamp();
        let claims = Claims {
            iss: self.issuer.clone(),
            sub: account_id.to_string(),
            aud: self.audience.clone(),
            iat: now as usize,
            nbf: now as usize,
            exp: (now + self.ttl_seconds) as usize,
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(JwtAlgorithm::RS256);
        header.kid = Some(self.kid.clone());
        if let Some(encoding) = &self.encoding {
            return encode(&header, &claims, encoding).map_err(|_| AppError::Internal);
        }
        let header =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(|_| AppError::Internal)?);
        let payload =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).map_err(|_| AppError::Internal)?);
        let signing_input = format!("{header}.{payload}");
        let kms = self.kms.as_ref().ok_or(AppError::Internal)?;
        let signature = kms
            .client
            .sign()
            .key_id(&kms.key_id)
            .message(Blob::new(signing_input.as_bytes()))
            .message_type(MessageType::Raw)
            .signing_algorithm(SigningAlgorithmSpec::RsassaPkcs1V15Sha256)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(%error, "KMS JWT signing failed");
                AppError::Internal
            })?
            .signature()
            .ok_or(AppError::Internal)?
            .as_ref()
            .to_vec();
        Ok(format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }

    pub fn validate(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::new(JwtAlgorithm::RS256);
        validation.set_audience(&[&self.audience]);
        validation.set_issuer(&[&self.issuer]);
        decode::<Claims>(token, &self.decoding, &validation)
            .map(|v| v.claims)
            .map_err(|_| AppError::Unauthorized)
    }

    pub fn jwks(&self) -> serde_json::Value {
        serde_json::json!({"keys":[{"kty":"RSA","use":"sig","alg":"RS256","kid":self.kid,"n":self.n,"e":self.e}]})
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    pub fn ttl(&self) -> i64 {
        self.ttl_seconds
    }
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let params = Params::new(19_456, 2, 1, None).map_err(|_| AppError::Internal)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|v| v.to_string())
        .map_err(|_| AppError::Internal)
}

pub fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

pub fn new_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_uses_random_salt_and_verifies() {
        let first = hash_password("correct horse battery staple").unwrap();
        let second = hash_password("correct horse battery staple").unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery staple", &first));
        assert!(!verify_password("wrong password", &first));
    }

    #[test]
    fn jwt_contains_required_claims_and_rejects_tampering() {
        let account_id = Uuid::new_v4();
        let keys = JwtKeys::generate("issuer".into(), "audience".into(), 900).unwrap();
        let token = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(keys.issue(account_id))
            .unwrap();
        let claims = keys.validate(&token).unwrap();
        assert_eq!(claims.sub, account_id.to_string());
        assert_eq!(claims.iss, "issuer");
        assert_eq!(claims.aud, "audience");
        assert!(claims.exp > claims.iat);
        assert!(!claims.jti.is_empty());
        assert!(keys.validate(&(token + "x")).is_err());
    }

    #[test]
    fn refresh_tokens_are_random_and_only_hashes_are_stable() {
        let first = new_refresh_token();
        let second = new_refresh_token();
        assert_ne!(first, second);
        assert_eq!(token_hash(&first), token_hash(&first));
        assert_ne!(token_hash(&first), token_hash(&second));
    }
}
