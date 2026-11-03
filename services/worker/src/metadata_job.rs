//! Read ERC-20 metadata from configured chain RPC nodes.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use trace_domain::ChainId;
use trace_jobs::Failure;

pub const FETCH_TOKEN_METADATA: &str = "fetch_token_metadata";
const MAX_TOKENS: i64 = 200;
const DECIMALS: &str = "0x313ce567";
const SYMBOL: &str = "0x95d89b41";
const NAME: &str = "0x06fdde03";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenMetadata {
    pub decimals: u8,
    pub symbol: Option<String>,
    pub name: Option<String>,
}

#[async_trait]
pub trait MetadataSource: Send + Sync {
    async fn fetch(&self, chain: ChainId, token: &str) -> Result<Option<TokenMetadata>, Failure>;
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MetadataReport {
    pub requested: usize,
    pub completed: usize,
    pub unavailable: usize,
}

pub struct EvmMetadataRpc {
    urls: HashMap<ChainId, String>,
    http: reqwest::Client,
}

impl EvmMetadataRpc {
    pub fn new(urls: HashMap<ChainId, String>) -> Result<Self, Failure> {
        let http = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(20))
            .build()
            .map_err(|e| Failure::Permanent(e.without_url().to_string()))?;
        Ok(Self { urls, http })
    }

    async fn call(
        &self,
        chain: ChainId,
        token: &str,
        data: &str,
    ) -> Result<Option<Value>, Failure> {
        let url = self
            .urls
            .get(&chain)
            .ok_or_else(|| Failure::Permanent(format!("no metadata RPC for {chain}")))?;
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "eth_call",
            "params": [{ "to": token, "data": data }, "latest"]
        });
        let response = self.http.post(url).json(&body).send().await.map_err(|e| {
            Failure::Transient(format!("metadata RPC request failed: {}", e.without_url()))
        })?;
        let code = response.status();
        if code.as_u16() == 429 || code.is_server_error() {
            return Err(Failure::Transient(format!(
                "metadata RPC returned HTTP {code}"
            )));
        }
        if !code.is_success() {
            return Err(Failure::Permanent(format!(
                "metadata RPC returned HTTP {code}"
            )));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|_| Failure::Transient("metadata RPC response was not JSON".into()))?;
        if value.get("error").is_some() {
            return Ok(None);
        }
        Ok(value.get("result").cloned())
    }
}

fn bytes(value: &Value) -> Option<Vec<u8>> {
    let raw = value.as_str()?.strip_prefix("0x")?;
    (!raw.is_empty() && raw.len() % 2 == 0)
        .then(|| hex::decode(raw).ok())
        .flatten()
}

fn decode_decimals(value: &Value) -> Option<u8> {
    let raw = bytes(value)?;
    if raw.len() != 32 || raw[..31].iter().any(|b| *b != 0) {
        return None;
    }
    Some(raw[31])
}

fn clean_text(raw: &[u8]) -> Option<String> {
    let value = std::str::from_utf8(raw)
        .ok()?
        .trim_matches(char::from(0))
        .trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        return None;
    }
    Some(value.to_string())
}

fn decode_text(value: &Value) -> Option<String> {
    let raw = bytes(value)?;
    if raw.len() == 32 {
        return clean_text(&raw);
    }
    if raw.len() < 64 {
        return None;
    }
    let offset = usize::try_from(u64::from_be_bytes(raw[24..32].try_into().ok()?)).ok()?;
    if offset.checked_add(32)? > raw.len() {
        return None;
    }
    let length = usize::try_from(u64::from_be_bytes(
        raw[offset + 24..offset + 32].try_into().ok()?,
    ))
    .ok()?;
    let start = offset.checked_add(32)?;
    let end = start.checked_add(length)?;
    clean_text(raw.get(start..end)?)
}

#[async_trait]
impl MetadataSource for EvmMetadataRpc {
    async fn fetch(&self, chain: ChainId, token: &str) -> Result<Option<TokenMetadata>, Failure> {
        if !matches!(chain, ChainId::Ethereum | ChainId::Base | ChainId::Bnb) {
            return Ok(None);
        }
        let Some(decimals) = self.call(chain, token, DECIMALS).await? else {
            return Ok(None);
        };
        let Some(decimals) = decode_decimals(&decimals).filter(|d| *d <= 38) else {
            return Ok(None);
        };
        let symbol = match self.call(chain, token, SYMBOL).await? {
            Some(v) => decode_text(&v),
            None => None,
        };
        let name = match self.call(chain, token, NAME).await? {
            Some(v) => decode_text(&v),
            None => None,
        };
        Ok(Some(TokenMetadata {
            decimals,
            symbol,
            name,
        }))
    }
}

pub async fn fetch_missing(
    pool: &PgPool,
    source: &dyn MetadataSource,
    now: DateTime<Utc>,
) -> Result<MetadataReport, Failure> {
    let rows = sqlx::query(
        "SELECT chain, contract_or_mint FROM tokens
         WHERE chain IN ('ethereum','base','bnb')
           AND (metadata_status='unknown' OR metadata_checked_at < $1)
           AND (decimals IS NULL OR symbol IS NULL OR name IS NULL)
         ORDER BY first_seen_at LIMIT $2",
    )
    .bind(now - Duration::days(7))
    .bind(MAX_TOKENS)
    .fetch_all(pool)
    .await
    .map_err(|e| Failure::Transient(e.to_string()))?;
    let mut report = MetadataReport {
        requested: rows.len(),
        ..Default::default()
    };
    for row in rows {
        let chain: ChainId = row
            .get::<String, _>("chain")
            .parse()
            .map_err(|e| Failure::Permanent(format!("invalid stored chain: {e}")))?;
        let token: String = row.get("contract_or_mint");
        match source.fetch(chain, &token).await? {
            Some(meta) => {
                sqlx::query(
                    "UPDATE tokens SET decimals=$3,
                       symbol=COALESCE(symbol,$4), name=COALESCE(name,$5),
                       metadata_status='complete', metadata_checked_at=$6
                     WHERE chain=$1 AND contract_or_mint=$2",
                )
                .bind(chain.as_str())
                .bind(&token)
                .bind(i16::from(meta.decimals))
                .bind(meta.symbol)
                .bind(meta.name)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|e| Failure::Transient(e.to_string()))?;
                report.completed += 1;
            }
            None => {
                sqlx::query(
                    "UPDATE tokens SET metadata_status='unavailable', metadata_checked_at=$3
                     WHERE chain=$1 AND contract_or_mint=$2",
                )
                .bind(chain.as_str())
                .bind(&token)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|e| Failure::Transient(e.to_string()))?;
                report.unavailable += 1;
            }
        }
    }
    Ok(report)
}

pub async fn schedule(
    pool: &PgPool,
    enabled: bool,
    now: DateTime<Utc>,
) -> Result<bool, trace_jobs::JobError> {
    if !enabled {
        return Ok(false);
    }
    let key = format!("{FETCH_TOKEN_METADATA}:{}", now.timestamp() / 300);
    trace_jobs::enqueue(pool, FETCH_TOKEN_METADATA, Some(&key), json!({}), now, 3).await?;
    Ok(true)
}

pub type SharedMetadataSource = Arc<dyn MetadataSource>;

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_string(value: &str) -> Value {
        let mut data = vec![0u8; 64 + value.len().div_ceil(32) * 32];
        data[31] = 32;
        data[63] = value.len() as u8;
        data[64..64 + value.len()].copy_from_slice(value.as_bytes());
        Value::String(format!("0x{}", hex::encode(data)))
    }

    #[test]
    fn decodes_standard_and_bytes32_metadata() {
        let mut decimals = vec![0u8; 32];
        decimals[31] = 18;
        assert_eq!(
            decode_decimals(&Value::String(format!("0x{}", hex::encode(decimals)))),
            Some(18)
        );
        assert_eq!(
            decode_text(&encoded_string("Token Name")).as_deref(),
            Some("Token Name")
        );
        let mut bytes32 = [0u8; 32];
        bytes32[..3].copy_from_slice(b"TKN");
        assert_eq!(
            decode_text(&Value::String(format!("0x{}", hex::encode(bytes32)))).as_deref(),
            Some("TKN")
        );
    }

    #[tokio::test]
    async fn rpc_failures_never_reveal_the_url_or_key() {
        let source = EvmMetadataRpc::new(HashMap::from([(
            ChainId::Ethereum,
            "http://127.0.0.1:9/private-key".to_string(),
        )]))
        .unwrap();
        let error = format!(
            "{:?}",
            source
                .fetch(
                    ChainId::Ethereum,
                    "0x00000000000000000000000000000000000000f1",
                )
                .await
                .unwrap_err()
        );
        assert!(!error.contains("private-key"));
        assert!(!error.contains("127.0.0.1"));
    }
}
