use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use trace_chain_adapters::registry;
use trace_domain::SupportStatus;
use trace_worker::backfill_job::{self, BackfillResult, BACKFILL_EVM, MAX_RUN_BLOCKS};
use trace_worker::ingest::{Bundle, EvmSource, Head, IngestError};
use trace_worker::ingest_job::{Ctx, SourceFactory};

struct Stub {
    finalized: Arc<AtomicU64>,
    fail: Arc<AtomicBool>,
}

#[async_trait]
impl EvmSource for Stub {
    async fn head(&self) -> Result<Head, IngestError> {
        Ok(Head {
            latest: 1_000,
            finalized: Some(self.finalized.load(Ordering::SeqCst)),
        })
    }

    async fn block_hash(&self, n: u64) -> Result<Option<String>, IngestError> {
        Ok(Some(format!("0xh{n}")))
    }

    async fn bundles(&self, _: u64, _: u64) -> Result<Vec<Bundle>, IngestError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(IngestError::Source("RPC unavailable".into()));
        }
        Ok(vec![])
    }
}

fn context(finalized: Arc<AtomicU64>, fail: Arc<AtomicBool>) -> Ctx {
    let mut protocols = registry::protocols();
    for protocol in protocols.iter_mut().filter(|p| p.protocol == "uniswap_v2") {
        protocol.status = SupportStatus::Enabled;
    }
    let source: SourceFactory = Box::new(move |_| {
        Ok(Box::new(Stub {
            finalized: finalized.clone(),
            fail: fail.clone(),
        }))
    });
    Ctx {
        protocols,
        rpc_urls: HashMap::from([("ethereum".into(), "http://unused".into())]),
        source: Some(source),
        prices: None,
        solana_source: None,
        metadata: None,
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

async fn lease(pool: &PgPool, at: chrono::DateTime<Utc>) -> trace_jobs::Job {
    trace_jobs::lease(pool, at, Duration::minutes(5), &[BACKFILL_EVM])
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn request_is_bounded_and_idempotent(pool: PgPool) {
    assert!(
        backfill_job::request(&pool, "ethereum", "uniswap_v2", 20, 10)
            .await
            .is_err()
    );
    assert!(
        backfill_job::request(&pool, "ethereum", "uniswap_v2", 0, MAX_RUN_BLOCKS)
            .await
            .is_err()
    );
    assert!(backfill_job::request(&pool, "ethereum", "unknown", 10, 20)
        .await
        .is_err());

    let first = backfill_job::request(&pool, "ethereum", "uniswap_v2", 10, 20)
        .await
        .unwrap();
    let same = backfill_job::request(&pool, "ethereum", "uniswap_v2", 10, 20)
        .await
        .unwrap();
    assert_eq!(first, same);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn backfill_is_chunked_resumable_and_does_not_move_the_live_cursor(pool: PgPool) {
    let finalized = Arc::new(AtomicU64::new(500));
    let fail = Arc::new(AtomicBool::new(false));
    let ctx = context(finalized, fail);
    let id = backfill_job::request(&pool, "ethereum", "uniswap_v2", 10, 450)
        .await
        .unwrap();

    let mut at = now();
    for expected_next in [210, 410, 451] {
        assert_eq!(backfill_job::schedule(&pool, &ctx, at).await.unwrap(), 1);
        let job = lease(&pool, at).await;
        let result = backfill_job::run(&pool, &ctx, &job).await.unwrap();
        assert!(matches!(result, BackfillResult::Stepped(_)));
        let run = backfill_job::list(&pool, Some(id)).await.unwrap().remove(0);
        assert_eq!(run.next_height, expected_next);
        at += Duration::minutes(1);
    }

    let run = backfill_job::list(&pool, Some(id)).await.unwrap().remove(0);
    assert_eq!(run.status, "completed");
    assert!(
        trace_database::ingestion::get_cursor(&pool, "ethereum", "uniswap_v2")
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn progress_moves_only_after_a_successful_range(pool: PgPool) {
    let finalized = Arc::new(AtomicU64::new(100));
    let fail = Arc::new(AtomicBool::new(true));
    let ctx = context(finalized, fail.clone());
    let id = backfill_job::request(&pool, "ethereum", "uniswap_v2", 10, 20)
        .await
        .unwrap();
    let at = now();
    backfill_job::schedule(&pool, &ctx, at).await.unwrap();
    let job = lease(&pool, at).await;
    let failure = backfill_job::run(&pool, &ctx, &job).await.unwrap_err();
    trace_jobs::fail(&pool, &job, &failure, at).await.unwrap();
    let run = backfill_job::list(&pool, Some(id)).await.unwrap().remove(0);
    assert_eq!(run.next_height, 10);
    assert_eq!(run.status, "queued");
    assert!(run
        .last_error
        .as_deref()
        .unwrap()
        .contains("RPC unavailable"));

    fail.store(false, Ordering::SeqCst);
    let retry_job = trace_jobs::lease(
        &pool,
        at + Duration::seconds(31),
        Duration::minutes(5),
        &[BACKFILL_EVM],
    )
    .await
    .unwrap()
    .unwrap();
    let result = backfill_job::run(&pool, &ctx, &retry_job).await.unwrap();
    assert!(matches!(result, BackfillResult::Stepped(_)));
    let run = backfill_job::list(&pool, Some(id)).await.unwrap().remove(0);
    assert_eq!(run.next_height, 21);
    assert_eq!(run.status, "completed");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_range_waits_until_its_blocks_are_final(pool: PgPool) {
    let finalized = Arc::new(AtomicU64::new(9));
    let fail = Arc::new(AtomicBool::new(false));
    let ctx = context(finalized.clone(), fail);
    let id = backfill_job::request(&pool, "ethereum", "uniswap_v2", 10, 20)
        .await
        .unwrap();
    let at = now();
    backfill_job::schedule(&pool, &ctx, at).await.unwrap();
    let job = lease(&pool, at).await;
    assert_eq!(
        backfill_job::run(&pool, &ctx, &job).await.unwrap(),
        BackfillResult::WaitingForFinality
    );
    let run = backfill_job::list(&pool, Some(id)).await.unwrap().remove(0);
    assert_eq!(run.next_height, 10);
    assert_eq!(run.status, "running");

    finalized.store(20, Ordering::SeqCst);
    let later = at + Duration::minutes(1);
    backfill_job::schedule(&pool, &ctx, later).await.unwrap();
    let job = lease(&pool, later).await;
    assert!(matches!(
        backfill_job::run(&pool, &ctx, &job).await.unwrap(),
        BackfillResult::Stepped(_)
    ));
}
