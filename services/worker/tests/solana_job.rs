//! Solana job gating and the RPC source against a local scripted server.
//! Synthetic: no real Solana node is involved.

use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use trace_chain_adapters::registry;
use trace_domain::{ChainId, SupportStatus};
use trace_jobs::{enqueue, lease, Failure};
use trace_worker::ingest::IngestError;
use trace_worker::ingest_job::Ctx;
use trace_worker::solana_ingest::{SigInfo, SolanaSource};
use trace_worker::solana_job::{run_solana, schedule_solana, targets, SolanaRun, INGEST_SOLANA};
use trace_worker::solana_rpc::SolanaRpcSource;

struct Stub(Arc<AtomicUsize>);

#[async_trait]
impl SolanaSource for Stub {
    async fn signatures(
        &self,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
        _: usize,
    ) -> Result<Vec<SigInfo>, IngestError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(vec![SigInfo {
            signature: "sig-newest".into(),
            slot: 9,
            failed: false,
        }])
    }
    async fn transaction(&self, _: &str) -> Result<Option<Value>, IngestError> {
        Ok(None)
    }
}

fn ctx(status: SupportStatus, with_url: bool) -> (Ctx, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut c = Ctx::empty();
    for p in c
        .protocols
        .iter_mut()
        .filter(|p| p.chain == ChainId::Solana && p.protocol == "raydium_amm_v4")
    {
        p.status = status;
    }
    if with_url {
        c.rpc_urls.insert("solana".into(), "http://unused".into());
    }
    let k = calls.clone();
    c.solana_source = Some(Box::new(move || Ok(Box::new(Stub(k.clone())))));
    (c, calls)
}

fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn nothing_is_scheduled_unless_enabled_and_an_rpc_url_is_set(pool: PgPool) {
    // The real registry: raydium is Disabled, so even a configured worker does nothing.
    let mut real = Ctx::empty();
    real.rpc_urls
        .insert("solana".into(), "http://unused".into());
    real.solana_source = Some(Box::new(|| Err(IngestError::Source("unused".into()))));
    assert_eq!(
        registry::protocols()
            .iter()
            .find(|p| p.protocol == "raydium_amm_v4")
            .unwrap()
            .status,
        SupportStatus::Disabled
    );
    assert!(targets(&real).is_empty());
    assert_eq!(schedule_solana(&pool, &real, now()).await.unwrap(), 0);

    let (no_url, _) = ctx(SupportStatus::Enabled, false);
    assert!(targets(&no_url).is_empty());
    let (c, _) = ctx(SupportStatus::Enabled, true);
    assert_eq!(schedule_solana(&pool, &c, now()).await.unwrap(), 1);
    schedule_solana(&pool, &c, now() + Duration::seconds(20))
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs WHERE kind = $1")
        .bind(INGEST_SOLANA)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_queued_job_does_not_run_after_the_protocol_is_disabled(pool: PgPool) {
    let (c, calls) = ctx(SupportStatus::Disabled, true);
    enqueue(
        &pool,
        INGEST_SOLANA,
        None,
        json!({"protocol":"raydium_amm_v4"}),
        now(),
        3,
    )
    .await
    .unwrap();
    let job = lease(&pool, now(), Duration::minutes(5), &[INGEST_SOLANA])
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        run_solana(&pool, &c, &job).await,
        Err(Failure::Permanent(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn drain_runs_the_job_only_on_a_configured_worker_and_one_worker_at_a_time(pool: PgPool) {
    let (c, calls) = ctx(SupportStatus::Enabled, true);
    schedule_solana(&pool, &c, now()).await.unwrap();
    assert_eq!(
        trace_worker::drain(&pool, now()).await.unwrap(),
        0,
        "a worker without Solana ingest does not lease it"
    );
    assert_eq!(trace_worker::drain_with(&pool, now(), &c).await.unwrap(), 1);
    assert!(calls.load(Ordering::SeqCst) > 0);
    let cur = trace_database::ingestion::get_cursor(&pool, "solana", "raydium_amm_v4")
        .await
        .unwrap();
    assert_eq!(cur, Some(("sig-newest".to_string(), Some(9))));

    // Lock held elsewhere: nothing happens.
    enqueue(
        &pool,
        INGEST_SOLANA,
        None,
        json!({"protocol":"raydium_amm_v4"}),
        now(),
        3,
    )
    .await
    .unwrap();
    let job = lease(&pool, now(), Duration::minutes(5), &[INGEST_SOLANA])
        .await
        .unwrap()
        .unwrap();
    let mut other = pool.acquire().await.unwrap();
    let held: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtextextended($1, 0))")
        .bind("ingest:solana:raydium_amm_v4")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(held);
    let before = calls.load(Ordering::SeqCst);
    assert_eq!(
        run_solana(&pool, &c, &job).await.unwrap(),
        SolanaRun::AlreadyRunning
    );
    assert_eq!(calls.load(Ordering::SeqCst), before);
}

// ---- RPC source against a local server ----

async fn serve(
    handler: impl Fn(&str, &Value) -> Result<Value, u16> + Send + Sync + 'static,
) -> String {
    let handler = Arc::new(handler);
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v2/SECRET-KEY", l.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let (mut s, _) = l.accept().await.unwrap();
            let h = handler.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                let start = loop {
                    let n = s.read(&mut tmp).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let head = String::from_utf8_lossy(&buf[..start]).to_lowercase();
                let len: usize = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                while buf.len() < start + len {
                    let n = s.read(&mut tmp).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                let req: Value = serde_json::from_slice(&buf[start..start + len]).unwrap();
                let (status, body) =
                    match h(req["method"].as_str().unwrap_or_default(), &req["params"]) {
                        Ok(v) => (200, json!({"jsonrpc":"2.0","id":1,"result":v})),
                        Err(0) => (
                            200,
                            json!({"jsonrpc":"2.0","id":1,"error":{"message":"nope"}}),
                        ),
                        Err(c) => (c, json!({})),
                    };
                let text = body.to_string();
                let _ = s.write_all(format!("HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}", text.len()).as_bytes()).await;
            });
        }
    });
    url
}

#[tokio::test]
async fn the_rpc_source_asks_for_finalized_data_and_maps_signatures() {
    let url = serve(|m, p| match m {
        "getSignaturesForAddress" => {
            assert_eq!(p[0], "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
            assert_eq!(p[1]["commitment"], "finalized");
            assert_eq!(p[1]["until"], "old");
            assert_eq!(p[1]["limit"], 50);
            Ok(json!([{ "signature": "a", "slot": 5, "err": null }, { "signature": "b", "slot": 4, "err": {"InstructionError":[0,"x"]} }]))
        }
        _ => Err(0),
    })
    .await;
    let src = SolanaRpcSource::new(url).unwrap();
    let sigs = src
        .signatures(
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            None,
            Some("old"),
            50,
        )
        .await
        .unwrap();
    assert_eq!(
        sigs,
        vec![
            SigInfo {
                signature: "a".into(),
                slot: 5,
                failed: false
            },
            SigInfo {
                signature: "b".into(),
                slot: 4,
                failed: true
            },
        ]
    );
}

#[tokio::test]
async fn the_rpc_source_refuses_a_transaction_that_is_not_the_one_requested() {
    let url = serve(|m, p| match m {
        "getTransaction" => {
            assert_eq!(p[1]["encoding"], "jsonParsed");
            assert_eq!(p[1]["commitment"], "finalized");
            assert_eq!(p[1]["maxSupportedTransactionVersion"], 0);
            Ok(json!({ "slot": 1, "transaction": { "signatures": ["someone-else"] } }))
        }
        _ => Err(0),
    })
    .await;
    assert!(SolanaRpcSource::new(url)
        .unwrap()
        .transaction("wanted")
        .await
        .is_err());
}

#[tokio::test]
async fn null_means_unavailable_and_errors_never_leak_the_url() {
    let url = serve(|_, _| Ok(Value::Null)).await;
    assert_eq!(
        SolanaRpcSource::new(url)
            .unwrap()
            .transaction("x")
            .await
            .unwrap(),
        None
    );
    let url = serve(|_, _| Err(500)).await;
    let e = SolanaRpcSource::new(url)
        .unwrap()
        .signatures("p", None, None, 1)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        !e.contains("SECRET") && !e.contains("127.0.0.1") && !e.contains("/v2/"),
        "{e}"
    );
}
