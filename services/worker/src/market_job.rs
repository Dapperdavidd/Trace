//! Build honest market snapshots from stored real swaps and accepted prices.
//!
//! Price and volume are derived from evidence already in PostgreSQL. Liquidity
//! stays NULL until a reviewed reserve or market-data source is implemented.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use trace_jobs::Failure;

pub const BUILD_MARKET_SNAPSHOTS: &str = "build_market_snapshots";
const MAX_POOLS: i64 = 200;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MarketReport {
    pub candidates: usize,
    pub written: usize,
    pub missing_prices: usize,
}

struct PriceAt {
    per_base: Decimal,
    observed_at: DateTime<Utc>,
    decimals: i16,
}

async fn price_at(
    pool: &PgPool,
    chain: &str,
    token: &str,
    at: DateTime<Utc>,
) -> Result<Option<PriceAt>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT p.usd_per_base_unit, p.observed_at, t.decimals
         FROM price_points p
         JOIN tokens t ON t.chain=p.chain AND t.contract_or_mint=p.asset
         WHERE p.chain=$1 AND p.asset=$2 AND p.observed_at <= $3
           AND p.observed_at >= $4
           AND NOT p.demo AND t.decimals IS NOT NULL
         ORDER BY p.observed_at DESC LIMIT 1",
    )
    .bind(chain)
    .bind(token)
    .bind(at)
    .bind(at - Duration::hours(6))
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| PriceAt {
        per_base: r.get("usd_per_base_unit"),
        observed_at: r.get("observed_at"),
        decimals: r.get("decimals"),
    }))
}

fn token_price(p: &PriceAt) -> Option<Decimal> {
    let d = u32::try_from(p.decimals).ok()?.min(28);
    let scale = Decimal::from_i128_with_scale(10i128.checked_pow(d)?, 0);
    p.per_base.checked_mul(scale)
}

async fn volume(
    pool: &PgPool,
    chain: &str,
    token: &str,
    market_pool: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Option<Decimal>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT CASE WHEN s.token_in=$2 THEN s.amount_in ELSE s.amount_out END AS amount,
           (SELECT p.usd_per_base_unit FROM price_points p
            WHERE p.chain=s.chain AND p.asset=$2 AND NOT p.demo
              AND p.observed_at <= s.observed_at
              AND p.observed_at >= s.observed_at - interval '6 hours'
            ORDER BY p.observed_at DESC LIMIT 1) AS usd_per_base_unit
         FROM normalized_swaps s
         WHERE s.chain=$1 AND s.pool=$3 AND NOT s.demo AND s.finality='final'
           AND (s.token_in=$2 OR s.token_out=$2)
           AND s.observed_at >= $4 AND s.observed_at < $5",
    )
    .bind(chain)
    .bind(token)
    .bind(market_pool)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    let mut total = Decimal::ZERO;
    for row in rows {
        let amount: Decimal = row.get("amount");
        let per_base: Option<Decimal> = row.get("usd_per_base_unit");
        let Some(value) = per_base.and_then(|p| amount.checked_mul(p)) else {
            return Ok(None);
        };
        let Some(next) = total.checked_add(value) else {
            return Ok(None);
        };
        total = next;
    }
    Ok(Some(total))
}

pub async fn build(pool: &PgPool, now: DateTime<Utc>) -> Result<MarketReport, Failure> {
    let transient = |e: sqlx::Error| Failure::Transient(e.to_string());
    let candidates = sqlx::query(
        "SELECT DISTINCT s.chain, s.pool, v.token
         FROM normalized_swaps s
         CROSS JOIN LATERAL (VALUES (s.token_in), (s.token_out)) v(token)
         JOIN tokens t ON t.chain=s.chain AND t.contract_or_mint=v.token
         WHERE NOT s.demo AND s.finality='final' AND NOT t.is_stable
           AND s.observed_at >= $1
         ORDER BY s.chain, s.pool, v.token LIMIT $2",
    )
    .bind(now - Duration::days(7))
    .bind(MAX_POOLS)
    .fetch_all(pool)
    .await
    .map_err(&transient)?;
    let mut report = MarketReport {
        candidates: candidates.len(),
        ..Default::default()
    };
    for row in candidates {
        let chain: String = row.get("chain");
        let market_pool: String = row.get("pool");
        let token: String = row.get("token");
        let Some(current) = price_at(pool, &chain, &token, now)
            .await
            .map_err(&transient)?
        else {
            report.missing_prices += 1;
            continue;
        };
        let Some(price_now) = token_price(&current) else {
            report.missing_prices += 1;
            continue;
        };
        for (win, duration) in [
            ("1h", Duration::hours(1)),
            ("24h", Duration::hours(24)),
            ("7d", Duration::days(7)),
        ] {
            let start = now - duration;
            let Some(first) = price_at(pool, &chain, &token, start)
                .await
                .map_err(&transient)?
            else {
                report.missing_prices += 1;
                continue;
            };
            let Some(price_start) = token_price(&first) else {
                report.missing_prices += 1;
                continue;
            };
            let Some(current_volume) = volume(pool, &chain, &token, &market_pool, start, now)
                .await
                .map_err(&transient)?
            else {
                report.missing_prices += 1;
                continue;
            };
            let Some(baseline) =
                volume(pool, &chain, &token, &market_pool, start - duration, start)
                    .await
                    .map_err(&transient)?
            else {
                report.missing_prices += 1;
                continue;
            };
            sqlx::query(
                "INSERT INTO market_snapshots
                   (chain, token, pool, win, price_now_usd, price_window_start_usd,
                    volume_window_usd, volume_baseline_usd, liquidity_usd,
                    last_updated, demo)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,NULL,$9,FALSE)
                 ON CONFLICT (chain, token, pool, win) DO UPDATE SET
                   price_now_usd=EXCLUDED.price_now_usd,
                   price_window_start_usd=EXCLUDED.price_window_start_usd,
                   volume_window_usd=EXCLUDED.volume_window_usd,
                   volume_baseline_usd=EXCLUDED.volume_baseline_usd,
                   last_updated=EXCLUDED.last_updated,
                   demo=FALSE",
            )
            .bind(&chain)
            .bind(&token)
            .bind(&market_pool)
            .bind(win)
            .bind(price_now)
            .bind(price_start)
            .bind(current_volume)
            .bind(baseline)
            .bind(current.observed_at)
            .execute(pool)
            .await
            .map_err(&transient)?;
            report.written += 1;
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
    let key = format!("{BUILD_MARKET_SNAPSHOTS}:{}", now.timestamp() / 300);
    trace_jobs::enqueue(
        pool,
        BUILD_MARKET_SNAPSHOTS,
        Some(&key),
        serde_json::json!({}),
        now,
        3,
    )
    .await?;
    Ok(true)
}
