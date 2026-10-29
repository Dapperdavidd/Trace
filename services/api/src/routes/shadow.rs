use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use trace_domain::{Attribution, DataStatus, Finality};
use trace_shadow::{simulate, ExitRule, LeaderTrade, Quote, QuoteProvider, SizeRule, Strategy};
use uuid::Uuid;

use super::common::*;
use crate::{
    analysis::{analyze_wallet, interpret},
    auth::authenticate,
    error::ApiError,
    state::AppState,
};

pub const MAX_STRATEGIES_PER_USER: i64 = 50;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/shadow-strategies", web::post().to(create))
        .route("/shadow-strategies", web::get().to(list))
        .route("/shadow-strategies/{id}/results", web::get().to(results));
}

#[derive(Deserialize)]
struct WalletBody {
    chain: String,
    address: String,
}

#[derive(Deserialize)]
struct CreateBody {
    wallet: WalletBody,
    /// A decimal string or a number.
    budget_usd: Value,
    size_rule: String,
    delay_seconds: i64,
    slippage_bps: i64,
    exit_rule: String,
}

fn parse_budget(v: &Value) -> Result<Decimal, ApiError> {
    let d = match v {
        Value::String(s) => Decimal::from_str(s.trim()),
        Value::Number(n) => Decimal::from_str(&n.to_string()),
        _ => {
            return Err(ApiError::BadRequest(
                "budget_usd must be a decimal string.".into(),
            ))
        }
    }
    .map_err(|_| ApiError::BadRequest("budget_usd is not a valid amount.".into()))?;
    if d <= Decimal::ZERO || d > Decimal::from(1_000_000) {
        return Err(ApiError::BadRequest(
            "budget_usd must be above 0 and at most 1,000,000.".into(),
        ));
    }
    Ok(d)
}

/// Rules the paper engine can model. Anything else is refused, not approximated.
fn size_rule(id: &str, budget: Decimal) -> Result<SizeRule, ApiError> {
    match id {
        "fixed_pct" => Ok(SizeRule::PercentOfBudget(Decimal::from(5))),
        "fixed_usd" => Ok(SizeRule::FixedUsd((budget / Decimal::from(20)).round_dp(2))),
        _ => Err(ApiError::BadRequest(
            "size_rule must be fixed_usd or fixed_pct. Other rules are not modelled yet.".into(),
        )),
    }
}

fn exit_rule(id: &str) -> Result<ExitRule, ApiError> {
    match id {
        "mirror_sells" => Ok(ExitRule::MirrorSell),
        "time_24h" => Ok(ExitRule::FixedHorizon(86_400)),
        "time_exit_7d" => Ok(ExitRule::FixedHorizon(604_800)),
        _ => Err(ApiError::BadRequest("exit_rule must be mirror_sells, time_24h or time_exit_7d. Other rules are not modelled yet.".into())),
    }
}

async fn create(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateBody>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let w = wallet_id(&body.wallet.chain, &body.wallet.address)?;
    let budget = parse_budget(&body.budget_usd)?;
    size_rule(&body.size_rule, budget)?;
    exit_rule(&body.exit_rule)?;
    if !(0..=3_600).contains(&body.delay_seconds) {
        return Err(ApiError::BadRequest(
            "delay_seconds must be between 0 and 3600.".into(),
        ));
    }
    if !(0..=2_000).contains(&body.slippage_bps) {
        return Err(ApiError::BadRequest(
            "slippage_bps must be between 0 and 2000.".into(),
        ));
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM shadow_strategies WHERE user_id = $1")
        .bind(u.user_id)
        .fetch_one(&state.pool)
        .await?;
    if n >= MAX_STRATEGIES_PER_USER {
        return Err(ApiError::LimitReached(format!(
            "At most {MAX_STRATEGIES_PER_USER} paper strategies per account."
        )));
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO shadow_strategies (id, user_id, chain, address, params) VALUES ($1,$2,$3,$4,$5)")
        .bind(id)
        .bind(u.user_id)
        .bind(w.chain.as_str())
        .bind(w.address.as_str())
        .bind(json!({
            "budget_usd": dec_str(budget), "size_rule": body.size_rule, "delay_seconds": body.delay_seconds,
            "slippage_bps": body.slippage_bps, "exit_rule": body.exit_rule,
        }))
        .execute(&state.pool)
        .await?;
    Ok(HttpResponse::Created().json(json!({ "id": id })))
}

async fn list(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let rows = sqlx::query("SELECT id, chain, address, params, created_at FROM shadow_strategies WHERE user_id = $1 ORDER BY created_at DESC, id")
        .bind(u.user_id)
        .fetch_all(&state.pool)
        .await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|r| json!({
            "id": r.get::<Uuid, _>("id"),
            "wallet": { "chain": r.get::<String, _>("chain"), "address": r.get::<String, _>("address"), "alias": null },
            "params": r.get::<Value, _>("params"),
            "created_at": r.get::<DateTime<Utc>, _>("created_at"),
        }))
        .collect();
    Ok(HttpResponse::Ok().json(json!({ "items": items })))
}

/// Quotes from stored price points. Liquidity is the latest snapshot value for
/// the token; with none, the order cannot be sized and is skipped as
/// insufficient liquidity.
struct StoredQuotes {
    by_token: BTreeMap<String, Vec<Quote>>,
}

impl QuoteProvider for StoredQuotes {
    fn first_at_or_after(&self, token: &str, t: DateTime<Utc>) -> Option<Quote> {
        self.by_token
            .get(token)?
            .iter()
            .find(|q| q.at >= t)
            .cloned()
    }
    fn latest_at_or_before(&self, token: &str, t: DateTime<Utc>) -> Option<Quote> {
        self.by_token
            .get(token)?
            .iter()
            .rev()
            .find(|q| q.at <= t)
            .cloned()
    }
}

async fn results(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let u = authenticate(&req, &state.pool).await?;
    let row = sqlx::query(
        "SELECT chain, address, params FROM shadow_strategies WHERE id = $1 AND user_id = $2",
    )
    .bind(id.into_inner())
    .bind(u.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let w = wallet_id(
        &row.get::<String, _>("chain"),
        &row.get::<String, _>("address"),
    )?;
    let p: Value = row.get("params");
    let budget = parse_budget(&p["budget_usd"])?;
    let strategy = Strategy {
        budget_usd: budget,
        size_rule: size_rule(p["size_rule"].as_str().unwrap_or_default(), budget)?,
        delay_seconds: p["delay_seconds"].as_i64().unwrap_or(0),
        slippage_bps: p["slippage_bps"].as_u64().unwrap_or(0) as u32,
        fee_bps: 30,
        exit_rule: exit_rule(p["exit_rule"].as_str().unwrap_or_default())?,
        max_entry_wait_seconds: 3_600,
        max_liquidity_fraction: Decimal::new(2, 2),
    };

    let a = analyze_wallet(&state.pool, &w).await?;
    let now = Utc::now();
    let mut leader = vec![];
    let (mut not_final, mut unpriced) = (0usize, 0usize);
    for s in a
        .swaps
        .iter()
        .filter(|s| s.attribution == Attribution::Direct)
    {
        if s.finality != Finality::Final {
            not_final += 1;
            continue;
        }
        let Some(t) = interpret(s, &a.prices) else {
            continue;
        };
        let Some(px) = t.unit_price_usd else {
            unpriced += 1;
            continue;
        };
        leader.push(LeaderTrade {
            tx_id: s.tx_id.clone(),
            token: t.token.contract_or_mint.to_string(),
            side: t.side,
            detected_at: s.observed_at,
            leader_price_usd: px,
        });
    }

    // Quote series for every token the leader traded.
    let tokens: BTreeSet<_> = leader.iter().map(|t| t.token.clone()).collect();
    let mut by_token: BTreeMap<String, Vec<Quote>> = BTreeMap::new();
    for t in tokens {
        let asset = trace_domain::AssetId::parse(w.chain, &t)?;
        let single: BTreeSet<_> = [asset.clone()].into();
        let prices = crate::analysis::DbPrices::load(&state.pool, &single).await?;
        let liq: Option<Decimal> = sqlx::query_scalar(
            "SELECT liquidity_usd FROM market_snapshots WHERE chain=$1 AND token=$2 ORDER BY last_updated DESC LIMIT 1",
        )
        .bind(w.chain.as_str())
        .bind(&t)
        .fetch_optional(&state.pool)
        .await?;
        let qs: Vec<Quote> = sqlx::query(
            "SELECT observed_at FROM price_points WHERE chain=$1 AND asset=$2 ORDER BY observed_at",
        )
        .bind(w.chain.as_str())
        .bind(&t)
        .fetch_all(&state.pool)
        .await?
        .iter()
        .filter_map(|r| {
            let at: DateTime<Utc> = r.get("observed_at");
            prices.unit_price(&asset, at).map(|price_usd| Quote {
                at,
                price_usd,
                liquidity_usd: liq.unwrap_or(Decimal::ZERO),
            })
        })
        .collect();
        by_token.insert(t, qs);
    }

    let sim = simulate(&strategy, &leader, &StoredQuotes { by_token }, now);
    let closes = sim.wins + sim.losses;
    let win_rate = (closes > 0).then(|| {
        dec_str(
            (Decimal::from(sim.wins as u64) / Decimal::from(closes as u64) * Decimal::from(100))
                .round_dp(1),
        )
    });

    let (status, demo) = if a.swaps.is_empty() {
        (DataStatus::Unavailable, false)
    } else if a.demo {
        (DataStatus::Demo, true)
    } else {
        (DataStatus::Partial, false)
    };
    let mut notes = vec!["Paper simulation only. Nothing was bought or sold.".to_string()];
    if a.swaps.is_empty() {
        notes.push("This wallet has no observed swaps, so there is nothing to simulate.".into());
    }
    if not_final > 0 {
        notes.push(format!(
            "{not_final} tentative trades were left out until they finalize."
        ));
    }
    if unpriced > 0 {
        notes.push(format!(
            "{unpriced} trades could not be priced and were not simulated."
        ));
    }
    let env = Envelope::new(status, now, demo).note(notes.join(" "));
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["starting_usd"] = json!(dec_str(sim.starting_usd));
    body["ending_usd"] = json!(dec_str(sim.ending_usd.round_dp(2)));
    body["return_pct"] = json!(dec_str(sim.return_pct.round_dp(2)));
    body["trades"] = json!(sim.filled_entries);
    body["win_rate_pct"] = json!(win_rate);
    body["skipped_unexecutable"] = json!(sim.skipped.len());
    body["skipped"] = serde_json::to_value(&sim.skipped).unwrap_or(Value::Null);
    body["fees_and_slippage_usd"] = json!(dec_str(sim.fees_and_slippage_usd.round_dp(2)));
    body["open_positions"] = json!(sim.open_positions);
    body["unmarked_open_positions"] = json!(sim.unmarked_open_positions);
    let ideal = sim
        .baseline
        .ideal_leader_execution_return_pct
        .map(|d| dec_str(d.round_dp(2)));
    body["baseline"] = json!({
        "label": "Ideal leader execution (comparison only)",
        "return_pct": ideal,
        "ideal_leader_execution_return_pct": ideal,
        "note": "Comparison only. Followers cannot get the leader's exact prices.",
    });
    body["assumptions"] = json!({
        "delay_seconds": sim.assumptions.delay_seconds,
        "slippage_bps": sim.assumptions.slippage_bps,
        "fee_bps": sim.assumptions.fee_bps,
        "max_entry_wait_seconds": sim.assumptions.max_entry_wait_seconds,
        "max_liquidity_fraction": dec_str(sim.assumptions.max_liquidity_fraction),
        "entry_rule": sim.assumptions.entry_rule,
        "liquidity_source": "latest market snapshot",
    });
    Ok(HttpResponse::Ok().json(body))
}
