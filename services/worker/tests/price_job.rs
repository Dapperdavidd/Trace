//! Price polling against a scripted provider. Synthetic: it proves storage and
//! rejection behaviour, not that any provider's live answers parse.

use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use trace_domain::{AssetId, ChainId};
use trace_jobs::Failure;
use trace_pricing::{Batch, PriceError, PriceProvider, Quote};
use trace_worker::ingest_job::Ctx;
use trace_worker::price_job::{poll, schedule_prices, tracked_assets, PriceCtx, POLL_PRICES};

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const DEMO: &str = "0x000000000000000000000000000000000000d001";
const OLD: &str = "0x00000000000000000000000000000000000000aa";
const WALLET: &str = "0x1111111111111111111111111111111111111111";

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

fn eth(a: &str) -> AssetId {
    AssetId::parse(ChainId::Ethereum, a).unwrap()
}

struct Stub {
    quotes: Mutex<Vec<Quote>>,
    fail: bool,
    calls: AtomicUsize,
}

impl Stub {
    fn new(quotes: Vec<Quote>) -> Arc<Self> {
        Arc::new(Self {
            quotes: Mutex::new(quotes),
            fail: false,
            calls: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl PriceProvider for Stub {
    fn name(&self) -> &'static str {
        "stub"
    }
    async fn fetch(&self, _: &[AssetId]) -> Result<Batch, PriceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(PriceError::Provider("down".into()));
        }
        Ok(Batch {
            quotes: self.quotes.lock().unwrap().clone(),
            rejected: vec![],
        })
    }
}

fn quote(asset: &str, price: &str, decimals: Option<u8>, age_secs: i64) -> Quote {
    Quote {
        asset: eth(asset),
        usd_per_token: Decimal::from_str(price).unwrap(),
        decimals,
        symbol: Some("SYM".into()),
        observed_at: now() - Duration::seconds(age_secs),
        confidence: Some(0.99),
    }
}

async fn swap(
    pool: &PgPool,
    n: i32,
    tin: &str,
    tout: &str,
    demo: bool,
    finality: &str,
    at: DateTime<Utc>,
) {
    sqlx::query(
        "INSERT INTO wallets (chain, address) VALUES ('ethereum', $1) ON CONFLICT DO NOTHING",
    )
    .bind(WALLET)
    .execute(pool)
    .await
    .unwrap();
    for t in [tin, tout] {
        sqlx::query("INSERT INTO tokens (chain, contract_or_mint) VALUES ('ethereum', $1) ON CONFLICT DO NOTHING")
            .bind(t).execute(pool).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO normalized_swaps (chain, tx_id, event_index, wallet, protocol, pool, token_in, amount_in,
           token_out, amount_out, observed_at, finality, attribution, parser_version, raw_ref, demo)
         VALUES ('ethereum', $1, 0, $2, 'uniswap_v2', '0xpool', $3, 1, $4, 1, $5, $6, 'direct', 't', 'r', $7)",
    )
    .bind(format!("0xtx{n}")).bind(WALLET).bind(tin).bind(tout).bind(at).bind(finality).bind(demo)
    .execute(pool).await.unwrap();
}

async fn seed(pool: &PgPool) {
    swap(
        pool,
        1,
        USDC,
        WETH,
        false,
        "final",
        now() - Duration::hours(1),
    )
    .await;
    swap(
        pool,
        2,
        DEMO,
        WETH,
        true,
        "final",
        now() - Duration::hours(1),
    )
    .await;
    swap(
        pool,
        3,
        OLD,
        WETH,
        false,
        "final",
        now() - Duration::days(30),
    )
    .await;
    swap(
        pool,
        4,
        OLD,
        WETH,
        false,
        "reorged",
        now() - Duration::hours(1),
    )
    .await;
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn only_real_recent_non_reorged_assets_are_requested(pool: PgPool) {
    seed(&pool).await;
    let got = tracked_assets(&pool, now()).await.unwrap();
    let mut addrs: Vec<&str> = got.iter().map(|a| a.contract_or_mint.as_str()).collect();
    addrs.sort();
    assert_eq!(
        addrs,
        vec![USDC, WETH],
        "demo, month-old and reorged-only tokens are not priced"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn an_accepted_quote_is_stored_at_its_own_time_per_base_unit(pool: PgPool) {
    seed(&pool).await;
    sqlx::query("UPDATE tokens SET decimals = 18 WHERE contract_or_mint = $1")
        .bind(WETH)
        .execute(&pool)
        .await
        .unwrap();
    let p = Stub::new(vec![quote(WETH, "3000", Some(18), 30)]);
    let r = poll(&pool, p.as_ref(), now()).await.unwrap();
    assert_eq!((r.requested, r.stored), (2, 1));
    let (at, v, src, demo): (DateTime<Utc>, Decimal, String, bool) = sqlx::query_as(
        "SELECT observed_at, usd_per_base_unit, source, demo FROM price_points WHERE asset = $1",
    )
    .bind(WETH)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        at,
        now() - Duration::seconds(30),
        "the quote's time, not the poll time"
    );
    assert_eq!(
        v,
        Decimal::from_str("3000").unwrap() / Decimal::from_str("1000000000000000000").unwrap()
    );
    assert_eq!((src.as_str(), demo), ("stub", false));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn decimals_are_learned_only_when_unknown_and_flagged(pool: PgPool) {
    seed(&pool).await;
    let p = Stub::new(vec![quote(USDC, "1.0", Some(6), 10)]);
    let r = poll(&pool, p.as_ref(), now()).await.unwrap();
    assert_eq!((r.stored, r.learned_decimals), (1, 1));
    let d: Option<i16> =
        sqlx::query_scalar("SELECT decimals FROM tokens WHERE contract_or_mint = $1")
            .bind(USDC)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(d, Some(6));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn rejected_quotes_store_nothing_and_are_counted_by_reason(pool: PgPool) {
    seed(&pool).await;
    sqlx::query("UPDATE tokens SET decimals = 18 WHERE contract_or_mint = $1")
        .bind(WETH)
        .execute(&pool)
        .await
        .unwrap();
    let p = Stub::new(vec![
        quote(WETH, "3000", Some(6), 10),  // decimals disagree with ours
        quote(USDC, "1.0", Some(6), 3600), // stale
    ]);
    let r = poll(&pool, p.as_ref(), now()).await.unwrap();
    assert_eq!(r.stored, 0);
    assert_eq!(r.rejected.get("DecimalsMismatch"), Some(&1));
    assert_eq!(r.rejected.get("Stale"), Some(&1));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM price_points")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn polling_twice_does_not_duplicate(pool: PgPool) {
    seed(&pool).await;
    sqlx::query("UPDATE tokens SET decimals = 18 WHERE contract_or_mint = $1")
        .bind(WETH)
        .execute(&pool)
        .await
        .unwrap();
    let p = Stub::new(vec![quote(WETH, "3000", Some(18), 30)]);
    poll(&pool, p.as_ref(), now()).await.unwrap();
    let r = poll(&pool, p.as_ref(), now()).await.unwrap();
    assert_eq!((r.stored, r.duplicates), (0, 1));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn an_outage_is_transient_and_nothing_is_asked_when_nothing_is_tracked(pool: PgPool) {
    let p = Arc::new(Stub {
        quotes: Mutex::new(vec![]),
        fail: true,
        calls: AtomicUsize::new(0),
    });
    poll(&pool, p.as_ref(), now()).await.unwrap();
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        0,
        "no tracked assets, no request"
    );
    seed(&pool).await;
    assert!(matches!(
        poll(&pool, p.as_ref(), now()).await,
        Err(Failure::Transient(_))
    ));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn the_job_exists_only_where_a_provider_is_configured(pool: PgPool) {
    assert!(!schedule_prices(&pool, false, now()).await.unwrap());
    assert!(schedule_prices(&pool, true, now()).await.unwrap());
    schedule_prices(&pool, true, now() + Duration::seconds(30))
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs WHERE kind = $1")
        .bind(POLL_PRICES)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "same 5-minute bucket, one job");

    // A worker without a provider does not lease it.
    assert_eq!(trace_worker::drain(&pool, now()).await.unwrap(), 0);
    seed(&pool).await;
    let p = Stub::new(vec![]);
    let mut ctx = Ctx::empty();
    ctx.prices = Some(PriceCtx {
        provider: p.clone(),
    });
    assert_eq!(
        trace_worker::drain_with(&pool, now(), &ctx).await.unwrap(),
        1
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
}
