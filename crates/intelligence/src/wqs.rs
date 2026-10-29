//! WQS: Wallet Quality Score. Historical, risk-adjusted evidence of trader
//! consistency from reconstructable closed lots only.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use trace_accounting::ClosedLot;

use crate::score::{Decision, Score};

pub const VERSION: &str = "wqs/1";

#[derive(Debug, Clone)]
pub struct WqsConfig {
    pub min_closes: usize,
    pub min_coverage: Decimal,
    /// Shrinkage strength: shrunk = mean * n / (n + k).
    pub shrink_k: Decimal,
    /// Per-close return is clamped to +/- this fraction so one outlier cannot dominate.
    pub winsor: Decimal,
    pub windows: u32,
    pub min_positive_window_share: Decimal,
    pub max_drawdown_ratio: Decimal,
    pub recency_days: i64,
    pub min_recent_closes: usize,
}

impl Default for WqsConfig {
    fn default() -> Self {
        Self {
            min_closes: 10,
            min_coverage: Decimal::new(5, 1),
            shrink_k: Decimal::from(10),
            winsor: Decimal::from(2),
            windows: 4,
            min_positive_window_share: Decimal::new(5, 1),
            max_drawdown_ratio: Decimal::new(5, 1),
            recency_days: 30,
            min_recent_closes: 1,
        }
    }
}

fn clamp01(x: Decimal) -> Decimal {
    x.max(Decimal::ZERO).min(Decimal::ONE)
}

pub fn score(
    cfg: &WqsConfig,
    closes: &[ClosedLot],
    coverage: Option<Decimal>,
    cohort: &str,
    now: DateTime<Utc>,
) -> Score {
    let n = closes.len();
    let mut comps: BTreeMap<String, String> = BTreeMap::new();
    comps.insert("sample_closes".into(), n.to_string());
    comps.insert(
        "coverage_ratio".into(),
        coverage
            .map(|c| c.round_dp(3).to_string())
            .unwrap_or_else(|| "unavailable".into()),
    );

    let window = match (
        closes.iter().map(|c| c.closed_at).min(),
        closes.iter().map(|c| c.closed_at).max(),
    ) {
        (Some(a), Some(b)) => format!("{} to {}", a.format("%Y-%m-%d"), b.format("%Y-%m-%d")),
        _ => "no closed trades".into(),
    };
    let mk = |decision, value, explanation: String, comps| Score {
        decision,
        value,
        version: VERSION,
        components: comps,
        cohort: cohort.to_string(),
        window: window.clone(),
        explanation,
    };

    let enough_coverage = coverage.is_some_and(|c| c >= cfg.min_coverage);
    if n < cfg.min_closes || !enough_coverage {
        return mk(
            Decision::InsufficientEvidence,
            None,
            format!(
                "Not enough reliable history to rate this wallet: {} reconstructable closed trades (need {}), and the share of its history we could reconstruct is {}.",
                n,
                cfg.min_closes,
                if enough_coverage { "adequate" } else { "too low or unknown" }
            ),
            comps,
        );
    }

    // Per-close returns, winsorized.
    let rets: Vec<Decimal> = closes
        .iter()
        .filter(|c| c.cost_usd > Decimal::ZERO)
        .map(|c| (c.pnl_usd / c.cost_usd).max(-cfg.winsor).min(cfg.winsor))
        .collect();
    let m = rets.len();
    if m < cfg.min_closes {
        return mk(
            Decision::InsufficientEvidence,
            None,
            "Too few closed trades had a usable cost basis.".into(),
            comps,
        );
    }
    let nd = Decimal::from(m as u64);
    let mean = rets.iter().copied().sum::<Decimal>() / nd;
    let shrunk = mean * nd / (nd + cfg.shrink_k);
    let wins = closes.iter().filter(|c| c.pnl_usd > Decimal::ZERO).count();
    let win_rate = Decimal::from(wins as u64) / Decimal::from(n as u64);

    // Independent time windows: share of non-empty windows with positive net PnL.
    let mut sorted: Vec<&ClosedLot> = closes.iter().collect();
    sorted.sort_by_key(|c| c.closed_at);
    let t0 = sorted.first().unwrap().closed_at.timestamp();
    let t1 = sorted.last().unwrap().closed_at.timestamp();
    let span = (t1 - t0).max(1);
    let w = cfg.windows.max(1) as i64;
    let mut buckets: BTreeMap<i64, Decimal> = BTreeMap::new();
    for c in &sorted {
        let idx = (((c.closed_at.timestamp() - t0) * w) / (span + 1)).min(w - 1);
        *buckets.entry(idx).or_default() += c.pnl_usd;
    }
    let positive_share =
        Decimal::from(buckets.values().filter(|v| **v > Decimal::ZERO).count() as u64)
            / Decimal::from(buckets.len() as u64);

    // Max drawdown of cumulative realized PnL relative to total cost.
    let total_cost: Decimal = closes.iter().map(|c| c.cost_usd).sum();
    let (mut cum, mut peak, mut max_dd) = (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    for c in &sorted {
        cum += c.pnl_usd;
        peak = peak.max(cum);
        max_dd = max_dd.max(peak - cum);
    }
    let dd_ratio = if total_cost > Decimal::ZERO {
        max_dd / total_cost
    } else {
        Decimal::ZERO
    };

    let recent = closes
        .iter()
        .filter(|c| c.closed_at >= now - Duration::days(cfg.recency_days))
        .count();

    comps.insert(
        "mean_return_winsorized".into(),
        mean.round_dp(4).to_string(),
    );
    comps.insert("shrunk_expectancy".into(), shrunk.round_dp(4).to_string());
    comps.insert("win_rate".into(), win_rate.round_dp(3).to_string());
    comps.insert(
        "positive_window_share".into(),
        positive_share.round_dp(3).to_string(),
    );
    comps.insert(
        "max_drawdown_ratio".into(),
        dd_ratio.round_dp(3).to_string(),
    );
    comps.insert("recent_closes".into(), recent.to_string());

    // Unvalidated default weights (ADR 0003).
    let e = clamp01(shrunk / Decimal::new(5, 1));
    let d = Decimal::ONE - clamp01(dd_ratio / cfg.max_drawdown_ratio);
    let s = clamp01(nd / Decimal::from(50));
    let raw = Decimal::new(40, 2) * e
        + Decimal::new(25, 2) * positive_share
        + Decimal::new(20, 2) * d
        + Decimal::new(15, 2) * s;
    let value = (raw * Decimal::from(100))
        .round()
        .to_string()
        .parse::<u8>()
        .ok();

    let (decision, why) = if shrunk <= Decimal::ZERO {
        (
            Decision::Rejected,
            "After adjusting for sample size, the average closed trade did not make money.",
        )
    } else if positive_share >= cfg.min_positive_window_share
        && dd_ratio <= cfg.max_drawdown_ratio
        && recent >= cfg.min_recent_closes
    {
        (Decision::Eligible, "Consistent results across time windows with contained drawdowns and recent activity. This describes the past and does not predict future results.")
    } else {
        (Decision::WatchOnly, "Positive on average, but results are uneven across time, drawdowns were large, or the wallet has gone quiet recently.")
    };
    mk(decision, value, why.into(), comps)
}
