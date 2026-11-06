use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::{authenticate, bearer, hash_token},
    error::ApiError,
    state::AppState,
};

const MAX_CONNECTIONS: i64 = 20;
const MAX_TTL_DAYS: i64 = 90;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/agent-connections", web::post().to(create_connection))
        .route("/agent-connections", web::get().to(list_connections))
        .route(
            "/agent-connections/{id}",
            web::delete().to(revoke_connection),
        )
        .route("/agent/context", web::get().to(context))
        .route("/agent/intents/preview", web::post().to(preview_intent));
}

#[derive(Deserialize)]
struct CreateConnectionBody {
    name: String,
    scopes: Vec<String>,
    expires_in_days: Option<u16>,
}

fn clean_name(value: &str) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 60 || value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "name must be 1 to 60 visible characters".into(),
        ));
    }
    Ok(value.to_owned())
}

fn clean_scopes(values: &[String]) -> Result<Vec<String>, ApiError> {
    let mut scopes: Vec<String> = values.iter().map(|value| value.trim().to_owned()).collect();
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty()
        || scopes
            .iter()
            .any(|scope| !matches!(scope.as_str(), "observe" | "propose"))
    {
        return Err(ApiError::BadRequest(
            "scopes may contain only observe and propose".into(),
        ));
    }
    Ok(scopes)
}

fn new_agent_token() -> String {
    format!(
        "trca_{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

async fn create_connection(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateConnectionBody>,
) -> Result<HttpResponse, ApiError> {
    let user = authenticate(&req, &state.pool).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM agent_connections WHERE user_id=$1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;
    if count >= MAX_CONNECTIONS {
        return Err(ApiError::LimitReached(format!(
            "An account can have at most {MAX_CONNECTIONS} active agent connections."
        )));
    }
    let name = clean_name(&body.name)?;
    let scopes = clean_scopes(&body.scopes)?;
    let days = i64::from(body.expires_in_days.unwrap_or(30));
    if !(1..=MAX_TTL_DAYS).contains(&days) {
        return Err(ApiError::BadRequest(format!(
            "expires_in_days must be between 1 and {MAX_TTL_DAYS}"
        )));
    }
    let id = Uuid::new_v4();
    let token = new_agent_token();
    let token_hint = token[token.len() - 6..].to_owned();
    let expires_at = Utc::now() + Duration::days(days);
    sqlx::query(
        "INSERT INTO agent_connections
         (id,user_id,name,token_hash,token_hint,scopes,expires_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(id)
    .bind(user.user_id)
    .bind(&name)
    .bind(hash_token(&token))
    .bind(&token_hint)
    .bind(&scopes)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "id": id,
        "name": name,
        "scopes": scopes,
        "expires_at": expires_at,
        "token": token,
        "token_visible_once": true,
        "execution_authority": false,
        "note": "This credential can observe or prepare proposals. Only a NEAR mandate can authorize an agent action."
    })))
}

async fn list_connections(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let user = authenticate(&req, &state.pool).await?;
    let rows = sqlx::query(
        "SELECT id,name,token_hint,scopes,created_at,expires_at,last_used_at,revoked_at
         FROM agent_connections WHERE user_id=$1 ORDER BY created_at DESC,id",
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<_> = rows
        .iter()
        .map(|row| {
            let expires_at: DateTime<Utc> = row.get("expires_at");
            let revoked_at: Option<DateTime<Utc>> = row.get("revoked_at");
            serde_json::json!({
                "id": row.get::<Uuid,_>("id"),
                "name": row.get::<String,_>("name"),
                "token_hint": row.get::<String,_>("token_hint"),
                "scopes": row.get::<Vec<String>,_>("scopes"),
                "created_at": row.get::<DateTime<Utc>,_>("created_at"),
                "expires_at": expires_at,
                "last_used_at": row.get::<Option<DateTime<Utc>>,_>("last_used_at"),
                "revoked_at": revoked_at,
                "status": if revoked_at.is_some() { "revoked" } else if expires_at <= Utc::now() { "expired" } else { "active" },
                "execution_authority": false,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(serde_json::json!({ "items": items })))
}

async fn revoke_connection(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let user = authenticate(&req, &state.pool).await?;
    let result = sqlx::query(
        "UPDATE agent_connections SET revoked_at=COALESCE(revoked_at,now()) WHERE id=$1 AND user_id=$2",
    )
    .bind(*id)
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug)]
pub(crate) struct AgentPrincipal {
    pub user_id: Uuid,
    pub connection_id: Uuid,
    pub scopes: Vec<String>,
}

async fn authenticate_agent(
    req: &HttpRequest,
    state: &AppState,
    required_scope: &str,
) -> Result<AgentPrincipal, ApiError> {
    let token = bearer(req).ok_or(ApiError::Unauthorized)?;
    if !token.starts_with("trca_") {
        return Err(ApiError::Unauthorized);
    }
    let row = sqlx::query(
        "SELECT c.id,c.user_id,c.scopes FROM agent_connections c
         JOIN users u ON u.id=c.user_id
         WHERE c.token_hash=$1 AND c.revoked_at IS NULL AND c.expires_at > now()
           AND u.deleted_at IS NULL",
    )
    .bind(hash_token(token))
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;
    let scopes: Vec<String> = row.get("scopes");
    if !scopes.iter().any(|scope| scope == required_scope) {
        return Err(ApiError::Forbidden(format!(
            "agent credential lacks the {required_scope} scope"
        )));
    }
    let connection_id: Uuid = row.get("id");
    sqlx::query("UPDATE agent_connections SET last_used_at=now() WHERE id=$1")
        .bind(connection_id)
        .execute(&state.pool)
        .await?;
    Ok(AgentPrincipal {
        user_id: row.get("user_id"),
        connection_id,
        scopes,
    })
}

async fn context(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let agent = authenticate_agent(&req, &state, "observe").await?;
    let wallets = super::watchlists::items_for(&state, agent.user_id, None).await?;
    let mandate_rows = sqlx::query(
        "SELECT id,mode,status,allowed_chains,allowed_assets,max_trade_usd,max_daily_usd,
                max_slippage_bps,expires_at,commitment,anchor_tx_hash
         FROM near_agent_mandates
         WHERE user_id=$1 AND status='anchored' AND expires_at > now()
         ORDER BY created_at DESC,id",
    )
    .bind(agent.user_id)
    .fetch_all(&state.pool)
    .await?;
    let mandates: Vec<_> = mandate_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<Uuid,_>("id"),
                "mode": row.get::<String,_>("mode"),
                "status": row.get::<String,_>("status"),
                "allowed_chains": row.get::<Vec<String>,_>("allowed_chains"),
                "allowed_assets": row.get::<serde_json::Value,_>("allowed_assets"),
                "max_trade_usd": row.get::<rust_decimal::Decimal,_>("max_trade_usd").to_string(),
                "max_daily_usd": row.get::<rust_decimal::Decimal,_>("max_daily_usd").to_string(),
                "max_slippage_bps": row.get::<i32,_>("max_slippage_bps"),
                "expires_at": row.get::<DateTime<Utc>,_>("expires_at"),
                "commitment": row.get::<String,_>("commitment"),
                "anchor_tx_hash": row.get::<Option<String>,_>("anchor_tx_hash"),
                "execution_authority": "near_contract",
            })
        })
        .collect();
    let alert_rows = sqlx::query(
        "SELECT id,type,title,body,evidence_tx,event_time,published_at,demo
         FROM alerts WHERE user_id=$1 ORDER BY published_at DESC,id DESC LIMIT 20",
    )
    .bind(agent.user_id)
    .fetch_all(&state.pool)
    .await?;
    let alerts: Vec<_> = alert_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<Uuid,_>("id"),
                "type": row.get::<String,_>("type"),
                "title": row.get::<String,_>("title"),
                "body": row.get::<String,_>("body"),
                "evidence_tx": row.get::<Vec<String>,_>("evidence_tx"),
                "event_time": row.get::<DateTime<Utc>,_>("event_time"),
                "published_at": row.get::<DateTime<Utc>,_>("published_at"),
                "data_status": if row.get::<bool,_>("demo") { "demo" } else { "live" },
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "data_status": if mandates.is_empty() { "partial" } else { "live" },
        "as_of": Utc::now(),
        "near_is_control_plane": true,
        "connection": { "id": agent.connection_id, "scopes": agent.scopes },
        "capabilities": {
            "observe": true,
            "propose": agent.scopes.iter().any(|scope| scope == "propose"),
            "execute": false,
            "authority": "anchored_near_mandate"
        },
        "watched_wallets": wallets,
        "active_near_mandates": mandates,
        "recent_alerts": alerts,
        "note": "This credential cannot sign or submit transactions. A NEAR mandate and owner signature remain authoritative."
    })))
}

async fn preview_intent(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<super::near::PreviewBody>,
) -> Result<HttpResponse, ApiError> {
    let agent = authenticate_agent(&req, &state, "propose").await?;
    super::near::preview_for_user(
        &state,
        agent.user_id,
        body.into_inner(),
        Some(agent.connection_id),
    )
    .await
}
