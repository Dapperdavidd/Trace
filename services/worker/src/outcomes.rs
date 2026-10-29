//! Signal outcome evaluation.
//!
//! For each published signal and each declared horizon that is due, record the
//! result of the predeclared exit rule: the FIRST available price at or after
//! the horizon, within the allowed lag. Outcomes are append-only (one row per
//! horizon, never replaced). Losses, unpriced and untradeable results are
//! recorded exactly like gains. A horizon with no price yet and still inside
//! its lag window is left alone and tried again later.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use trace_risk::RiskPolicy;
use trace_signals::{evaluate_horizon, OutcomeKind, PricePoint, Receipt, SignalState};
use uuid::Uuid;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub recorded: usize,
    pub not_ready: usize,
    pub integrity_failures: usize,
    pub transitions: usize,
}

fn kind_name(k: &OutcomeKind) -> &'static str {
    match k {
        OutcomeKind::Observed { .. } => "observed",
        OutcomeKind::Unpriced => "unpriced",
        OutcomeKind::Untradeable => "untradeable",
        OutcomeKind::DataUnavailable => "data_unavailable",
        OutcomeKind::Expired => "expired",
        OutcomeKind::Error { .. } => "error",
    }
}

fn parse_state(s: &str) -> Option<SignalState> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

fn state_name(s: SignalState) -> String {
    serde_json::to_value(s)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

async fn prices_for(
    pool: &PgPool,
    r: &Receipt,
    from: DateTime<Utc>,
) -> Result<Vec<PricePoint>, sqlx::Error> {
    let decimals: Option<i16> =
        sqlx::query_scalar("SELECT decimals FROM tokens WHERE chain=$1 AND contract_or_mint=$2")
            .bind(&r.chain)
            .bind(&r.token)
            .fetch_optional(pool)
            .await?
            .flatten();
    let Some(d) = decimals else { return Ok(vec![]) };
    let scale = Decimal::from_i128_with_scale(10i128.pow(d.clamp(0, 28) as u32), 0);
    // Exit liquidity comes from the latest snapshot. With no snapshot we cannot
    // claim the position could be sold, so the exit is treated as not liquid.
    let liquidity: Option<Decimal> = sqlx::query_scalar(
        "SELECT liquidity_usd FROM market_snapshots WHERE chain=$1 AND token=$2 ORDER BY last_updated DESC LIMIT 1",
    )
    .bind(&r.chain)
    .bind(&r.token)
    .fetch_optional(pool)
    .await?;
    let liquid = liquidity.is_some_and(|l| l >= RiskPolicy::default().min_liquidity_usd);
    let rows = sqlx::query(
        "SELECT observed_at, usd_per_base_unit FROM price_points WHERE chain=$1 AND asset=$2 AND observed_at >= $3 ORDER BY observed_at",
    )
    .bind(&r.chain)
    .bind(&r.token)
    .bind(from)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| PricePoint {
            at: row.get("observed_at"),
            price_usd: row.get::<Decimal, _>("usd_per_base_unit") * scale,
            exit_liquid: liquid,
        })
        .collect())
}

pub async fn evaluate_due(pool: &PgPool, now: DateTime<Utc>) -> Result<Report, sqlx::Error> {
    let mut report = Report::default();
    let signals = sqlx::query("SELECT id, receipt, receipt_hash, state FROM signals WHERE state IN ('published_as_research','observation_due') ORDER BY published_at, id")
        .fetch_all(pool)
        .await?;
    for row in signals {
        let id: Uuid = row.get("id");
        let receipt: Receipt = match serde_json::from_value(row.get("receipt")) {
            Ok(r) => r,
            Err(_) => {
                report.integrity_failures += 1;
                continue;
            }
        };
        if receipt.seal() != row.get::<String, _>("receipt_hash") {
            tracing::error!(signal = %id, "receipt hash mismatch, skipping");
            report.integrity_failures += 1;
            continue;
        }
        let have: Vec<i64> =
            sqlx::query_scalar("SELECT horizon_secs FROM signal_outcomes WHERE signal_id=$1")
                .bind(id)
                .fetch_all(pool)
                .await?;
        let mut any_due = false;
        for &h in &receipt.horizons_secs {
            let target = receipt.published_at + Duration::seconds(h);
            if target > now {
                continue;
            }
            any_due = true;
            if have.contains(&h) {
                continue;
            }
            let prices = prices_for(pool, &receipt, target).await?;
            let outcome = evaluate_horizon(&receipt, h, &prices, now);
            if matches!(outcome.kind, OutcomeKind::DataUnavailable) {
                report.not_ready += 1;
                continue;
            }
            let (price, ret) = match &outcome.kind {
                OutcomeKind::Observed {
                    return_bps,
                    exit_price_usd,
                } => (Some(*exit_price_usd), Some(Decimal::new(*return_bps, 2))),
                _ => (None, None),
            };
            let inserted = sqlx::query("INSERT INTO signal_outcomes (signal_id, horizon_secs, kind, price_usd, return_pct, evaluated_at) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
                .bind(id)
                .bind(h)
                .bind(kind_name(&outcome.kind))
                .bind(price)
                .bind(ret)
                .bind(now)
                .execute(pool)
                .await?;
            if inserted.rows_affected() == 1 {
                report.recorded += 1;
            }
        }
        report.transitions += advance(
            pool,
            id,
            &receipt,
            parse_state(&row.get::<String, _>("state")),
            any_due,
            now,
        )
        .await?;
    }
    Ok(report)
}

async fn advance(
    pool: &PgPool,
    id: Uuid,
    receipt: &Receipt,
    current: Option<SignalState>,
    any_due: bool,
    now: DateTime<Utc>,
) -> Result<usize, sqlx::Error> {
    let Some(mut state) = current else {
        return Ok(0);
    };
    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM signal_outcomes WHERE signal_id=$1")
            .bind(id)
            .fetch_all(pool)
            .await?;
    let mut steps = vec![];
    if state == SignalState::PublishedAsResearch
        && any_due
        && state.can_become(SignalState::ObservationDue)
    {
        state = SignalState::ObservationDue;
        steps.push((state, "First horizon is due"));
    }
    if state == SignalState::ObservationDue
        && !receipt.horizons_secs.is_empty()
        && kinds.len() >= receipt.horizons_secs.len()
    {
        let next = if kinds.iter().any(|k| k == "observed") {
            SignalState::Observed
        } else {
            SignalState::Unpriced
        };
        if state.can_become(next) {
            steps.push((next, "All horizons recorded"));
        }
    }
    if steps.is_empty() {
        return Ok(0);
    }
    let mut seq: i32 = sqlx::query_scalar(
        "SELECT COALESCE(max(seq), -1) + 1 FROM signal_events WHERE signal_id=$1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    for (st, note) in &steps {
        sqlx::query(
            "INSERT INTO signal_events (signal_id, seq, state, at, note) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(seq)
        .bind(state_name(*st))
        .bind(now)
        .bind(note)
        .execute(pool)
        .await?;
        seq += 1;
    }
    let last = steps.last().map(|s| s.0).unwrap_or(state);
    sqlx::query("UPDATE signals SET state=$2 WHERE id=$1")
        .bind(id)
        .bind(state_name(last))
        .execute(pool)
        .await?;
    Ok(steps.len())
}
