use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use super::common::wallet_id;
use crate::{auth::authenticate, error::ApiError, state::AppState};

pub const MAX_WATCHED: i64 = 200;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/watchlists", web::get().to(list))
        .route("/watchlists", web::post().to(add))
        .route("/watchlists/{id}", web::patch().to(update))
        .route("/watchlists/{id}", web::delete().to(remove));
}

#[derive(Deserialize)]
struct AddBody {
    chain: String,
    address: String,
    alias: Option<String>,
}

#[derive(Deserialize)]
struct PatchBody {
    alias: Option<String>,
    paused: Option<bool>,
}

fn clean_alias(a: Option<String>) -> Result<Option<String>, ApiError> {
    match a.map(|s| s.trim().to_string()) {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) if s.chars().count() > 60 => Err(ApiError::BadRequest(
            "alias is limited to 60 characters.".into(),
        )),
        Some(s) if s.chars().any(char::is_control) => {
            Err(ApiError::BadRequest("alias has invalid characters.".into()))
        }
        other => Ok(other),
    }
}

/// Monitoring state derived from real coverage and observed activity, never assumed.
async fn items_for(
    state: &AppState,
    user_id: Uuid,
    only: Option<Uuid>,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, chain, address, alias, paused, last_checked_at
           FROM watchlist_wallets WHERE user_id = $1 AND ($2::uuid IS NULL OR id = $2)
           ORDER BY created_at DESC, id"#,
    )
    .bind(user_id)
    .bind(only)
    .fetch_all(&state.pool)
    .await?;

    let enabled: BTreeSet<String> = sqlx::query_scalar(
        "SELECT DISTINCT chain FROM protocols WHERE support_status IN ('enabled','partial')",
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect();
    let partial_only: BTreeSet<String> = sqlx::query_scalar(
        "SELECT chain FROM protocols GROUP BY chain HAVING bool_and(support_status <> 'enabled')",
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect();
    let seen: BTreeMap<(String, String), i64> = sqlx::query(
        r#"SELECT w.chain, w.address, count(s.id) AS n
           FROM watchlist_wallets w LEFT JOIN normalized_swaps s
             ON s.chain = w.chain AND s.wallet = w.address AND s.finality <> 'reorged'
           WHERE w.user_id = $1 GROUP BY w.chain, w.address"#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| ((r.get("chain"), r.get("address")), r.get("n")))
    .collect();

    Ok(rows
        .into_iter()
        .map(|r| {
            let chain: String = r.get("chain");
            let address: String = r.get("address");
            let paused: bool = r.get("paused");
            let n = seen.get(&(chain.clone(), address.clone())).copied().unwrap_or(0);
            let status = if paused {
                "paused"
            } else if !enabled.contains(&chain) {
                "unsupported_protocol"
            } else if n == 0 {
                "waiting_for_activity"
            } else if partial_only.contains(&chain) {
                "partial_monitoring"
            } else {
                "active"
            };
            serde_json::json!({
                "id": r.get::<Uuid, _>("id"),
                "wallet": { "chain": chain, "address": address, "alias": r.get::<Option<String>, _>("alias") },
                "status": status,
                "last_checked_at": r.get::<Option<DateTime<Utc>>, _>("last_checked_at"),
            })
        })
        .collect())
}

async fn list(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let items = items_for(&state, u.user_id, None).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "items": items })))
}

async fn add(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AddBody>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let w = wallet_id(&body.chain, &body.address)?;
    let alias = clean_alias(body.alias.clone())?;
    let (chain, address) = (w.chain.as_str(), w.address.as_str());

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM watchlist_wallets WHERE user_id = $1")
            .bind(u.user_id)
            .fetch_one(&state.pool)
            .await?;
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM watchlist_wallets WHERE user_id=$1 AND chain=$2 AND address=$3)",
    )
    .bind(u.user_id)
    .bind(chain)
    .bind(address)
    .fetch_one(&state.pool)
    .await?;
    if !already && count >= MAX_WATCHED {
        return Err(ApiError::LimitReached(format!(
            "A watchlist holds at most {MAX_WATCHED} wallets."
        )));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO wallets (chain, address) VALUES ($1,$2) ON CONFLICT DO NOTHING")
        .bind(chain)
        .bind(address)
        .execute(&mut *tx)
        .await?;
    // Idempotent on user + chain + address; a new alias updates the existing row.
    let id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO watchlist_wallets (id, user_id, chain, address, alias)
           VALUES ($1,$2,$3,$4,$5)
           ON CONFLICT (user_id, chain, address)
           DO UPDATE SET alias = COALESCE(EXCLUDED.alias, watchlist_wallets.alias)
           RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(u.user_id)
    .bind(chain)
    .bind(address)
    .bind(&alias)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    let item = items_for(&state, u.user_id, Some(id))
        .await?
        .into_iter()
        .next();
    let status = if already {
        actix_web::http::StatusCode::OK
    } else {
        actix_web::http::StatusCode::CREATED
    };
    Ok(HttpResponse::build(status).json(item))
}

async fn update(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<PatchBody>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let id = path.into_inner();
    let alias_set = body.alias.is_some();
    let alias = clean_alias(body.alias.clone())?;
    let res = sqlx::query(
        r#"UPDATE watchlist_wallets
           SET alias = CASE WHEN $3 THEN $4 ELSE alias END,
               paused = COALESCE($5, paused)
           WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(u.user_id)
    .bind(alias_set)
    .bind(&alias)
    .bind(body.paused)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    let item = items_for(&state, u.user_id, Some(id))
        .await?
        .into_iter()
        .next();
    Ok(HttpResponse::Ok().json(item))
}

async fn remove(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let res = sqlx::query("DELETE FROM watchlist_wallets WHERE id = $1 AND user_id = $2")
        .bind(path.into_inner())
        .bind(u.user_id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}
