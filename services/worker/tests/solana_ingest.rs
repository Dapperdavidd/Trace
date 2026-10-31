//! Solana ingestion against a scripted chain. The transactions are synthetic;
//! this proves cursor, gap and idempotency behaviour, not parsing on real data.

use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use trace_chain_adapters::solana::ProgramAllow;
use trace_database::ingestion::{get_cursor, set_cursor};
use trace_domain::SupportStatus;
use trace_worker::ingest::IngestError;
use trace_worker::solana_ingest::{step, SigInfo, SolanaIngestConfig, SolanaSource};

const PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const OTHER_PROGRAM: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const PAYER: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
const POOL: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const BONK: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

static ALLOW: [ProgramAllow; 1] = [ProgramAllow {
    program_id: PROGRAM,
    protocol: "raydium_amm_v4",
}];

fn bal(idx: usize, mint: &str, owner: &str, amount: u64) -> Value {
    json!({ "accountIndex": idx, "mint": mint, "owner": owner,
            "uiTokenAmount": { "amount": amount.to_string(), "decimals": 6 } })
}

/// Payer spends 100 USDC and receives `bonk` BONK, no native SOL movement beyond the fee.
fn swap_tx(sig: &str, slot: u64, program: &str, bonk: u64) -> Value {
    json!({
        "slot": slot, "blockTime": 1_793_000_000u64 + slot,
        "transaction": {
            "signatures": [sig],
            "message": { "accountKeys": [PAYER, POOL], "instructions": [{ "programId": program }] }
        },
        "meta": {
            "err": null, "fee": 5000,
            "preBalances": [1_000_000_000u64, 1], "postBalances": [999_995_000u64, 1],
            "preTokenBalances":  [bal(2, USDC, PAYER, 500_000_000), bal(3, BONK, PAYER, 0),
                                  bal(4, USDC, POOL, 0), bal(5, BONK, POOL, bonk + 1000)],
            "postTokenBalances": [bal(2, USDC, PAYER, 400_000_000), bal(3, BONK, PAYER, bonk),
                                  bal(4, USDC, POOL, 100_000_000), bal(5, BONK, POOL, 1000)]
        }
    })
}

#[derive(Default)]
struct Chain {
    /// Newest first.
    sigs: Vec<(SigInfo, Option<Value>)>,
    fetched: Vec<String>,
}

struct Mock(Mutex<Chain>, AtomicUsize);

impl Mock {
    fn new() -> Self {
        Mock(Mutex::new(Chain::default()), AtomicUsize::new(0))
    }
    /// Add a newer signature.
    fn push(&self, name: &str, slot: u64, tx: Option<Value>, failed: bool) {
        self.0.lock().unwrap().sigs.insert(
            0,
            (
                SigInfo {
                    signature: name.into(),
                    slot,
                    failed,
                },
                tx,
            ),
        );
    }
    fn push_swap(&self, name: &str, slot: u64) {
        self.push(name, slot, Some(swap_tx(name, slot, PROGRAM, 777)), false);
    }
    fn fetched(&self) -> Vec<String> {
        self.0.lock().unwrap().fetched.clone()
    }
}

#[async_trait]
impl SolanaSource for Mock {
    async fn signatures(
        &self,
        _: &str,
        before: Option<&str>,
        until: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SigInfo>, IngestError> {
        self.1.fetch_add(1, Ordering::SeqCst);
        let c = self.0.lock().unwrap();
        let start = before.map_or(0, |b| {
            c.sigs
                .iter()
                .position(|(s, _)| s.signature == b)
                .map_or(c.sigs.len(), |i| i + 1)
        });
        let end = until.map_or(c.sigs.len(), |u| {
            c.sigs
                .iter()
                .position(|(s, _)| s.signature == u)
                .unwrap_or(c.sigs.len())
        });
        Ok(c.sigs
            .iter()
            .take(end)
            .skip(start)
            .take(limit)
            .map(|(s, _)| s.clone())
            .collect())
    }
    async fn transaction(&self, signature: &str) -> Result<Option<Value>, IngestError> {
        let mut c = self.0.lock().unwrap();
        c.fetched.push(signature.into());
        Ok(c.sigs
            .iter()
            .find(|(s, _)| s.signature == signature)
            .and_then(|(_, t)| t.clone()))
    }
}

fn cfg() -> SolanaIngestConfig {
    SolanaIngestConfig {
        protocol: "raydium_amm_v4".into(),
        status: SupportStatus::Enabled,
        poll_program: PROGRAM,
        programs: &ALLOW,
        page: 3,
        max_backlog: 10,
        max_per_step: 100,
    }
}

async fn cursor(pool: &PgPool) -> Option<String> {
    get_cursor(pool, "solana", "raydium_amm_v4")
        .await
        .unwrap()
        .map(|c| c.0)
}

async fn count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM normalized_swaps WHERE chain='solana'")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_protocol_that_is_not_enabled_is_never_ingested(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s1", 1);
    for st in [SupportStatus::Disabled, SupportStatus::Unsupported] {
        let mut c = cfg();
        c.status = st;
        assert!(matches!(
            step(&pool, &m, &c).await,
            Err(IngestError::NotEnabled { .. })
        ));
    }
    assert_eq!(
        m.1.load(Ordering::SeqCst),
        0,
        "the source was never touched"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn the_first_run_starts_at_the_newest_signature_without_backfill(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s1", 1);
    m.push_swap("s2", 2);
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert!(r.bootstrapped);
    assert_eq!(cursor(&pool).await.as_deref(), Some("s2"));
    assert_eq!(count(&pool).await, 0);
    assert!(m.fetched().is_empty());
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn new_signatures_are_ingested_oldest_first_as_final_and_the_cursor_follows(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s0", 1);
    step(&pool, &m, &cfg()).await.unwrap();
    m.push_swap("s1", 2);
    m.push_swap("s2", 3);
    m.push_swap("s3", 4);
    m.push_swap("s4", 5);
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!((r.collected, r.inserted, r.gap), (4, 4, false));
    assert_eq!(m.fetched(), vec!["s1", "s2", "s3", "s4"], "oldest first");
    assert_eq!(cursor(&pool).await.as_deref(), Some("s4"));
    let finals: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM normalized_swaps WHERE finality='final' AND chain='solana'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(finals, 4, "everything is read at finalized commitment");
    let raw: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM raw_chain_events WHERE chain='solana' AND NOT fixture",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(raw, 4);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn failed_and_non_allowlisted_transactions_are_counted_and_the_cursor_still_advances(
    pool: PgPool,
) {
    let m = Mock::new();
    m.push_swap("s0", 1);
    step(&pool, &m, &cfg()).await.unwrap();
    m.push("bad", 2, None, true);
    m.push(
        "other",
        3,
        Some(swap_tx("other", 3, OTHER_PROGRAM, 5)),
        false,
    );
    m.push_swap("good", 4);
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!((r.failed_onchain, r.inserted), (1, 1));
    assert_eq!(r.skipped.get("NoAllowlistedProgram"), Some(&1));
    assert!(
        !m.fetched().contains(&"bad".to_string()),
        "failed transactions are not fetched"
    );
    assert_eq!(cursor(&pool).await.as_deref(), Some("good"));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn nothing_new_means_no_fetches(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s0", 1);
    step(&pool, &m, &cfg()).await.unwrap();
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!((r.collected, r.fetched), (0, 0));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn replaying_a_range_never_double_counts(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s0", 1);
    step(&pool, &m, &cfg()).await.unwrap();
    m.push_swap("s1", 2);
    step(&pool, &m, &cfg()).await.unwrap();
    set_cursor(&pool, "solana", "raydium_amm_v4", "s0", Some(1))
        .await
        .unwrap();
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert_eq!((r.inserted, r.duplicates), (0, 1));
    assert_eq!(count(&pool).await, 1);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn an_unavailable_finalized_transaction_is_an_error_and_the_cursor_stays(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s0", 1);
    step(&pool, &m, &cfg()).await.unwrap();
    m.push_swap("s1", 2);
    m.push("s2", 3, None, false); // the RPC cannot return it yet
    assert!(step(&pool, &m, &cfg()).await.is_err());
    assert_eq!(
        cursor(&pool).await.as_deref(),
        Some("s0"),
        "nothing is skipped silently"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_backlog_beyond_the_limit_is_audited_as_a_gap_not_hidden(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s0", 1);
    step(&pool, &m, &cfg()).await.unwrap();
    for i in 1..=14 {
        m.push_swap(&format!("n{i:02}"), 1 + i);
    }
    let r = step(&pool, &m, &cfg()).await.unwrap();
    assert!(r.gap);
    assert_eq!(r.collected, 10, "only the newest max_backlog are read");
    assert_eq!(m.fetched().first().map(String::as_str), Some("n05"));
    let detail: Value =
        sqlx::query_scalar("SELECT detail FROM audit_events WHERE action='ingest_gap'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(detail["first_ingested_signature"], "n05");
    assert_eq!(detail["after_signature"], "s0");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn max_per_step_processes_the_oldest_first_and_loses_nothing(pool: PgPool) {
    let m = Mock::new();
    m.push_swap("s0", 1);
    step(&pool, &m, &cfg()).await.unwrap();
    for i in 1..=5 {
        m.push_swap(&format!("n{i}"), 1 + i);
    }
    let mut c = cfg();
    c.max_per_step = 2;
    step(&pool, &m, &c).await.unwrap();
    assert_eq!(cursor(&pool).await.as_deref(), Some("n2"));
    step(&pool, &m, &c).await.unwrap();
    step(&pool, &m, &c).await.unwrap();
    assert_eq!(count(&pool).await, 5);
    assert_eq!(cursor(&pool).await.as_deref(), Some("n5"));
    let gaps: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE action='ingest_gap'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(gaps, 0);
}
