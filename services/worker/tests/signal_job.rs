use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use trace_signals::Receipt;
use trace_worker::signal_job::{publish, schedule, PUBLISH_SIGNALS};

const TOKEN: &str = "0x00000000000000000000000000000000000000f1";
const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const POOL: &str = "0x2222222222222222222222222222222222222222";

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

fn complete_risk() -> serde_json::Value {
    json!({
        "honeypot": false,
        "sell_tax_bps": 0,
        "mint_authority_active": false,
        "freeze_authority_active": false,
        "blacklist_function": false,
        "liquidity_usd": "500000",
        "price_impact_bps": 50,
        "reference_size_usd": "1000",
        "top10_holder_pct": "20",
        "abnormal_activity": false
    })
}

async fn seed(pool: &PgPool, risk: serde_json::Value) {
    sqlx::query(
        "INSERT INTO tokens (chain,contract_or_mint,symbol,decimals)
         VALUES ('ethereum',$1,'REAL',18)",
    )
    .bind(TOKEN)
    .execute(pool)
    .await
    .unwrap();
    for n in 1..=3 {
        let wallet = format!("0x{n:040x}");
        sqlx::query("INSERT INTO wallets (chain,address) VALUES ('ethereum',$1)")
            .bind(&wallet)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO normalized_swaps
             (chain,tx_id,event_index,wallet,protocol,pool,token_in,amount_in,
              token_out,amount_out,observed_at,finality,attribution,parser_version,raw_ref,demo)
             VALUES ('ethereum',$1,0,$2,'uniswap_v2',$3,$4,1000000000,$5,
                     500000000000000000000,$6,'final','direct','test','test',FALSE)",
        )
        .bind(format!("0xsignal{n}"))
        .bind(wallet)
        .bind(POOL)
        .bind(USDC)
        .bind(TOKEN)
        .bind(now() - Duration::minutes(10 - n))
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO market_snapshots
         (chain,token,pool,win,price_now_usd,price_window_start_usd,
          volume_window_usd,liquidity_usd,risk_inputs,last_updated,demo)
         VALUES ('ethereum',$1,$2,'24h',2,1,3000,500000,$3,$4,FALSE)",
    )
    .bind(TOKEN)
    .bind(POOL)
    .bind(risk)
    .bind(now() - Duration::minutes(1))
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn publishes_one_sealed_signal_for_three_distinct_buyers(pool: PgPool) {
    seed(&pool, complete_risk()).await;
    let report = publish(&pool, now()).await.unwrap();
    assert_eq!((report.candidates, report.published), (1, 1));

    let (receipt_json, hash, state, demo): (serde_json::Value, String, String, bool) =
        sqlx::query_as("SELECT receipt,receipt_hash,state,demo FROM signals")
            .fetch_one(&pool)
            .await
            .unwrap();
    let receipt: Receipt = serde_json::from_value(receipt_json).unwrap();
    assert_eq!(receipt.source_tx_ids.len(), 3);
    assert_eq!(receipt.wallet_evidence_ids.len(), 3);
    assert_eq!(receipt.reference_price_usd, Decimal::from(2));
    assert_eq!(receipt.reference_amount_usd, Decimal::from(1000));
    assert_eq!(receipt.slippage_bps, 50);
    assert_eq!(receipt.seal(), hash);
    assert_eq!(state, "published_as_research");
    assert!(!demo);

    let events: Vec<String> = sqlx::query_scalar("SELECT state FROM signal_events ORDER BY seq")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(events, ["detected", "evaluated", "published_as_research"]);

    let second = publish(&pool, now() + Duration::minutes(1)).await.unwrap();
    assert_eq!((second.candidates, second.published), (0, 0));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn incomplete_risk_is_watch_only_and_missing_trade_size_is_skipped(pool: PgPool) {
    let mut risk = complete_risk();
    risk.as_object_mut().unwrap().remove("honeypot");
    seed(&pool, risk).await;
    let report = publish(&pool, now()).await.unwrap();
    assert_eq!((report.watch_only, report.published), (1, 0));
    let state: String = sqlx::query_scalar("SELECT state FROM signals")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "watch_only");

    sqlx::query("DELETE FROM signal_events")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM signals")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE market_snapshots SET risk_inputs = risk_inputs - 'reference_size_usd'")
        .execute(&pool)
        .await
        .unwrap();
    let skipped = publish(&pool, now() + Duration::minutes(1)).await.unwrap();
    assert_eq!((skipped.incomplete, skipped.published), (1, 0));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM signals")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn signal_job_is_scheduled_once_per_minute(pool: PgPool) {
    assert!(schedule(&pool, now()).await.unwrap());
    assert!(schedule(&pool, now()).await.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs WHERE kind=$1")
        .bind(PUBLISH_SIGNALS)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
