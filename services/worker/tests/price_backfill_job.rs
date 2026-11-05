use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use trace_domain::AssetId;
use trace_pricing::{
    Batch, HistoricalPriceProvider, HistoricalRequest, PriceError, PriceProvider, Quote,
};
use trace_worker::ingest_job::Ctx;
use trace_worker::price_backfill_job::{self, BACKFILL_PRICES};
use trace_worker::price_job::PriceCtx;

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

struct Stub {
    fail: AtomicBool,
}

#[async_trait]
impl PriceProvider for Stub {
    fn name(&self) -> &'static str {
        "stub"
    }

    async fn fetch(&self, _: &[AssetId]) -> Result<Batch, PriceError> {
        Ok(Batch::default())
    }
}

#[async_trait]
impl HistoricalPriceProvider for Stub {
    fn name(&self) -> &'static str {
        "stub"
    }

    async fn fetch_historical(&self, requests: &[HistoricalRequest]) -> Result<Batch, PriceError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(PriceError::Provider("provider down".into()));
        }
        Ok(Batch {
            quotes: requests
                .iter()
                .map(|request| Quote {
                    asset: request.asset.clone(),
                    usd_per_token: Decimal::from_str("2000.25").unwrap(),
                    decimals: None,
                    symbol: Some("WETH".into()),
                    observed_at: request.at + Duration::seconds(30),
                    confidence: Some(0.99),
                })
                .collect(),
            rejected: vec![],
        })
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

async fn token(pool: &PgPool, decimals: Option<i16>) {
    sqlx::query(
        "INSERT INTO tokens (chain, contract_or_mint, decimals)
         VALUES ('ethereum',$1,$2)",
    )
    .bind(WETH)
    .bind(decimals)
    .execute(pool)
    .await
    .unwrap();
}

fn context(stub: Arc<Stub>) -> Ctx {
    let mut ctx = Ctx::empty();
    ctx.prices = Some(PriceCtx {
        provider: stub.clone(),
        historical: Some(stub),
    });
    ctx
}

async fn lease(pool: &PgPool, at: chrono::DateTime<Utc>) -> trace_jobs::Job {
    trace_jobs::lease(pool, at, Duration::minutes(5), &[BACKFILL_PRICES])
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn requests_require_verified_decimals_and_are_idempotent(pool: PgPool) {
    token(&pool, None).await;
    let from = now() - Duration::days(2);
    let to = now() - Duration::days(1);
    assert!(
        price_backfill_job::request(&pool, "ethereum", WETH, from, to, 3600)
            .await
            .is_err()
    );
    sqlx::query("UPDATE tokens SET decimals=18 WHERE contract_or_mint=$1")
        .bind(WETH)
        .execute(&pool)
        .await
        .unwrap();
    let first = price_backfill_job::request(&pool, "ethereum", WETH, from, to, 3600)
        .await
        .unwrap();
    let same = price_backfill_job::request(&pool, "ethereum", WETH, from, to, 3600)
        .await
        .unwrap();
    assert_eq!(first, same);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn backfill_is_chunked_exact_and_resumable(pool: PgPool) {
    token(&pool, Some(18)).await;
    let stub = Arc::new(Stub {
        fail: AtomicBool::new(false),
    });
    let ctx = context(stub);
    let from = now() - Duration::days(3);
    let to = from + Duration::hours(44);
    let id = price_backfill_job::request(&pool, "ethereum", WETH, from, to, 3600)
        .await
        .unwrap();

    let at = now();
    assert_eq!(
        price_backfill_job::schedule(&pool, true, at).await.unwrap(),
        1
    );
    let first = price_backfill_job::run(&pool, &ctx, &lease(&pool, at).await)
        .await
        .unwrap();
    assert_eq!(
        (first.requested, first.stored, first.completed),
        (40, 40, false)
    );

    let later = at + Duration::minutes(1);
    price_backfill_job::schedule(&pool, true, later)
        .await
        .unwrap();
    let second = price_backfill_job::run(&pool, &ctx, &lease(&pool, later).await)
        .await
        .unwrap();
    assert_eq!(
        (second.requested, second.stored, second.completed),
        (5, 5, true)
    );
    let run = price_backfill_job::list(&pool, Some(id))
        .await
        .unwrap()
        .remove(0);
    assert_eq!((run.status.as_str(), run.stored_points), ("completed", 45));
    let points: i64 = sqlx::query_scalar("SELECT count(*) FROM price_points")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(points, 45);
    let value: Decimal = sqlx::query_scalar(
        "SELECT usd_per_base_unit FROM price_points ORDER BY observed_at LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        value,
        Decimal::from_str("2000.25").unwrap() / Decimal::from_str("1000000000000000000").unwrap()
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn provider_failure_does_not_advance_progress(pool: PgPool) {
    token(&pool, Some(18)).await;
    let stub = Arc::new(Stub {
        fail: AtomicBool::new(true),
    });
    let ctx = context(stub.clone());
    let from = now() - Duration::days(2);
    let id = price_backfill_job::request(
        &pool,
        "ethereum",
        WETH,
        from,
        from + Duration::hours(2),
        3600,
    )
    .await
    .unwrap();
    let at = now();
    price_backfill_job::schedule(&pool, true, at).await.unwrap();
    let job = lease(&pool, at).await;
    let failure = price_backfill_job::run(&pool, &ctx, &job)
        .await
        .unwrap_err();
    trace_jobs::fail(&pool, &job, &failure, at).await.unwrap();
    let run = price_backfill_job::list(&pool, Some(id))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(run.next_time, from);
    assert_eq!(run.stored_points, 0);
    assert!(run.last_error.as_deref().unwrap().contains("provider down"));

    stub.fail.store(false, Ordering::SeqCst);
    let retry = trace_jobs::lease(
        &pool,
        at + Duration::seconds(31),
        Duration::minutes(5),
        &[BACKFILL_PRICES],
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        price_backfill_job::run(&pool, &ctx, &retry)
            .await
            .unwrap()
            .stored,
        3
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn concurrent_workers_do_not_share_a_price_backfill(pool: PgPool) {
    token(&pool, Some(18)).await;
    let stub = Arc::new(Stub {
        fail: AtomicBool::new(false),
    });
    let ctx = context(stub);
    let from = now() - Duration::days(1);
    let id = price_backfill_job::request(
        &pool,
        "ethereum",
        WETH,
        from,
        from + Duration::hours(1),
        3600,
    )
    .await
    .unwrap();
    price_backfill_job::schedule(&pool, true, now())
        .await
        .unwrap();
    let job = lease(&pool, now()).await;
    let mut other = pool.acquire().await.unwrap();
    let key = format!("price-backfill:{id}");
    let held: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtextextended($1, 0))")
        .bind(&key)
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(held);

    assert_eq!(
        price_backfill_job::run(&pool, &ctx, &job).await.unwrap(),
        price_backfill_job::PriceBackfillReport::default()
    );
    let run = price_backfill_job::list(&pool, Some(id))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(run.next_time, from);
    assert_eq!(run.stored_points, 0);

    sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
        .bind(&key)
        .fetch_one(&mut *other)
        .await
        .unwrap();
}
