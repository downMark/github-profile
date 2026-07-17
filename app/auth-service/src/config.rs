use std::env;

use crate::error::AppError;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub database_schema: Option<String>,
    pub database_max_connections: u32,
    pub port: u16,
    pub allowed_origin: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub access_ttl_seconds: i64,
    pub refresh_ttl_seconds: i64,
    pub cookie_secure: bool,
    pub cookie_same_site: String,
    pub cookie_path: String,
    pub api_base_path: String,
    pub kms_signing_key_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let database_max_connections =
            value("DATABASE_MAX_CONNECTIONS", "5")
                .parse()
                .map_err(|_| {
                    AppError::Config("DATABASE_MAX_CONNECTIONS must be a positive integer".into())
                })?;
        if database_max_connections == 0 {
            return Err(AppError::Config(
                "DATABASE_MAX_CONNECTIONS must be a positive integer".into(),
            ));
        }
        let api_base_path = value("API_BASE_PATH", "");
        if !api_base_path.is_empty()
            && (!api_base_path.starts_with('/') || api_base_path.ends_with('/'))
        {
            return Err(AppError::Config(
                "API_BASE_PATH must be empty or start with / and not end with /".into(),
            ));
        }
        let database_schema = env::var("DB_SCHEMA").ok();
        if let Some(schema) = &database_schema {
            if !valid_schema(schema) {
                return Err(AppError::Config(
                    "DB_SCHEMA must be prod, staging or pr_<number>".into(),
                ));
            }
        }
        let cookie_secure = value("COOKIE_SECURE", "false")
            .parse()
            .map_err(|_| AppError::Config("COOKIE_SECURE must be true or false".into()))?;
        let cookie_same_site = value(
            "COOKIE_SAME_SITE",
            if cookie_secure { "None" } else { "Lax" },
        );
        if !matches!(cookie_same_site.as_str(), "Lax" | "Strict" | "None") {
            return Err(AppError::Config(
                "COOKIE_SAME_SITE must be Lax, Strict or None".into(),
            ));
        }
        if cookie_same_site == "None" && !cookie_secure {
            return Err(AppError::Config(
                "COOKIE_SAME_SITE=None requires COOKIE_SECURE=true".into(),
            ));
        }
        Ok(Self {
            database_url: database_url()?,
            database_schema,
            database_max_connections,
            port: value("PORT", "3000")
                .parse()
                .map_err(|_| AppError::Config("PORT must be a valid port".into()))?,
            allowed_origin: value("ALLOWED_ORIGIN", "http://localhost:5173"),
            jwt_issuer: value("JWT_ISSUER", "http://auth:3000"),
            jwt_audience: value("JWT_AUDIENCE", "github-profile"),
            access_ttl_seconds: positive_i64("ACCESS_TOKEN_TTL_SECONDS", 900)?,
            refresh_ttl_seconds: positive_i64("REFRESH_TOKEN_TTL_SECONDS", 2_592_000)?,
            cookie_secure,
            cookie_same_site,
            cookie_path: value("COOKIE_PATH", &format!("{api_base_path}/api/auth")),
            api_base_path,
            kms_signing_key_id: env::var("KMS_SIGNING_KEY_ID").ok(),
        })
    }
}

fn database_url() -> Result<String, AppError> {
    if let Ok(url) = env::var("DATABASE_URL") {
        return Ok(url);
    }
    let host = required("DB_HOST")?;
    let port = value("DB_PORT", "5432");
    let database = value("DB_NAME", "postgres");
    let username = required("DB_USERNAME")?;
    let password = required("DB_PASSWORD")?;
    let ssl_mode = value("DB_SSL_MODE", "require");
    let mut url = url::Url::parse("postgres://localhost")
        .map_err(|_| AppError::Config("failed to build DATABASE_URL".into()))?;
    url.set_host(Some(&host))
        .map_err(|_| AppError::Config("invalid DB_HOST".into()))?;
    url.set_port(Some(
        port.parse()
            .map_err(|_| AppError::Config("invalid DB_PORT".into()))?,
    ))
    .map_err(|_| AppError::Config("invalid DB_PORT".into()))?;
    url.set_path(&database);
    url.set_username(&username)
        .map_err(|_| AppError::Config("invalid DB_USERNAME".into()))?;
    url.set_password(Some(&password))
        .map_err(|_| AppError::Config("invalid DB_PASSWORD".into()))?;
    url.query_pairs_mut().append_pair("sslmode", &ssl_mode);
    Ok(url.into())
}

fn valid_schema(schema: &str) -> bool {
    matches!(schema, "prod" | "staging")
        || schema.strip_prefix("pr_").is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn required(name: &str) -> Result<String, AppError> {
    env::var(name).map_err(|_| AppError::Config(format!("{name} must be set")))
}

fn value(name: &str, fallback: &str) -> String {
    env::var(name).unwrap_or_else(|_| fallback.into())
}

fn positive_i64(name: &str, fallback: i64) -> Result<i64, AppError> {
    let parsed = env::var(name)
        .ok()
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| AppError::Config(format!("{name} must be an integer")))?
        .unwrap_or(fallback);
    if parsed <= 0 {
        return Err(AppError::Config(format!("{name} must be positive")));
    }
    Ok(parsed)
}
