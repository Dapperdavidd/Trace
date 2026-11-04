//! Publish research signals from distinct, finalized Uniswap V2 buys.
//!
//! A token is considered only when at least three distinct direct wallets buy it
//! in the last hour. The market snapshot must be fresh and must contain the
//! reviewed risk inputs needed to state a reference size and price impact. Any
//! unknown risk check becomes watch-only, and a blocking check is rejected.

use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Row};
use trace_jobs::Failure;
use trace_risk::{assess, Check, RiskInputs, RiskPolicy, RiskState};
use trace_signals::{Receipt, HORIZONS_SECS, RULES_VERSION};
use uuid::Uuid;

pub const PUBLISH_SIGNALS: &str = "publish_research_signals";
const MIN_DISTINCT_BUYERS: i64 = 3;
const SIGNAL_COOLDOWN: Duration = Duration::hours(1);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SignalReport {
    pub candidates: usize,
    pub published: usize,
    pub watch_only: usize,
    pub rejected: usize,
    pub incomplete: usize,
}

fn risk_name(state: RiskState) -> &'static str {
    match state {
        RiskState::Pass => "pass",
        RiskState::Warn => "warn",
        RiskState::Block => "block",
        RiskState::Unknown => "unknown",
    }
}

fn findings(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.state != RiskState::Pass)
        .map(|check| check.note.clone())
        .collect()
}

pub async fn publish(pool: &PgPool, now: DateTime<Utc>) -> Result<SignalReport, Failure> {
    let transient = |e: sqlx::Error| Failure::Transient(e.to_string());
    let mut tx = pool.begin().await.map_err(&transient)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('trace_signal_publisher'))")
        .execute(&mut *tx)
        .await
        .map_err(&transient)?;

    let rows = sqlx::query(
        "SELECT m.chain, m.token, m.pool, m.price_now_usd, m.liquidity_usd,
                m.risk_inputs, m.last_updated,
                min(s.wallet) AS representative_wallet,
                array_agg(DISTINCT s.wallet ORDER BY s.wallet) AS wallets,
                array_agg(DISTINCT s.tx_id ORDER BY s.tx_id) AS tx_ids,
                max(s.observed_at) AS block_time,
                count(DISTINCT s.wallet) AS buyer_count
         FROM market_snapshots m
         JOIN normalized_swaps s ON s.chain=m.chain AND s.pool=m.pool AND s.token_out=m.token
         JOIN tokens stable ON stable.chain=s.chain AND stable.contract_or_mint=s.token_in
         WHERE m.win='24h' AND NOT m.demo AND m.last_updated >= $1
           AND m.liquidity_usd IS NOT NULL
           AND NOT s.demo AND s.finality='final' AND s.attribution='direct'
           AND s.protocol='uniswap_v2' AND stable.is_stable
           AND s.observed_at >= $2 AND s.observed_at <= $3
           AND NOT EXISTS (
             SELECT 1 FROM signals existing
             WHERE NOT existing.demo AND existing.token_chain=m.chain
               AND existing.token=m.token AND existing.published_at >= $4
           )
         GROUP BY m.chain, m.token, m.pool, m.price_now_usd, m.liquidity_usd,
                  m.risk_inputs, m.last_updated
         HAVING count(DISTINCT s.wallet) >= $5
         ORDER BY max(s.observed_at), m.chain, m.token
         LIMIT 100",
    )
    .bind(now - Duration::minutes(5))
    .bind(now - Duration::hours(1))
    .bind(now)
    .bind(now - SIGNAL_COOLDOWN)
    .bind(MIN_DISTINCT_BUYERS)
    .fetch_all(&mut *tx)
    .await
    .map_err(&transient)?;

    let mut report = SignalReport {
        candidates: rows.len(),
        ..Default::default()
    };
    for row in rows {
        let mut inputs: RiskInputs =
            serde_json::from_value(row.get("risk_inputs")).unwrap_or_default();
        if inputs.liquidity_usd.is_none() {
            inputs.liquidity_usd = row.get("liquidity_usd");
        }
        let last_updated: DateTime<Utc> = row.get("last_updated");
        inputs.quote_age_seconds = Some((now - last_updated).num_seconds().max(0) as u64);
        let (Some(reference_amount), Some(price_impact_bps)) =
            (inputs.reference_size_usd, inputs.price_impact_bps)
        else {
            report.incomplete += 1;
            continue;
        };
        let risk = assess(&RiskPolicy::default(), &inputs, now);
        let (state, note) = match risk.overall {
            RiskState::Pass | RiskState::Warn => {
                report.published += 1;
                ("published_as_research", "Published as watch-only research")
            }
            RiskState::Block => {
                report.rejected += 1;
                ("rejected", "Rejected by the token risk policy")
            }
            RiskState::Unknown => {
                report.watch_only += 1;
                ("watch_only", "Risk data is incomplete; observation only")
            }
        };
        let published_at = now;
        let receipt = Receipt {
            chain: row.get("chain"),
            token: row.get("token"),
            source_tx_ids: row.get("tx_ids"),
            wallet_evidence_ids: row.get("wallets"),
            block_time: row.get("block_time"),
            detected_at: now,
            published_at,
            reference_price_usd: row.get("price_now_usd"),
            reference_amount_usd: reference_amount,
            fee_bps: 30,
            slippage_bps: price_impact_bps,
            risk_verdict: risk_name(risk.overall).into(),
            security_findings: findings(&risk.checks),
            wqs: None,
            wcs: None,
            tcq: None,
            score_versions: vec![risk.policy_version.into()],
            rules_version: RULES_VERSION.into(),
            exit_policy: "Observe the first available price at or after each declared horizon."
                .into(),
            horizons_secs: HORIZONS_SECS.to_vec(),
        };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO signals
             (id,wallet_chain,wallet_address,token_chain,token,receipt,receipt_hash,state,published_at,demo)
             VALUES ($1,$2,$3,$2,$4,$5,$6,$7,$8,FALSE)",
        )
        .bind(id)
        .bind(&receipt.chain)
        .bind(row.get::<String, _>("representative_wallet"))
        .bind(&receipt.token)
        .bind(serde_json::to_value(&receipt).map_err(|e| Failure::Permanent(e.to_string()))?)
        .bind(receipt.seal())
        .bind(state)
        .bind(published_at)
        .execute(&mut *tx)
        .await
        .map_err(&transient)?;
        for (seq, event_state, event_note) in [
            (0_i32, "detected", "Distinct wallet activity detected"),
            (1_i32, "evaluated", "Risk and evidence checks completed"),
            (2_i32, state, note),
        ] {
            sqlx::query(
                "INSERT INTO signal_events (signal_id,seq,state,at,note) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(id)
            .bind(seq)
            .bind(event_state)
            .bind(now)
            .bind(event_note)
            .execute(&mut *tx)
            .await
            .map_err(&transient)?;
        }
    }
    tx.commit().await.map_err(&transient)?;
    Ok(report)
}

pub async fn schedule(pool: &PgPool, now: DateTime<Utc>) -> Result<bool, trace_jobs::JobError> {
    let key = format!("{PUBLISH_SIGNALS}:{}", now.timestamp() / 60);
    trace_jobs::enqueue(
        pool,
        PUBLISH_SIGNALS,
        Some(&key),
        serde_json::json!({}),
        now,
        3,
    )
    .await?;
    Ok(true)
}
