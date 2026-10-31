//! JSON-RPC backed `SolanaSource`. Finalized commitment only.
//!
//! The RPC URL may contain an API key and never appears in an error.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;

use crate::ingest::IngestError;
use crate::solana_ingest::{SigInfo, SolanaSource};

pub struct SolanaRpcSource {
    url: String,
    http: reqwest::Client,
}

fn err(msg: impl Into<String>) -> IngestError {
    IngestError::Source(msg.into())
}

impl SolanaRpcSource {
    pub fn new(url: String) -> Result<Self, IngestError> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| err(e.without_url().to_string()))?,
            url,
        })
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, IngestError> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let mut last = String::new();
        for attempt in 0..3u64 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
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
}

#[async_trait]
impl SolanaSource for SolanaRpcSource {
    async fn signatures(
        &self,
        program: &str,
        before: Option<&str>,
        until: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SigInfo>, IngestError> {
        let mut opts = json!({ "limit": limit.min(1000), "commitment": "finalized" });
        if let Some(b) = before {
            opts["before"] = json!(b);
        }
        if let Some(u) = until {
            opts["until"] = json!(u);
        }
        let v = self
            .call("getSignaturesForAddress", json!([program, opts]))
            .await?;
        v.as_array()
            .ok_or_else(|| err("getSignaturesForAddress result is not a list"))?
            .iter()
            .map(|e| {
                Ok(SigInfo {
                    signature: e["signature"]
                        .as_str()
                        .ok_or_else(|| err("signature entry without signature"))?
                        .to_string(),
                    slot: e["slot"]
                        .as_u64()
                        .ok_or_else(|| err("signature entry without slot"))?,
                    failed: !e["err"].is_null(),
                })
            })
            .collect()
    }

    async fn transaction(&self, signature: &str) -> Result<Option<Value>, IngestError> {
        let v = self
            .call(
                "getTransaction",
                json!([signature, { "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0, "commitment": "finalized" }]),
            )
            .await?;
        if v.is_null() {
            return Ok(None);
        }
        // The RPC must return the transaction we asked for.
        let matches = v["transaction"]["signatures"]
            .as_array()
            .is_some_and(|s| s.iter().any(|x| x.as_str() == Some(signature)));
        if !matches {
            return Err(err("RPC returned a different transaction than requested"));
        }
        Ok(Some(v))
    }
}
