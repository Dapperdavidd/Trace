//! Solana ingestion engine (finalized commitment only).
//!
//! Signatures for the DEX program are read newest-first until the stored
//! cursor, then processed oldest-first so the cursor only ever moves forward
//! over transactions that were actually handled.
//!
//! Rules:
//! * Only Enabled or Partial protocols are ingested.
//! * Everything is fetched at `finalized`, so every swap is `final`. There is
//!   no reorg handling because finalized slots do not reorg.
//! * Failed on-chain transactions are skipped without being fetched.
//! * A finalized signature whose transaction cannot be fetched is an error
//!   (retry), never a silent skip.
//! * If more than `max_backlog` signatures arrived since the cursor, the oldest
//!   ones are NOT ingested. That gap is audited (`ingest_gap`), not hidden.
//! * First run starts at the newest signature and does not backfill.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use trace_chain_adapters::solana::{parse_tx, Config, ProgramAllow, TxResult};
use trace_database::ingestion::{self, InsertOutcome, RawEvent};
use trace_domain::SupportStatus;

use crate::ingest::IngestError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigInfo {
    pub signature: String,
    pub slot: u64,
    /// The transaction failed on chain.
    pub failed: bool,
}

#[async_trait]
pub trait SolanaSource: Send + Sync {
    /// Signatures mentioning `program`, newest first, strictly between `until` and `before`.
    async fn signatures(
        &self,
        program: &str,
        before: Option<&str>,
        until: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SigInfo>, IngestError>;
    /// `getTransaction` (jsonParsed, finalized) result, or `None` if unavailable.
    async fn transaction(&self, signature: &str) -> Result<Option<serde_json::Value>, IngestError>;
}

pub struct SolanaIngestConfig {
    pub protocol: String,
    pub status: SupportStatus,
    /// Program whose signatures are polled.
    pub poll_program: &'static str,
    pub programs: &'static [ProgramAllow],
    /// Signatures per RPC page.
    pub page: usize,
    /// Most signatures considered per step. More than this is a gap.
    pub max_backlog: usize,
    /// Most transactions fetched and parsed per step.
    pub max_per_step: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SolanaReport {
    pub collected: usize,
    pub failed_onchain: usize,
    pub fetched: usize,
    pub inserted: usize,
    pub duplicates: usize,
    pub skipped: BTreeMap<String, usize>,
    pub gap: bool,
    pub bootstrapped: bool,
}

pub async fn step(
    pool: &sqlx::PgPool,
    src: &dyn SolanaSource,
    cfg: &SolanaIngestConfig,
) -> Result<SolanaReport, IngestError> {
    if !matches!(cfg.status, SupportStatus::Enabled | SupportStatus::Partial) {
        return Err(IngestError::NotEnabled {
            chain: "solana".into(),
            protocol: cfg.protocol.clone(),
        });
    }
    let mut report = SolanaReport::default();
    let cursor = ingestion::get_cursor(pool, "solana", &cfg.protocol).await?;

    let Some((cursor_sig, _)) = cursor else {
        // No backfill: start from the newest signature.
        if let Some(newest) = src
            .signatures(cfg.poll_program, None, None, 1)
            .await?
            .first()
        {
            ingestion::set_cursor(
                pool,
                "solana",
                &cfg.protocol,
                &newest.signature,
                Some(newest.slot as i64),
            )
            .await?;
        }
        report.bootstrapped = true;
        ingestion::record_successful_scan(pool, "solana", &cfg.protocol, None).await?;
        return Ok(report);
    };

    // Newest-first until the cursor.
    let mut collected: Vec<SigInfo> = Vec::new();
    let mut before: Option<String> = None;
    let mut more_beyond_limit = false;
    loop {
        let page = src
            .signatures(
                cfg.poll_program,
                before.as_deref(),
                Some(&cursor_sig),
                cfg.page,
            )
            .await?;
        let full = page.len() >= cfg.page;
        before = page.last().map(|s| s.signature.clone());
        collected.extend(page);
        if collected.len() >= cfg.max_backlog {
            more_beyond_limit = full || collected.len() > cfg.max_backlog;
            collected.truncate(cfg.max_backlog);
            break;
        }
        if !full {
            break;
        }
    }
    report.collected = collected.len();
    report.gap = more_beyond_limit;
    let oldest_collected = collected.last().map(|s| s.signature.clone());

    // Oldest first, bounded per step.
    let batch: Vec<SigInfo> = collected.into_iter().rev().take(cfg.max_per_step).collect();
    let parse_cfg = Config {
        programs: cfg.programs,
        finalized: true,
    };
    for s in &batch {
        if s.failed {
            report.failed_onchain += 1;
            continue;
        }
        let raw = src.transaction(&s.signature).await?.ok_or_else(|| {
            IngestError::Source(format!(
                "finalized transaction {} is not available",
                s.signature
            ))
        })?;
        report.fetched += 1;
        let tx: TxResult = match serde_json::from_value(raw.clone()) {
            Ok(t) => t,
            Err(_) => {
                *report.skipped.entry("Unparseable".into()).or_default() += 1;
                continue;
            }
        };
        match parse_tx(&parse_cfg, &tx) {
            Ok(swap) => {
                let digest =
                    hex::encode(Sha256::digest(serde_json::to_vec(&raw).unwrap_or_default()));
                ingestion::store_raw_event(
                    pool,
                    &RawEvent {
                        chain: "solana",
                        tx_id: &s.signature,
                        event_index: 0,
                        source: "solana_rpc",
                        payload: &raw,
                        payload_sha256: &digest,
                        fixture: false,
                        observed_at: swap.observed_at,
                    },
                )
                .await?;
                match ingestion::insert_swap(pool, &swap).await? {
                    InsertOutcome::Inserted => report.inserted += 1,
                    InsertOutcome::Duplicate => report.duplicates += 1,
                }
            }
            Err(skip) => {
                let name = format!("{skip:?}");
                let key = name
                    .split(['(', ' ', '{'])
                    .next()
                    .unwrap_or("Skipped")
                    .to_string();
                *report.skipped.entry(key).or_default() += 1;
            }
        }
    }

    // Advance only after every transaction in the batch was handled.
    if let Some(last) = batch.last() {
        ingestion::set_cursor(
            pool,
            "solana",
            &cfg.protocol,
            &last.signature,
            Some(last.slot as i64),
        )
        .await?;
    }
    if report.gap {
        sqlx::query("INSERT INTO audit_events (id, actor, action, subject, detail) VALUES ($1,'indexer','ingest_gap',$2,$3)")
            .bind(uuid::Uuid::new_v4())
            .bind(format!("solana:{}", cfg.protocol))
            .bind(serde_json::json!({
                "after_signature": cursor_sig,
                "first_ingested_signature": oldest_collected,
                "max_backlog": cfg.max_backlog,
                "note": "signatures older than the first ingested one were not read",
            }))
            .execute(pool)
            .await
            .map_err(trace_database::DbError::from)?;
    }
    ingestion::record_successful_scan(pool, "solana", &cfg.protocol, None).await?;
    Ok(report)
}
