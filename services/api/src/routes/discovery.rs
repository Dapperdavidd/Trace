use actix_web::{web, HttpResponse};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use trace_discovery::{
    candidates::{classify, summarize, CandidateConfig, CandidateDecision},
    movers::{rank, MoverConfig, Tab},
};
use trace_domain::{DataStatus, WalletId};
use trace_intelligence::{dna, wqs};

use super::common::*;
use crate::{
    analysis::{analyze_loaded, load_snapshots, load_wallet},
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

#[derive(Clone)]
struct DexMover {
    chain: String,
    token: String,
    symbol: Option<String>,
    name: Option<String>,
    pool: String,
    price_usd: Decimal,
    change_pct: Decimal,
    volume_usd: Decimal,
    liquidity_usd: Decimal,
}

fn dex_decimal(value: &Value) -> Option<Decimal> {
    match value {
        Value::String(s) => Decimal::from_str(s).ok(),
        Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        _ => None,
    }
}

fn dex_key(chain: &str, address: &str) -> String {
    if chain == "solana" {
        format!("{chain}:{address}")
    } else {
        format!("{chain}:{}", address.to_ascii_lowercase())
    }
}

fn parse_dex_pairs(
    body: &Value,
    chain: &str,
    requested: &BTreeSet<String>,
    change_key: &str,
    volume_key: &str,
    best: &mut HashMap<String, DexMover>,
) {
    for pair in body.as_array().into_iter().flatten() {
        if pair["chainId"].as_str() != Some(chain) {
            continue;
        }
        let Some(raw_token) = pair["baseToken"]["address"].as_str() else {
            continue;
        };
        let token = if chain == "solana" {
            raw_token.to_string()
        } else {
            raw_token.to_ascii_lowercase()
        };
        if !requested.contains(&token) {
            continue;
        }
        let (Some(price_usd), Some(change_pct), Some(volume_usd), Some(liquidity_usd)) = (
            dex_decimal(&pair["priceUsd"]),
            dex_decimal(&pair["priceChange"][change_key]),
            dex_decimal(&pair["volume"][volume_key]),
            dex_decimal(&pair["liquidity"]["usd"]),
        ) else {
            continue;
        };
        if price_usd <= Decimal::ZERO
            || volume_usd < Decimal::ZERO
            || liquidity_usd < Decimal::from(25_000)
        {
            continue;
        }
        let Some(pool) = pair["pairAddress"].as_str() else {
            continue;
        };
        let candidate = DexMover {
            chain: chain.to_string(),
            token: token.clone(),
            symbol: pair["baseToken"]["symbol"].as_str().map(str::to_string),
            name: pair["baseToken"]["name"].as_str().map(str::to_string),
            pool: pool.to_string(),
            price_usd,
            change_pct,
            volume_usd,
            liquidity_usd,
        };
        let key = dex_key(chain, &token);
        if best
            .get(&key)
            .is_none_or(|current| candidate.liquidity_usd > current.liquidity_usd)
        {
            best.insert(key, candidate);
        }
    }
}

async fn add_trending_pools(
    client: &reqwest::Client,
    win: &str,
    best: &mut HashMap<String, DexMover>,
) {
    let base = std::env::var("TRACE_MARKET_TRENDS_API_BASE")
        .unwrap_or_else(|_| "https://api.geckoterminal.com/api/v2".into());
    for (network, chain) in [("eth", "ethereum"), ("solana", "solana")] {
        let url = format!(
            "{}/networks/{network}/trending_pools?page=1&include=base_token",
            base.trim_end_matches('/')
        );
        let body: Value = match client.get(url).send().await {
            Ok(response) if response.status().is_success() => match response.json().await {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, network, "trending pools response was not JSON");
                    continue;
                }
            },
            Ok(response) => {
                tracing::warn!(status = %response.status(), network, "trending pools request refused");
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, network, "trending pools request failed");
                continue;
            }
        };
        let mut tokens: HashMap<String, (String, Option<String>, Option<String>)> = HashMap::new();
        for token in body["included"].as_array().into_iter().flatten() {
            let (Some(id), Some(address)) = (
                token["id"].as_str(),
                token["attributes"]["address"].as_str(),
            ) else {
                continue;
            };
            tokens.insert(
                id.to_string(),
                (
                    address.to_string(),
                    token["attributes"]["symbol"].as_str().map(str::to_string),
                    token["attributes"]["name"].as_str().map(str::to_string),
                ),
            );
        }
        for pool in body["data"].as_array().into_iter().flatten() {
            let Some(token_id) = pool["relationships"]["base_token"]["data"]["id"].as_str() else {
                continue;
            };
            let Some((raw_address, symbol, name)) = tokens.get(token_id) else {
                continue;
            };
            let attributes = &pool["attributes"];
            let (Some(price_usd), Some(change_pct), Some(volume_usd), Some(liquidity_usd)) = (
                dex_decimal(&attributes["base_token_price_usd"]),
                dex_decimal(&attributes["price_change_percentage"][win]),
                dex_decimal(&attributes["volume_usd"][win]),
                dex_decimal(&attributes["reserve_in_usd"]),
            ) else {
                continue;
            };
            if price_usd <= Decimal::ZERO
                || volume_usd < Decimal::ZERO
                || liquidity_usd < Decimal::from(25_000)
            {
                continue;
            }
            let Some(pool_address) = attributes["address"].as_str() else {
                continue;
            };
            let token = if chain == "solana" {
                raw_address.clone()
            } else {
                raw_address.to_ascii_lowercase()
            };
            let candidate = DexMover {
                chain: chain.to_string(),
                token: token.clone(),
                symbol: symbol.clone(),
                name: name.clone(),
                pool: pool_address.to_string(),
                price_usd,
                change_pct,
                volume_usd,
                liquidity_usd,
            };
            let key = dex_key(chain, &token);
            if best
                .get(&key)
                .is_none_or(|current| candidate.liquidity_usd > current.liquidity_usd)
            {
                best.insert(key, candidate);
            }
        }
    }
}

async fn add_jupiter_trending(
    client: &reqwest::Client,
    win: &str,
    best: &mut HashMap<String, DexMover>,
) {
    let interval = match win {
        "1h" => "1h",
        "24h" => "24h",
        _ => return,
    };
    let base = std::env::var("TRACE_SOLANA_TRENDS_API_BASE")
        .unwrap_or_else(|_| "https://lite-api.jup.ag".into());
    let url = format!(
        "{}/tokens/v2/toptrending/{interval}",
        base.trim_end_matches('/')
    );
    let body: Value = match client.get(url).send().await {
        Ok(response) if response.status().is_success() => match response.json().await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error, "Solana trending response was not JSON");
                return;
            }
        },
        Ok(response) => {
            tracing::warn!(status = %response.status(), "Solana trending request refused");
            return;
        }
        Err(error) => {
            tracing::warn!(%error, "Solana trending request failed");
            return;
        }
    };
    let stats_key = if win == "1h" { "stats1h" } else { "stats24h" };
    for item in body.as_array().into_iter().flatten() {
        let (Some(token), Some(price_usd), Some(liquidity_usd), Some(change_pct)) = (
            item["id"].as_str(),
            dex_decimal(&item["usdPrice"]),
            dex_decimal(&item["liquidity"]),
            dex_decimal(&item[stats_key]["priceChange"]),
        ) else {
            continue;
        };
        let Some(buy_volume) = dex_decimal(&item[stats_key]["buyVolume"]) else {
            continue;
        };
        let Some(sell_volume) = dex_decimal(&item[stats_key]["sellVolume"]) else {
            continue;
        };
        let Some(volume_usd) = buy_volume.checked_add(sell_volume) else {
            continue;
        };
        if price_usd <= Decimal::ZERO
            || volume_usd < Decimal::ZERO
            || liquidity_usd < Decimal::from(25_000)
        {
            continue;
        }
        let pool = item["graduatedPool"].as_str().unwrap_or(token).to_string();
        let candidate = DexMover {
            chain: "solana".to_string(),
            token: token.to_string(),
            symbol: item["symbol"].as_str().map(str::to_string),
            name: item["name"].as_str().map(str::to_string),
            pool,
            price_usd,
            change_pct,
            volume_usd,
            liquidity_usd,
        };
        let key = dex_key("solana", token);
        if best
            .get(&key)
            .is_none_or(|current| candidate.liquidity_usd > current.liquidity_usd)
        {
            best.insert(key, candidate);
        }
    }
}

async fn live_dex_movers(
    rows: &[crate::analysis::SnapshotRow],
    win: &str,
    now: chrono::DateTime<Utc>,
) -> Vec<Value> {
    let (change_key, volume_key) = match win {
        "1h" => ("h1", "h1"),
        "24h" => ("h24", "h24"),
        _ => return vec![],
    };
    let mut by_chain: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in rows {
        let chain = row.snap.token.chain.as_str();
        if !["ethereum", "solana"].contains(&chain) {
            continue;
        }
        let address = row.snap.token.contract_or_mint.as_str();
        let address = if chain == "solana" {
            address.to_string()
        } else {
            address.to_ascii_lowercase()
        };
        by_chain
            .entry(chain.to_string())
            .or_default()
            .insert(address);
    }
    let base = std::env::var("TRACE_LIQUIDITY_API_BASE")
        .unwrap_or_else(|_| "https://api.dexscreener.com".into());
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("trace-api")
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut best = HashMap::new();
    for (chain, tokens) in by_chain {
        let all: Vec<String> = tokens.iter().cloned().collect();
        for batch in all.chunks(30) {
            let url = format!(
                "{}/tokens/v1/{chain}/{}",
                base.trim_end_matches('/'),
                batch.join(",")
            );
            let body: Value = match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => match response.json().await {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::warn!(%error, "live DEX mover response was not JSON");
                        continue;
                    }
                },
                Ok(response) => {
                    tracing::warn!(status = %response.status(), "live DEX mover request refused");
                    continue;
                }
                Err(error) => {
                    tracing::warn!(%error, "live DEX mover request failed");
                    continue;
                }
            };
            parse_dex_pairs(&body, &chain, &tokens, change_key, volume_key, &mut best);
        }
    }
    // A new deployment can have chain events before it has useful token metadata or
    // liquidity snapshots. In that case, use the provider's newest token profiles as
    // discovery candidates instead of returning an empty market screen. These rows are
    // deliberately Partial because they do not yet carry TRACE wallet-flow attribution.
    if best.is_empty() {
        let profiles_url = format!("{}/token-profiles/latest/v1", base.trim_end_matches('/'));
        let profiles: Value = match client.get(profiles_url).send().await {
            Ok(response) if response.status().is_success() => match response.json().await {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, "live DEX profiles response was not JSON");
                    Value::Null
                }
            },
            Ok(response) => {
                tracing::warn!(status = %response.status(), "live DEX profiles request refused");
                Value::Null
            }
            Err(error) => {
                tracing::warn!(%error, "live DEX profiles request failed");
                Value::Null
            }
        };
        let mut discovered: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for profile in profiles.as_array().into_iter().flatten() {
            let (Some(chain), Some(address)) = (
                profile["chainId"].as_str(),
                profile["tokenAddress"].as_str(),
            ) else {
                continue;
            };
            if !["ethereum", "solana"].contains(&chain) {
                continue;
            }
            let address = if chain == "solana" {
                address.to_string()
            } else {
                address.to_ascii_lowercase()
            };
            discovered
                .entry(chain.to_string())
                .or_default()
                .insert(address);
        }
        for (chain, tokens) in discovered {
            let all: Vec<String> = tokens.iter().cloned().collect();
            for batch in all.chunks(30) {
                let url = format!(
                    "{}/tokens/v1/{chain}/{}",
                    base.trim_end_matches('/'),
                    batch.join(",")
                );
                let body: Value = match client.get(&url).send().await {
                    Ok(response) if response.status().is_success() => match response.json().await {
                        Ok(value) => value,
                        Err(error) => {
                            tracing::warn!(%error, "discovered DEX mover response was not JSON");
                            continue;
                        }
                    },
                    Ok(response) => {
                        tracing::warn!(status = %response.status(), "discovered DEX mover request refused");
                        continue;
                    }
                    Err(error) => {
                        tracing::warn!(%error, "discovered DEX mover request failed");
                        continue;
                    }
                };
                parse_dex_pairs(&body, &chain, &tokens, change_key, volume_key, &mut best);
            }
        }
    }
    if best.len() < 10 {
        add_trending_pools(&client, win, &mut best).await;
    }
    if best.len() < 10 {
        add_jupiter_trending(&client, win, &mut best).await;
    }
    let mut movers: Vec<DexMover> = best.into_values().collect();
    movers.sort_by(|a, b| {
        b.change_pct
            .cmp(&a.change_pct)
            .then_with(|| b.volume_usd.cmp(&a.volume_usd))
    });
    movers
        .into_iter()
        .take(20)
        .enumerate()
        .map(|(index, mover)| {
            json!({
                "rank": index + 1,
                "token": token_ref(&mover.chain, &mover.token, mover.symbol.as_deref(), mover.name.as_deref()),
                "price_usd": dec_str(mover.price_usd),
                "change_pct": dec_str(mover.change_pct.round_dp(2)),
                "volume_usd": dec_str(mover.volume_usd),
                "liquidity_usd": dec_str(mover.liquidity_usd),
                "pool": mover.pool,
                "reason": { "code": "market_movement", "label": format!("{} market movement", win) },
                "sparkline": [],
                "risk": "unknown",
                "risk_notes": ["Live DEX market data. TRACE wallet-flow and contract-risk evidence is not complete yet."],
                "source_timestamp": now,
            })
        })
        .collect()
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

    let mut items: Vec<Value> = ranked
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
    let mut status = if rows.is_empty() {
        DataStatus::Unavailable
    } else if ranked.is_empty() && excluded.stale == rows.len() {
        DataStatus::Stale
    } else {
        status_for(used_demo, newest, now, cfg.max_age.num_seconds())
    };
    if status == DataStatus::Live && excluded.unavailable_liquidity > 0 {
        status = DataStatus::Partial;
    }
    let mut fallback_used = false;
    if items.is_empty() && !rows.is_empty() && !used_demo {
        let fallback = live_dex_movers(&rows, win, now).await;
        if !fallback.is_empty() {
            items = fallback;
            status = DataStatus::Partial;
            fallback_used = true;
        }
    }
    let mut env = Envelope::new(
        status,
        if fallback_used {
            now
        } else {
            newest.unwrap_or(now)
        },
        used_demo,
    );
    let mut notes = vec![];
    if fallback_used {
        notes.push(
            "Live DEX market data. Wallet-flow attribution and contract-risk evidence are incomplete."
                .to_string(),
        );
    } else if rows.is_empty() {
        notes.push("No market snapshots are available yet.".to_string());
    }
    if !fallback_used {
        for (n, what) in [
            (excluded.stale, "stale"),
            (excluded.unavailable_liquidity, "liquidity unavailable"),
            (excluded.thin_liquidity, "thin liquidity"),
            (excluded.invalid_price, "invalid price"),
            (excluded.blocked_risk, "blocked by risk checks"),
        ] {
            if n > 0 {
                notes.push(format!("{n} excluded: {what}."));
            }
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

    let mut entries: Vec<Entry> = vec![];
    for (c, addr, _n) in active {
        if chain.is_some_and(|ch| ch.as_str() != c) {
            continue;
        }
        let Ok(w) = WalletId::parse(c.parse()?, &addr) else {
            continue;
        };
        let (swaps, prices) = load_wallet(&state.pool, &w).await?;
        // One wallet with pathological data (an overflow, an empty edge case) must not take
        // the whole ranking down with it. Analysis is pure, so a panic is isolated and skipped.
        let analysed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rank_entry(analyze_loaded(swaps, prices), &c, &addr, &followers, now)
        }));
        match analysed {
            Ok(Some(e)) => entries.push(e),
            Ok(None) => {}
            Err(_) => {
                tracing::error!(chain = %c, wallet = %addr, "wallet analysis panicked; skipped from discovery")
            }
        }
    }
    finish_wallets(tab, entries, offset, now)
}

struct Entry {
    sort_score: i64,
    first_seen: chrono::DateTime<Utc>,
    followers: i64,
    json: Value,
    demo: bool,
}

fn rank_entry(
    a: crate::analysis::WalletAnalysis,
    c: &str,
    addr: &str,
    followers: &BTreeMap<(String, String), i64>,
    now: chrono::DateTime<Utc>,
) -> Option<Entry> {
    {
        let activity = summarize(&a.swaps);
        let mut tags: Vec<String> = vec![];
        if let Some(act) = activity.first() {
            match classify(&CandidateConfig::default(), act, &Default::default()) {
                CandidateDecision::ExcludedInfrastructure => return None,
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
        // Checked: dust-priced lots can make p / cost overflow Decimal, which panics.
        let realized_pct = match (a.accounting.realized_pnl_usd, cost > Decimal::ZERO) {
            (Some(p), true) => p
                .checked_div(cost)
                .and_then(|r| r.checked_mul(Decimal::from(100)))
                .map(|r| dec_str(r.round_dp(2))),
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
            .get(&(c.to_string(), addr.to_string()))
            .copied()
            .unwrap_or(0);
        Some(Entry {
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
        })
    }
}

fn finish_wallets(
    tab: &str,
    mut entries: Vec<Entry>,
    offset: usize,
    now: chrono::DateTime<Utc>,
) -> Result<HttpResponse, ApiError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dex_fallback_keeps_the_most_liquid_requested_base_pair() {
        let body = json!([
            {
                "chainId": "solana",
                "pairAddress": "thin-pair",
                "baseToken": {"address": "MintA", "symbol": "AAA", "name": "Alpha"},
                "quoteToken": {"address": "USDC"},
                "priceUsd": "1.25",
                "priceChange": {"h24": 12.5},
                "volume": {"h24": 500000},
                "liquidity": {"usd": 30000}
            },
            {
                "chainId": "solana",
                "pairAddress": "deep-pair",
                "baseToken": {"address": "MintA", "symbol": "AAA", "name": "Alpha"},
                "quoteToken": {"address": "SOL"},
                "priceUsd": "1.24",
                "priceChange": {"h24": "11.75"},
                "volume": {"h24": "900000"},
                "liquidity": {"usd": "250000"}
            },
            {
                "chainId": "solana",
                "pairAddress": "wrong-token",
                "baseToken": {"address": "MintB", "symbol": "BBB"},
                "quoteToken": {"address": "MintA"},
                "priceUsd": "9",
                "priceChange": {"h24": 99},
                "volume": {"h24": 999999},
                "liquidity": {"usd": 999999}
            }
        ]);
        let requested = BTreeSet::from(["MintA".to_string()]);
        let mut best = HashMap::new();
        parse_dex_pairs(&body, "solana", &requested, "h24", "h24", &mut best);

        let item = best.get("solana:MintA").expect("requested token pair");
        assert_eq!(item.pool, "deep-pair");
        assert_eq!(item.change_pct, Decimal::from_str("11.75").unwrap());
        assert_eq!(best.len(), 1);
    }

    #[test]
    fn dex_fallback_rejects_thin_liquidity() {
        let body = json!([{
            "chainId": "ethereum",
            "pairAddress": "0x0000000000000000000000000000000000000002",
            "baseToken": {
                "address": "0x0000000000000000000000000000000000000001",
                "symbol": "TINY"
            },
            "priceUsd": "0.01",
            "priceChange": {"h24": 20},
            "volume": {"h24": 100000},
            "liquidity": {"usd": 24999}
        }]);
        let requested = BTreeSet::from(["0x0000000000000000000000000000000000000001".to_string()]);
        let mut best = HashMap::new();
        parse_dex_pairs(&body, "ethereum", &requested, "h24", "h24", &mut best);
        assert!(best.is_empty());
    }
}
