//! JSON-RPC over HTTPS. The only part of this crate that touches the network.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

pub struct Rpc {
    url: String,
    http: reqwest::Client,
}

impl Rpc {
    pub fn new(url: String) -> Result<Self> {
        Ok(Self {
            url,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()?,
        })
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let mut last = String::new();
        for attempt in 0..3u64 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
            }
            // Error text never includes the URL, which may hold an API key.
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
                bail!("{method}: HTTP {status}");
            }
            let v: Value = resp.json().await.context("response was not JSON")?;
            if let Some(e) = v.get("error") {
                bail!(
                    "{method}: RPC error {}",
                    e["message"].as_str().unwrap_or("unknown")
                );
            }
            return Ok(v["result"].clone());
        }
        bail!("{method} failed after 3 attempts: {last}")
    }
}
