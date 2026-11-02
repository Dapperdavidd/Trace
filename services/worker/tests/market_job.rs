use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use trace_worker::market_job::{build, schedule, BUILD_MARKET_SNAPSHOTS};

const TOKEN: &str = "0x00000000000000000000000000000000000000f1";
const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const WALLET: &str = "0x1111111111111111111111111111111111111111";
const POOL: &str = "0x2222222222222222222222222222222222222222";

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

async fn seed(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO tokens (chain, contract_or_mint, symbol, decimals)
         VALUES ('ethereum',$1,'REAL',18)",
    )
    .bind(TOKEN)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO wallets (chain,address) VALUES ('ethereum',$1)")
        .bind(WALLET)
        .execute(pool)
        .await
        .unwrap();
    let per_base = Decimal::from_str("0.000000000000000002").unwrap();
    for (n, at) in [
        now() - Duration::days(7) - Duration::hours(1),
        now() - Duration::hours(25),
        now() - Duration::minutes(70),
        now() - Duration::minutes(1),
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query(
            "INSERT INTO price_points
             (chain,asset,observed_at,usd_per_base_unit,source,demo)
             VALUES ('ethereum',$1,$2,$3,$4,FALSE)",
        )
        .bind(TOKEN)
        .bind(at)
        .bind(per_base)
        .bind(format!("source-{n}"))
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO normalized_swaps
         (chain,tx_id,event_index,wallet,protocol,pool,token_in,amount_in,
          token_out,amount_out,observed_at,finality,attribution,parser_version,raw_ref,demo)
         VALUES ('ethereum','0xmarket',0,$1,'uniswap_v2',$2,$3,2000000,$4,
                 1000000000000000000,$5,'final','direct','test','test',FALSE)",
    )
    .bind(WALLET)
    .bind(POOL)
    .bind(USDC)
    .bind(TOKEN)
    .bind(now() - Duration::minutes(30))
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn real_prices_and_swaps_build_partial_snapshots(pool: PgPool) {
    seed(&pool).await;
    let report = build(&pool, now()).await.unwrap();
    assert_eq!((report.candidates, report.written), (1, 3));

    let row = sqlx::query(
        "SELECT price_now_usd, volume_window_usd, liquidity_usd, demo
         FROM market_snapshots WHERE chain='ethereum' AND token=$1 AND win='1h'",
    )
    .bind(TOKEN)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<Decimal, _>("price_now_usd"), Decimal::from(2));
    assert_eq!(row.get::<Decimal, _>("volume_window_usd"), Decimal::from(2));
    assert_eq!(row.get::<Option<Decimal>, _>("liquidity_usd"), None);
    assert!(!row.get::<bool, _>("demo"));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn market_job_is_only_scheduled_with_real_price_polling(pool: PgPool) {
    assert!(!schedule(&pool, false, now()).await.unwrap());
    assert!(schedule(&pool, true, now()).await.unwrap());
    assert!(schedule(&pool, true, now()).await.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs WHERE kind=$1")
        .bind(BUILD_MARKET_SNAPSHOTS)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
