//! DefiLlama coins API (`/prices/current/{coins}`), which needs no key.
//!
//! The response shape is parsed by `parse_defillama` and unit-tested on a
//! synthetic edge cases and a captured live WETH response. The capture and
//! independent comparison are recorded in `docs/pricing.md`.

use async_trait::async_trait;
use std::time::Duration;
use trace_domain::AssetId;

use crate::{
    coin_key, parse_defillama, parse_defillama_historical, Batch, HistoricalPriceProvider,
    HistoricalRequest, PriceError, PriceProvider,
};

pub const DEFAULT_BASE: &str = "https://coins.llama.fi";
/// Keeps the request URL short.
pub const BATCH: usize = 40;

pub struct DefiLlama {
    base: String,
    http: reqwest::Client,
}

impl DefiLlama {
    pub fn new(base: &str) -> Result<Self, PriceError> {
        Ok(Self {
            base: base.trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|e| PriceError::Provider(e.without_url().to_string()))?,
        })
    }
}

#[async_trait]
impl PriceProvider for DefiLlama {
    fn name(&self) -> &'static str {
        "defillama"
    }

    async fn fetch(&self, assets: &[AssetId]) -> Result<Batch, PriceError> {
        let mut all = Batch::default();
        for chunk in assets.chunks(BATCH) {
            let keys = chunk.iter().map(coin_key).collect::<Vec<_>>().join(",");
            let url = format!("{}/prices/current/{keys}", self.base);
            let resp = self.http.get(&url).send().await.map_err(|e| {
                PriceError::Provider(format!("request failed: {}", e.without_url()))
            })?;
            let status = resp.status();
            if !status.is_success() {
                return Err(PriceError::Provider(format!("HTTP {status}")));
            }
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|_| PriceError::Provider("response was not JSON".into()))?;
            let part = parse_defillama(&body, chunk);
            all.quotes.extend(part.quotes);
            all.rejected.extend(part.rejected);
        }
        Ok(all)
    }
}

#[async_trait]
impl HistoricalPriceProvider for DefiLlama {
    fn name(&self) -> &'static str {
        "defillama"
    }

    async fn fetch_historical(&self, requests: &[HistoricalRequest]) -> Result<Batch, PriceError> {
        let mut all = Batch::default();
        for chunk in requests.chunks(BATCH) {
            let mut coins = serde_json::Map::new();
            for request in chunk {
                coins
                    .entry(coin_key(&request.asset))
                    .or_insert_with(|| serde_json::Value::Array(vec![]))
                    .as_array_mut()
                    .expect("new historical request value is an array")
                    .push(serde_json::Value::from(request.at.timestamp()));
            }
            let url = format!("{}/batchHistorical", self.base);
            let encoded = serde_json::Value::Object(coins).to_string();
            let resp = self
                .http
                .get(&url)
                .query(&[("coins", encoded)])
                .send()
                .await
                .map_err(|e| {
                    PriceError::Provider(format!("request failed: {}", e.without_url()))
                })?;
            let status = resp.status();
            if !status.is_success() {
                return Err(PriceError::Provider(format!("HTTP {status}")));
            }
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|_| PriceError::Provider("response was not JSON".into()))?;
            let part = parse_defillama_historical(&body, chunk);
            all.quotes.extend(part.quotes);
            all.rejected.extend(part.rejected);
        }
        Ok(all)
    }
}
