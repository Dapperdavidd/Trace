use chrono::{DateTime, Utc};
use serde::Serialize;
use std::str::FromStr;
use trace_domain::{AssetId, ChainId, DataStatus, WalletId};

use crate::error::ApiError;

/// Honesty envelope carried by every analytic response.
#[derive(Debug, Clone, Serialize)]
pub struct Envelope {
    pub data_status: DataStatus,
    pub as_of: DateTime<Utc>,
    pub demo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_note: Option<String>,
}

impl Envelope {
    pub fn new(data_status: DataStatus, as_of: DateTime<Utc>, demo: bool) -> Self {
        Self {
            data_status,
            as_of,
            demo,
            coverage_note: None,
        }
    }
    pub fn note(mut self, n: impl Into<String>) -> Self {
        self.coverage_note = Some(n.into());
        self
    }
}

/// Status for rows that may be demo, stale or live. Demo data is never
/// labelled live, and a source older than `max_age_secs` is stale.
pub fn status_for(
    demo: bool,
    as_of: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    max_age_secs: i64,
) -> DataStatus {
    match (demo, as_of) {
        (true, _) => DataStatus::Demo,
        (false, None) => DataStatus::Unavailable,
        (false, Some(t)) if (now - t).num_seconds() > max_age_secs => DataStatus::Stale,
        _ => DataStatus::Live,
    }
}

pub fn parse_chain(s: &str) -> Result<ChainId, ApiError> {
    ChainId::from_str(s).map_err(ApiError::from)
}

pub fn wallet_id(chain: &str, address: &str) -> Result<WalletId, ApiError> {
    Ok(WalletId::parse(parse_chain(chain)?, address)?)
}

pub fn asset_id(chain: &str, address: &str) -> Result<AssetId, ApiError> {
    Ok(AssetId::parse(parse_chain(chain)?, address)?)
}

/// Opaque keyset cursor: hex("<micros>|<uuid>").
pub fn encode_cursor(at: DateTime<Utc>, id: uuid::Uuid) -> String {
    hex::encode(format!("{}|{}", at.timestamp_micros(), id))
}

pub fn decode_cursor(c: &str) -> Result<(DateTime<Utc>, uuid::Uuid), ApiError> {
    let bad = || ApiError::BadRequest("Invalid cursor.".into());
    let raw = String::from_utf8(hex::decode(c).map_err(|_| bad())?).map_err(|_| bad())?;
    let (m, id) = raw.split_once('|').ok_or_else(bad)?;
    let at = DateTime::from_timestamp_micros(m.parse().map_err(|_| bad())?).ok_or_else(bad)?;
    Ok((at, id.parse().map_err(|_| bad())?))
}

pub fn page_limit(raw: Option<u32>) -> i64 {
    raw.unwrap_or(20).clamp(1, 50) as i64
}

use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;

pub fn token_ref(chain: &str, address: &str, symbol: Option<&str>, name: Option<&str>) -> Value {
    json!({ "chain": chain, "address": address, "symbol": symbol, "name": name })
}

/// symbol and name for a set of tokens on one chain, keyed by address.
pub async fn token_meta(
    pool: &PgPool,
    chain: &str,
    addrs: &[String],
) -> Result<BTreeMap<String, (Option<String>, Option<String>)>, ApiError> {
    Ok(sqlx::query(
        "SELECT contract_or_mint, symbol, name FROM tokens WHERE chain = $1 AND contract_or_mint = ANY($2)",
    )
    .bind(chain)
    .bind(addrs)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.get("contract_or_mint"), (r.get("symbol"), r.get("name"))))
    .collect())
}

/// Most recent successful scan across the chain's protocols, if any ran.
pub async fn chain_freshness(
    pool: &PgPool,
    chain: &str,
) -> Result<Option<DateTime<Utc>>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT max(last_successful_scan_at) FROM protocols WHERE chain = $1 AND support_status IN ('enabled','partial')",
    )
    .bind(chain)
    .fetch_one(pool)
    .await?)
}

pub fn dec_str(d: rust_decimal::Decimal) -> String {
    d.normalize().to_string()
}
