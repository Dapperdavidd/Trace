use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use super::common::{decode_cursor, encode_cursor, page_limit};
use crate::{auth::authenticate, error::ApiError, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/alert-preferences", web::get().to(get_prefs))
        .route("/alert-preferences", web::patch().to(patch_prefs))
        .route("/alerts", web::get().to(feed));
}

#[derive(Deserialize)]
struct PrefsPatch {
    wallet_buy: Option<bool>,
    wallet_sell: Option<bool>,
    risk_event: Option<bool>,
    signal: Option<bool>,
}

fn prefs_json(r: &sqlx::postgres::PgRow) -> serde_json::Value {
    serde_json::json!({
        "wallet_buy": r.get::<bool, _>("wallet_buy"),
        "wallet_sell": r.get::<bool, _>("wallet_sell"),
        "risk_event": r.get::<bool, _>("risk_event"),
        "signal": r.get::<bool, _>("signal"),
    })
}

async fn get_prefs(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    sqlx::query("INSERT INTO alert_preferences (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(u.user_id)
        .execute(&state.pool)
        .await?;
    let r = sqlx::query("SELECT * FROM alert_preferences WHERE user_id = $1")
        .bind(u.user_id)
        .fetch_one(&state.pool)
        .await?;
    Ok(HttpResponse::Ok().json(prefs_json(&r)))
}

async fn patch_prefs(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<PrefsPatch>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    sqlx::query("INSERT INTO alert_preferences (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(u.user_id)
        .execute(&state.pool)
        .await?;
    let r = sqlx::query(
        r#"UPDATE alert_preferences SET
             wallet_buy = COALESCE($2, wallet_buy), wallet_sell = COALESCE($3, wallet_sell),
             risk_event = COALESCE($4, risk_event), signal = COALESCE($5, signal), updated_at = now()
           WHERE user_id = $1 RETURNING *"#,
    )
    .bind(u.user_id)
    .bind(body.wallet_buy)
    .bind(body.wallet_sell)
    .bind(body.risk_event)
    .bind(body.signal)
    .fetch_one(&state.pool)
    .await?;
    Ok(HttpResponse::Ok().json(prefs_json(&r)))
}

#[derive(Deserialize)]
struct FeedQuery {
    cursor: Option<String>,
    limit: Option<u32>,
}

/// Newest first, keyset paginated. Alerts the user has switched off are not returned.
async fn feed(
    req: HttpRequest,
    state: web::Data<AppState>,
    q: web::Query<FeedQuery>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let limit = page_limit(q.limit);
    let after = q.cursor.as_deref().map(decode_cursor).transpose()?;
    let rows = sqlx::query(
        r#"SELECT a.id, a.type, a.title, a.body, a.evidence_tx, a.event_time, a.published_at, a.demo
           FROM alerts a LEFT JOIN alert_preferences p ON p.user_id = a.user_id
           WHERE a.user_id = $1
             AND ($2::timestamptz IS NULL OR (a.published_at, a.id) < ($2, $3))
             AND CASE a.type WHEN 'wallet_buy' THEN COALESCE(p.wallet_buy, TRUE)
                             WHEN 'wallet_sell' THEN COALESCE(p.wallet_sell, TRUE)
                             WHEN 'risk_event' THEN COALESCE(p.risk_event, TRUE)
                             ELSE COALESCE(p.signal, TRUE) END
           ORDER BY a.published_at DESC, a.id DESC LIMIT $4"#,
    )
    .bind(u.user_id)
    .bind(after.map(|a| a.0))
    .bind(after.map(|a| a.1).unwrap_or_else(Uuid::nil))
    .bind(limit + 1)
    .fetch_all(&state.pool)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let page: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        page.last()
            .map(|r| encode_cursor(r.get::<DateTime<Utc>, _>("published_at"), r.get("id")))
    } else {
        None
    };
    let demo = page.iter().any(|r| r.get::<bool, _>("demo"));
    let items: Vec<_> = page
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<Uuid, _>("id"),
                "type": r.get::<String, _>("type"),
                "title": r.get::<String, _>("title"),
                "body": r.get::<String, _>("body"),
                "evidence_tx": r.get::<Vec<String>, _>("evidence_tx"),
                "event_time": r.get::<DateTime<Utc>, _>("event_time"),
                "published_at": r.get::<DateTime<Utc>, _>("published_at"),
                "demo": r.get::<bool, _>("demo"),
            })
        })
        .collect();
    Ok(HttpResponse::Ok()
        .json(serde_json::json!({ "items": items, "next_cursor": next_cursor, "demo": demo })))
}
