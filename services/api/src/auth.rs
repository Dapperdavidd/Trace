//! Authentication: ID token verification, app sessions and the bearer check.
//!
//! The app issues its own opaque session token. Only its SHA-256 is stored.
//! A signed-in user has no spendable wallet; nothing here grants signing power.

use actix_web::HttpRequest;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
    decode, decode_header,
    jwk::{Jwk, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::ApiError;

pub const SESSION_TTL_DAYS: i64 = 30;
pub const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const JWKS_TTL: StdDuration = StdDuration::from_secs(3600);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub provider: &'static str,
    pub subject: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("sign-in is not configured")]
    NotConfigured,
    #[error("invalid token: {0}")]
    Invalid(String),
    #[error("could not reach the identity provider: {0}")]
    Upstream(String),
}

impl From<VerifyError> for ApiError {
    fn from(e: VerifyError) -> Self {
        match e {
            VerifyError::NotConfigured => {
                ApiError::Unavailable("Google sign-in is not configured on this server.".into())
            }
            VerifyError::Invalid(_) => ApiError::Unauthorized,
            VerifyError::Upstream(m) => ApiError::Unavailable(m),
        }
    }
}

#[async_trait]
pub trait IdTokenVerifier: Send + Sync {
    async fn verify(&self, id_token: &str) -> Result<Identity, VerifyError>;
}

/// Default when no client id is configured.
pub struct Disabled;

#[async_trait]
impl IdTokenVerifier for Disabled {
    async fn verify(&self, _id_token: &str) -> Result<Identity, VerifyError> {
        Err(VerifyError::NotConfigured)
    }
}

/// Local development only. Accepts `dev:<email>`. Must be enabled explicitly
/// with `TRACE_DEV_AUTH=true` and is never constructed otherwise.
pub struct DevVerifier;

#[async_trait]
impl IdTokenVerifier for DevVerifier {
    async fn verify(&self, id_token: &str) -> Result<Identity, VerifyError> {
        let email = id_token
            .strip_prefix("dev:")
            .filter(|e| e.contains('@') && e.len() <= 254)
            .ok_or_else(|| VerifyError::Invalid("expected dev:<email>".into()))?;
        Ok(Identity {
            provider: "dev",
            subject: email.to_lowercase(),
            email: email.to_lowercase(),
            name: None,
        })
    }
}

#[derive(Debug, Deserialize)]
struct GoogleClaims {
    sub: String,
    email: Option<String>,
    email_verified: Option<serde_json::Value>,
    name: Option<String>,
}

pub struct GoogleVerifier {
    client_id: String,
    http: reqwest::Client,
    keys: RwLock<Option<(Instant, JwkSet)>>,
    /// When set, keys are never fetched (tests).
    static_keys: bool,
}

impl GoogleVerifier {
    pub fn new(client_id: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(5))
            .build()
            .expect("http client");
        Self {
            client_id,
            http,
            keys: RwLock::new(None),
            static_keys: false,
        }
    }

    pub fn with_static_keys(client_id: String, keys: JwkSet) -> Self {
        Self {
            client_id,
            http: reqwest::Client::new(),
            keys: RwLock::new(Some((Instant::now(), keys))),
            static_keys: true,
        }
    }

    async fn fetch(&self) -> Result<JwkSet, VerifyError> {
        let set: JwkSet = self
            .http
            .get(GOOGLE_JWKS_URL)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| VerifyError::Upstream(e.to_string()))?
            .json()
            .await
            .map_err(|e| VerifyError::Upstream(e.to_string()))?;
        *self.keys.write().await = Some((Instant::now(), set.clone()));
        Ok(set)
    }

    async fn key_for(&self, kid: &str) -> Result<Jwk, VerifyError> {
        let cached = self.keys.read().await.clone();
        if let Some((at, set)) = cached {
            if self.static_keys || at.elapsed() < JWKS_TTL {
                if let Some(j) = set.find(kid) {
                    return Ok(j.clone());
                }
                if self.static_keys {
                    return Err(VerifyError::Invalid("unknown key id".into()));
                }
            }
        }
        // Unknown kid or stale cache: refresh once.
        let set = self.fetch().await?;
        set.find(kid)
            .cloned()
            .ok_or_else(|| VerifyError::Invalid("unknown key id".into()))
    }
}

#[async_trait]
impl IdTokenVerifier for GoogleVerifier {
    async fn verify(&self, id_token: &str) -> Result<Identity, VerifyError> {
        let header = decode_header(id_token).map_err(|e| VerifyError::Invalid(e.to_string()))?;
        if header.alg != Algorithm::RS256 {
            return Err(VerifyError::Invalid("unexpected algorithm".into()));
        }
        let kid = header
            .kid
            .ok_or_else(|| VerifyError::Invalid("missing key id".into()))?;
        let jwk = self.key_for(&kid).await?;
        let key = DecodingKey::from_jwk(&jwk).map_err(|e| VerifyError::Invalid(e.to_string()))?;
        let mut v = Validation::new(Algorithm::RS256);
        v.set_audience(&[&self.client_id]);
        v.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
        v.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let claims = decode::<GoogleClaims>(id_token, &key, &v)
            .map_err(|e| VerifyError::Invalid(e.to_string()))?
            .claims;
        let verified = match claims.email_verified {
            Some(serde_json::Value::Bool(b)) => b,
            Some(serde_json::Value::String(s)) => s == "true",
            _ => false,
        };
        let email = claims
            .email
            .filter(|_| verified)
            .ok_or_else(|| VerifyError::Invalid("email missing or not verified".into()))?;
        Ok(Identity {
            provider: "google",
            subject: claims.sub,
            email,
            name: claims.name,
        })
    }
}

// ----- sessions -----

pub fn new_token() -> String {
    format!("trc_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(String, DateTime<Utc>), sqlx::Error> {
    let token = new_token();
    let expires = now + Duration::days(SESSION_TTL_DAYS);
    sqlx::query(
        "INSERT INTO app_sessions (id, user_id, token_hash, created_at, expires_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(hash_token(&token))
    .bind(now)
    .bind(expires)
    .execute(pool)
    .await?;
    Ok((token, expires))
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub email: String,
    pub name: Option<String>,
}

pub fn bearer(req: &HttpRequest) -> Option<&str> {
    req.headers()
        .get(actix_web::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty() && t.len() <= 200)
}

/// Resolve the bearer token to a live session and user, or 401.
pub async fn authenticate(req: &HttpRequest, pool: &PgPool) -> Result<AuthUser, ApiError> {
    let token = bearer(req).ok_or(ApiError::Unauthorized)?;
    let row = sqlx::query(
        r#"SELECT s.id AS session_id, u.id AS user_id, u.email, u.name
           FROM app_sessions s JOIN users u ON u.id = s.user_id
           WHERE s.token_hash = $1 AND s.revoked_at IS NULL
             AND s.expires_at > now() AND u.deleted_at IS NULL"#,
    )
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;
    Ok(AuthUser {
        user_id: row.get("user_id"),
        session_id: row.get("session_id"),
        email: row.get("email"),
        name: row.get("name"),
    })
}
