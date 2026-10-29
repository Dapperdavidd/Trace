use actix_web::{web, HttpResponse};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
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

/// A daily summary built only from stored data. A section with nothing to say
/// is omitted rather than padded.
async fn briefing(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let now = Utc::now();
    let rows = load_snapshots(&state.pool, "24h", None, now).await?;
    let cfg = MoverConfig::default();
    let snaps: Vec<_> = rows.iter().map(|r| r.snap.clone()).collect();
    let (movers, _) = rank(&cfg, Tab::Trending, &snaps, now);

    let mut sections: Vec<Value> = vec![];
    if !movers.is_empty() {
        sections.push(json!({
            "title": "Top movers",
            "items": movers.iter().take(3).map(|m| json!({
                "text": format!("{} {} {}% over 24h. {}.", m.symbol.clone().unwrap_or_else(|| "A token".into()), if m.change_pct >= rust_decimal::Decimal::ZERO { "is up" } else { "is down" }, dec_str(m.change_pct.abs().round_dp(1)), m.reason.label()),
                "link": link(m.token.chain.as_str(), m.token.contract_or_mint.as_str()),
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
            "items": risky.iter().take(3).map(|r| json!({
                "text": format!("{}: {}", r.snap.symbol.clone().unwrap_or_else(|| "A token".into()),
                    r.risk.checks.iter().find(|c| c.state != RiskState::Pass).map(|c| c.note.clone()).unwrap_or_default()),
                "link": link(r.snap.token.chain.as_str(), r.snap.token.contract_or_mint.as_str()),
            })).collect::<Vec<_>>(),
        }));
    }
    let buys: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM normalized_swaps WHERE observed_at >= $1 AND attribution='direct' AND finality <> 'reorged'",
    )
    .bind(now - Duration::hours(24))
    .fetch_one(&state.pool)
    .await?;
    if buys > 0 {
        sections.push(json!({ "title": "Wallet activity", "items": [{ "text": format!("{buys} swaps credited to tracked wallets in the last 24 hours.") }] }));
    }
    let signals: i64 = sqlx::query_scalar("SELECT count(*) FROM signals WHERE published_at >= $1")
        .bind(now - Duration::hours(24))
        .fetch_one(&state.pool)
        .await?;
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
