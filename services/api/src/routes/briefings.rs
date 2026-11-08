use actix_web::{web, HttpResponse};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use trace_discovery::movers::{rank, MoverConfig, Tab};
use trace_domain::DataStatus;
use trace_risk::RiskState;

use super::common::*;
use crate::{analysis::load_snapshots, error::ApiError, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/briefings", web::get().to(briefing));
}

fn link(chain: &str, address: &str) -> String {
    format!("/tokens/{chain}/{address}")
}

const WRAPPED_NATIVES: &[&str] = &["WETH", "WSOL", "SOL", "ETH", "WBNB"];

fn token_ref(chain: &str, address: &str) -> Value {
    json!({ "chain": chain, "address": address })
}

fn chain_name(chain: &str) -> &str {
    match chain {
        "ethereum" => "Ethereum",
        "solana" => "Solana",
        "base" => "Base",
        "bnb" => "BNB Chain",
        other => other,
    }
}

fn short(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}…{}", &addr[..4], &addr[addr.len() - 3..])
    } else {
        addr.to_string()
    }
}

/// The daily report, built only from stored, finalized evidence. Every section is
/// derived from real swaps, prices and risk checks; a section with nothing to say is
/// omitted rather than padded. Items carry `token` or `wallet` references so clients
/// can link them without knowing web routes.
async fn briefing(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let now = Utc::now();
    let since = now - Duration::hours(24);
    let cfg = MoverConfig::default();

    // Movers: 24h when the price history allows it, otherwise the last hour.
    let mut window = "24 hours";
    let mut rows = load_snapshots(&state.pool, "24h", None, now).await?;
    let snaps: Vec<_> = rows.iter().map(|r| r.snap.clone()).collect();
    let (mut movers, _) = rank(&cfg, Tab::Trending, &snaps, now);
    if movers.is_empty() {
        let hour = load_snapshots(&state.pool, "1h", None, now).await?;
        let hour_snaps: Vec<_> = hour.iter().map(|r| r.snap.clone()).collect();
        let (m, _) = rank(&cfg, Tab::Trending, &hour_snaps, now);
        if !m.is_empty() {
            window = "hour";
            movers = m;
            rows = hour;
        }
    }

    // Activity per chain.
    let per_chain: Vec<(String, i64, i64)> = sqlx::query(
        "SELECT chain, count(*) AS n, count(DISTINCT wallet) AS w FROM normalized_swaps
         WHERE observed_at >= $1 AND NOT demo AND finality <> 'reorged'
         GROUP BY chain ORDER BY n DESC",
    )
    .bind(since)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| (r.get("chain"), r.get("n"), r.get("w")))
    .collect();
    let total_swaps: i64 = per_chain.iter().map(|c| c.1).sum();
    let total_wallets: i64 = per_chain.iter().map(|c| c.2).sum();

    // Coins several different wallets bought (token_out is what a wallet received).
    let crowded: Vec<(String, String, Option<String>, i64)> = sqlx::query(
        "SELECT s.chain, s.token_out AS token, t.symbol, count(DISTINCT s.wallet) AS w
         FROM normalized_swaps s
         JOIN tokens t ON t.chain = s.chain AND t.contract_or_mint = s.token_out
         WHERE s.observed_at >= $1 AND NOT s.demo AND s.finality <> 'reorged'
           AND s.attribution = 'direct' AND NOT t.is_stable
           AND coalesce(upper(t.symbol), '') <> ALL($2)
         GROUP BY 1, 2, 3 HAVING count(DISTINCT s.wallet) >= 3
         ORDER BY w DESC LIMIT 5",
    )
    .bind(since)
    .bind(
        WRAPPED_NATIVES
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| (r.get("chain"), r.get("token"), r.get("symbol"), r.get("w")))
    .collect();

    // Most traded coins by number of swaps either way.
    let traded: Vec<(String, String, Option<String>, i64, i64)> = sqlx::query(
        "SELECT s.chain, v.token, t.symbol, count(*) AS n, count(DISTINCT s.wallet) AS w
         FROM normalized_swaps s
         CROSS JOIN LATERAL (VALUES (s.token_in), (s.token_out)) v(token)
         JOIN tokens t ON t.chain = s.chain AND t.contract_or_mint = v.token
         WHERE s.observed_at >= $1 AND NOT s.demo AND s.finality <> 'reorged'
           AND NOT t.is_stable AND coalesce(upper(t.symbol), '') <> ALL($2)
         GROUP BY 1, 2, 3 ORDER BY n DESC LIMIT 5",
    )
    .bind(since)
    .bind(
        WRAPPED_NATIVES
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| {
        (
            r.get("chain"),
            r.get("token"),
            r.get("symbol"),
            r.get("n"),
            r.get("w"),
        )
    })
    .collect();

    // Busiest wallets trading directly (routers and relayed routes are excluded).
    let busy: Vec<(String, String, i64)> = sqlx::query(
        "SELECT chain, wallet, count(*) AS n FROM normalized_swaps
         WHERE observed_at >= $1 AND NOT demo AND finality <> 'reorged' AND attribution = 'direct'
         GROUP BY 1, 2 ORDER BY n DESC LIMIT 3",
    )
    .bind(since)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| (r.get("chain"), r.get("wallet"), r.get("n")))
    .collect();

    let signals: i64 = sqlx::query_scalar("SELECT count(*) FROM signals WHERE published_at >= $1")
        .bind(since)
        .fetch_one(&state.pool)
        .await?;

    let mut sections: Vec<Value> = vec![];

    // The short version: one paragraph a person can read in ten seconds.
    if total_swaps > 0 || !movers.is_empty() {
        let mut parts: Vec<String> = vec![];
        if total_swaps > 0 {
            let chains = per_chain
                .iter()
                .map(|c| chain_name(&c.0).to_string())
                .collect::<Vec<_>>()
                .join(" and ");
            parts.push(format!(
                "trace read {total_swaps} swaps from about {total_wallets} wallets on {chains} in the last 24 hours."
            ));
        }
        if let Some(m) = movers.first() {
            parts.push(format!(
                "The biggest mover over the last {window} is {}, {} {}%.",
                m.symbol.clone().unwrap_or_else(|| "a token".into()),
                if m.change_pct >= rust_decimal::Decimal::ZERO {
                    "up"
                } else {
                    "down"
                },
                dec_str(m.change_pct.abs().round_dp(1))
            ));
        }
        if let Some((_, _, sym, w)) = crowded.first() {
            parts.push(format!(
                "{w} different wallets bought {}.",
                sym.clone().unwrap_or_else(|| "one coin".into())
            ));
        }
        sections
            .push(json!({ "title": "The short version", "items": [{ "text": parts.join(" ") }] }));
    }

    if !movers.is_empty() {
        sections.push(json!({
            "title": format!("Biggest moves, last {window}"),
            "items": movers.iter().take(5).map(|m| json!({
                "text": format!("{} {} {}%. {}. Liquidity ${}.",
                    m.symbol.clone().unwrap_or_else(|| "A token".into()),
                    if m.change_pct >= rust_decimal::Decimal::ZERO { "is up" } else { "is down" },
                    dec_str(m.change_pct.abs().round_dp(1)),
                    m.reason.label(),
                    dec_str(m.liquidity_usd.round_dp(0))),
                "link": link(m.token.chain.as_str(), m.token.contract_or_mint.as_str()),
                "token": token_ref(m.token.chain.as_str(), m.token.contract_or_mint.as_str()),
            })).collect::<Vec<_>>(),
        }));
    }

    if !crowded.is_empty() {
        sections.push(json!({
            "title": "Coins wallets crowded into",
            "items": crowded.iter().map(|(c, t, sym, w)| json!({
                "text": format!("{} on {}: bought by {w} different wallets in 24 hours.",
                    sym.clone().unwrap_or_else(|| short(t)), chain_name(c)),
                "link": link(c, t),
                "token": token_ref(c, t),
            })).collect::<Vec<_>>(),
        }));
    }

    if !traded.is_empty() {
        sections.push(json!({
            "title": "Most traded coins",
            "items": traded.iter().map(|(c, t, sym, n, w)| json!({
                "text": format!("{} on {}: {n} swaps by {w} wallets.",
                    sym.clone().unwrap_or_else(|| short(t)), chain_name(c)),
                "link": link(c, t),
                "token": token_ref(c, t),
            })).collect::<Vec<_>>(),
        }));
    }

    if !busy.is_empty() {
        sections.push(json!({
            "title": "Most active wallets",
            "items": busy.iter().map(|(c, w, n)| json!({
                "text": format!("{} on {} made {n} direct swaps. Open it to see its record before you follow it.", short(w), chain_name(c)),
                "link": format!("/wallets/{c}/{w}"),
                "wallet": { "chain": c, "address": w },
            })).collect::<Vec<_>>(),
        }));
    }

    let risky: Vec<_> = rows
        .iter()
        .filter(|r| matches!(r.risk.overall, RiskState::Warn | RiskState::Block))
        .collect();
    if !risky.is_empty() {
        sections.push(json!({
            "title": "Risk watch",
            "items": risky.iter().take(5).map(|r| json!({
                "text": format!("{}: {}", r.snap.symbol.clone().unwrap_or_else(|| "A token".into()),
                    r.risk.checks.iter().find(|c| c.state != RiskState::Pass).map(|c| c.note.clone()).unwrap_or_default()),
                "link": link(r.snap.token.chain.as_str(), r.snap.token.contract_or_mint.as_str()),
                "token": token_ref(r.snap.token.chain.as_str(), r.snap.token.contract_or_mint.as_str()),
            })).collect::<Vec<_>>(),
        }));
    }

    if !per_chain.is_empty() {
        sections.push(json!({
            "title": "By chain",
            "items": per_chain.iter().map(|(c, n, w)| json!({
                "text": format!("{}: {n} swaps from {w} wallets in 24 hours.", chain_name(c)),
            })).collect::<Vec<_>>(),
        }));
    }

    if signals > 0 {
        sections.push(json!({ "title": "Signals", "items": [{ "text": format!("{signals} research signals published in the last 24 hours. Open one to see its receipt and outcomes."), "link": "/timeline" }] }));
    }

    let demo = rows.iter().any(|r| r.demo);
    let newest = rows.iter().map(|r| r.snap.last_updated).max();
    let status = if sections.is_empty() {
        DataStatus::Unavailable
    } else {
        status_for(demo, newest.or(Some(now)), now, 3_600)
    };
    let mut env = Envelope::new(status, newest.unwrap_or(now), demo);
    if sections.is_empty() {
        env = env.note("There is not enough data for a briefing yet.");
    }
    let mut body = serde_json::to_value(&env).unwrap_or(Value::Null);
    body["sections"] = Value::Array(sections);
    Ok(HttpResponse::Ok().json(body))
}
