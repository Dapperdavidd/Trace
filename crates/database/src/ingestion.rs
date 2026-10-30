use sqlx::{PgPool, Postgres, Row, Transaction};
use trace_domain::{Finality, NormalizedSwap};

use crate::DbError;

/// Result of persisting a swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted,
    /// Same `(chain, tx, event_index, revision)` already stored. Replays never double count.
    Duplicate,
}

/// Raw provider payload kept so any parser can be replayed.
pub struct RawEvent<'a> {
    pub chain: &'a str,
    pub tx_id: &'a str,
    pub event_index: i32,
    pub source: &'a str,
    pub payload: &'a serde_json::Value,
    pub payload_sha256: &'a str,
    /// True for fixtures and seeded data, never for live captures.
    pub fixture: bool,
    pub observed_at: chrono::DateTime<chrono::Utc>,
}

pub async fn store_raw_event(pool: &PgPool, e: &RawEvent<'_>) -> Result<InsertOutcome, DbError> {
    let r = sqlx::query(
        r#"INSERT INTO raw_chain_events (chain, tx_id, event_index, source, payload, payload_sha256, fixture, observed_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING"#,
    )
    .bind(e.chain)
    .bind(e.tx_id)
    .bind(e.event_index)
    .bind(e.source)
    .bind(e.payload)
    .bind(e.payload_sha256)
    .bind(e.fixture)
    .bind(e.observed_at)
    .execute(pool)
    .await?;
    Ok(if r.rows_affected() == 1 {
        InsertOutcome::Inserted
    } else {
        InsertOutcome::Duplicate
    })
}

/// Idempotently persist a swap together with its wallet and token rows in one transaction.
pub async fn insert_swap(pool: &PgPool, s: &NormalizedSwap) -> Result<InsertOutcome, DbError> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    let chain = s.chain.as_str();
    sqlx::query(
        "INSERT INTO wallets (chain, address, last_seen_at) VALUES ($1,$2,$3)
         ON CONFLICT (chain, address) DO UPDATE SET last_seen_at = GREATEST(wallets.last_seen_at, EXCLUDED.last_seen_at)",
    )
    .bind(chain)
    .bind(s.wallet.address.as_str())
    .bind(s.observed_at)
    .execute(&mut *tx)
    .await?;
    for t in [&s.token_in, &s.token_out] {
        sqlx::query(
            "INSERT INTO tokens (chain, contract_or_mint) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(chain)
        .bind(t.contract_or_mint.as_str())
        .execute(&mut *tx)
        .await?;
    }
    let r = sqlx::query(
        r#"INSERT INTO normalized_swaps
           (chain, tx_id, event_index, finality_revision, wallet, protocol, pool, token_in, amount_in,
            token_out, amount_out, block_height, block_hash, observed_at, finality, attribution,
            parser_version, raw_ref, demo)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9::numeric,$10,$11::numeric,$12,$13,$14,$15,$16,$17,$18,$19)
           ON CONFLICT (chain, tx_id, event_index, finality_revision) DO NOTHING"#,
    )
    .bind(chain)
    .bind(&s.tx_id)
    .bind(s.event_index as i32)
    .bind(s.finality_revision as i32)
    .bind(s.wallet.address.as_str())
    .bind(&s.protocol)
    .bind(&s.pool)
    .bind(s.token_in.contract_or_mint.as_str())
    .bind(s.amount_in.0.to_string())
    .bind(s.token_out.contract_or_mint.as_str())
    .bind(s.amount_out.0.to_string())
    .bind(s.block_height.map(|h| h as i64))
    .bind(&s.block_hash)
    .bind(s.observed_at)
    .bind(s.finality.as_str())
    .bind(serde_json::to_value(s.attribution).unwrap().as_str().unwrap().to_string())
    .bind(&s.parser_version)
    .bind(&s.raw_ref)
    .bind(s.demo)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(if r.rows_affected() == 1 {
        InsertOutcome::Inserted
    } else {
        InsertOutcome::Duplicate
    })
}

/// Mark tentative swaps at or above `from_height` as reorged. History is kept
/// (never deleted); derived data should treat `reorged` rows as non-existent.
/// Returns the number of swaps affected.
pub async fn mark_reorged(pool: &PgPool, chain: &str, from_height: i64) -> Result<u64, DbError> {
    let r = sqlx::query(
        "UPDATE normalized_swaps SET finality = 'reorged'
         WHERE chain = $1 AND block_height >= $2 AND finality = 'tentative'",
    )
    .bind(chain)
    .bind(from_height)
    .execute(pool)
    .await?;
    if r.rows_affected() > 0 {
        sqlx::query("INSERT INTO audit_events (id, actor, action, subject, detail) VALUES ($1,'indexer','reorg',$2,$3)")
            .bind(uuid::Uuid::new_v4())
            .bind(chain)
            .bind(serde_json::json!({ "from_height": from_height, "swaps_marked": r.rows_affected() }))
            .execute(pool)
            .await?;
    }
    Ok(r.rows_affected())
}

/// Promote tentative swaps at or below the finalized height.
pub async fn finalize_up_to(
    pool: &PgPool,
    chain: &str,
    finalized_height: i64,
) -> Result<u64, DbError> {
    let r = sqlx::query(
        "UPDATE normalized_swaps SET finality = 'final'
         WHERE chain = $1 AND block_height <= $2 AND finality = 'tentative'",
    )
    .bind(chain)
    .bind(finalized_height)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

/// What to do with a freshly parsed swap given what is already stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionPlan {
    /// A non-reorged row already exists for this event. Do not insert.
    Live,
    /// No live row. Insert with this revision (history of reorged rows is kept).
    Next(u32),
}

pub async fn revision_plan(
    pool: &PgPool,
    chain: &str,
    tx_id: &str,
    event_index: i32,
) -> Result<RevisionPlan, DbError> {
    let rows = sqlx::query(
        "SELECT finality_revision, finality FROM normalized_swaps
         WHERE chain=$1 AND tx_id=$2 AND event_index=$3 ORDER BY finality_revision DESC",
    )
    .bind(chain)
    .bind(tx_id)
    .bind(event_index)
    .fetch_all(pool)
    .await?;
    if rows
        .iter()
        .any(|r| r.get::<String, _>("finality") != "reorged")
    {
        return Ok(RevisionPlan::Live);
    }
    Ok(RevisionPlan::Next(rows.first().map_or(0, |r| {
        r.get::<i32, _>("finality_revision") as u32 + 1
    })))
}

pub async fn get_cursor(
    pool: &PgPool,
    chain: &str,
    protocol: &str,
) -> Result<Option<(String, Option<i64>)>, DbError> {
    let row =
        sqlx::query("SELECT cursor, height FROM ingestion_cursors WHERE chain=$1 AND protocol=$2")
            .bind(chain)
            .bind(protocol)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| (r.get("cursor"), r.get("height"))))
}

pub async fn set_cursor(
    pool: &PgPool,
    chain: &str,
    protocol: &str,
    cursor: &str,
    height: Option<i64>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO ingestion_cursors (chain, protocol, cursor, height) VALUES ($1,$2,$3,$4)
         ON CONFLICT (chain, protocol) DO UPDATE SET cursor=EXCLUDED.cursor, height=EXCLUDED.height, updated_at=now()",
    )
    .bind(chain)
    .bind(protocol)
    .bind(cursor)
    .bind(height)
    .execute(pool)
    .await?;
    Ok(())
}

/// Record that a protocol scan completed so freshness can be reported honestly.
pub async fn record_successful_scan(
    pool: &PgPool,
    chain: &str,
    protocol: &str,
    lookback_blocks: Option<i64>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE protocols SET last_successful_scan_at = now(), lookback_blocks = COALESCE($3, lookback_blocks) WHERE chain=$1 AND protocol=$2")
        .bind(chain)
        .bind(protocol)
        .bind(lookback_blocks)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn count_swaps(pool: &PgPool, chain: &str, finality: Finality) -> Result<i64, DbError> {
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM normalized_swaps WHERE chain=$1 AND finality=$2")
            .bind(chain)
            .bind(finality.as_str())
            .fetch_one(pool)
            .await?;
    Ok(n)
}
