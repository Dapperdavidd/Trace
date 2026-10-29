use actix_web::{web, HttpResponse};
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;

use super::common::*;
use crate::{
    analysis::{load_token_snapshot, SnapshotRow},
    error::ApiError,
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/tokens/compare", web::get().to(compare))
        .route("/tokens/{chain}/{address}", web::get().to(token))
        .route(
            "/tokens/{chain}/{address}/timeline",
            web::get().to(timeline),
        );
}

/// Market data older than this is reported stale.
const MAX_AGE_SECS: i64 = 600;

fn snapshot_envelope(r: &SnapshotRow, now: chrono::DateTime<Utc>) -> Envelope {
    Envelope::new(
        status_for(r.demo, Some(r.snap.last_updated), now, MAX_AGE_SECS),
        r.snap.last_updated,
        r.demo,
    )
}

fn change_pct(r: &SnapshotRow) -> Option<Decimal> {
    let s = &r.snap;
    (s.price_window_start_usd > Decimal::ZERO).then(|| {
        ((s.price_now_usd - s.price_window_start_usd) / s.price_window_start_usd
            * Decimal::from(100))
        .round_dp(2)
    })
}

fn risk_json(r: &SnapshotRow) -> Value {
    json!({
        "overall": r.risk.overall,
        "checks": r.risk.checks.iter().map(|c| json!({ "key": c.key, "state": c.state, "note": c.note })).collect::<Vec<_>>(),
        "fetched_at": r.risk.fetched_at,
        "policy_version": r.risk.policy_version,
    })
}

async fn recent_buys(
    state: &AppState,
    chain: &str,
    token: &str,
    hours: i64,
) -> Result<Vec<trace_domain::NormalizedSwap>, ApiError> {
    trace_database::reads::token_buys(
        &state.pool,
        chain,
        token,
        Utc::now() - Duration::hours(hours),
        200,
    )
    .await
    .map_err(|e| ApiError::Internal(e.into()))
}

async fn token(
    state: web::Data<AppState>,
    p: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (chain, address) = p.into_inner();
    let a = asset_id(&chain, &address)?;
    let now = Utc::now();
    let row = load_token_snapshot(
        &state.pool,
        a.chain.as_str(),
        a.contract_or_mint.as_str(),
        now,
    )
    .await?
    .ok_or(ApiError::NotFound)?;
    let s = &row.snap;
    let is24 = true; // load_token_snapshot prefers the 24h window; the label below states the window used
    let _ = is24;
    let buys = recent_buys(&state, a.chain.as_str(), a.contract_or_mint.as_str(), 24).await?;
    let tx_ids: Vec<String> = buys.iter().take(10).map(|b| b.tx_id.clone()).collect();
    let buyer_set: BTreeSet<String> = buys.iter().map(|b| b.wallet.address.to_string()).collect();

    let mut why = vec![];
    if let Some(c) = change_pct(&row) {
        why.push(json!({ "kind": "fact", "text": format!("Price changed {}% over the snapshot window.", dec_str(c)), "evidence_ids": [] }));
    }
    if let Some(b) = s.volume_baseline_usd.filter(|b| *b > Decimal::ZERO) {
        why.push(json!({ "kind": "fact", "text": format!("Volume in the window was {}x its baseline.", dec_str((s.volume_window_usd / b).round_dp(1))), "evidence_ids": [] }));
    }
    if s.independent_smart_buyers > 0 {
        why.push(json!({ "kind": "fact", "text": format!("{} independent smart wallets bought in the window.", s.independent_smart_buyers), "evidence_ids": tx_ids }));
    }
    if s.independent_smart_buyers >= 2 && change_pct(&row).is_some_and(|c| c > Decimal::ZERO) {
        why.push(json!({ "kind": "hypothesis", "text": "Buying by several independent smart wallets may be contributing to the move. This is a hypothesis, not a confirmed cause.", "evidence_ids": tx_ids }));
    }
    let buyers: Vec<Value> = buyer_set
        .iter()
        .take(10)
        .map(|b| json!({ "chain": a.chain.as_str(), "address": b, "alias": null }))
        .collect();

    let mut body = serde_json::to_value(snapshot_envelope(&row, now)).unwrap_or(Value::Null);
    body["token"] = token_ref(
        a.chain.as_str(),
        a.contract_or_mint.as_str(),
        s.symbol.as_deref(),
        row.name.as_deref(),
    );
    body["price_usd"] = json!(dec_str(s.price_now_usd));
    body["change_pct_24h"] = json!(change_pct(&row).map(dec_str));
    body["market_cap_usd"] = json!(row.market_cap_usd.map(dec_str));
    body["volume_24h_usd"] = json!(dec_str(s.volume_window_usd));
    body["liquidity_usd"] = json!(dec_str(s.liquidity_usd));
    body["holders"] = json!(row.holders);
    body["risk"] = risk_json(&row);
    body["why_moving"] = Value::Array(why);
    body["buyers"] = Value::Array(buyers);
    Ok(HttpResponse::Ok().json(body))
}

async fn timeline(
    state: web::Data<AppState>,
    p: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
    let (chain, address) = p.into_inner();
    let a = asset_id(&chain, &address)?;
    let now = Utc::now();
    let row = load_token_snapshot(
        &state.pool,
        a.chain.as_str(),
        a.contract_or_mint.as_str(),
        now,
    )
    .await?;
    let buys = recent_buys(&state, a.chain.as_str(), a.contract_or_mint.as_str(), 72).await?;
    let sym = row
        .as_ref()
        .and_then(|r| r.snap.symbol.clone())
        .unwrap_or_else(|| "this token".into());
    let mut items: Vec<Value> = buys
        .iter()
        .take(30)
        .map(|b| {
            let w = b.wallet.address.to_string();
            let short = format!("{}...{}", &w[..w.len().min(6)], &w[w.len().saturating_sub(4)..]);
            json!({
                "time": b.observed_at,
                "kind": "fact",
                "title": format!("Wallet {short} bought {sym}"),
                "detail": format!("Swap observed on {} ({}).", b.protocol, if b.finality == trace_domain::Finality::Final { "final" } else { "tentative" }),
                "evidence_ids": [b.tx_id],
                "possible_impact": null,
            })
        })
        .collect();
    if let Some(r) = &row {
        if r.snap.independent_smart_buyers >= 2 {
            items.push(json!({
                "time": r.snap.last_updated,
                "kind": "hypothesis",
                "title": "Several independent smart wallets are buying",
                "detail": "This clustering may add buying pressure. It is a hypothesis and does not predict price.",
                "evidence_ids": buys.iter().take(5).map(|b| b.tx_id.clone()).collect::<Vec<_>>(),
                "possible_impact": "May support price while buying continues.",
            }));
        }
    }
    items.sort_by(|a, b| b["time"].as_str().cmp(&a["time"].as_str()));
    let (status, as_of, demo) = match &row {
        Some(r) => (
            snapshot_envelope(r, now).data_status,
            r.snap.last_updated,
            r.demo,
        ),
        None if buys.is_empty() => (trace_domain::DataStatus::Unavailable, now, false),
        None => (
            trace_domain::DataStatus::Partial,
            now,
            buys.iter().any(|b| b.demo),
        ),
    };
    let mut env = Envelope::new(status, as_of, demo);
    if items.is_empty() {
        env = env.note("No events observed for this token yet.");
    }
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["items"] = Value::Array(items);
    Ok(HttpResponse::Ok().json(body))
}

#[derive(Deserialize)]
struct CompareQuery {
    a: Option<String>,
    b: Option<String>,
    c: Option<String>,
    d: Option<String>,
}

async fn compare(
    state: web::Data<AppState>,
    q: web::Query<CompareQuery>,
) -> Result<HttpResponse, ApiError> {
    let refs: Vec<&String> = [&q.a, &q.b, &q.c, &q.d].into_iter().flatten().collect();
    if refs.len() < 2 {
        return Err(ApiError::BadRequest(
            "Provide at least two tokens as a and b, each as chain:address.".into(),
        ));
    }
    let now = Utc::now();
    let mut items = vec![];
    let mut seen = BTreeSet::new();
    let (mut demo, mut oldest) = (false, now);
    for r in refs {
        let (c, addr) = r
            .split_once(':')
            .ok_or_else(|| ApiError::BadRequest("Tokens are written chain:address.".into()))?;
        let a = asset_id(c, addr)?;
        if !seen.insert(a.clone()) {
            return Err(ApiError::BadRequest(
                "Each token can be compared once.".into(),
            ));
        }
        let row = load_token_snapshot(
            &state.pool,
            a.chain.as_str(),
            a.contract_or_mint.as_str(),
            now,
        )
        .await?
        .ok_or(ApiError::NotFound)?;
        demo |= row.demo;
        oldest = oldest.min(row.snap.last_updated);
        let s = &row.snap;
        items.push(json!({
            "token": token_ref(a.chain.as_str(), a.contract_or_mint.as_str(), s.symbol.as_deref(), row.name.as_deref()),
            "price_usd": dec_str(s.price_now_usd),
            "change_pct_24h": change_pct(&row).map(dec_str).unwrap_or_default(),
            "market_cap_usd": row.market_cap_usd.map(dec_str),
            "volume_24h_usd": dec_str(s.volume_window_usd),
            "liquidity_usd": dec_str(s.liquidity_usd),
            "risk": row.risk.overall,
            "note": row.risk.checks.iter().find(|c| c.state != trace_risk::RiskState::Pass).map(|c| c.note.clone()),
        }));
    }
    let env = Envelope::new(
        status_for(demo, Some(oldest), now, MAX_AGE_SECS),
        oldest,
        demo,
    );
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["items"] = Value::Array(items);
    body["verdict_note"] =
        json!("This compares facts side by side. It is not a recommendation to buy or sell.");
    Ok(HttpResponse::Ok().json(body))
}
