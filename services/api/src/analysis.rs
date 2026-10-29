//! Glue between stored rows and the pure logic crates. Nothing here invents
//! data: a missing price, snapshot or history yields `None` or an empty list,
//! and the route reports that state instead of a made-up value.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, BTreeSet};
use trace_accounting::{compute, Mark, PriceSource, PricedValue, WalletAccounting};
use trace_discovery::movers::Snapshot;
use trace_domain::{AssetId, BaseUnits, ChainId, NormalizedSwap, WalletId};
use trace_risk::{assess, RiskInputs, RiskPolicy, RiskReport};
use trace_shadow::Side;

use crate::error::ApiError;

/// A price older or newer than this from the trade is not used for it.
pub const PRICE_TOLERANCE_HOURS: i64 = 6;
pub const MAX_SWAPS_PER_WALLET: i64 = 5_000;

pub struct DbPrices {
    points: BTreeMap<AssetId, Vec<(DateTime<Utc>, Decimal, String)>>,
    stable: BTreeSet<AssetId>,
    decimals: BTreeMap<AssetId, u8>,
    tolerance: Duration,
}

fn pow10(d: u8) -> Decimal {
    Decimal::from_i128_with_scale(10i128.pow(d.min(28) as u32), 0)
}

impl DbPrices {
    pub async fn load(pool: &PgPool, assets: &BTreeSet<AssetId>) -> Result<Self, ApiError> {
        let mut out = Self {
            points: BTreeMap::new(),
            stable: BTreeSet::new(),
            decimals: BTreeMap::new(),
            tolerance: Duration::hours(PRICE_TOLERANCE_HOURS),
        };
        let mut by_chain: BTreeMap<ChainId, Vec<String>> = BTreeMap::new();
        for a in assets {
            by_chain
                .entry(a.chain)
                .or_default()
                .push(a.contract_or_mint.to_string());
        }
        for (chain, addrs) in by_chain {
            for r in sqlx::query(
                "SELECT contract_or_mint, decimals, is_stable FROM tokens WHERE chain = $1 AND contract_or_mint = ANY($2)",
            )
            .bind(chain.as_str())
            .bind(&addrs)
            .fetch_all(pool)
            .await?
            {
                let Ok(a) = AssetId::parse(chain, &r.get::<String, _>("contract_or_mint")) else { continue };
                if r.get::<bool, _>("is_stable") {
                    out.stable.insert(a.clone());
                }
                if let Some(d) = r.get::<Option<i16>, _>("decimals") {
                    out.decimals.insert(a, d.clamp(0, 28) as u8);
                }
            }
            for r in sqlx::query(
                "SELECT asset, observed_at, usd_per_base_unit, source FROM price_points
                 WHERE chain = $1 AND asset = ANY($2) ORDER BY observed_at",
            )
            .bind(chain.as_str())
            .bind(&addrs)
            .fetch_all(pool)
            .await?
            {
                let Ok(a) = AssetId::parse(chain, &r.get::<String, _>("asset")) else {
                    continue;
                };
                out.points.entry(a).or_default().push((
                    r.get("observed_at"),
                    r.get("usd_per_base_unit"),
                    r.get("source"),
                ));
            }
        }
        Ok(out)
    }

    pub fn decimals(&self, a: &AssetId) -> Option<u8> {
        self.decimals.get(a).copied()
    }

    pub fn is_stable_asset(&self, a: &AssetId) -> bool {
        self.stable.contains(a)
    }

    /// Latest known price per asset, for marking open positions.
    pub fn marks(&self) -> BTreeMap<AssetId, Mark> {
        self.points
            .iter()
            .filter_map(|(a, v)| {
                v.last().map(|(t, p, s)| {
                    (
                        a.clone(),
                        Mark {
                            usd_per_base_unit: *p,
                            observed_at: *t,
                            source: s.clone(),
                        },
                    )
                })
            })
            .collect()
    }

    pub fn last_price_at(&self, a: &AssetId) -> Option<DateTime<Utc>> {
        self.points.get(a).and_then(|v| v.last()).map(|p| p.0)
    }

    /// USD per whole token at `at` (nearest point within tolerance).
    pub fn unit_price(&self, a: &AssetId, at: DateTime<Utc>) -> Option<Decimal> {
        let d = self.decimals(a)?;
        let v = self.points.get(a)?;
        let p = v
            .iter()
            .filter(|(t, _, _)| (*t - at).abs() <= self.tolerance)
            .min_by_key(|(t, _, _)| (*t - at).num_seconds().abs())?;
        Some(p.1 * pow10(d))
    }
}

impl PriceSource for DbPrices {
    fn usd_value(
        &self,
        asset: &AssetId,
        amount: BaseUnits,
        at: DateTime<Utc>,
    ) -> Option<PricedValue> {
        let amt = Decimal::from_u128(amount.0)?;
        let stable = self.stable.contains(asset);
        if let Some(v) = self.points.get(asset) {
            if let Some((_, p, src)) = v
                .iter()
                .filter(|(t, _, _)| (*t - at).abs() <= self.tolerance)
                .min_by_key(|(t, _, _)| (*t - at).num_seconds().abs())
            {
                return Some(PricedValue {
                    usd: amt * *p,
                    source: src.clone(),
                    is_quote: stable,
                });
            }
        }
        if stable {
            let d = self.decimals.get(asset)?;
            return Some(PricedValue {
                usd: amt / pow10(*d),
                source: "stable_peg_assumption".into(),
                is_quote: true,
            });
        }
        None
    }

    fn is_stable(&self, asset: &AssetId) -> bool {
        self.stable.contains(asset)
    }
}

/// A swap read as a trade of one non-stable token.
#[derive(Debug, Clone)]
pub struct TradeView {
    pub side: Side,
    pub token: AssetId,
    pub amount_usd: Option<Decimal>,
    pub unit_price_usd: Option<Decimal>,
}

/// Stable in, token out is a buy; token in, stable out is a sell. A swap
/// between two non-stable tokens is read as a buy of the received token.
/// Stable-to-stable is not a trade of interest.
pub fn interpret(s: &NormalizedSwap, p: &DbPrices) -> Option<TradeView> {
    let (side, token, token_amt, other, other_amt) =
        match (p.is_stable(&s.token_in), p.is_stable(&s.token_out)) {
            (true, true) => return None,
            (true, false) => (
                Side::Buy,
                &s.token_out,
                s.amount_out,
                &s.token_in,
                s.amount_in,
            ),
            (false, true) => (
                Side::Sell,
                &s.token_in,
                s.amount_in,
                &s.token_out,
                s.amount_out,
            ),
            (false, false) => (
                Side::Buy,
                &s.token_out,
                s.amount_out,
                &s.token_in,
                s.amount_in,
            ),
        };
    let usd = p.usd_value(other, other_amt, s.observed_at).map(|v| v.usd);
    let unit = match (usd, p.decimals(token)) {
        (Some(u), Some(d)) => {
            let qty = Decimal::from_u128(token_amt.0)? / pow10(d);
            if qty.is_zero() {
                None
            } else {
                Some(u / qty)
            }
        }
        _ => None,
    };
    Some(TradeView {
        side,
        token: token.clone(),
        amount_usd: usd,
        unit_price_usd: unit,
    })
}

pub struct WalletAnalysis {
    pub swaps: Vec<NormalizedSwap>,
    pub accounting: WalletAccounting,
    pub prices: DbPrices,
    pub demo: bool,
}

pub async fn analyze_wallet(pool: &PgPool, w: &WalletId) -> Result<WalletAnalysis, ApiError> {
    let swaps = trace_database::reads::wallet_swaps(
        pool,
        w.chain.as_str(),
        w.address.as_str(),
        MAX_SWAPS_PER_WALLET,
    )
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;
    let assets: BTreeSet<AssetId> = swaps
        .iter()
        .flat_map(|s| [s.token_in.clone(), s.token_out.clone()])
        .collect();
    let prices = DbPrices::load(pool, &assets).await?;
    let accounting = compute(&swaps, &prices, &prices.marks());
    let demo = swaps.iter().any(|s| s.demo);
    Ok(WalletAnalysis {
        swaps,
        accounting,
        prices,
        demo,
    })
}

pub struct SnapshotRow {
    pub snap: Snapshot,
    pub demo: bool,
    pub sparkline: Vec<f64>,
    pub market_cap_usd: Option<Decimal>,
    pub holders: Option<i32>,
    pub risk: RiskReport,
    pub name: Option<String>,
}

fn build_snapshot(r: &sqlx::postgres::PgRow, now: DateTime<Utc>) -> Option<SnapshotRow> {
    let chain: ChainId = r.get::<String, _>("chain").parse().ok()?;
    let token = AssetId::parse(chain, &r.get::<String, _>("token")).ok()?;
    let last_updated: DateTime<Utc> = r.get("last_updated");
    let liquidity: Decimal = r.get("liquidity_usd");
    let pool_age = r
        .get::<Option<i32>, _>("pool_age_hours")
        .map(|h| h.max(0) as u32);
    let mut inputs: RiskInputs = serde_json::from_value(r.get("risk_inputs")).unwrap_or_default();
    inputs.liquidity_usd.get_or_insert(liquidity);
    if inputs.pool_age_hours.is_none() {
        inputs.pool_age_hours = pool_age;
    }
    inputs
        .quote_age_seconds
        .get_or_insert((now - last_updated).num_seconds().max(0) as u64);
    let report = assess(&RiskPolicy::default(), &inputs, now);
    let snap = Snapshot {
        token,
        symbol: r.get("symbol"),
        pool: r.get("pool"),
        price_now_usd: r.get("price_now_usd"),
        price_window_start_usd: r.get("price_window_start_usd"),
        volume_window_usd: r.get("volume_window_usd"),
        volume_baseline_usd: r.get("volume_baseline_usd"),
        liquidity_usd: liquidity,
        pool_age_hours: pool_age,
        independent_smart_buyers: r.get::<i32, _>("independent_smart_buyers").max(0) as u32,
        large_buyer_flows: r.get::<i32, _>("large_buyer_flows").max(0) as u32,
        risk: report.overall,
        last_updated,
    };
    let sparkline = r
        .get::<serde_json::Value, _>("sparkline")
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();
    Some(SnapshotRow {
        snap,
        demo: r.get("demo"),
        sparkline,
        market_cap_usd: r.get("market_cap_usd"),
        holders: r.get("holders"),
        risk: report,
        name: r.get("name"),
    })
}

const SNAP_SELECT: &str = r#"SELECT m.*, t.symbol, t.name FROM market_snapshots m
    JOIN tokens t ON t.chain = m.chain AND t.contract_or_mint = m.token"#;

pub async fn load_snapshots(
    pool: &PgPool,
    win: &str,
    chain: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Vec<SnapshotRow>, ApiError> {
    let sql = format!("{SNAP_SELECT} WHERE m.win = $1 AND ($2::text IS NULL OR m.chain = $2)");
    Ok(sqlx::query(&sql)
        .bind(win)
        .bind(chain)
        .fetch_all(pool)
        .await?
        .iter()
        .filter_map(|r| build_snapshot(r, now))
        .collect())
}

/// Best snapshot for a token: the 24h window when present, otherwise the newest.
pub async fn load_token_snapshot(
    pool: &PgPool,
    chain: &str,
    token: &str,
    now: DateTime<Utc>,
) -> Result<Option<SnapshotRow>, ApiError> {
    let sql = format!(
        "{SNAP_SELECT} WHERE m.chain = $1 AND m.token = $2
         ORDER BY (m.win = '24h') DESC, m.liquidity_usd DESC, m.last_updated DESC LIMIT 1"
    );
    Ok(sqlx::query(&sql)
        .bind(chain)
        .bind(token)
        .fetch_optional(pool)
        .await?
        .and_then(|r| build_snapshot(&r, now)))
}
