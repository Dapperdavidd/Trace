//! The RPC source against a local scripted JSON-RPC server. Synthetic: this
//! proves request/response handling and the honesty rules, not real-chain parsing.

use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use trace_chain_adapters::evm_v2::swap_topic;
use trace_domain::{ChainId, SupportStatus};
use trace_worker::ingest::{step, EvmIngestConfig, EvmSource};
use trace_worker::rpc_source::EvmRpcSource;

const FACTORY: &str = "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";
const PAIR: &str = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";
const FORK_PAIR: &str = "0x397ff1542f962076d0bfe58ea045ffa2d347aca0";
const T0: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const T1: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const ROUTER: &str = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d";
const TRADER: &str = "0x1111111111111111111111111111111111111111";
const TX: &str = "0x00000000000000000000000000000000000000000000000000000000000000aa";
const BLOCK_HASH: &str = "0x00000000000000000000000000000000000000000000000000000000000000b1";

type Handler = Arc<dyn Fn(&str, &Value) -> Result<Value, u16> + Send + Sync>;

struct Server {
    url: String,
    calls: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn count(&self, needle: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.contains(needle))
            .count()
    }
}

async fn serve(handler: Handler) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let calls = Arc::new(Mutex::new(vec![]));
    let log = calls.clone();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let handler = handler.clone();
            let log = log.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                let body_start;
                loop {
                    let n = sock.read(&mut tmp).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        body_start = i + 4;
                        break;
                    }
                }
                let head = String::from_utf8_lossy(&buf[..body_start]).to_lowercase();
                let len: usize = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                while buf.len() < body_start + len {
                    let n = sock.read(&mut tmp).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                let req: Value =
                    serde_json::from_slice(&buf[body_start..body_start + len]).unwrap();
                let method = req["method"].as_str().unwrap_or_default().to_string();
                log.lock()
                    .unwrap()
                    .push(format!("{method} {}", req["params"]));
                let (status, body) = match handler(&method, &req["params"]) {
                    Ok(v) => (200, json!({ "jsonrpc": "2.0", "id": 1, "result": v })),
                    Err(0) => (
                        200,
                        json!({ "jsonrpc": "2.0", "id": 1, "error": { "message": "nope" } }),
                    ),
                    Err(code) => (code, json!({})),
                };
                let text = body.to_string();
                let resp = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
                    text.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            });
        }
    });
    Server {
        url: format!("http://127.0.0.1:{port}/v2/SECRET-API-KEY"),
        calls,
    }
}

fn pad(a: &str) -> String {
    format!("0x{:0>64}", a.trim_start_matches("0x"))
}

fn swap_log(pair: &str) -> Value {
    json!({
        "address": pair, "topics": [swap_topic(), pad(ROUTER), pad(TRADER)],
        "data": format!("0x{:064x}{:064x}{:064x}{:064x}", 1_000_000u128, 0u128, 0u128, 5_000u128),
        "logIndex": "0x1", "transactionHash": TX, "blockHash": BLOCK_HASH, "blockNumber": "0x64",
    })
}

/// A chain with one swap tx in block 0x64. `receipt_block` lets a test move the chain.
fn chain(pair: &'static str, receipt_block: &'static str) -> Handler {
    Arc::new(move |m, p| match m {
        "eth_blockNumber" => Ok(json!("0x66")),
        "eth_getBlockByNumber" => match p[0].as_str().unwrap() {
            "finalized" => Ok(json!({ "number": "0x62", "hash": "0xf" })),
            "0x64" => {
                Ok(json!({ "number": "0x64", "hash": BLOCK_HASH, "timestamp": "0x6553f100" }))
            }
            _ => Ok(Value::Null),
        },
        "eth_getLogs" => Ok(json!([swap_log(pair)])),
        "eth_getTransactionReceipt" => Ok(json!({
            "transactionHash": TX, "blockHash": receipt_block, "blockNumber": "0x64",
            "from": TRADER, "to": ROUTER, "logs": [swap_log(pair)]
        })),
        "eth_getTransactionByHash" => Ok(json!({ "from": TRADER, "to": ROUTER })),
        "eth_call" => {
            let to = p[0]["to"].as_str().unwrap().to_lowercase();
            let sel = p[0]["data"].as_str().unwrap();
            if to != PAIR && to != FORK_PAIR {
                return Err(0);
            }
            Ok(json!(match sel {
                "0x0dfe1681" => pad(T0),
                "0xd21220a7" => pad(T1),
                "0xc45a0155" => pad(if to == PAIR {
                    FACTORY
                } else {
                    "0x9999999999999999999999999999999999999999"
                }),
                _ => return Err(0),
            }))
        }
        _ => Err(0),
    })
}

fn source(url: &str) -> EvmRpcSource {
    EvmRpcSource::new(url.into(), ChainId::Ethereum, "uniswap_v2", FACTORY).unwrap()
}

#[tokio::test]
async fn head_reads_latest_and_the_finalized_tag() {
    let s = serve(chain(PAIR, BLOCK_HASH)).await;
    let h = source(&s.url).head().await.unwrap();
    assert_eq!((h.latest, h.finalized), (0x66, Some(0x62)));
}

#[tokio::test]
async fn a_provider_without_the_finalized_tag_means_not_proven_final() {
    let base = chain(PAIR, BLOCK_HASH);
    let s = serve(Arc::new(move |m, p| {
        if m == "eth_getBlockByNumber" && p[0] == "finalized" {
            return Err(0);
        }
        base(m, p)
    }))
    .await;
    assert_eq!(source(&s.url).head().await.unwrap().finalized, None);
}

#[tokio::test]
async fn block_hash_is_none_for_a_missing_block() {
    let s = serve(chain(PAIR, BLOCK_HASH)).await;
    let src = source(&s.url);
    assert_eq!(
        src.block_hash(0x64).await.unwrap().as_deref(),
        Some(BLOCK_HASH)
    );
    assert_eq!(src.block_hash(0x65).await.unwrap(), None);
}

#[tokio::test]
async fn bundles_carry_the_full_receipt_and_verified_pair_tokens() {
    let s = serve(chain(PAIR, BLOCK_HASH)).await;
    let b = source(&s.url).bundles(0x64, 0x64).await.unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].tx.tx_hash, TX);
    assert_eq!(b[0].tx.block_number, 100);
    assert_eq!(b[0].tx.from, TRADER);
    assert_eq!(b[0].tx.block_time.timestamp(), 0x6553f100);
    assert!(
        !b[0].tx.finalized,
        "the engine judges finality, not the source"
    );
    assert_eq!(b[0].pairs.len(), 1);
    let info = b[0].pairs.values().next().unwrap();
    assert_eq!((info.token0.as_str(), info.token1.as_str()), (T0, T1));
}

#[tokio::test]
async fn a_fork_with_the_same_event_is_not_labelled_uniswap_and_is_asked_about_once() {
    let s = serve(chain(FORK_PAIR, BLOCK_HASH)).await;
    let src = source(&s.url);
    assert!(src.bundles(0x64, 0x64).await.unwrap()[0].pairs.is_empty());
    assert!(src.bundles(0x64, 0x64).await.unwrap()[0].pairs.is_empty());
    assert_eq!(s.count("0xc45a0155"), 1, "the verdict is cached");
    assert_eq!(
        s.count("0x0dfe1681"),
        0,
        "tokens of a foreign pair are never trusted"
    );
}

#[tokio::test]
async fn a_chain_that_moved_mid_read_is_an_error_not_a_guess() {
    let s = serve(chain(
        PAIR,
        "0x00000000000000000000000000000000000000000000000000000000000000c2",
    ))
    .await;
    let e = source(&s.url).bundles(0x64, 0x64).await.unwrap_err();
    assert!(e.to_string().contains("chain moved"), "{e}");
}

#[tokio::test]
async fn errors_never_reveal_the_rpc_url_or_key() {
    let s = serve(Arc::new(|_, _| Err(500))).await;
    let e = source(&s.url).head().await.unwrap_err().to_string();
    assert!(
        !e.contains("SECRET") && !e.contains("127.0.0.1") && !e.contains("/v2/"),
        "{e}"
    );
    let s = serve(Arc::new(|_, _| Err(401))).await;
    let e = source(&s.url).head().await.unwrap_err().to_string();
    assert!(!e.contains("SECRET"), "{e}");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn the_engine_ingests_through_the_rpc_source(pool: PgPool) {
    let base = chain(PAIR, BLOCK_HASH);
    let s = serve(Arc::new(move |m, p| {
        if m == "eth_blockNumber" {
            return Ok(json!("0x64"));
        }
        base(m, p)
    }))
    .await;
    let cfg = EvmIngestConfig {
        chain: ChainId::Ethereum,
        protocol: "uniswap_v2".into(),
        status: SupportStatus::Enabled,
        routers: vec![trace_domain::Address::parse(ChainId::Ethereum, ROUTER).unwrap()],
        max_range: 10,
        reorg_window: 3,
        start_height: Some(100),
    };
    let r = step(&pool, &source(&s.url), &cfg).await.unwrap();
    assert_eq!((r.inserted, r.skipped), (1, 0));
    // Block 0x64 is above the finalized block 0x62, so the swap must be tentative.
    let (finality, wallet): (String, String) =
        sqlx::query_as("SELECT finality, wallet FROM normalized_swaps")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((finality.as_str(), wallet.as_str()), ("tentative", TRADER));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_fork_pair_yields_no_swap_and_is_reported_skipped(pool: PgPool) {
    let base = chain(FORK_PAIR, BLOCK_HASH);
    let s = serve(Arc::new(move |m, p| {
        if m == "eth_blockNumber" {
            return Ok(json!("0x64"));
        }
        base(m, p)
    }))
    .await;
    let cfg = EvmIngestConfig {
        chain: ChainId::Ethereum,
        protocol: "uniswap_v2".into(),
        status: SupportStatus::Enabled,
        routers: vec![],
        max_range: 10,
        reorg_window: 3,
        start_height: Some(100),
    };
    let r = step(&pool, &source(&s.url), &cfg).await.unwrap();
    assert_eq!((r.inserted, r.skipped), (0, 1));
}
