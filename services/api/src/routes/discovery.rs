use actix_web::{web, HttpResponse};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::BTreeMap;
use trace_discovery::{
    candidates::{classify, summarize, CandidateConfig, CandidateDecision},
    movers::{rank, MoverConfig, Tab},
};
use trace_domain::{DataStatus, WalletId};
use trace_intelligence::{dna, wqs};

use super::common::*;
use crate::{
    analysis::{analyze_wallet, load_snapshots},
    error::ApiError,
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/discovery/movers", web::get().to(movers))
        .route("/discovery/wallets", web::get().to(wallets));
}

#[derive(Deserialize)]
struct MoversQuery {
    tab: Option<String>,
    window: Option<String>,
    chain: Option<String>,
}

fn parse_tab(s: &str) -> Result<Tab, ApiError> {
    Ok(match s {
        "trending" => Tab::Trending,
        "early_momentum" => Tab::EarlyMomentum,
        "smart_money" => Tab::SmartMoney,
        "new" => Tab::New,
        _ => {
            return Err(ApiError::BadRequest(
                "tab must be trending, early_momentum, smart_money or new.".into(),
            ))
        }
    })
}

async fn movers(
    state: web::Data<AppState>,
    q: web::Query<MoversQuery>,
) -> Result<HttpResponse, ApiError> {
    let tab = parse_tab(q.tab.as_deref().unwrap_or("trending"))?;
    let win = q.window.as_deref().unwrap_or("24h");
    if !["1h", "24h", "7d"].contains(&win) {
        return Err(ApiError::BadRequest("window must be 1h, 24h or 7d.".into()));
    }
    let chain = q.chain.as_deref().map(parse_chain).transpose()?;
    let now = Utc::now();
    let rows = load_snapshots(&state.pool, win, chain.map(|c| c.as_str()), now).await?;
    let cfg = MoverConfig::default();
    let snaps: Vec<_> = rows.iter().map(|r| r.snap.clone()).collect();
    let (ranked, excluded) = rank(&cfg, tab, &snaps, now);
    let by_key: BTreeMap<_, _> = rows
        .iter()
        .map(|r| ((r.snap.token.clone(), r.snap.pool.clone()), r))
        .collect();

    let items: Vec<Value> = ranked
        .iter()
        .map(|m| {
            let row = by_key.get(&(m.token.clone(), m.pool.clone()));
            let notes: Vec<String> = row
                .map(|r| {
                    r.risk
                        .checks
                        .iter()
                        .filter(|c| c.state != trace_risk::RiskState::Pass)
                        .map(|c| c.note.clone())
                        .collect()
                })
                .unwrap_or_default();
            json!({
                "rank": m.rank,
                "token": token_ref(m.token.chain.as_str(), m.token.contract_or_mint.as_str(), m.symbol.as_deref(), row.and_then(|r| r.name.as_deref())),
                "price_usd": dec_str(m.price_usd),
                "change_pct": dec_str(m.change_pct.round_dp(2)),
                "volume_usd": dec_str(m.volume_usd),
                "liquidity_usd": dec_str(m.liquidity_usd),
                "pool": m.pool,
                "reason": { "code": serde_json::to_value(m.reason).unwrap_or(Value::Null), "label": m.reason.label() },
                "sparkline": row.map(|r| r.sparkline.clone()).unwrap_or_default(),
                "risk": m.risk,
                "risk_notes": notes,
                "source_timestamp": m.source_timestamp,
            })
        })
        .collect();

    let used_demo = ranked.iter().any(|m| {
        by_key
            .get(&(m.token.clone(), m.pool.clone()))
            .is_some_and(|r| r.demo)
    });
    let newest = rows.iter().map(|r| r.snap.last_updated).max();
    let status = if rows.is_empty() {
        DataStatus::Unavailable
    } else if ranked.is_empty() && excluded.stale == rows.len() {
        DataStatus::Stale
    } else {
        status_for(used_demo, newest, now, cfg.max_age.num_seconds())
    };
    let mut env = Envelope::new(status, newest.unwrap_or(now), used_demo);
    let mut notes = vec![];
    if rows.is_empty() {
        notes.push("No market snapshots are available yet.".to_string());
    }
    for (n, what) in [
        (excluded.stale, "stale"),
        (excluded.thin_liquidity, "thin liquidity"),
        (excluded.invalid_price, "invalid price"),
        (excluded.blocked_risk, "blocked by risk checks"),
    ] {
        if n > 0 {
            notes.push(format!("{n} excluded: {what}."));
        }
    }
    if !notes.is_empty() {
        env = env.note(notes.join(" "));
    }
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["items"] = Value::Array(items);
    Ok(HttpResponse::Ok().json(body))
}

#[derive(Deserialize)]
struct WalletsQuery {
    tab: Option<String>,
    chain: Option<String>,
    cursor: Option<String>,
}

const WALLET_PAGE: usize = 20;

async fn wallets(
    state: web::Data<AppState>,
    q: web::Query<WalletsQuery>,
) -> Result<HttpResponse, ApiError> {
    let tab = q.tab.as_deref().unwrap_or("top");
    if !["top", "new", "most_followed"].contains(&tab) {
        return Err(ApiError::BadRequest(
            "tab must be top, new or most_followed.".into(),
        ));
    }
    let chain = q.chain.as_deref().map(parse_chain).transpose()?;
    let offset: usize = match q.cursor.as_deref() {
        None => 0,
        Some(c) => c
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid cursor.".into()))?,
    };
    let now = Utc::now();
    let active = trace_database::reads::active_wallets(&state.pool, 100)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let followers: BTreeMap<(String, String), i64> = sqlx::query(
        "SELECT chain, address, count(*) AS n FROM watchlist_wallets GROUP BY chain, address",
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| ((r.get("chain"), r.get("address")), r.get("n")))
    .collect();

    struct Entry {
        sort_score: i64,
        first_seen: chrono::DateTime<Utc>,
        followers: i64,
        json: Value,
        demo: bool,
    }
    let mut entries: Vec<Entry> = vec![];
    for (c, addr, _n) in active {
        if chain.is_some_and(|ch| ch.as_str() != c) {
            continue;
        }
        let Ok(w) = WalletId::parse(c.parse()?, &addr) else {
            continue;
        };
        let a = analyze_wallet(&state.pool, &w).await?;
        let activity = summarize(&a.swaps);
        let mut tags: Vec<String> = vec![];
        if let Some(act) = activity.first() {
            match classify(&CandidateConfig::default(), act, &Default::default()) {
                CandidateDecision::ExcludedInfrastructure => continue,
                CandidateDecision::UncertainBot { .. } => tags.push("possible_bot".into()),
                _ => {}
            }
        }
        let score = wqs::score(
            &wqs::WqsConfig::default(),
            &a.accounting.closed_lots,
            a.accounting.coverage_ratio,
            "all_wallets",
            now,
        );
        let d = dna::analyze(&a.accounting.closed_lots);
        tags.extend(d.traits.iter().take(2).map(|t| t.key.clone()));
        let cost: Decimal = a.accounting.closed_lots.iter().map(|l| l.cost_usd).sum();
        let realized_pct = match (a.accounting.realized_pnl_usd, cost > Decimal::ZERO) {
            (Some(p), true) => Some(dec_str((p / cost * Decimal::from(100)).round_dp(2))),
            _ => None,
        };
        let closes = a.accounting.winning_closes + a.accounting.losing_closes;
        let win_rate = (closes > 0).then(|| {
            dec_str(
                (Decimal::from(a.accounting.winning_closes as u64) / Decimal::from(closes as u64)
                    * Decimal::from(100))
                .round_dp(1),
            )
        });
        // A possible bot is never promoted past watch-only.
        let mut decision = serde_json::to_value(score.decision).unwrap_or(Value::Null);
        if tags.iter().any(|t| t == "possible_bot") && decision == "eligible" {
            decision = json!("watch_only");
        }
        let f = followers
            .get(&(c.clone(), addr.clone()))
            .copied()
            .unwrap_or(0);
        entries.push(Entry {
            sort_score: score.value.map(i64::from).unwrap_or(-1),
            first_seen: a.swaps.first().map(|s| s.observed_at).unwrap_or(now),
            followers: f,
            demo: a.demo,
            json: json!({
                "wallet": { "chain": c, "address": addr, "alias": null },
                "realized_return_pct": realized_pct,
                "win_rate_pct": win_rate,
                "sample_size": a.accounting.closed_lots.len(),
                "tags": tags,
                "decision": decision,
                "pnl_confidence": a.accounting.pnl_confidence,
                "coverage_ratio": a.accounting.coverage_ratio.map(dec_str),
            }),
        });
    }
    match tab {
        "new" => entries.sort_by_key(|e| std::cmp::Reverse(e.first_seen)),
        "most_followed" => entries.sort_by(|a, b| {
            b.followers
                .cmp(&a.followers)
                .then(b.sort_score.cmp(&a.sort_score))
        }),
        _ => {
            // Top: only wallets with enough evidence to be scored are ranked.
            entries.retain(|e| e.sort_score >= 0);
            entries.sort_by_key(|e| std::cmp::Reverse(e.sort_score));
        }
    }
    let total = entries.len();
    let page: Vec<&Entry> = entries.iter().skip(offset).take(WALLET_PAGE).collect();
    let next = (offset + WALLET_PAGE < total).then(|| (offset + WALLET_PAGE).to_string());
    let demo = page.iter().any(|e| e.demo);
    let status = if total == 0 {
        DataStatus::Unavailable
    } else if demo {
        DataStatus::Demo
    } else {
        DataStatus::Partial
    };
    let mut env = Envelope::new(status, now, demo);
    env = env.note(if total == 0 {
        "No wallets have enough observed activity yet.".to_string()
    } else {
        "Scores use unvalidated default weights and are not predictions.".to_string()
    });
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["items"] = Value::Array(page.iter().map(|e| e.json.clone()).collect());
    body["next_cursor"] = json!(next);
    Ok(HttpResponse::Ok().json(body))
}
