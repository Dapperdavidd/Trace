//! Token price intake.
//!
//! A provider answers "what is this token worth, and when was that observed".
//! Nothing here invents a price. Every quote is checked before it can become a
//! `price_points` row, and a rejected quote is counted with its reason, never
//! silently dropped and never replaced by a guess.
//!
//! Rules:
//! * `observed_at` is the quote's own timestamp, never the poll time.
//! * A quote older than `MAX_QUOTE_AGE` is stale and rejected.
//! * Low provider confidence is rejected.
//! * Decimals decide the unit conversion. If we already know a token's
//!   decimals and the provider disagrees, the quote is rejected. If we know
//!   nothing, the provider's decimals are accepted and flagged for review.
//! * A price that underflows to zero per base unit is rejected, not rounded to zero.

use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;
use trace_domain::{AssetId, ChainId};

pub mod defillama;

pub const MAX_QUOTE_AGE_MINUTES: i64 = 15;
pub const MIN_CONFIDENCE: f64 = 0.8;

#[derive(Debug, thiserror::Error)]
pub enum PriceError {
    #[error("price provider error: {0}")]
    Provider(String),
}

/// What a provider said about one asset. Not yet trusted.
#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub asset: AssetId,
    /// USD per whole token, exactly as the provider printed it.
    pub usd_per_token: Decimal,
    pub decimals: Option<u8>,
    pub symbol: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The provider returned nothing for this asset.
    Missing,
    /// Not a positive finite number.
    BadPrice,
    Stale,
    LowConfidence,
    /// Provider decimals differ from the decimals we already store.
    DecimalsMismatch,
    /// Neither we nor the provider know the decimals.
    DecimalsUnknown,
    /// Price per base unit rounds to zero.
    Underflow,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accepted {
    pub asset: AssetId,
    pub observed_at: DateTime<Utc>,
    pub usd_per_base_unit: Decimal,
    /// Set when decimals came from the provider because we had none.
    pub learned_decimals: Option<u8>,
    pub symbol: Option<String>,
}

#[derive(Debug, Default)]
pub struct Batch {
    pub quotes: Vec<Quote>,
    pub rejected: Vec<(AssetId, Reason)>,
}

#[async_trait::async_trait]
pub trait PriceProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn fetch(&self, assets: &[AssetId]) -> Result<Batch, PriceError>;
}

fn pow10(d: u8) -> Decimal {
    Decimal::from_i128_with_scale(10i128.pow(d.min(28) as u32), 0)
}

/// Check one quote against what we know. `known_decimals` comes from our own `tokens` row.
pub fn accept(
    q: &Quote,
    known_decimals: Option<u8>,
    now: DateTime<Utc>,
) -> Result<Accepted, Reason> {
    if q.usd_per_token <= Decimal::ZERO {
        return Err(Reason::BadPrice);
    }
    if now - q.observed_at > Duration::minutes(MAX_QUOTE_AGE_MINUTES)
        || q.observed_at - now > Duration::minutes(2)
    {
        return Err(Reason::Stale);
    }
    if q.confidence
        .is_some_and(|c| !(MIN_CONFIDENCE..=1.0).contains(&c))
    {
        return Err(Reason::LowConfidence);
    }
    let (decimals, learned) = match (known_decimals, q.decimals) {
        (Some(k), Some(p)) if k != p => return Err(Reason::DecimalsMismatch),
        (Some(k), _) => (k, None),
        (None, Some(p)) => (p, Some(p)),
        (None, None) => return Err(Reason::DecimalsUnknown),
    };
    if decimals > 28 {
        return Err(Reason::DecimalsUnknown);
    }
    let per_base = q.usd_per_token / pow10(decimals);
    if per_base.is_zero() {
        return Err(Reason::Underflow);
    }
    Ok(Accepted {
        asset: q.asset.clone(),
        observed_at: q.observed_at,
        usd_per_base_unit: per_base,
        learned_decimals: learned,
        symbol: q.symbol.clone(),
    })
}

/// Provider key prefix for a chain.
pub fn llama_chain(c: ChainId) -> &'static str {
    match c {
        ChainId::Ethereum => "ethereum",
        ChainId::Base => "base",
        ChainId::Bnb => "bsc",
        ChainId::Solana => "solana",
    }
}

/// `chain:address`, the DefiLlama coin key.
pub fn coin_key(a: &AssetId) -> String {
    format!("{}:{}", llama_chain(a.chain), a.contract_or_mint.as_str())
}

/// Parse a DefiLlama `coins` response for the requested assets.
///
/// The price is read from the JSON number's text, so no float arithmetic ever
/// touches money. EVM keys are matched case-insensitively; Solana mints exactly.
pub fn parse_defillama(body: &Value, requested: &[AssetId]) -> Batch {
    let mut out = Batch::default();
    let coins = body.get("coins").and_then(Value::as_object);
    for a in requested {
        let want = coin_key(a);
        let entry = coins.and_then(|c| {
            c.get(&want).or_else(|| {
                (a.chain != ChainId::Solana)
                    .then(|| {
                        c.iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(&want))
                            .map(|(_, v)| v)
                    })
                    .flatten()
            })
        });
        let Some(e) = entry else {
            out.rejected.push((a.clone(), Reason::Missing));
            continue;
        };
        let price = e
            .get("price")
            .and_then(Value::as_number)
            .and_then(|n| Decimal::from_str(&n.to_string()).ok());
        let ts = e
            .get("timestamp")
            .and_then(Value::as_i64)
            .and_then(|t| Utc.timestamp_opt(t, 0).single());
        let (Some(price), Some(ts)) = (price, ts) else {
            out.rejected.push((a.clone(), Reason::BadPrice));
            continue;
        };
        out.quotes.push(Quote {
            asset: a.clone(),
            usd_per_token: price,
            decimals: e
                .get("decimals")
                .and_then(Value::as_u64)
                .and_then(|d| u8::try_from(d).ok()),
            symbol: e.get("symbol").and_then(Value::as_str).map(str::to_string),
            observed_at: ts,
            confidence: e.get("confidence").and_then(Value::as_f64),
        });
    }
    out
}
