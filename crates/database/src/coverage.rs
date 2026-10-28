use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::DbError;

/// Persisted facts about one protocol's ingestion on one chain.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProtocolCoverageRow {
    pub chain: String,
    pub protocol: String,
    pub support_status: String,
    pub last_successful_scan_at: Option<DateTime<Utc>>,
    pub lookback_blocks: Option<i64>,
    pub limitations: Vec<String>,
}

/// Upsert the registry (code-defined) into the `protocols` table so that
/// ingestion state and the API read from one source of truth.
pub async fn upsert_protocol(
    pool: &PgPool,
    chain: &str,
    protocol: &str,
    support_status: &str,
    limitations: &[String],
    blocker: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO protocols (chain, protocol, support_status, limitations, blocker)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (chain, protocol) DO UPDATE
             SET support_status = EXCLUDED.support_status,
                 limitations = EXCLUDED.limitations,
                 blocker = EXCLUDED.blocker,
                 updated_at = now()"#,
    )
    .bind(chain)
    .bind(protocol)
    .bind(support_status)
    .bind(limitations)
    .bind(blocker)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_protocol_coverage(pool: &PgPool) -> Result<Vec<ProtocolCoverageRow>, DbError> {
    let rows = sqlx::query(
        r#"SELECT chain, protocol, support_status, last_successful_scan_at, lookback_blocks, limitations
           FROM protocols ORDER BY chain, protocol"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ProtocolCoverageRow {
            chain: r.get("chain"),
            protocol: r.get("protocol"),
            support_status: r.get("support_status"),
            last_successful_scan_at: r.get("last_successful_scan_at"),
            lookback_blocks: r.get("lookback_blocks"),
            limitations: r.get("limitations"),
        })
        .collect())
}
