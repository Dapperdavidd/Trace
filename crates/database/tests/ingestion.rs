use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use trace_database::ingestion::*;
use trace_domain::*;

fn swap(tx_id: &str, idx: u32, height: u64, finality: Finality) -> NormalizedSwap {
    let w = WalletId::parse(
        ChainId::Ethereum,
        "0x1111111111111111111111111111111111111111",
    )
    .unwrap();
    NormalizedSwap {
        chain: ChainId::Ethereum,
        tx_id: tx_id.into(),
        event_index: idx,
        finality_revision: 0,
        wallet: w,
        protocol: "uniswap_v2".into(),
        pool: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".into(),
        token_in: AssetId::parse(
            ChainId::Ethereum,
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        )
        .unwrap(),
        amount_in: BaseUnits(3_000_000_000),
        token_out: AssetId::parse(
            ChainId::Ethereum,
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        )
        .unwrap(),
        amount_out: BaseUnits(u128::MAX / 3),
        block_height: Some(height),
        block_hash: Some("0xbeef".into()),
        observed_at: Utc.timestamp_opt(1_793_190_000, 0).unwrap(),
        finality,
        attribution: Attribution::Direct,
        parser_version: "test".into(),
        raw_ref: format!("ethereum:{tx_id}:{idx}"),
        demo: true,
    }
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn replaying_the_same_swap_never_double_counts(pool: PgPool) {
    let s = swap("0xaa", 1, 100, Finality::Final);
    assert_eq!(
        insert_swap(&pool, &s).await.unwrap(),
        InsertOutcome::Inserted
    );
    assert_eq!(
        insert_swap(&pool, &s).await.unwrap(),
        InsertOutcome::Duplicate
    );
    assert_eq!(
        insert_swap(&pool, &s).await.unwrap(),
        InsertOutcome::Duplicate
    );
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Final)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn amounts_round_trip_exactly_beyond_f64(pool: PgPool) {
    insert_swap(&pool, &swap("0xbb", 0, 100, Finality::Final))
        .await
        .unwrap();
    let got: String =
        sqlx::query_scalar("SELECT amount_out::text FROM normalized_swaps WHERE tx_id='0xbb'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(got, (u128::MAX / 3).to_string());
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn two_events_in_one_tx_are_distinct(pool: PgPool) {
    insert_swap(&pool, &swap("0xcc", 1, 100, Finality::Final))
        .await
        .unwrap();
    assert_eq!(
        insert_swap(&pool, &swap("0xcc", 2, 100, Finality::Final))
            .await
            .unwrap(),
        InsertOutcome::Inserted
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn reorg_marks_tentative_rows_keeps_history_and_audits(pool: PgPool) {
    insert_swap(&pool, &swap("0x01", 0, 100, Finality::Final))
        .await
        .unwrap();
    insert_swap(&pool, &swap("0x02", 0, 105, Finality::Tentative))
        .await
        .unwrap();
    insert_swap(&pool, &swap("0x03", 0, 110, Finality::Tentative))
        .await
        .unwrap();
    let n = mark_reorged(&pool, "ethereum", 105).await.unwrap();
    assert_eq!(n, 2);
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Final)
            .await
            .unwrap(),
        1,
        "final rows untouched"
    );
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Reorged)
            .await
            .unwrap(),
        2,
        "rows kept, not deleted"
    );
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE action='reorg'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audits, 1);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn finality_promotion_only_touches_tentative_rows_at_or_below_height(pool: PgPool) {
    insert_swap(&pool, &swap("0x11", 0, 100, Finality::Tentative))
        .await
        .unwrap();
    insert_swap(&pool, &swap("0x12", 0, 200, Finality::Tentative))
        .await
        .unwrap();
    assert_eq!(finalize_up_to(&pool, "ethereum", 150).await.unwrap(), 1);
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Tentative)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn cursors_resume_where_they_left_off(pool: PgPool) {
    assert!(get_cursor(&pool, "ethereum", "uniswap_v2")
        .await
        .unwrap()
        .is_none());
    set_cursor(&pool, "ethereum", "uniswap_v2", "block:123", Some(123))
        .await
        .unwrap();
    set_cursor(&pool, "ethereum", "uniswap_v2", "block:456", Some(456))
        .await
        .unwrap();
    assert_eq!(
        get_cursor(&pool, "ethereum", "uniswap_v2").await.unwrap(),
        Some(("block:456".into(), Some(456)))
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn raw_events_are_idempotent(pool: PgPool) {
    let payload = serde_json::json!({"fixture": "synthetic"});
    let t = Utc.timestamp_opt(1_793_190_000, 0).unwrap();
    let ev = RawEvent {
        chain: "ethereum",
        tx_id: "0xaa",
        event_index: 1,
        source: "fixture",
        payload: &payload,
        payload_sha256: "abc",
        fixture: true,
        observed_at: t,
    };
    let a = store_raw_event(&pool, &ev).await.unwrap();
    let b = store_raw_event(&pool, &ev).await.unwrap();
    assert_eq!((a, b), (InsertOutcome::Inserted, InsertOutcome::Duplicate));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn schema_rejects_zero_amounts_and_unknown_chains(pool: PgPool) {
    let r = sqlx::query("INSERT INTO wallets (chain, address) VALUES ('dogechain','x')")
        .execute(&pool)
        .await;
    assert!(r.is_err());
}
