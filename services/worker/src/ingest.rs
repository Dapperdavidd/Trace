//! EVM ingestion engine.
//!
//! One `step` reads a bounded block range from a source, parses it with the
//! verified parser, stores raw evidence and swaps idempotently, then advances
//! the cursor. Rules:
//!
//! * Only protocols the registry marks Enabled or Partial are ingested.
//! * The cursor advances only after every insert in the range succeeded, so a
//!   failed step is safely retried (inserts are idempotent).
//! * Finality comes from the chain head we read, never from the source's claim.
//! * A block-hash mismatch at the cursor is a reorg: tentative rows in the
//!   reorg window are marked `reorged` (never deleted), the cursor rolls back,
//!   and the range is re-read. Finalized rows are never touched.
//! * Rows outside the requested range are dropped and counted.

use async_trait::async_trait;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use trace_chain_adapters::evm_v2::{parse_tx, Config, PairInfo, TxContext};
use trace_database::ingestion::{self, InsertOutcome, RawEvent, RevisionPlan};
use trace_database::DbError;
use trace_domain::{Address, ChainId, SupportStatus};

#[derive(Debug, Clone, Copy)]
pub struct Head {
    pub latest: u64,
    pub finalized: Option<u64>,
}

/// A transaction with Swap logs plus the pair metadata needed to parse it.
#[derive(Debug, Clone)]
pub struct Bundle {
    pub tx: TxContext,
    pub pairs: HashMap<Address, PairInfo>,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("source error: {0}")]
    Source(String),
    #[error("protocol {chain}:{protocol} is not enabled; refusing to ingest")]
    NotEnabled { chain: String, protocol: String },
    #[error(transparent)]
    Db(#[from] DbError),
}

#[async_trait]
pub trait EvmSource: Send + Sync {
    async fn head(&self) -> Result<Head, IngestError>;
    /// Canonical hash of block `n`, or `None` if the chain has no such block.
    async fn block_hash(&self, n: u64) -> Result<Option<String>, IngestError>;
    /// Transactions with Swap logs for blocks `from..=to`.
    async fn bundles(&self, from: u64, to: u64) -> Result<Vec<Bundle>, IngestError>;
}

pub struct EvmIngestConfig {
    pub chain: ChainId,
    pub protocol: String,
    /// The registry status of this protocol. Callers read it from `registry::protocols()`.
    pub status: SupportStatus,
    pub routers: Vec<Address>,
    /// Blocks read per step.
    pub max_range: u64,
    /// Blocks rolled back when a reorg is detected.
    pub reorg_window: u64,
    /// Where to start when no cursor exists. `None` starts at the current head (no backfill).
    pub start_height: Option<u64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StepReport {
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub inserted: usize,
    pub duplicates: usize,
    pub skipped: usize,
    pub out_of_range: usize,
    pub reorged: u64,
    pub finalized: u64,
}

fn raw_payload(tx: &TxContext) -> serde_json::Value {
    json!({
        "tx_hash": tx.tx_hash, "from": tx.from, "to": tx.to,
        "block_number": tx.block_number, "block_hash": tx.block_hash, "block_time": tx.block_time,
        "logs": tx.logs.iter().map(|l| json!({
            "address": l.address, "topics": l.topics, "data": l.data, "logIndex": l.log_index
        })).collect::<Vec<_>>(),
    })
}

pub async fn step(
    pool: &sqlx::PgPool,
    src: &dyn EvmSource,
    cfg: &EvmIngestConfig,
) -> Result<StepReport, IngestError> {
    let chain = cfg.chain.as_str();
    if !matches!(cfg.status, SupportStatus::Enabled | SupportStatus::Partial) {
        return Err(IngestError::NotEnabled {
            chain: chain.into(),
            protocol: cfg.protocol.clone(),
        });
    }
    let mut report = StepReport::default();
    let head = src.head().await?;

    // Where are we, and is our last block still canonical?
    let mut next = match ingestion::get_cursor(pool, chain, &cfg.protocol).await? {
        Some((stored_hash, Some(h))) => {
            let h = h as u64;
            if src.block_hash(h).await?.as_deref() == Some(stored_hash.as_str()) {
                h + 1
            } else {
                let back = h.saturating_sub(cfg.reorg_window);
                report.reorged = ingestion::mark_reorged(pool, chain, back as i64 + 1).await?;
                let hash = src.block_hash(back).await?.ok_or_else(|| {
                    IngestError::Source(format!("no block {back} to roll back to"))
                })?;
                ingestion::set_cursor(pool, chain, &cfg.protocol, &hash, Some(back as i64)).await?;
                back + 1
            }
        }
        _ => cfg.start_height.unwrap_or(head.latest),
    };
    if next > head.latest {
        next = head.latest + 1; // nothing new
    }

    if next <= head.latest {
        let to = head.latest.min(next + cfg.max_range.max(1) - 1);
        report.from = Some(next);
        report.to = Some(to);
        let routers = cfg.routers.clone();
        let parse_cfg = Config {
            chain: cfg.chain,
            routers: &routers,
        };
        for mut b in src.bundles(next, to).await? {
            if b.tx.block_number < next || b.tx.block_number > to {
                report.out_of_range += 1;
                continue;
            }
            // Finality is judged from the head we read, not the source's claim.
            b.tx.finalized = head.finalized.is_some_and(|f| b.tx.block_number <= f);
            let payload = raw_payload(&b.tx);
            let digest = hex::encode(Sha256::digest(
                serde_json::to_vec(&payload).unwrap_or_default(),
            ));
            let outcome = parse_tx(&parse_cfg, &b.tx, &b.pairs);
            report.skipped += outcome.skipped.len();
            if !outcome.swaps.is_empty() {
                ingestion::store_raw_event(
                    pool,
                    &RawEvent {
                        chain,
                        tx_id: &b.tx.tx_hash,
                        event_index: 0,
                        source: "evm_rpc",
                        payload: &payload,
                        payload_sha256: &digest,
                        fixture: false,
                        observed_at: b.tx.block_time,
                    },
                )
                .await?;
            }
            for mut s in outcome.swaps {
                match ingestion::revision_plan(pool, chain, &s.tx_id, s.event_index as i32).await? {
                    RevisionPlan::Live => report.duplicates += 1,
                    RevisionPlan::Next(rev) => {
                        s.finality_revision = rev;
                        match ingestion::insert_swap(pool, &s).await? {
                            InsertOutcome::Inserted => report.inserted += 1,
                            InsertOutcome::Duplicate => report.duplicates += 1,
                        }
                    }
                }
            }
        }
        // Advance only after every insert succeeded.
        let hash = src
            .block_hash(to)
            .await?
            .ok_or_else(|| IngestError::Source(format!("no block {to} after reading it")))?;
        ingestion::set_cursor(pool, chain, &cfg.protocol, &hash, Some(to as i64)).await?;
    }

    if let Some(f) = head.finalized {
        report.finalized = ingestion::finalize_up_to(pool, chain, f as i64).await?;
    }
    ingestion::record_successful_scan(pool, chain, &cfg.protocol, None).await?;
    Ok(report)
}
