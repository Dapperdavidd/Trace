use actix_web::{web, HttpResponse};
use chrono::Utc;

use serde::Deserialize;
use serde_json::{json, Value};

use std::collections::BTreeSet;
use trace_domain::{DataStatus, Finality, WalletId};
use trace_intelligence::{dna, wcs, wqs};
use trace_shadow::Side;

use super::common::*;
use crate::{
    analysis::{analyze_wallet, interpret, WalletAnalysis},
    error::ApiError,
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/wallets/{chain}/{address}", web::get().to(overview))
        .route("/wallets/{chain}/{address}/trades", web::get().to(trades))
        .route("/wallets/{chain}/{address}/metrics", web::get().to(metrics))
        .route("/wallets/{chain}/{address}/dna", web::get().to(dna_route));
}

type P = web::Path<(String, String)>;

/// Envelope for a wallet: demo if any swap is demo, unavailable with no swaps,
/// otherwise as fresh as the chain's last successful scan.
async fn wallet_envelope(
    state: &AppState,
    w: &WalletId,
    a: &WalletAnalysis,
) -> Result<Envelope, ApiError> {
    let now = Utc::now();
    if a.swaps.is_empty() {
        return Ok(Envelope::new(DataStatus::Unavailable, now, false)
            .note("No swaps observed for this wallet. Any valid address can be watched; history appears once activity is ingested."));
    }
    if a.demo {
        return Ok(
            Envelope::new(DataStatus::Demo, now, true).note("Seeded demo data. Not real activity.")
        );
    }
    let scan = chain_freshness(&state.pool, w.chain.as_str()).await?;
    Ok(match scan {
        Some(t) => Envelope::new(status_for(false, Some(t), now, 900), t, false),
        None => Envelope::new(DataStatus::Partial, now, false)
            .note("No successful scan time is recorded for this chain."),
    })
}

fn metrics_json(a: &WalletAnalysis) -> Value {
    let acc = &a.accounting;
    json!({
        "realized_pnl_usd": acc.realized_pnl_usd.map(dec_str),
        "unrealized_pnl_usd": acc.unrealized_pnl_usd.map(dec_str),
        "total_trades": acc.credited_swaps,
        "coverage_ratio": acc.coverage_ratio.map(dec_str),
        "pnl_confidence": acc.pnl_confidence,
        "unresolved_disposals": acc.unresolved_disposals,
    })
}

async fn overview(state: web::Data<AppState>, p: P) -> Result<HttpResponse, ApiError> {
    let (chain, address) = p.into_inner();
    let w = wallet_id(&chain, &address)?;
    let a = analyze_wallet(&state.pool, &w).await?;
    let env = wallet_envelope(&state, &w, &a).await?;
    let row = sqlx::query(
        "SELECT first_seen_at, last_seen_at FROM wallets WHERE chain=$1 AND address=$2",
    )
    .bind(w.chain.as_str())
    .bind(w.address.as_str())
    .fetch_optional(&state.pool)
    .await?;
    let seen = a
        .swaps
        .first()
        .map(|s| s.observed_at)
        .zip(a.swaps.last().map(|s| s.observed_at));
    let (first_seen, last_seen) = (seen.map(|s| s.0), seen.map(|s| s.1));
    let enabled: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM protocols WHERE chain=$1 AND support_status IN ('enabled','partial'))",
    )
    .bind(w.chain.as_str())
    .fetch_one(&state.pool)
    .await?;
    let monitoring = if !enabled {
        "unsupported_protocol"
    } else if a.swaps.is_empty() {
        "waiting_for_activity"
    } else {
        "active"
    };
    let _ = row;
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["wallet"] =
        json!({ "chain": w.chain.as_str(), "address": w.address.as_str(), "alias": null });
    body["first_seen"] = json!(first_seen);
    body["last_seen"] = json!(last_seen);
    body["monitoring"] = json!(monitoring);
    body["metrics"] = metrics_json(&a);
    Ok(HttpResponse::Ok().json(body))
}

#[derive(Deserialize)]
struct TradesQuery {
    cursor: Option<String>,
    limit: Option<u32>,
}

async fn trades(
    state: web::Data<AppState>,
    p: P,
    q: web::Query<TradesQuery>,
) -> Result<HttpResponse, ApiError> {
    let (chain, address) = p.into_inner();
    let w = wallet_id(&chain, &address)?;
    let a = analyze_wallet(&state.pool, &w).await?;
    let env = wallet_envelope(&state, &w, &a).await?;
    let limit = page_limit(q.limit) as usize;
    let offset: usize = match q.cursor.as_deref() {
        None => 0,
        Some(c) => c
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid cursor.".into()))?,
    };
    // Only swaps credited to this wallet, newest first.
    let mut credited: Vec<_> = a
        .swaps
        .iter()
        .filter(|s| s.attribution == trace_domain::Attribution::Direct)
        .filter_map(|s| interpret(s, &a.prices).map(|t| (s, t)))
        .collect();
    credited.reverse();
    let total = credited.len();
    let addrs: Vec<String> = credited
        .iter()
        .map(|(_, t)| t.token.contract_or_mint.to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let meta = token_meta(&state.pool, w.chain.as_str(), &addrs).await?;
    let items: Vec<Value> = credited
        .iter()
        .skip(offset)
        .take(limit)
        .map(|(s, t)| {
            let (sym, name) = meta.get(t.token.contract_or_mint.as_str()).cloned().unwrap_or((None, None));
            json!({
                "tx_id": s.tx_id,
                "event_time": s.observed_at,
                "side": if t.side == Side::Buy { "buy" } else { "sell" },
                "token": token_ref(w.chain.as_str(), t.token.contract_or_mint.as_str(), sym.as_deref(), name.as_deref()),
                "amount_usd": t.amount_usd.map(|d| dec_str(d.round_dp(2))),
                "finality": if s.finality == Finality::Final { "final" } else { "tentative" },
            })
        })
        .collect();
    let excluded = a.swaps.len()
        - a.swaps
            .iter()
            .filter(|s| s.attribution == trace_domain::Attribution::Direct)
            .count();
    let mut env = env;
    if excluded > 0 {
        let n = format!("{excluded} routed or ambiguous swaps are not credited to this wallet.");
        let note = match env.coverage_note.take() {
            Some(x) => format!("{x} {n}"),
            None => n,
        };
        env = env.note(note);
    }
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["items"] = Value::Array(items);
    body["next_cursor"] = json!((offset + limit < total).then(|| (offset + limit).to_string()));
    Ok(HttpResponse::Ok().json(body))
}

async fn metrics(state: web::Data<AppState>, p: P) -> Result<HttpResponse, ApiError> {
    let (chain, address) = p.into_inner();
    let w = wallet_id(&chain, &address)?;
    let a = analyze_wallet(&state.pool, &w).await?;
    let env = wallet_envelope(&state, &w, &a).await?;
    let now = Utc::now();
    let score = wqs::score(
        &wqs::WqsConfig::default(),
        &a.accounting.closed_lots,
        a.accounting.coverage_ratio,
        "all_wallets",
        now,
    );
    // WCS needs followed-trade quotes from the shadow flow; none exist for a bare wallet view.
    let _ = wcs::VERSION;
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    if let (Value::Object(b), Value::Object(m)) = (&mut body, metrics_json(&a)) {
        b.extend(m);
    }
    body["wqs"] = serde_json::to_value(&score).unwrap_or(Value::Null);
    body["wcs"] = Value::Null;
    Ok(HttpResponse::Ok().json(body))
}

async fn dna_route(state: web::Data<AppState>, p: P) -> Result<HttpResponse, ApiError> {
    let (chain, address) = p.into_inner();
    let w = wallet_id(&chain, &address)?;
    let a = analyze_wallet(&state.pool, &w).await?;
    let env = wallet_envelope(&state, &w, &a).await?;
    let d = dna::analyze(&a.accounting.closed_lots);
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["state"] = serde_json::to_value(d.state).unwrap_or(Value::Null);
    body["rules_version"] = json!(d.rules_version);
    body["sample_size"] = json!(d.sample_size);
    body["traits"] = serde_json::to_value(&d.traits).unwrap_or(Value::Null);
    Ok(HttpResponse::Ok().json(body))
}
