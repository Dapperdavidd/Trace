use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::{authenticate, create_session},
    error::ApiError,
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/auth/google", web::post().to(google))
        .route("/auth/logout", web::post().to(logout))
        .route("/me", web::get().to(me))
        .route("/me", web::delete().to(delete_me))
        .route("/me/wallets", web::get().to(my_wallets));
}

#[derive(Deserialize)]
struct GoogleBody {
    id_token: String,
}

#[derive(Serialize)]
struct UserOut {
    id: Uuid,
    email: String,
    name: Option<String>,
}

async fn google(
    state: web::Data<AppState>,
    body: web::Json<GoogleBody>,
) -> Result<HttpResponse, ApiError> {
    if body.id_token.is_empty() || body.id_token.len() > 8192 {
        return Err(ApiError::BadRequest("id_token is required.".into()));
    }
    let ident = state.verifier.verify(&body.id_token).await?;
    let mut tx = state.pool.begin().await?;

    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT i.user_id FROM oauth_identities i JOIN users u ON u.id = i.user_id
         WHERE i.provider = $1 AND i.subject = $2 AND u.deleted_at IS NULL",
    )
    .bind(ident.provider)
    .bind(&ident.subject)
    .fetch_optional(&mut *tx)
    .await?;

    let user_id = match existing {
        Some(id) => {
            sqlx::query("UPDATE users SET name = COALESCE($2, name) WHERE id = $1")
                .bind(id)
                .bind(&ident.name)
                .execute(&mut *tx)
                .await?;
            id
        }
        None => {
            // The provider verified this email, so an existing account with it is the same person.
            let by_email: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM users WHERE lower(email) = lower($1) AND deleted_at IS NULL",
            )
            .bind(&ident.email)
            .fetch_optional(&mut *tx)
            .await?;
            let id = match by_email {
                Some(id) => id,
                None => {
                    let id = Uuid::new_v4();
                    sqlx::query("INSERT INTO users (id, email, name) VALUES ($1,$2,$3)")
                        .bind(id)
                        .bind(&ident.email)
                        .bind(&ident.name)
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("INSERT INTO alert_preferences (user_id) VALUES ($1)")
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                    id
                }
            };
            sqlx::query(
                "INSERT INTO oauth_identities (provider, subject, user_id) VALUES ($1,$2,$3)",
            )
            .bind(ident.provider)
            .bind(&ident.subject)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            id
        }
    };
    tx.commit().await?;

    if state.demo {
        crate::demo::seed_alerts_for(&state.pool, user_id, Utc::now()).await?;
    }
    let (token, expires) = create_session(&state.pool, user_id, Utc::now()).await?;
    let row = sqlx::query("SELECT email, name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "access_token": token,
        "expires_at": expires,
        "user": UserOut { id: user_id, email: row.get("email"), name: row.get("name") },
    })))
}

async fn logout(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    sqlx::query("UPDATE app_sessions SET revoked_at = now() WHERE id = $1")
        .bind(u.session_id)
        .execute(&state.pool)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn me(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let watch_only: i64 =
        sqlx::query_scalar("SELECT count(*) FROM watchlist_wallets WHERE user_id = $1")
            .bind(u.user_id)
            .fetch_one(&state.pool)
            .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "id": u.user_id, "email": u.email, "name": u.name,
        "wallets": { "watch_only": watch_only, "connected": 0 },
    })))
}

/// Deletes the account and everything keyed to it (sessions, watchlists,
/// alerts, shadow strategies). Leaves an audit event with no personal data.
async fn delete_me(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO audit_events (id, actor, action, subject) VALUES ($1,'user','account_deleted',$2)")
        .bind(Uuid::new_v4())
        .bind(u.user_id.to_string())
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(u.user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn my_wallets(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let rows = sqlx::query(
        "SELECT chain, address, alias FROM watchlist_wallets WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(u.user_id)
    .fetch_all(&state.pool)
    .await?;
    let watch_only: Vec<_> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "chain": r.get::<String, _>("chain"),
                "address": r.get::<String, _>("address"),
                "alias": r.get::<Option<String>, _>("alias"),
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "watch_only": watch_only,
        "connected": [],
        "embedded": {
            "available": false,
            "reason": "TRACE is watch-only in this version. It never holds keys or signs transactions."
        },
    })))
}
