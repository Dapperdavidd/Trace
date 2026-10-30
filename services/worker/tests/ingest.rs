//! Ingestion engine against a scripted in-memory chain. The chain is synthetic;
//! this proves cursor, reorg, finality and idempotency behaviour, not parsing
//! accuracy on real transactions.

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use trace_chain_adapters::evm_v2::{swap_topic, Log, PairInfo, TxContext};
use trace_database::ingestion::{count_swaps, get_cursor, set_cursor};
use trace_domain::{Address, ChainId, Finality, SupportStatus};
use trace_worker::ingest::{step, Bundle, EvmIngestConfig, EvmSource, Head, IngestError};

const TRADER: &str = "0x1111111111111111111111111111111111111111";
const ROUTER: &str = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d";
const PAIR: &str = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";
const T0: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const T1: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

fn a(s: &str) -> Address {
    Address::parse(ChainId::Ethereum, s).unwrap()
}

fn pad(s: &str) -> String {
    format!("0x{:0>64}", s.trim_start_matches("0x"))
}

fn bundle(block: u64, hash: &str, tx_no: u64, amount_out: u128) -> Bundle {
    let data = format!(
        "0x{:064x}{:064x}{:064x}{:064x}",
        1_000_000u128, 0u128, 0u128, amount_out
    );
    Bundle {
        tx: TxContext {
            tx_hash: format!("0x{:064x}", tx_no),
            from: TRADER.into(),
            to: Some(ROUTER.into()),
            block_number: block,
            block_hash: hash.into(),
            block_time: Utc
                .timestamp_opt(1_793_000_000 + block as i64 * 12, 0)
                .unwrap(),
            logs: vec![Log {
                address: PAIR.into(),
                topics: vec![swap_topic(), pad(ROUTER), pad(TRADER)],
                data,
                log_index: 1,
            }],
            // The engine must ignore this and judge finality itself.
            finalized: true,
        },
        pairs: HashMap::from([(
            a(PAIR),
            PairInfo {
                token0: a(T0),
                token1: a(T1),
                protocol: "uniswap_v2".into(),
            },
        )]),
    }
}

#[derive(Default)]
struct Chain {
    blocks: BTreeMap<u64, (String, Vec<Bundle>)>,
    finalized: Option<u64>,
    fail_hash_for: Option<u64>,
    lie_about_range: bool,
}

#[derive(Default)]
struct Mock(Mutex<Chain>);

impl Mock {
    fn set(&self, n: u64, hash: &str, bundles: Vec<Bundle>) {
        self.0
            .lock()
            .unwrap()
            .blocks
            .insert(n, (hash.into(), bundles));
    }
    fn finalize(&self, n: Option<u64>) {
        self.0.lock().unwrap().finalized = n;
    }
}

#[async_trait]
impl EvmSource for Mock {
    async fn head(&self) -> Result<Head, IngestError> {
        let c = self.0.lock().unwrap();
        Ok(Head {
            latest: *c.blocks.keys().max().unwrap(),
            finalized: c.finalized,
        })
    }
    async fn block_hash(&self, n: u64) -> Result<Option<String>, IngestError> {
        let c = self.0.lock().unwrap();
        if c.fail_hash_for == Some(n) {
            return Err(IngestError::Source("rpc timeout".into()));
        }
        Ok(c.blocks.get(&n).map(|b| b.0.clone()))
    }
    async fn bundles(&self, from: u64, to: u64) -> Result<Vec<Bundle>, IngestError> {
        let c = self.0.lock().unwrap();
        let mut out: Vec<Bundle> = c
            .blocks
            .range(from..=to)
            .flat_map(|(_, (_, b))| b.clone())
            .collect();
        if c.lie_about_range {
            out.push(bundle(to + 50, "0xlie", 999, 5));
        }
        Ok(out)
    }
}

fn cfg() -> EvmIngestConfig {
    EvmIngestConfig {
        chain: ChainId::Ethereum,
        protocol: "uniswap_v2".into(),
        status: SupportStatus::Enabled,
        routers: vec![a(ROUTER)],
        max_range: 100,
        reorg_window: 3,
        start_height: Some(10),
    }
}

fn three_blocks() -> Mock {
    let m = Mock::default();
    m.set(9, "0xa09", vec![]);
    m.set(10, "0xa10", vec![bundle(10, "0xa10", 1, 100)]);
    m.set(11, "0xa11", vec![bundle(11, "0xa11", 2, 200)]);
    m.set(12, "0xa12", vec![bundle(12, "0xa12", 3, 300)]);
    m
}

async fn live(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM normalized_swaps WHERE finality <> 'reorged'")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_protocol_that_is_not_enabled_is_never_ingested(pool: PgPool) {
    let m = three_blocks();
    for status in [SupportStatus::Disabled, SupportStatus::Unsupported] {
        let mut c = cfg();
        c.status = status;
        assert!(matches!(
            step(&pool, &m, &c).await,
            Err(IngestError::NotEnabled { .. })
        ));
    }
    assert_eq!(live(&pool).await, 0);
    assert!(get_cursor(&pool, "ethereum", "uniswap_v2")
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_step_stores_swaps_and_advances_the_cursor(pool: PgPool) {
    let m = three_blocks();
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!((r.from, r.to, r.inserted), (Some(10), Some(12), 3));
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Tentative)
            .await
            .unwrap(),
        3
    );
    let (hash, height) = get_cursor(&pool, "ethereum", "uniswap_v2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!((hash.as_str(), height), ("0xa12", Some(12)));
    let raw: i64 =
        sqlx::query_scalar("SELECT count(*) FROM raw_chain_events WHERE fixture = false")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(raw, 3, "raw evidence is kept for replay");

    let again = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!(
        (again.from, again.inserted),
        (None, 0),
        "no new blocks, nothing to do"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn replaying_a_range_never_double_counts(pool: PgPool) {
    let m = three_blocks();
    step(&pool, &m, &cfg()).await.unwrap();
    set_cursor(&pool, "ethereum", "uniswap_v2", "0xa10", Some(10))
        .await
        .unwrap();
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!((r.inserted, r.duplicates), (0, 2));
    assert_eq!(live(&pool).await, 3);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn finality_comes_from_the_head_not_from_the_source(pool: PgPool) {
    let m = three_blocks();
    m.finalize(None);
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!(r.finalized, 0);
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Final)
            .await
            .unwrap(),
        0
    );

    m.finalize(Some(11));
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!(r.finalized, 2);
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Final)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Tentative)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_reorg_keeps_history_and_ingests_the_new_chain(pool: PgPool) {
    let m = three_blocks();
    step(&pool, &m, &cfg()).await.unwrap();

    // Blocks 11 and 12 are replaced. The tx from 11 moves to a new block 12 with
    // a different amount, and tx 3 is gone.
    m.set(11, "0xb11", vec![]);
    m.set(12, "0xb12", vec![bundle(12, "0xb12", 2, 250)]);
    m.set(13, "0xb13", vec![bundle(13, "0xb13", 4, 400)]);
    let r = step(&pool, &m, &cfg()).await.unwrap();

    assert!(
        r.reorged >= 2,
        "old tentative rows in the window are marked reorged"
    );
    // Live rows: tx1 (re-read, same chain), tx2 (new block, revision 1), tx4.
    assert_eq!(live(&pool).await, 3);
    let reorged: i64 =
        sqlx::query_scalar("SELECT count(*) FROM normalized_swaps WHERE finality='reorged'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(reorged >= 2, "history is retained, never deleted");
    let rev: i32 =
        sqlx::query_scalar("SELECT max(finality_revision) FROM normalized_swaps WHERE tx_id = $1")
            .bind(format!("0x{:064x}", 2))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rev, 1, "a re-included tx gets a new revision");
    let audited: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE action='reorg'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audited, 1);
    let (hash, height) = get_cursor(&pool, "ethereum", "uniswap_v2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!((hash.as_str(), height), ("0xb13", Some(13)));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn finalized_swaps_survive_a_reorg_of_later_blocks(pool: PgPool) {
    let m = three_blocks();
    m.finalize(Some(10));
    step(&pool, &m, &cfg()).await.unwrap();
    m.set(11, "0xb11", vec![]);
    m.set(12, "0xb12", vec![]);
    step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!(
        count_swaps(&pool, "ethereum", Finality::Final)
            .await
            .unwrap(),
        1
    );
    assert_eq!(live(&pool).await, 1);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_failing_source_leaves_the_cursor_where_it_was(pool: PgPool) {
    let m = three_blocks();
    m.0.lock().unwrap().fail_hash_for = Some(12);
    assert!(step(&pool, &m, &cfg()).await.is_err());
    assert!(get_cursor(&pool, "ethereum", "uniswap_v2")
        .await
        .unwrap()
        .is_none());

    m.0.lock().unwrap().fail_hash_for = None;
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!(
        live(&pool).await,
        3,
        "retry catches up; earlier partial inserts were idempotent"
    );
    assert_eq!(r.to, Some(12));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn max_range_bounds_each_step(pool: PgPool) {
    let m = three_blocks();
    let mut c = cfg();
    c.max_range = 2;
    let r1 = step(&pool, &m, &c).await.unwrap();
    assert_eq!((r1.from, r1.to), (Some(10), Some(11)));
    let r2 = step(&pool, &m, &c).await.unwrap();
    assert_eq!((r2.from, r2.to), (Some(12), Some(12)));
    assert_eq!(live(&pool).await, 3);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_source_returning_blocks_outside_the_range_is_not_trusted(pool: PgPool) {
    let m = three_blocks();
    m.0.lock().unwrap().lie_about_range = true;
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!(r.out_of_range, 1);
    assert_eq!(live(&pool).await, 3);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn first_run_without_a_start_height_does_not_backfill(pool: PgPool) {
    let m = three_blocks();
    let mut c = cfg();
    c.start_height = None;
    let r = step(&pool, &m, &c).await.unwrap();
    assert_eq!((r.from, r.to, r.inserted), (Some(12), Some(12), 1));
}
