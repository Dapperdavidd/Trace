use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use trace_domain::{ChainId, SupportStatus};
use trace_worker::ingest::IngestError;
use trace_worker::ingest_job::Ctx;
use trace_worker::solana_backfill_job::{self, BACKFILL_SOLANA};
use trace_worker::solana_ingest::{SigInfo, SolanaSource};

#[derive(Clone)]
struct Stub {
    signatures: Arc<Vec<SigInfo>>,
}

#[async_trait]
impl SolanaSource for Stub {
    async fn signatures(
        &self,
        _: &str,
        before: Option<&str>,
        until: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SigInfo>, IngestError> {
        let start = before.map_or(0, |before| {
            self.signatures
                .iter()
                .position(|item| item.signature == before)
                .map_or(self.signatures.len(), |index| index + 1)
        });
        let end = until.map_or(self.signatures.len(), |until| {
            self.signatures
                .iter()
                .position(|item| item.signature == until)
                .unwrap_or(self.signatures.len())
        });
        Ok(self
            .signatures
            .iter()
            .take(end)
            .skip(start)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn transaction(&self, _: &str) -> Result<Option<Value>, IngestError> {
        panic!("failed on-chain signatures must not be fetched")
    }
}

fn source() -> Stub {
    Stub {
        signatures: Arc::new(
            (0..=5)
                .rev()
                .map(|number| SigInfo {
                    signature: format!("s{number}"),
                    slot: number,
                    failed: true,
                })
                .collect(),
        ),
    }
}

fn context(status: SupportStatus) -> Ctx {
    let mut ctx = Ctx::empty();
    for protocol in ctx
        .protocols
        .iter_mut()
        .filter(|p| p.chain == ChainId::Solana && p.protocol == "raydium_amm_v4")
    {
        protocol.status = status;
    }
    ctx.rpc_urls.insert("solana".into(), "http://unused".into());
    let stub = source();
    ctx.solana_source = Some(Box::new(move || Ok(Box::new(stub.clone()))));
    ctx
}

fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

async fn lease(pool: &PgPool, at: chrono::DateTime<Utc>) -> trace_jobs::Job {
    trace_jobs::lease(pool, at, Duration::minutes(5), &[BACKFILL_SOLANA])
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn request_is_validated_and_idempotent(pool: PgPool) {
    assert!(
        solana_backfill_job::request(&pool, "unknown", "s5", "s0", 10)
            .await
            .is_err()
    );
    assert!(
        solana_backfill_job::request(&pool, "raydium_amm_v4", "s5", "s5", 10)
            .await
            .is_err()
    );
    let first = solana_backfill_job::request(&pool, "raydium_amm_v4", "s5", "s0", 10)
        .await
        .unwrap();
    let same = solana_backfill_job::request(&pool, "raydium_amm_v4", "s5", "s0", 10)
        .await
        .unwrap();
    assert_eq!(first, same);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn explicit_boundaries_complete_without_moving_the_live_cursor(pool: PgPool) {
    let ctx = context(SupportStatus::Enabled);
    let id = solana_backfill_job::request(&pool, "raydium_amm_v4", "s5", "s0", 10)
        .await
        .unwrap();
    assert_eq!(
        solana_backfill_job::schedule(&pool, &ctx, now())
            .await
            .unwrap(),
        1
    );
    let report = solana_backfill_job::run(&pool, &ctx, &lease(&pool, now()).await)
        .await
        .unwrap();
    assert_eq!(report.signatures, 4);
    assert_eq!(report.ingestion.failed_onchain, 4);
    assert!(report.completed);
    let run = solana_backfill_job::list(&pool, Some(id))
        .await
        .unwrap()
        .remove(0);
    assert_eq!((run.status.as_str(), run.processed), ("completed", 4));
    assert!(
        trace_database::ingestion::get_cursor(&pool, "solana", "raydium_amm_v4")
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn configured_limit_is_reported_instead_of_hiding_a_partial_range(pool: PgPool) {
    let ctx = context(SupportStatus::Enabled);
    let id = solana_backfill_job::request(&pool, "raydium_amm_v4", "s5", "s0", 2)
        .await
        .unwrap();
    solana_backfill_job::schedule(&pool, &ctx, now())
        .await
        .unwrap();
    let report = solana_backfill_job::run(&pool, &ctx, &lease(&pool, now()).await)
        .await
        .unwrap();
    assert!(report.limit_reached);
    assert!(!report.completed);
    let run = solana_backfill_job::list(&pool, Some(id))
        .await
        .unwrap()
        .remove(0);
    assert_eq!((run.status.as_str(), run.processed), ("limit_reached", 2));
    assert_eq!(run.next_before, "s3");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn disabled_protocols_are_not_scheduled(pool: PgPool) {
    solana_backfill_job::request(&pool, "raydium_amm_v4", "s5", "s0", 10)
        .await
        .unwrap();
    assert_eq!(
        solana_backfill_job::schedule(&pool, &context(SupportStatus::Disabled), now())
            .await
            .unwrap(),
        0
    );
}
