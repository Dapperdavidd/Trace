use actix_web::{web, HttpRequest, HttpResponse};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;
use trace_near::{
    AgentMandate, AssetPermission, IntentsError, MandateMode, MandateStatus, NearRpcError,
    QuoteRequest,
};
use uuid::Uuid;

use crate::{auth::authenticate, error::ApiError, state::AppState};

const MAX_MANDATES_PER_USER: i64 = 50;
const MAX_QUOTE_DEADLINE_MINUTES: i64 = 15;
const FUNCTION_CALL_GAS: &str = "100000000000000";

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/near", web::get().to(status))
        .route("/near/assets", web::get().to(assets))
        .route("/near/mandates", web::post().to(create_mandate))
        .route("/near/mandates", web::get().to(list_mandates))
        .route("/near/mandates/{id}/anchor", web::post().to(confirm_anchor))
        .route("/near/intents/preview", web::post().to(preview_intent));
}

async fn status(state: web::Data<AppState>) -> HttpResponse {
    let contract_configured = state.near.contract_id.is_some();
    // Verified means the code deployed on the contract account is byte-for-byte the
    // reviewed build. Any RPC failure reports unverified rather than guessing.
    let observed_code_hash = match (
        state.near.contract_id.as_deref(),
        state.near.expected_code_hash.is_some(),
    ) {
        (Some(id), true) => state.near.rpc.code_hash(id).await.ok(),
        _ => None,
    };
    let deployment_verified = matches!(
        (observed_code_hash.as_deref(), state.near.expected_code_hash.as_deref()),
        (Some(seen), Some(want)) if seen == want
    );
    let agent_configured = state.near.agent_account_id.is_some();
    HttpResponse::Ok().json(json!({
        "data_status": if contract_configured && agent_configured { "partial" } else { "unavailable" },
        "as_of": Utc::now(),
        "demo": false,
        "near_is_control_plane": true,
        "network": state.near.network.as_ref(),
        "contract": {
            "configured": contract_configured,
            "account_id": state.near.contract_id.as_deref(),
            "deployment_verified": deployment_verified,
            "code_hash": observed_code_hash,
            "expected_code_hash": state.near.expected_code_hash.as_deref(),
        },
        "agent": {
            "configured": agent_configured,
            "account_id": state.near.agent_account_id.as_deref(),
            "delegation_enabled": state.near.delegation_enabled,
        },
        "intents": {
            "asset_discovery": true,
            "dry_quote_previews": true,
            "server_side_signing": false,
            "server_side_submission": false,
        },
        "storage": {
            "mandates": "near_contract_state",
            "receipt_commitments": "near_contract_state",
            "full_evidence": "trace_append_only_database",
        },
        "coverage_note": if deployment_verified {
            "NEAR owns agent authorization and evidence. Ethereum and Solana are observed markets. The deployed mandate contract matches the reviewed build. Auto copy stays off until the security review."
        } else {
            "NEAR owns agent authorization and evidence. Ethereum and Solana are observed markets. Live authority remains unavailable until the contract deployment is verified."
        },
    }))
}

async fn assets(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    authenticate(&req, &state.pool).await?;
    let assets = state
        .near
        .intents
        .assets()
        .await
        .map_err(map_intents_error)?;
    Ok(HttpResponse::Ok().json(json!({
        "data_status": "live",
        "as_of": Utc::now(),
        "demo": false,
        "source": "near_intents_1click",
        "items": assets,
    })))
}

#[derive(Deserialize)]
struct CreateMandateBody {
    shadow_strategy_id: Uuid,
    owner_near_account: String,
    agent_near_account: Option<String>,
    mode: String,
    allowed_chains: BTreeSet<String>,
    #[serde(default)]
    allowed_assets: BTreeSet<AssetPermission>,
    max_trade_usd: Value,
    max_daily_usd: Value,
    max_slippage_bps: u16,
    expires_at: DateTime<Utc>,
}

fn parse_money(value: &Value, field: &str) -> Result<Decimal, ApiError> {
    let parsed = match value {
        Value::String(value) => Decimal::from_str(value.trim()),
        Value::Number(value) => Decimal::from_str(&value.to_string()),
        _ => {
            return Err(ApiError::BadRequest(format!(
                "{field} must be a decimal string."
            )))
        }
    }
    .map_err(|_| ApiError::BadRequest(format!("{field} is not a valid amount.")))?;
    if parsed <= Decimal::ZERO || parsed > Decimal::from(1_000_000) {
        return Err(ApiError::BadRequest(format!(
            "{field} must be above 0 and at most 1,000,000."
        )));
    }
    if parsed.round_dp(6) != parsed {
        return Err(ApiError::BadRequest(format!(
            "{field} supports at most 6 decimal places."
        )));
    }
    Ok(parsed)
}

fn parse_mode(value: &str) -> Result<MandateMode, ApiError> {
    match value {
        "paper" => Ok(MandateMode::Paper),
        "approval_required" => Ok(MandateMode::ApprovalRequired),
        "delegated" => Ok(MandateMode::Delegated),
        _ => Err(ApiError::BadRequest(
            "mode must be paper, approval_required or delegated.".into(),
        )),
    }
}

fn mode_name(value: MandateMode) -> &'static str {
    match value {
        MandateMode::Paper => "paper",
        MandateMode::ApprovalRequired => "approval_required",
        MandateMode::Delegated => "delegated",
    }
}

async fn create_mandate(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateMandateBody>,
) -> Result<HttpResponse, ApiError> {
    let user = authenticate(&req, &state.pool).await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM shadow_strategies WHERE id=$1 AND user_id=$2)",
    )
    .bind(body.shadow_strategy_id)
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;
    if !exists {
        return Err(ApiError::NotFound);
    }

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM near_agent_mandates WHERE user_id=$1")
            .bind(user.user_id)
            .fetch_one(&state.pool)
            .await?;
    if count >= MAX_MANDATES_PER_USER {
        return Err(ApiError::LimitReached(format!(
            "At most {MAX_MANDATES_PER_USER} NEAR mandates per account."
        )));
    }

    let mode = parse_mode(&body.mode)?;
    if mode == MandateMode::Delegated && !state.near.delegation_enabled {
        return Err(ApiError::Unavailable(
            "delegated execution stays off until the deployed contract and agent pass review"
                .into(),
        ));
    }
    let agent_near_account = body
        .agent_near_account
        .clone()
        .or_else(|| state.near.agent_account_id.as_deref().map(str::to_owned))
        .ok_or_else(|| {
            ApiError::Unavailable("TRACE_NEAR_AGENT_ACCOUNT_ID is not configured".into())
        })?;
    if mode == MandateMode::Delegated
        && state.near.agent_account_id.as_deref() != Some(agent_near_account.as_str())
    {
        return Err(ApiError::BadRequest(
            "delegated mode may name only the configured TRACE agent account.".into(),
        ));
    }

    let max_trade = parse_money(&body.max_trade_usd, "max_trade_usd")?;
    let max_daily = parse_money(&body.max_daily_usd, "max_daily_usd")?;
    let id = Uuid::new_v4();
    let nonce = if mode == MandateMode::Delegated {
        Utc::now().timestamp_millis().unsigned_abs().max(1)
    } else {
        0
    };
    let mandate = AgentMandate {
        id,
        owner_near_account: body.owner_near_account.clone(),
        agent_near_account,
        strategy_id: body.shadow_strategy_id,
        mode,
        status: MandateStatus::Active,
        allowed_chains: body.allowed_chains.clone(),
        allowed_assets: body.allowed_assets.clone(),
        max_trade_usd: max_trade,
        max_daily_usd: max_daily,
        max_slippage_bps: body.max_slippage_bps,
        expires_at: body.expires_at,
        nonce,
    };
    mandate
        .validate(Utc::now())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let commitment = mandate
        .commitment()
        .map_err(|error| ApiError::Internal(error.into()))?;
    let allowed_assets = serde_json::to_value(&mandate.allowed_assets)
        .map_err(|error| ApiError::Internal(error.into()))?;
    let contract_id = state.near.contract_id.as_deref();
    let transaction = contract_id.map(|contract| create_transaction(contract, &mandate));
    let contract_args = transaction
        .as_ref()
        .map(|value| value["actions"][0]["params"]["args"].clone())
        .unwrap_or_else(|| json!({}));

    sqlx::query(
        "INSERT INTO near_agent_mandates
         (id,user_id,shadow_strategy_id,owner_near_account,agent_near_account,mode,status,
          allowed_chains,allowed_assets,max_trade_usd,max_daily_usd,max_slippage_bps,
          expires_at,nonce,commitment,near_contract_id,contract_args)
         VALUES ($1,$2,$3,$4,$5,$6,'draft',$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
    )
    .bind(id)
    .bind(user.user_id)
    .bind(body.shadow_strategy_id)
    .bind(&mandate.owner_near_account)
    .bind(&mandate.agent_near_account)
    .bind(mode_name(mode))
    .bind(mandate.allowed_chains.iter().cloned().collect::<Vec<_>>())
    .bind(allowed_assets)
    .bind(max_trade)
    .bind(max_daily)
    .bind(i32::from(mandate.max_slippage_bps))
    .bind(mandate.expires_at)
    .bind(i64::try_from(nonce).map_err(|error| ApiError::Internal(error.into()))?)
    .bind(&commitment)
    .bind(contract_id)
    .bind(contract_args)
    .execute(&state.pool)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            ApiError::Conflict("This Shadow strategy already has a NEAR mandate.".into())
        }
        _ => ApiError::Internal(error.into()),
    })?;

    Ok(HttpResponse::Created().json(json!({
        "id": id,
        "status": "draft",
        "commitment": commitment,
        "near_transaction": transaction,
        "owner_signature_required": true,
        "note": if transaction.is_some() {
            "Sign this contract call in the named NEAR owner's wallet. A draft is not active authority."
        } else {
            "Mandate saved as a draft. Contract deployment must be configured before it can be anchored."
        },
    })))
}

fn create_transaction(contract_id: &str, mandate: &AgentMandate) -> Value {
    // The contract declares these as plain u128, which near-sdk reads from JSON numbers.
    // Strings make create_mandate trap on chain. u64 covers any validated mandate limit.
    let micros = |value: Decimal| {
        (value * Decimal::from(1_000_000))
            .to_u64()
            .expect("validated positive microdollar amount")
    };
    json!({
        "receiverId": contract_id,
        "actions": [{
            "type": "FunctionCall",
            "params": {
                "methodName": "create_mandate",
                "args": { "mandate": {
                    "id": mandate.id,
                    "owner": mandate.owner_near_account,
                    "agent": mandate.agent_near_account,
                    "strategy_id": mandate.strategy_id,
                    "mode": match mandate.mode {
                        MandateMode::Paper => "Paper",
                        MandateMode::ApprovalRequired => "ApprovalRequired",
                        MandateMode::Delegated => "Delegated",
                    },
                    "status": "Active",
                    "allowed_chains": mandate.allowed_chains,
                    "allowed_assets": mandate.allowed_assets.iter().map(|a| a.asset_id.clone()).collect::<Vec<_>>(),
                    "max_trade_usd_micros": micros(mandate.max_trade_usd),
                    "max_daily_usd_micros": micros(mandate.max_daily_usd),
                    "max_slippage_bps": mandate.max_slippage_bps,
                    "expires_at_ms": mandate.expires_at.timestamp_millis(),
                    "nonce": mandate.nonce,
                    "created_at_ms": Utc::now().timestamp_millis(),
                }},
                "gas": FUNCTION_CALL_GAS,
                "deposit": "0",
            }
        }]
    })
}

#[derive(Deserialize)]
struct ConfirmAnchorBody {
    tx_hash: String,
    signer_account_id: String,
}

async fn confirm_anchor(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<Uuid>,
    body: web::Json<ConfirmAnchorBody>,
) -> Result<HttpResponse, ApiError> {
    let user = authenticate(&req, &state.pool).await?;
    if body.tx_hash.len() < 32 || body.tx_hash.len() > 64 {
        return Err(ApiError::BadRequest("tx_hash is not plausible.".into()));
    }
    let row = sqlx::query(
        "SELECT owner_near_account,status,near_contract_id,contract_args
         FROM near_agent_mandates WHERE id=$1 AND user_id=$2",
    )
    .bind(*id)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let owner: String = row.get("owner_near_account");
    if body.signer_account_id != owner {
        return Err(ApiError::BadRequest(
            "signer_account_id must match the mandate owner.".into(),
        ));
    }
    let current_status: String = row.get("status");
    if current_status != "draft" {
        return Err(ApiError::Conflict(format!(
            "only a draft can be anchored; mandate is {current_status}"
        )));
    }
    let contract_id: String = row
        .get::<Option<String>, _>("near_contract_id")
        .ok_or_else(|| ApiError::Unavailable("NEAR contract is not configured".into()))?;
    let expected_args: Value = row.get("contract_args");
    if expected_args
        .as_object()
        .is_none_or(|value| value.is_empty())
    {
        return Err(ApiError::Conflict(
            "draft has no deployable contract arguments".into(),
        ));
    }

    let outcome = state
        .near
        .rpc
        .transaction_status(&body.tx_hash, &body.signer_account_id)
        .await
        .map_err(map_rpc_error)?;
    if !outcome.succeeded_finally() {
        return Err(ApiError::Conflict(
            "NEAR transaction is not finalized successfully.".into(),
        ));
    }
    let transaction = outcome
        .transaction
        .as_object()
        .ok_or_else(|| ApiError::Unavailable("NEAR RPC omitted transaction details".into()))?;
    if transaction.get("hash").and_then(Value::as_str) != Some(body.tx_hash.as_str())
        || transaction.get("signer_id").and_then(Value::as_str) != Some(owner.as_str())
        || transaction.get("receiver_id").and_then(Value::as_str) != Some(contract_id.as_str())
    {
        return Err(ApiError::Conflict(
            "NEAR transaction identity does not match this mandate.".into(),
        ));
    }
    let actions = transaction
        .get("actions")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::Unavailable("NEAR RPC omitted transaction actions".into()))?;
    let function_call = actions
        .iter()
        .find_map(|action| action.get("FunctionCall"))
        .ok_or_else(|| {
            ApiError::Conflict("transaction did not call the mandate contract".into())
        })?;
    if function_call.get("method_name").and_then(Value::as_str) != Some("create_mandate") {
        return Err(ApiError::Conflict(
            "transaction called the wrong contract method".into(),
        ));
    }
    let encoded_args = function_call
        .get("args")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::Unavailable("NEAR RPC omitted function arguments".into()))?;
    let actual_args: Value = serde_json::from_slice(
        &BASE64
            .decode(encoded_args)
            .map_err(|_| ApiError::Unavailable("NEAR RPC returned invalid arguments".into()))?,
    )
    .map_err(|_| ApiError::Unavailable("NEAR RPC returned invalid arguments".into()))?;
    if actual_args != expected_args {
        return Err(ApiError::Conflict(
            "on-chain mandate arguments do not match the reviewed draft".into(),
        ));
    }

    sqlx::query(
        "UPDATE near_agent_mandates
         SET status='anchored',anchor_tx_hash=$1,anchored_at=now(),updated_at=now()
         WHERE id=$2 AND user_id=$3 AND status='draft'",
    )
    .bind(&body.tx_hash)
    .bind(*id)
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    Ok(HttpResponse::Ok().json(json!({
        "id": *id,
        "status": "anchored",
        "anchor_tx_hash": body.tx_hash,
        "anchor_verified": true,
        "as_of": Utc::now(),
    })))
}

async fn list_mandates(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let user = authenticate(&req, &state.pool).await?;
    let rows = sqlx::query(
        "SELECT id,shadow_strategy_id,owner_near_account,agent_near_account,mode,status,
                allowed_chains,allowed_assets,max_trade_usd,max_daily_usd,max_slippage_bps,
                expires_at,commitment,near_contract_id,anchor_tx_hash,anchored_at,created_at
         FROM near_agent_mandates WHERE user_id=$1 ORDER BY created_at DESC,id",
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<Uuid,_>("id"),
                "shadow_strategy_id": row.get::<Uuid,_>("shadow_strategy_id"),
                "owner_near_account": row.get::<String,_>("owner_near_account"),
                "agent_near_account": row.get::<String,_>("agent_near_account"),
                "mode": row.get::<String,_>("mode"),
                "status": row.get::<String,_>("status"),
                "allowed_chains": row.get::<Vec<String>,_>("allowed_chains"),
                "allowed_assets": row.get::<Value,_>("allowed_assets"),
                "max_trade_usd": row.get::<Decimal,_>("max_trade_usd").to_string(),
                "max_daily_usd": row.get::<Decimal,_>("max_daily_usd").to_string(),
                "max_slippage_bps": row.get::<i32,_>("max_slippage_bps"),
                "expires_at": row.get::<DateTime<Utc>,_>("expires_at"),
                "commitment": row.get::<String,_>("commitment"),
                "near_contract_id": row.get::<Option<String>,_>("near_contract_id"),
                "anchor_tx_hash": row.get::<Option<String>,_>("anchor_tx_hash"),
                "anchored_at": row.get::<Option<DateTime<Utc>>,_>("anchored_at"),
                "created_at": row.get::<DateTime<Utc>,_>("created_at"),
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(json!({
        "data_status": if items.is_empty() { "unavailable" } else { "partial" },
        "as_of": Utc::now(),
        "demo": false,
        "items": items,
        "coverage_note": "Draft mandates are not on-chain authority. Only a verified anchor transaction can make one active.",
    })))
}

#[derive(Deserialize)]
pub(crate) struct PreviewBody {
    pub mandate_id: Uuid,
    pub quote: QuoteRequest,
}

async fn preview_intent(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<PreviewBody>,
) -> Result<HttpResponse, ApiError> {
    let user = authenticate(&req, &state.pool).await?;
    preview_for_user(&state, user.user_id, body.into_inner(), None).await
}

pub(crate) async fn preview_for_user(
    state: &AppState,
    user_id: Uuid,
    body: PreviewBody,
    agent_connection_id: Option<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let row = sqlx::query(
        "SELECT status,allowed_assets,max_trade_usd,max_slippage_bps,expires_at
         FROM near_agent_mandates WHERE id=$1 AND user_id=$2",
    )
    .bind(body.mandate_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let status: String = row.get("status");
    if matches!(status.as_str(), "revoked" | "expired") {
        return Err(ApiError::Conflict(format!("mandate is {status}")));
    }
    let expires_at: DateTime<Utc> = row.get("expires_at");
    if expires_at <= Utc::now() {
        return Err(ApiError::Conflict("mandate has expired".into()));
    }
    if body.quote.deadline > Utc::now() + Duration::minutes(MAX_QUOTE_DEADLINE_MINUTES) {
        return Err(ApiError::BadRequest(format!(
            "quote deadline must be within {MAX_QUOTE_DEADLINE_MINUTES} minutes."
        )));
    }
    let permissions: BTreeSet<AssetPermission> =
        serde_json::from_value(row.get::<Value, _>("allowed_assets"))
            .map_err(|error| ApiError::Internal(error.into()))?;
    let allowed: HashSet<&str> = permissions
        .iter()
        .map(|value| value.asset_id.as_str())
        .collect();
    if !allowed.contains(body.quote.origin_asset.as_str())
        || !allowed.contains(body.quote.destination_asset.as_str())
    {
        return Err(ApiError::BadRequest(
            "origin and destination assets must both be allowed by the mandate.".into(),
        ));
    }
    let max_slippage: i32 = row.get("max_slippage_bps");
    if i32::from(body.quote.slippage_tolerance) > max_slippage {
        return Err(ApiError::BadRequest(
            "quote slippage exceeds the mandate limit.".into(),
        ));
    }

    let preview = state
        .near
        .intents
        .preview(&body.quote)
        .await
        .map_err(map_intents_error)?;
    let amount_in_usd = Decimal::from_str(&preview.quote.amount_in_usd)
        .map_err(|_| ApiError::Unavailable("NEAR Intents returned an invalid USD amount".into()))?;
    let max_trade: Decimal = row.get("max_trade_usd");
    if amount_in_usd > max_trade {
        return Err(ApiError::BadRequest(
            "quoted input value exceeds the mandate trade limit.".into(),
        ));
    }
    let id = Uuid::new_v4();
    let request_value =
        serde_json::to_value(&body.quote).map_err(|error| ApiError::Internal(error.into()))?;
    let response_value =
        serde_json::to_value(&preview).map_err(|error| ApiError::Internal(error.into()))?;
    sqlx::query(
        "INSERT INTO near_intent_previews
         (id,user_id,mandate_id,correlation_id,request,response,response_signature,agent_connection_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(id)
    .bind(user_id)
    .bind(body.mandate_id)
    .bind(&preview.correlation_id)
    .bind(request_value)
    .bind(response_value)
    .bind(&preview.signature)
    .bind(agent_connection_id)
    .execute(&state.pool)
    .await?;

    Ok(HttpResponse::Ok().json(json!({
        "data_status": "live",
        "as_of": Utc::now(),
        "demo": false,
        "preview_id": id,
        "source": "near_intents_1click",
        "execution": "not_submitted",
        "owner_signature_required": true,
        "quote": preview,
        "note": "This is a dry quote. TRACE did not sign, deposit or submit funds.",
    })))
}

fn map_intents_error(error: IntentsError) -> ApiError {
    match error {
        IntentsError::InvalidAmount
        | IntentsError::InvalidSlippage
        | IntentsError::InvalidDeadline => ApiError::BadRequest(error.to_string()),
        IntentsError::Request(_) | IntentsError::Upstream { .. } => {
            tracing::warn!(error = ?error, "NEAR Intents unavailable");
            ApiError::Unavailable("NEAR Intents quote service is unavailable".into())
        }
    }
}

fn map_rpc_error(error: NearRpcError) -> ApiError {
    tracing::warn!(error = ?error, "NEAR RPC unavailable while verifying mandate anchor");
    ApiError::Unavailable("NEAR RPC could not verify the transaction".into())
}
