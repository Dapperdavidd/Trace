use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use trace_signals::{Receipt, HORIZONS_SECS};
use trace_worker::outcomes::evaluate_due;
use uuid::Uuid;

const TOKEN: &str = "0x000000000000000000000000000000000000d001";

fn published() -> DateTime<Utc> {
    Utc.timestamp_opt(1_790_000_000, 0).unwrap()
}

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn receipt() -> Receipt {
    Receipt {
        chain: "ethereum".into(),
        token: TOKEN.into(),
        source_tx_ids: vec!["0xabc".into()],
        wallet_evidence_ids: vec![],
        block_time: published(),
        detected_at: published(),
        published_at: published(),
        reference_price_usd: d("1.00"),
        reference_amount_usd: d("1000"),
        fee_bps: 30,
        slippage_bps: 50,
        risk_verdict: "pass".into(),
        security_findings: vec![],
        wqs: Some(70),
        wcs: None,
        tcq: Some(60),
        score_versions: vec![],
        rules_version: "signals/1".into(),
        exit_policy: "first available price at or after each horizon".into(),
        horizons_secs: HORIZONS_SECS.to_vec(),
    }
}

async fn setup_token(pool: &PgPool, liquidity: Option<i64>) {
    sqlx::query("INSERT INTO tokens (chain, contract_or_mint, symbol, decimals) VALUES ('ethereum',$1,'T',18) ON CONFLICT DO NOTHING")
        .bind(TOKEN)
        .execute(pool)
        .await
        .unwrap();
    if let Some(l) = liquidity {
        sqlx::query("INSERT INTO market_snapshots (chain, token, pool, win, price_now_usd, price_window_start_usd, volume_window_usd, liquidity_usd, last_updated) VALUES ('ethereum',$1,'p','24h',1,1,1,$2,$3)")
            .bind(TOKEN)
            .bind(Decimal::from(l))
            .bind(published())
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn price(pool: &PgPool, at: DateTime<Utc>, usd: &str) {
    sqlx::query("INSERT INTO price_points (chain, asset, observed_at, usd_per_base_unit, source) VALUES ('ethereum',$1,$2,$3,'test')")
        .bind(TOKEN)
        .bind(at)
        .bind(d(usd) / Decimal::from(1_000_000_000_000_000_000u64))
        .execute(pool)
        .await
        .unwrap();
}

async fn signal(pool: &PgPool, r: &Receipt, state: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO signals (id, wallet_chain, wallet_address, token_chain, token, receipt, receipt_hash, state, published_at, demo) VALUES ($1,'ethereum','0x00000000000000000000000000000000000a0001','ethereum',$2,$3,$4,$5,$6,TRUE)")
        .bind(id)
        .bind(TOKEN)
        .bind(serde_json::to_value(r).unwrap())
        .bind(r.seal())
        .bind(state)
        .bind(r.published_at)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO signal_events (signal_id, seq, state, at) VALUES ($1,0,$2,$3)")
        .bind(id)
        .bind(state)
        .bind(r.published_at)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn outcome(
    pool: &PgPool,
    id: Uuid,
    h: i64,
) -> Option<(String, Option<Decimal>, Option<Decimal>)> {
    sqlx::query_as("SELECT kind, price_usd, return_pct FROM signal_outcomes WHERE signal_id=$1 AND horizon_secs=$2")
        .bind(id)
        .bind(h)
        .fetch_optional(pool)
        .await
        .unwrap()
}

async fn state(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT state FROM signals WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn uses_the_first_price_after_the_horizon_not_a_later_peak(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    price(&pool, published() + Duration::minutes(10), "9.00").await; // before the horizon, must be ignored
    price(&pool, published() + Duration::minutes(70), "1.30").await; // first at or after 1h
    price(&pool, published() + Duration::hours(3), "5.00").await; // later peak, must be ignored
    let id = signal(&pool, &receipt(), "published_as_research").await;
    let now = published() + Duration::hours(2);
    let rep = evaluate_due(&pool, now).await.unwrap();
    assert_eq!(rep.recorded, 1);
    let (kind, px, ret) = outcome(&pool, id, 3_600).await.unwrap();
    assert_eq!(kind, "observed");
    assert_eq!(px.unwrap().normalize(), d("1.3"));
    // 3000 bps gross, minus 2 x 30 fee and 50 slippage = 2890 bps
    assert_eq!(ret.unwrap(), d("28.90"));
    assert!(
        outcome(&pool, id, 86_400).await.is_none(),
        "later horizons are not due"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn losses_and_untradeable_exits_are_recorded_like_gains(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    price(&pool, published() + Duration::hours(1), "0.80").await;
    let id = signal(&pool, &receipt(), "published_as_research").await;
    evaluate_due(&pool, published() + Duration::hours(2))
        .await
        .unwrap();
    let (kind, _, ret) = outcome(&pool, id, 3_600).await.unwrap();
    assert_eq!(kind, "observed");
    assert!(ret.unwrap() < Decimal::ZERO, "a loss is stored as a loss");

    // no liquidity data: the exit cannot be claimed
    let pool2 = pool.clone();
    sqlx::query("DELETE FROM market_snapshots")
        .execute(&pool2)
        .await
        .unwrap();
    let id2 = signal(&pool, &receipt(), "published_as_research").await;
    evaluate_due(&pool, published() + Duration::hours(2))
        .await
        .unwrap();
    assert_eq!(outcome(&pool, id2, 3_600).await.unwrap().0, "untradeable");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn waiting_unpriced_and_expired_are_distinguished(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    let id = signal(&pool, &receipt(), "published_as_research").await;
    // due, no price yet, still inside the lag window: wait, record nothing
    let rep = evaluate_due(&pool, published() + Duration::hours(2))
        .await
        .unwrap();
    assert_eq!((rep.recorded, rep.not_ready), (0, 1));
    assert!(outcome(&pool, id, 3_600).await.is_none());
    // lag window over with no price: unpriced
    evaluate_due(&pool, published() + Duration::hours(10))
        .await
        .unwrap();
    assert_eq!(outcome(&pool, id, 3_600).await.unwrap().0, "unpriced");

    // a price exists but only after the allowed lag: expired, not used
    let id2 = signal(&pool, &receipt(), "published_as_research").await;
    price(
        &pool,
        published() + Duration::hours(1) + Duration::hours(9),
        "2.00",
    )
    .await;
    evaluate_due(&pool, published() + Duration::hours(12))
        .await
        .unwrap();
    assert_eq!(outcome(&pool, id2, 3_600).await.unwrap().0, "expired");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn an_outcome_is_never_replaced(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    price(&pool, published() + Duration::hours(1), "1.10").await;
    let id = signal(&pool, &receipt(), "published_as_research").await;
    let now = published() + Duration::hours(2);
    evaluate_due(&pool, now).await.unwrap();
    let first = outcome(&pool, id, 3_600).await.unwrap();
    // a better-looking price shows up later; the recorded result must not move
    price(
        &pool,
        published() + Duration::hours(1) + Duration::minutes(1),
        "9.99",
    )
    .await;
    sqlx::query("DELETE FROM price_points WHERE usd_per_base_unit < 0.000000000000000002")
        .execute(&pool)
        .await
        .unwrap();
    let rep = evaluate_due(&pool, now + Duration::hours(1)).await.unwrap();
    assert_eq!(rep.recorded, 0);
    assert_eq!(outcome(&pool, id, 3_600).await.unwrap(), first);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn state_follows_the_lifecycle_and_records_events(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    let r = receipt();
    for (h, p) in [
        (3_600, "1.1"),
        (86_400, "0.9"),
        (604_800, "1.2"),
        (2_592_000, "1.0"),
    ] {
        price(&pool, published() + Duration::seconds(h), p).await;
    }
    let id = signal(&pool, &r, "published_as_research").await;
    assert_eq!(
        evaluate_due(&pool, published() + Duration::minutes(30))
            .await
            .unwrap()
            .transitions,
        0,
        "nothing due yet"
    );
    assert_eq!(state(&pool, id).await, "published_as_research");

    evaluate_due(&pool, published() + Duration::hours(2))
        .await
        .unwrap();
    assert_eq!(state(&pool, id).await, "observation_due");

    let rep = evaluate_due(&pool, published() + Duration::days(31))
        .await
        .unwrap();
    assert_eq!(rep.recorded, 3);
    assert_eq!(state(&pool, id).await, "observed");
    let states: Vec<String> =
        sqlx::query_scalar("SELECT state FROM signal_events WHERE signal_id=$1 ORDER BY seq")
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        states,
        vec!["published_as_research", "observation_due", "observed"]
    );

    // finished signals are not touched again
    let again = evaluate_due(&pool, published() + Duration::days(40))
        .await
        .unwrap();
    assert_eq!((again.recorded, again.transitions), (0, 0));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_signal_with_no_priced_outcome_ends_as_unpriced(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    let id = signal(&pool, &receipt(), "published_as_research").await;
    evaluate_due(&pool, published() + Duration::days(40))
        .await
        .unwrap();
    assert_eq!(state(&pool, id).await, "unpriced");
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM signal_outcomes WHERE signal_id=$1 AND kind='unpriced'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 4, "every horizon is accounted for");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_tampered_receipt_is_skipped_and_counted(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    price(&pool, published() + Duration::hours(1), "1.5").await;
    let id = signal(&pool, &receipt(), "published_as_research").await;
    sqlx::query("UPDATE signals SET receipt = jsonb_set(receipt, '{reference_price_usd}', '\"0.01\"') WHERE id=$1").bind(id).execute(&pool).await.unwrap();
    let rep = evaluate_due(&pool, published() + Duration::days(40))
        .await
        .unwrap();
    assert_eq!((rep.integrity_failures, rep.recorded), (1, 0));
    assert!(outcome(&pool, id, 3_600).await.is_none());
    assert_eq!(state(&pool, id).await, "published_as_research");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn the_queue_runs_the_job_and_does_not_double_schedule(pool: PgPool) {
    setup_token(&pool, Some(500_000)).await;
    price(&pool, published() + Duration::hours(1), "1.2").await;
    let id = signal(&pool, &receipt(), "published_as_research").await;
    let now = published() + Duration::hours(2);
    trace_worker::schedule_periodic(&pool, now).await.unwrap();
    trace_worker::schedule_periodic(&pool, now + Duration::seconds(30))
        .await
        .unwrap();
    let queued: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(queued, 1, "same five minute window, one job");
    assert_eq!(trace_worker::drain(&pool, now).await.unwrap(), 1);
    assert!(outcome(&pool, id, 3_600).await.is_some());
    let status: String = sqlx::query_scalar("SELECT status FROM worker_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn unknown_job_kinds_die_instead_of_looping(pool: PgPool) {
    trace_jobs::enqueue(&pool, "mystery", None, serde_json::json!({}), Utc::now(), 3)
        .await
        .unwrap();
    // drain only leases kinds it knows, so an unknown kind is never picked up
    assert_eq!(
        trace_worker::drain(&pool, Utc::now() + Duration::seconds(1))
            .await
            .unwrap(),
        0
    );
    let job = trace_jobs::lease(
        &pool,
        Utc::now() + Duration::seconds(1),
        Duration::minutes(1),
        &["mystery"],
    )
    .await
    .unwrap()
    .unwrap();
    let f = trace_worker::run_job(&pool, &job, Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(f, trace_jobs::Failure::Permanent(_)));
}
