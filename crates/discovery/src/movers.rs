//! Top Movers. Built from independent pool snapshots with an explicit window,
//! last_updated, chain/address and liquidity depth filter, so thin-liquidity
//! pump artifacts and stale data are excluded rather than ranked.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use trace_domain::AssetId;
use trace_risk::RiskState;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub token: AssetId,
    pub symbol: Option<String>,
    pub pool: String,
    pub price_now_usd: Decimal,
    pub price_window_start_usd: Decimal,
    pub volume_window_usd: Decimal,
    /// Typical volume for the same window length, from a baseline period.
    pub volume_baseline_usd: Option<Decimal>,
    pub liquidity_usd: Decimal,
    pub pool_age_hours: Option<u32>,
    pub independent_smart_buyers: u32,
    pub large_buyer_flows: u32,
    pub risk: RiskState,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tab {
    Trending,
    EarlyMomentum,
    SmartMoney,
    New,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    SmartWalletsBuying,
    HighVolumeSpike,
    WhaleAccumulation,
    UnusualActivity,
    NewPool,
}

impl ReasonCode {
    pub fn label(self) -> &'static str {
        match self {
            ReasonCode::SmartWalletsBuying => "Smart wallets buying",
            ReasonCode::HighVolumeSpike => "High volume spike",
            ReasonCode::WhaleAccumulation => "Large wallets accumulating",
            ReasonCode::UnusualActivity => "Unusual activity",
            ReasonCode::NewPool => "New pool",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mover {
    pub rank: usize,
    pub token: AssetId,
    pub symbol: Option<String>,
    pub pool: String,
    pub price_usd: Decimal,
    pub change_pct: Decimal,
    pub volume_usd: Decimal,
    pub liquidity_usd: Decimal,
    pub reason: ReasonCode,
    pub risk: RiskState,
    pub source_timestamp: DateTime<Utc>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Excluded {
    pub stale: usize,
    pub thin_liquidity: usize,
    pub invalid_price: usize,
    pub blocked_risk: usize,
}

#[derive(Debug, Clone)]
pub struct MoverConfig {
    pub min_liquidity_usd: Decimal,
    pub max_age: Duration,
    pub volume_spike_ratio: Decimal,
    pub new_pool_max_age_hours: u32,
    pub min_smart_buyers: u32,
    pub limit: usize,
}

impl Default for MoverConfig {
    fn default() -> Self {
        Self {
            min_liquidity_usd: Decimal::from(25_000),
            max_age: Duration::minutes(10),
            volume_spike_ratio: Decimal::from(3),
            new_pool_max_age_hours: 48,
            min_smart_buyers: 2,
            limit: 20,
        }
    }
}

fn reason(cfg: &MoverConfig, s: &Snapshot) -> Option<ReasonCode> {
    if s.independent_smart_buyers >= cfg.min_smart_buyers {
        Some(ReasonCode::SmartWalletsBuying)
    } else if s.large_buyer_flows >= 3 {
        Some(ReasonCode::WhaleAccumulation)
    } else if s
        .volume_baseline_usd
        .is_some_and(|b| b > Decimal::ZERO && s.volume_window_usd / b >= cfg.volume_spike_ratio)
    {
        Some(ReasonCode::HighVolumeSpike)
    } else if s
        .pool_age_hours
        .is_some_and(|h| h <= cfg.new_pool_max_age_hours)
    {
        Some(ReasonCode::NewPool)
    } else {
        None
    }
}

/// Rank movers for one tab. Returns the ranked list and what was excluded.
pub fn rank(
    cfg: &MoverConfig,
    tab: Tab,
    snaps: &[Snapshot],
    now: DateTime<Utc>,
) -> (Vec<Mover>, Excluded) {
    let mut ex = Excluded::default();
    let mut items: Vec<(Decimal, Mover)> = vec![];
    for s in snaps {
        if now - s.last_updated > cfg.max_age {
            ex.stale += 1;
            continue;
        }
        if s.price_now_usd <= Decimal::ZERO || s.price_window_start_usd <= Decimal::ZERO {
            ex.invalid_price += 1;
            continue;
        }
        if s.liquidity_usd < cfg.min_liquidity_usd {
            ex.thin_liquidity += 1;
            continue;
        }
        if s.risk == RiskState::Block {
            ex.blocked_risk += 1;
            continue;
        }
        let Some(r) = reason(cfg, s) else { continue };
        let in_tab = match tab {
            Tab::Trending => true,
            Tab::EarlyMomentum => {
                s.liquidity_usd < Decimal::from(1_000_000)
                    && (s.independent_smart_buyers > 0 || s.large_buyer_flows > 0)
            }
            Tab::SmartMoney => s.independent_smart_buyers >= cfg.min_smart_buyers,
            Tab::New => s
                .pool_age_hours
                .is_some_and(|h| h <= cfg.new_pool_max_age_hours),
        };
        if !in_tab {
            continue;
        }
        let change = (s.price_now_usd - s.price_window_start_usd) / s.price_window_start_usd
            * Decimal::from(100);
        // Rank by attention weighted by depth: volume ratio when known, otherwise raw volume.
        let attention = match s.volume_baseline_usd {
            Some(b) if b > Decimal::ZERO => s.volume_window_usd / b,
            _ => s.volume_window_usd / Decimal::from(1_000_000),
        };
        let key = match tab {
            Tab::SmartMoney => {
                Decimal::from(s.independent_smart_buyers) * Decimal::from(1_000) + change
            }
            Tab::New => s.liquidity_usd,
            _ => change * attention.max(Decimal::new(1, 1)),
        };
        items.push((
            key,
            Mover {
                rank: 0,
                token: s.token.clone(),
                symbol: s.symbol.clone(),
                pool: s.pool.clone(),
                price_usd: s.price_now_usd,
                change_pct: change,
                volume_usd: s.volume_window_usd,
                liquidity_usd: s.liquidity_usd,
                reason: r,
                risk: s.risk,
                source_timestamp: s.last_updated,
            },
        ));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.token.cmp(&b.1.token)));
    let ranked = items
        .into_iter()
        .take(cfg.limit)
        .enumerate()
        .map(|(i, (_, mut m))| {
            m.rank = i + 1;
            m
        })
        .collect();
    (ranked, ex)
}
