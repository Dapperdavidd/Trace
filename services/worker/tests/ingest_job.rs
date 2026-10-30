//! Scheduling and running ingest jobs. Sources are scripted stand-ins.

use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use trace_chain_adapters::registry;
use trace_domain::SupportStatus;
use trace_jobs::{enqueue, lease, Failure};
use trace_worker::ingest::{Bundle, EvmSource, Head, IngestError};
use trace_worker::ingest_job::{
    run_ingest, schedule_ingest, Ctx, IngestRun, SourceFactory, INGEST_EVM,
};

struct Stub {
    calls: Arc<AtomicUsize>,
    fail: bool,
}

#[async_trait]
impl EvmSource for Stub {
    async fn head(&self) -> Result<Head, IngestError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(IngestError::Source("rpc down".into()));
        }
        Ok(Head {
            latest: 5,
            finalized: Some(3),
        })
    }
    async fn block_hash(&self, n: u64) -> Result<Option<String>, IngestError> {
        Ok(Some(format!("0xh{n}")))
    }
    async fn bundles(&self, _: u64, _: u64) -> Result<Vec<Bundle>, IngestError> {
        Ok(vec![])
    }
}

fn ctx(status: SupportStatus, with_url: bool, fail: bool) -> (Ctx, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut protocols = registry::protocols();
    for p in protocols.iter_mut().filter(|p| p.protocol == "uniswap_v2") {
        p.status = status;
    }
    let c = calls.clone();
    let source: SourceFactory = Box::new(move |_| {
        Ok(Box::new(Stub {
            calls: c.clone(),
            fail,
        }))
    });
    let rpc_urls = if with_url {
        HashMap::from([("ethereum".to_string(), "http://unused".to_string())])
    } else {
        HashMap::new()
    };
    (
        Ctx {
            protocols,
            rpc_urls,
            source: Some(source),
        },
        calls,
    )
}

fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

async fn queued_job(pool: &PgPool, t: chrono::DateTime<Utc>) -> trace_jobs::Job {
    enqueue(
        pool,
        INGEST_EVM,
        None,
        serde_json::json!({"chain":"ethereum","protocol":"uniswap_v2"}),
        t,
        3,
    )
    .await
    .unwrap();
    lease(pool, t, Duration::minutes(5), &[INGEST_EVM])
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn nothing_is_scheduled_while_every_protocol_is_disabled(pool: PgPool) {
    // The real registry today: nothing is enabled, even with an RPC URL set.
    let (mut c, _) = ctx(SupportStatus::Disabled, true, false);
    assert!(c.targets().is_empty());
    assert_eq!(schedule_ingest(&pool, &c, now()).await.unwrap(), 0);
    c.protocols = registry::protocols();
    assert!(c.targets().is_empty());
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn an_enabled_protocol_needs_an_rpc_url_and_is_scheduled_once_per_minute(pool: PgPool) {
    let (no_url, _) = ctx(SupportStatus::Enabled, false, false);
    assert!(no_url.targets().is_empty());

    let (c, _) = ctx(SupportStatus::Enabled, true, false);
    assert_eq!(c.targets().len(), 1);
    schedule_ingest(&pool, &c, now()).await.unwrap();
    schedule_ingest(&pool, &c, now() + Duration::seconds(20))
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs WHERE kind = 'ingest_evm'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "same minute, one job");
    schedule_ingest(&pool, &c, now() + Duration::seconds(90))
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs WHERE kind = 'ingest_evm'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 2);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_job_queued_before_the_protocol_was_disabled_does_not_run(pool: PgPool) {
    let (c, calls) = ctx(SupportStatus::Disabled, true, false);
    let job = queued_job(&pool, now()).await;
    assert!(matches!(
        run_ingest(&pool, &c, &job).await,
        Err(Failure::Permanent(_))
    ));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "the source was never touched"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_worker_without_ingest_configured_does_not_even_lease_the_job(pool: PgPool) {
    let t = now();
    enqueue(
        &pool,
        INGEST_EVM,
        None,
        serde_json::json!({"chain":"ethereum","protocol":"uniswap_v2"}),
        t,
        3,
    )
    .await
    .unwrap();
    assert_eq!(trace_worker::drain(&pool, t).await.unwrap(), 0);
    let status: String =
        sqlx::query_scalar("SELECT status FROM worker_jobs WHERE kind='ingest_evm'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "queued");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn drain_runs_an_ingest_job_and_sets_the_cursor(pool: PgPool) {
    let (c, calls) = ctx(SupportStatus::Enabled, true, false);
    let t = now();
    schedule_ingest(&pool, &c, t).await.unwrap();
    assert_eq!(trace_worker::drain_with(&pool, t, &c).await.unwrap(), 1);
    assert!(calls.load(Ordering::SeqCst) > 0);
    let cur = trace_database::ingestion::get_cursor(&pool, "ethereum", "uniswap_v2")
        .await
        .unwrap();
    assert_eq!(
        cur,
        Some(("0xh5".to_string(), Some(5))),
        "starts at the head, no backfill"
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM worker_jobs WHERE kind='ingest_evm'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "succeeded");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_source_outage_is_retried_not_killed(pool: PgPool) {
    let (c, _) = ctx(SupportStatus::Enabled, true, true);
    let t = now();
    schedule_ingest(&pool, &c, t).await.unwrap();
    trace_worker::drain_with(&pool, t, &c).await.unwrap();
    let (status, attempts): (String, i32) =
        sqlx::query_as("SELECT status, attempts FROM worker_jobs WHERE kind='ingest_evm'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((status.as_str(), attempts), ("queued", 1));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn only_one_worker_ingests_a_protocol_at_a_time(pool: PgPool) {
    let (c, calls) = ctx(SupportStatus::Enabled, true, false);
    let job = queued_job(&pool, now()).await;
    // Another worker holds the lock.
    let mut other = pool.acquire().await.unwrap();
    let held: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtextextended($1, 0))")
        .bind("ingest:ethereum:uniswap_v2")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(held);
    assert_eq!(
        run_ingest(&pool, &c, &job).await.unwrap(),
        IngestRun::AlreadyRunning
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
        .bind("ingest:ethereum:uniswap_v2")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(matches!(
        run_ingest(&pool, &c, &job).await.unwrap(),
        IngestRun::Stepped(_)
    ));
}
