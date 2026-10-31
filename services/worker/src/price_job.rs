//! Poll a price provider for recently traded tokens and store accepted quotes.
//!
//! Only real (non-demo) tokens that traded in the last week are requested.
//! Every quote passes `trace_pricing::accept`; rejections are counted by reason.
//! The job is off unless `TRACE_PRICE_PROVIDER` is set.

use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
use std::sync::Arc;
use trace_domain::{AssetId, ChainId};
use trace_jobs::Failure;
use trace_pricing::{accept, PriceProvider};

pub const POLL_PRICES: &str = "poll_prices";
pub const LOOKBACK_DAYS: i64 = 7;
pub const MAX_ASSETS: i64 = 200;

#[derive(Clone)]
pub struct PriceCtx {
    pub provider: Arc<dyn PriceProvider>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PriceReport {
    pub requested: usize,
    pub stored: usize,
    pub duplicates: usize,
    pub learned_decimals: usize,
    pub rejected: BTreeMap<String, usize>,
}

/// Real tokens that traded recently, busiest first.
pub async fn tracked_assets(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<Vec<AssetId>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT chain, asset FROM (
             SELECT chain, token_in AS asset FROM normalized_swaps
               WHERE NOT demo AND finality <> 'reorged' AND observed_at >= $1
             UNION ALL
             SELECT chain, token_out FROM normalized_swaps
               WHERE NOT demo AND finality <> 'reorged' AND observed_at >= $1
           ) t GROUP BY chain, asset ORDER BY count(*) DESC, chain, asset LIMIT $2"#,
    )
    .bind(now - Duration::days(LOOKBACK_DAYS))
    .bind(MAX_ASSETS)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let chain: ChainId = r.get::<String, _>("chain").parse().ok()?;
            AssetId::parse(chain, &r.get::<String, _>("asset")).ok()
        })
        .collect())
}

pub async fn poll(
    pool: &PgPool,
    provider: &dyn PriceProvider,
    now: DateTime<Utc>,
) -> Result<PriceReport, Failure> {
    let transient = |e: &dyn std::fmt::Display| Failure::Transient(e.to_string());
    let assets = tracked_assets(pool, now).await.map_err(|e| transient(&e))?;
    let mut report = PriceReport {
        requested: assets.len(),
        ..Default::default()
    };
    if assets.is_empty() {
        return Ok(report);
    }
    let batch = provider.fetch(&assets).await.map_err(|e| transient(&e))?;
    let mut count = |r: trace_pricing::Reason| {
        *report.rejected.entry(format!("{r:?}")).or_default() += 1;
    };
    for (_, r) in &batch.rejected {
        count(*r);
    }
    for q in &batch.quotes {
        let known: Option<i16> = sqlx::query_scalar(
            "SELECT decimals FROM tokens WHERE chain = $1 AND contract_or_mint = $2",
        )
        .bind(q.asset.chain.as_str())
        .bind(q.asset.contract_or_mint.as_str())
        .fetch_optional(pool)
        .await
        .map_err(|e| transient(&e))?
        .flatten();
        let known = known.and_then(|d| u8::try_from(d).ok());
        let a = match accept(q, known, now) {
            Ok(a) => a,
            Err(r) => {
                count(r);
                continue;
            }
        };
        if let Some(d) = a.learned_decimals {
            sqlx::query(
                "UPDATE tokens SET decimals = $3, symbol = COALESCE(symbol, $4)
                 WHERE chain = $1 AND contract_or_mint = $2 AND decimals IS NULL",
            )
            .bind(a.asset.chain.as_str())
            .bind(a.asset.contract_or_mint.as_str())
            .bind(d as i16)
            .bind(&a.symbol)
            .execute(pool)
            .await
            .map_err(|e| transient(&e))?;
            tracing::warn!(asset = %a.asset.contract_or_mint.as_str(), decimals = d, "decimals taken from price provider; review");
            report.learned_decimals += 1;
        }
        let r = sqlx::query(
            "INSERT INTO price_points (chain, asset, observed_at, usd_per_base_unit, source, demo)
             VALUES ($1, $2, $3, $4, $5, FALSE) ON CONFLICT DO NOTHING",
        )
        .bind(a.asset.chain.as_str())
        .bind(a.asset.contract_or_mint.as_str())
        .bind(a.observed_at)
        .bind(a.usd_per_base_unit)
        .bind(provider.name())
        .execute(pool)
        .await
        .map_err(|e| transient(&e))?;
        if r.rows_affected() == 1 {
            report.stored += 1;
        } else {
            report.duplicates += 1;
        }
    }
    Ok(report)
}

pub async fn schedule_prices(
    pool: &PgPool,
    enabled: bool,
    now: DateTime<Utc>,
) -> Result<bool, trace_jobs::JobError> {
    if !enabled {
        return Ok(false);
    }
    let key = format!("{POLL_PRICES}:{}", now.timestamp() / 300);
    trace_jobs::enqueue(pool, POLL_PRICES, Some(&key), serde_json::json!({}), now, 3).await?;
    Ok(true)
}
