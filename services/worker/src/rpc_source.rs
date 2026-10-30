//! JSON-RPC backed `EvmSource` for Uniswap V2 style pairs.
//!
//! Honesty rules:
//! * A pair is trusted only if its `factory()` equals the configured factory.
//!   Forks that emit the same Swap event are left out of the pair map, so the
//!   parser reports them as skipped rather than mislabelling them.
//! * A receipt whose block hash differs from the log's block hash means the
//!   chain moved while reading. That is an error (retry), never a guess.
//! * The RPC URL may contain an API key. It never appears in an error.

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Duration;
use trace_chain_adapters::evm_v2::{swap_topic, Log, PairInfo, TxContext};
use trace_domain::{Address, ChainId};

use crate::ingest::{Bundle, EvmSource, Head, IngestError};

const TOKEN0: &str = "0x0dfe1681";
const TOKEN1: &str = "0xd21220a7";
const FACTORY: &str = "0xc45a0155";

pub struct EvmRpcSource {
    url: String,
    http: reqwest::Client,
    chain: ChainId,
    protocol: String,
    factory: Address,
    /// `None` records "checked, not ours" so forks are not re-queried every block.
    pairs: Mutex<HashMap<Address, Option<PairInfo>>>,
}

fn err(msg: impl Into<String>) -> IngestError {
    IngestError::Source(msg.into())
}

fn hex_u64(v: &Value, what: &str) -> Result<u64, IngestError> {
    let s = v.as_str().ok_or_else(|| err(format!("{what} missing")))?;
    u64::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|_| err(format!("{what} is not hex: {s}")))
}

fn word_address(result: &Value) -> Option<String> {
    let h = result.as_str()?.strip_prefix("0x")?;
    if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("0x{}", &h[24..]).to_lowercase())
}

impl EvmRpcSource {
    pub fn new(
        url: String,
        chain: ChainId,
        protocol: &str,
        factory: &str,
    ) -> Result<Self, IngestError> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|e| err(e.without_url().to_string()))?,
            url,
            chain,
            protocol: protocol.into(),
            factory: Address::parse(chain, factory).map_err(|e| err(e.to_string()))?,
            pairs: Mutex::new(HashMap::new()),
        })
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, IngestError> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let mut last = String::new();
        for attempt in 0..3u64 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(400 * attempt)).await;
            }
            let resp = match self.http.post(&self.url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    last = format!("request failed: {}", e.without_url());
                    continue;
                }
            };
            let status = resp.status();
            if status.as_u16() == 429 || status.is_server_error() {
                last = format!("{method}: HTTP {status}");
                continue;
            }
            if !status.is_success() {
                return Err(err(format!("{method}: HTTP {status}")));
            }
            let v: Value = resp
                .json()
                .await
                .map_err(|_| err(format!("{method}: response was not JSON")))?;
            if let Some(e) = v.get("error") {
                return Err(err(format!(
                    "{method}: RPC error {}",
                    e["message"].as_str().unwrap_or("unknown")
                )));
            }
            return Ok(v["result"].clone());
        }
        Err(err(format!("{method} failed after 3 attempts: {last}")))
    }

    async fn block(&self, tag: Value) -> Result<Value, IngestError> {
        self.call("eth_getBlockByNumber", json!([tag, false])).await
    }

    /// Token pair for `pair`, or `None` when it is not a pair of the configured factory.
    async fn pair_info(&self, pair: &Address) -> Result<Option<PairInfo>, IngestError> {
        if let Some(known) = self.pairs.lock().unwrap().get(pair) {
            return Ok(known.clone());
        }
        let addr = pair.as_str().to_string();
        let ask = |sel: &'static str| {
            let addr = addr.clone();
            async move {
                self.call("eth_call", json!([{ "to": addr, "data": sel }, "latest"]))
                    .await
            }
        };
        // A non-pair contract may revert; that means "not ours", not a source failure.
        let factory = ask(FACTORY).await.ok().and_then(|v| word_address(&v));
        let info = match factory {
            Some(f) if f == self.factory.as_str() => {
                let t0 = word_address(&ask(TOKEN0).await?);
                let t1 = word_address(&ask(TOKEN1).await?);
                match (t0, t1) {
                    (Some(a), Some(b)) => Some(PairInfo {
                        token0: Address::parse(self.chain, &a).map_err(|e| err(e.to_string()))?,
                        token1: Address::parse(self.chain, &b).map_err(|e| err(e.to_string()))?,
                        protocol: self.protocol.clone(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        };
        self.pairs
            .lock()
            .unwrap()
            .insert(pair.clone(), info.clone());
        Ok(info)
    }
}

#[async_trait]
impl EvmSource for EvmRpcSource {
    async fn head(&self) -> Result<Head, IngestError> {
        let latest = hex_u64(
            &self.call("eth_blockNumber", json!([])).await?,
            "blockNumber",
        )?;
        // Not every provider supports the tag. Absence means "not proven final".
        let finalized = match self.block(json!("finalized")).await {
            Ok(b) if !b.is_null() => hex_u64(&b["number"], "finalized.number").ok(),
            _ => None,
        };
        Ok(Head { latest, finalized })
    }

    async fn block_hash(&self, n: u64) -> Result<Option<String>, IngestError> {
        let b = self.block(json!(format!("0x{n:x}"))).await?;
        Ok(b["hash"].as_str().map(str::to_lowercase))
    }

    async fn bundles(&self, from: u64, to: u64) -> Result<Vec<Bundle>, IngestError> {
        let logs = self
            .call(
                "eth_getLogs",
                json!([{ "fromBlock": format!("0x{from:x}"), "toBlock": format!("0x{to:x}"), "topics": [swap_topic()] }]),
            )
            .await?;
        // tx hash -> block hash seen on the swap log
        let mut txs: BTreeMap<String, String> = BTreeMap::new();
        for l in logs
            .as_array()
            .ok_or_else(|| err("getLogs result is not a list"))?
        {
            if l["removed"].as_bool() == Some(true) {
                continue;
            }
            let h = l["transactionHash"]
                .as_str()
                .ok_or_else(|| err("log without transactionHash"))?;
            let bh = l["blockHash"]
                .as_str()
                .ok_or_else(|| err("log without blockHash"))?;
            txs.insert(h.to_lowercase(), bh.to_lowercase());
        }

        let mut out = Vec::new();
        let mut block_times: HashMap<u64, chrono::DateTime<Utc>> = HashMap::new();
        for (hash, log_block_hash) in txs {
            let receipt = self
                .call("eth_getTransactionReceipt", json!([hash]))
                .await?;
            if receipt.is_null() {
                return Err(err("receipt vanished mid-read; chain moved"));
            }
            let r_block_hash = receipt["blockHash"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase();
            if r_block_hash != log_block_hash {
                return Err(err("receipt block differs from log block; chain moved"));
            }
            let number = hex_u64(&receipt["blockNumber"], "blockNumber")?;
            let time = match block_times.get(&number) {
                Some(t) => *t,
                None => {
                    let b = self.block(json!(format!("0x{number:x}"))).await?;
                    if b["hash"].as_str().map(str::to_lowercase).as_deref() != Some(&r_block_hash) {
                        return Err(err("block hash changed mid-read; chain moved"));
                    }
                    let secs = hex_u64(&b["timestamp"], "timestamp")? as i64;
                    let t = Utc
                        .timestamp_opt(secs, 0)
                        .single()
                        .ok_or_else(|| err("bad timestamp"))?;
                    block_times.insert(number, t);
                    t
                }
            };
            let tx = self.call("eth_getTransactionByHash", json!([hash])).await?;
            let from_addr = tx["from"]
                .as_str()
                .or(receipt["from"].as_str())
                .ok_or_else(|| err("no sender"))?;
            let to_addr = tx["to"]
                .as_str()
                .or(receipt["to"].as_str())
                .map(str::to_lowercase);

            let mut logs = Vec::new();
            let mut pairs = HashMap::new();
            for l in receipt["logs"]
                .as_array()
                .ok_or_else(|| err("receipt has no logs"))?
            {
                let address = l["address"]
                    .as_str()
                    .ok_or_else(|| err("log without address"))?
                    .to_lowercase();
                let topics: Vec<String> = l["topics"]
                    .as_array()
                    .ok_or_else(|| err("log without topics"))?
                    .iter()
                    .filter_map(|t| t.as_str().map(str::to_lowercase))
                    .collect();
                if topics.first().map(String::as_str) == Some(swap_topic().as_str()) {
                    if let Ok(a) = Address::parse(self.chain, &address) {
                        if let Some(info) = self.pair_info(&a).await? {
                            pairs.insert(a, info);
                        }
                    }
                }
                logs.push(Log {
                    address,
                    topics,
                    data: l["data"]
                        .as_str()
                        .ok_or_else(|| err("log without data"))?
                        .to_lowercase(),
                    log_index: hex_u64(&l["logIndex"], "logIndex")? as u32,
                });
            }
            out.push(Bundle {
                tx: TxContext {
                    tx_hash: hash,
                    from: from_addr.to_lowercase(),
                    to: to_addr,
                    block_number: number,
                    block_hash: r_block_hash,
                    block_time: time,
                    logs,
                    finalized: false, // the engine judges finality from the head
                },
                pairs,
            });
        }
        Ok(out)
    }
}
