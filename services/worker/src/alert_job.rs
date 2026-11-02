//! Create in-app alerts from finalized activity on watched wallets.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Row, Transaction};
use trace_jobs::Failure;
use uuid::Uuid;

pub const GENERATE_ALERTS: &str = "generate_alerts";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AlertReport {
    pub scanned: usize,
    pub created: usize,
    pub disabled: usize,
    pub unsupported: usize,
}

async fn advance_cursors(
    tx: &mut Transaction<'_, Postgres>,
    scan_started: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE watchlist_wallets SET last_checked_at=$1
         WHERE last_checked_at IS NULL OR last_checked_at < $1",
    )
    .bind(scan_started)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn generate(pool: &PgPool, scan_started: DateTime<Utc>) -> Result<AlertReport, Failure> {
    let transient = |e: sqlx::Error| Failure::Transient(e.to_string());
    let mut tx = pool.begin().await.map_err(&transient)?;
    let rows = sqlx::query(
        "SELECT w.user_id, w.alias, w.address, s.chain, s.tx_id, s.event_index,
                s.protocol, s.token_in, s.token_out, s.observed_at,
                ti.symbol AS symbol_in, ti.is_stable AS stable_in,
                tout.symbol AS symbol_out, tout.is_stable AS stable_out,
                COALESCE(p.wallet_buy, TRUE) AS wallet_buy,
                COALESCE(p.wallet_sell, TRUE) AS wallet_sell
         FROM watchlist_wallets w
         JOIN normalized_swaps s ON s.chain=w.chain AND s.wallet=w.address
         JOIN tokens ti ON ti.chain=s.chain AND ti.contract_or_mint=s.token_in
         JOIN tokens tout ON tout.chain=s.chain AND tout.contract_or_mint=s.token_out
         LEFT JOIN alert_preferences p ON p.user_id=w.user_id
         WHERE NOT w.paused AND NOT s.demo AND s.finality='final'
           AND s.attribution='direct'
           AND s.ingested_at > COALESCE(w.last_checked_at, w.created_at)
           AND s.ingested_at <= $1
         ORDER BY s.ingested_at, s.id",
    )
    .bind(scan_started)
    .fetch_all(&mut *tx)
    .await
    .map_err(&transient)?;
    let mut report = AlertReport {
        scanned: rows.len(),
        ..Default::default()
    };
    for row in rows {
        let stable_in: bool = row.get("stable_in");
        let stable_out: bool = row.get("stable_out");
        let (kind, enabled, symbol): (&str, bool, Option<String>) = match (stable_in, stable_out) {
            (true, false) => ("wallet_buy", row.get("wallet_buy"), row.get("symbol_out")),
            (false, true) => ("wallet_sell", row.get("wallet_sell"), row.get("symbol_in")),
            _ => {
                report.unsupported += 1;
                continue;
            }
        };
        if !enabled {
            report.disabled += 1;
            continue;
        }
        let user_id: Uuid = row.get("user_id");
        let chain: String = row.get("chain");
        let tx_id: String = row.get("tx_id");
        let event_index: i32 = row.get("event_index");
        let address: String = row.get("address");
        let who = row
            .get::<Option<String>, _>("alias")
            .unwrap_or_else(|| format!("{}...{}", &address[..6], &address[address.len() - 4..]));
        let asset = symbol.unwrap_or_else(|| "a token".into());
        let action = if kind == "wallet_buy" {
            "bought"
        } else {
            "sold"
        };
        let dedupe = format!("{user_id}:{chain}:{tx_id}:{event_index}:{kind}");
        let result = sqlx::query(
            "INSERT INTO alerts
             (id,user_id,type,title,body,evidence_tx,event_time,published_at,demo,dedupe_key)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,FALSE,$9)
             ON CONFLICT (dedupe_key) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(kind)
        .bind(format!("{who} {action} {asset}"))
        .bind(format!(
            "Finalized swap observed on {}.",
            row.get::<String, _>("protocol")
        ))
        .bind(vec![tx_id])
        .bind(row.get::<DateTime<Utc>, _>("observed_at"))
        .bind(scan_started)
        .bind(dedupe)
        .execute(&mut *tx)
        .await
        .map_err(&transient)?;
        report.created += result.rows_affected() as usize;
    }
    advance_cursors(&mut tx, scan_started)
        .await
        .map_err(&transient)?;
    tx.commit().await.map_err(&transient)?;
    Ok(report)
}
