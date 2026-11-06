use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;
use webauthn_rs::prelude::{
    Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential,
};

use crate::{
    auth::create_session, error::ApiError, routes::account::device_label, state::AppState,
};

const CEREMONY_MINUTES: i64 = 5;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/auth/passkeys/register/options",
        web::post().to(register_options),
    )
    .route(
        "/auth/passkeys/register/finish",
        web::post().to(register_finish),
    )
    .route(
        "/auth/passkeys/authenticate/options",
        web::post().to(authenticate_options),
    )
    .route(
        "/auth/passkeys/authenticate/finish",
        web::post().to(authenticate_finish),
    );
}

#[derive(Deserialize)]
struct RegisterOptionsBody {
    username: String,
    display_name: String,
}

fn normalized_username(value: &str) -> Result<String, ApiError> {
    let value = value.trim().to_ascii_lowercase();
    if !(3..=32).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(ApiError::BadRequest(
            "username must be 3 to 32 lowercase letters, numbers or underscores".into(),
        ));
    }
    Ok(value)
}

fn display_name(value: &str) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 {
        return Err(ApiError::BadRequest(
            "display_name must be 1 to 80 characters".into(),
        ));
    }
    Ok(value.to_string())
}

fn ceremony_error(error: impl std::fmt::Display) -> ApiError {
    tracing::info!(error = %error, "passkey ceremony rejected");
    ApiError::Unauthorized
}

async fn register_options(
    state: web::Data<AppState>,
    body: web::Json<RegisterOptionsBody>,
) -> Result<HttpResponse, ApiError> {
    let username = normalized_username(&body.username)?;
    let display_name = display_name(&body.display_name)?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE lower(username)=lower($1) AND deleted_at IS NULL)",
    )
    .bind(&username)
    .fetch_one(&state.pool)
    .await?;
    if exists {
        return Err(ApiError::Conflict("username is already registered".into()));
    }

    let user_id = Uuid::new_v4();
    let (options, ceremony) = state
        .passkeys
        .start_passkey_registration(user_id, &username, &display_name, None)
        .map_err(ceremony_error)?;
    let ceremony_id = Uuid::new_v4();
    let serialized =
        serde_json::to_value(ceremony).map_err(|error| ApiError::Internal(error.into()))?;
    sqlx::query(
        "INSERT INTO passkey_ceremonies
         (id,kind,user_id,username,display_name,state,expires_at)
         VALUES ($1,'register',$2,$3,$4,$5,$6)",
    )
    .bind(ceremony_id)
    .bind(user_id)
    .bind(&username)
    .bind(&display_name)
    .bind(serialized)
    .bind(Utc::now() + Duration::minutes(CEREMONY_MINUTES))
    .execute(&state.pool)
    .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "ceremony_id": ceremony_id,
        "options": options.public_key,
    })))
}

#[derive(Deserialize)]
struct RegisterFinishBody {
    ceremony_id: Uuid,
    credential: RegisterPublicKeyCredential,
}

async fn register_finish(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<RegisterFinishBody>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "SELECT user_id,username,display_name,state FROM passkey_ceremonies
         WHERE id=$1 AND kind='register' AND used_at IS NULL AND expires_at > now()
         FOR UPDATE",
    )
    .bind(body.ceremony_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::Unauthorized)?;
    let user_id: Uuid = row.get("user_id");
    let username: String = row.get("username");
    let name: String = row.get("display_name");
    let ceremony: PasskeyRegistration = serde_json::from_value(row.get("state"))
        .map_err(|error| ApiError::Internal(error.into()))?;
    let passkey = state
        .passkeys
        .finish_passkey_registration(&body.credential, &ceremony)
        .map_err(ceremony_error)?;
    let credential_id = passkey.cred_id().to_vec();
    let passkey_json =
        serde_json::to_value(&passkey).map_err(|error| ApiError::Internal(error.into()))?;

    sqlx::query("INSERT INTO users (id,email,username,name) VALUES ($1,NULL,$2,$3)")
        .bind(user_id)
        .bind(&username)
        .bind(&name)
        .execute(&mut *tx)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                ApiError::Conflict("username or passkey is already registered".into())
            }
            _ => ApiError::Internal(error.into()),
        })?;
    sqlx::query("INSERT INTO alert_preferences (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO passkey_credentials (id,user_id,credential_id,passkey,label)
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(credential_id)
    .bind(passkey_json)
    .bind(device_label(&req))
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE passkey_ceremonies SET used_at=now() WHERE id=$1")
        .bind(body.ceremony_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    finish_session(&req, &state, user_id, Some(username), Some(name)).await
}

#[derive(Deserialize)]
struct AuthenticateOptionsBody {
    username: String,
}

async fn authenticate_options(
    state: web::Data<AppState>,
    body: web::Json<AuthenticateOptionsBody>,
) -> Result<HttpResponse, ApiError> {
    let username = normalized_username(&body.username)?;
    let user_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM users WHERE lower(username)=lower($1) AND deleted_at IS NULL",
    )
    .bind(&username)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;
    let rows = sqlx::query("SELECT passkey FROM passkey_credentials WHERE user_id=$1")
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?;
    let passkeys: Vec<Passkey> = rows
        .iter()
        .map(|row| serde_json::from_value(row.get("passkey")))
        .collect::<Result<_, _>>()
        .map_err(|error| ApiError::Internal(error.into()))?;
    if passkeys.is_empty() {
        return Err(ApiError::Unauthorized);
    }
    let (options, ceremony) = state
        .passkeys
        .start_passkey_authentication(&passkeys)
        .map_err(ceremony_error)?;
    let ceremony_id = Uuid::new_v4();
    let serialized =
        serde_json::to_value(ceremony).map_err(|error| ApiError::Internal(error.into()))?;
    sqlx::query(
        "INSERT INTO passkey_ceremonies (id,kind,user_id,state,expires_at)
         VALUES ($1,'authenticate',$2,$3,$4)",
    )
    .bind(ceremony_id)
    .bind(user_id)
    .bind(serialized)
    .bind(Utc::now() + Duration::minutes(CEREMONY_MINUTES))
    .execute(&state.pool)
    .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "ceremony_id": ceremony_id,
        "options": options.public_key,
    })))
}

#[derive(Deserialize)]
struct AuthenticateFinishBody {
    ceremony_id: Uuid,
    credential: PublicKeyCredential,
}

async fn authenticate_finish(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AuthenticateFinishBody>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "SELECT user_id,state FROM passkey_ceremonies
         WHERE id=$1 AND kind='authenticate' AND used_at IS NULL AND expires_at > now()
         FOR UPDATE",
    )
    .bind(body.ceremony_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::Unauthorized)?;
    let user_id: Uuid = row.get("user_id");
    let ceremony: PasskeyAuthentication = serde_json::from_value(row.get("state"))
        .map_err(|error| ApiError::Internal(error.into()))?;
    let result = state
        .passkeys
        .finish_passkey_authentication(&body.credential, &ceremony)
        .map_err(ceremony_error)?;
    let credential_id = result.cred_id().to_vec();
    let stored = sqlx::query(
        "SELECT id,passkey FROM passkey_credentials
         WHERE user_id=$1 AND credential_id=$2 FOR UPDATE",
    )
    .bind(user_id)
    .bind(&credential_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::Unauthorized)?;
    let mut passkey: Passkey = serde_json::from_value(stored.get("passkey"))
        .map_err(|error| ApiError::Internal(error.into()))?;
    passkey.update_credential(&result);
    sqlx::query("UPDATE passkey_credentials SET passkey=$1,last_used_at=now() WHERE id=$2")
        .bind(serde_json::to_value(passkey).map_err(|error| ApiError::Internal(error.into()))?)
        .bind(stored.get::<Uuid, _>("id"))
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE passkey_ceremonies SET used_at=now() WHERE id=$1")
        .bind(body.ceremony_id)
        .execute(&mut *tx)
        .await?;
    let user = sqlx::query("SELECT username,name FROM users WHERE id=$1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let username: Option<String> = user.get("username");
    let name: Option<String> = user.get("name");
    tx.commit().await?;
    finish_session(&req, &state, user_id, username, name).await
}

async fn finish_session(
    req: &HttpRequest,
    state: &AppState,
    user_id: Uuid,
    username: Option<String>,
    name: Option<String>,
) -> Result<HttpResponse, ApiError> {
    if state.demo {
        crate::demo::seed_alerts_for(&state.pool, user_id, Utc::now()).await?;
    }
    let (token, expires_at) =
        create_session(&state.pool, user_id, device_label(req), Utc::now()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "access_token": token,
        "expires_at": expires_at,
        "user": {
            "id": user_id,
            "username": username,
            "email": null,
            "name": name,
        },
    })))
}
