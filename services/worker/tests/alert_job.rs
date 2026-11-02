use chrono::{DateTime, Duration, TimeZone, Utc};
use sqlx::PgPool;
use trace_worker::alert_job::generate;
use uuid::Uuid;

const WALLET: &str = "0x1111111111111111111111111111111111111111";
const TOKEN: &str = "0x00000000000000000000000000000000000000f1";
const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

async fn user(pool: &PgPool, email: &str, buy_alerts: bool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id,email) VALUES ($1,$2)")
        .bind(id)
        .bind(email)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO alert_preferences (user_id,wallet_buy) VALUES ($1,$2)")
        .bind(id)
        .bind(buy_alerts)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO watchlist_wallets (id,user_id,chain,address,alias,created_at)
         VALUES ($1,$2,'ethereum',$3,'Tracked wallet',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(id)
    .bind(WALLET)
    .bind(now() - Duration::hours(1))
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn seed(pool: &PgPool) -> (Uuid, Uuid) {
    sqlx::query("INSERT INTO wallets (chain,address) VALUES ('ethereum',$1)")
        .bind(WALLET)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO tokens (chain,contract_or_mint,symbol,decimals)
         VALUES ('ethereum',$1,'REAL',18)",
    )
    .bind(TOKEN)
    .execute(pool)
    .await
    .unwrap();
    let enabled = user(pool, "enabled@example.com", true).await;
    let disabled = user(pool, "disabled@example.com", false).await;
    sqlx::query(
        "INSERT INTO normalized_swaps
         (chain,tx_id,event_index,wallet,protocol,pool,token_in,amount_in,
          token_out,amount_out,observed_at,ingested_at,finality,attribution,
          parser_version,raw_ref,demo)
         VALUES ('ethereum','0xalert',0,$1,'uniswap_v2','pool',$2,2000000,
                 $3,1000000000000000000,$4,$5,'final','direct','test','test',FALSE)",
    )
    .bind(WALLET)
    .bind(USDC)
    .bind(TOKEN)
    .bind(now() - Duration::minutes(2))
    .bind(now() - Duration::minutes(1))
    .execute(pool)
    .await
    .unwrap();
    (enabled, disabled)
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn finalized_watchlist_activity_creates_idempotent_preference_aware_alerts(pool: PgPool) {
    let (enabled, disabled) = seed(&pool).await;
    let report = generate(&pool, now()).await.unwrap();
    assert_eq!((report.scanned, report.created, report.disabled), (2, 1, 1));
    let alert: (Uuid, String, String, bool) =
        sqlx::query_as("SELECT user_id,type,title,demo FROM alerts WHERE dedupe_key IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(alert.0, enabled);
    assert_eq!(alert.1, "wallet_buy");
    assert_eq!(alert.2, "Tracked wallet bought REAL");
    assert!(!alert.3);
    let disabled_count: i64 = sqlx::query_scalar("SELECT count(*) FROM alerts WHERE user_id=$1")
        .bind(disabled)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(disabled_count, 0);

    let second = generate(&pool, now() + Duration::minutes(1)).await.unwrap();
    assert_eq!((second.scanned, second.created), (0, 0));
}
