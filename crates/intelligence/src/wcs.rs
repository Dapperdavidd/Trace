//! WCS: Wallet Copyability Score. Could a follower with a defined order size
//! and detection delay still get a viable trade at their own price?
//! Stored together with its reference_amount, delay_assumption and quote_time.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

use crate::score::{Decision, Score};

pub const VERSION: &str = "wcs/1";

#[derive(Debug, Clone)]
pub struct FollowerQuote {
    pub reference_amount_usd: Decimal,
    pub delay_seconds: u32,
    pub quote_time: DateTime<Utc>,
    /// How much worse the follower's entry is than the leader's, in bps.
    pub entry_deterioration_bps: i64,
    pub price_impact_bps: u32,
    pub fee_bps: u32,
    pub pool_liquidity_usd: Decimal,
}

#[derive(Debug, Clone)]
pub struct WcsConfig {
    /// Pool liquidity must be at least this multiple of the follower amount.
    pub min_liquidity_multiple: Decimal,
    /// Leader's typical edge per trade in bps is compared to follower costs.
    pub max_quote_age_seconds: i64,
}

impl Default for WcsConfig {
    fn default() -> Self {
        Self {
            min_liquidity_multiple: Decimal::from(50),
            max_quote_age_seconds: 300,
        }
    }
}

/// `leader_edge_bps`: the leader's typical net edge per closed trade in bps
/// (from WQS inputs). `None` means unknown.
pub fn score(
    cfg: &WcsConfig,
    leader_edge_bps: Option<i64>,
    quote: Option<&FollowerQuote>,
    cohort: &str,
    now: DateTime<Utc>,
) -> Score {
    let mut comps: BTreeMap<String, String> = BTreeMap::new();
    let mk =
        |decision, value, explanation: &str, comps: BTreeMap<String, String>, window: String| {
            Score {
                decision,
                value,
                version: VERSION,
                components: comps,
                cohort: cohort.to_string(),
                window,
                explanation: explanation.to_string(),
            }
        };

    let Some(q) = quote else {
        return mk(Decision::WatchOnly, None, "No usable follower quote is available, so this wallet can only be observed, not simulated as a copy.", comps, "n/a".into());
    };
    comps.insert(
        "reference_amount_usd".into(),
        q.reference_amount_usd.to_string(),
    );
    comps.insert(
        "delay_assumption_seconds".into(),
        q.delay_seconds.to_string(),
    );
    comps.insert("quote_time".into(), q.quote_time.to_rfc3339());
    comps.insert(
        "entry_deterioration_bps".into(),
        q.entry_deterioration_bps.to_string(),
    );
    comps.insert("price_impact_bps".into(), q.price_impact_bps.to_string());
    comps.insert("fee_bps".into(), q.fee_bps.to_string());
    let window = format!("delay {}s at ${}", q.delay_seconds, q.reference_amount_usd);

    if (now - q.quote_time).num_seconds() > cfg.max_quote_age_seconds {
        return mk(
            Decision::WatchOnly,
            None,
            "The follower quote is too old to rely on, so copyability is unknown right now.",
            comps,
            window,
        );
    }
    let Some(edge) = leader_edge_bps else {
        return mk(
            Decision::InsufficientEvidence,
            None,
            "The leader's typical edge is unknown, so follower economics cannot be judged.",
            comps,
            window,
        );
    };
    let costs = q.entry_deterioration_bps.max(0) + q.price_impact_bps as i64 + 2 * q.fee_bps as i64;
    let net = edge - costs;
    comps.insert("leader_edge_bps".into(), edge.to_string());
    comps.insert("follower_cost_bps".into(), costs.to_string());
    comps.insert("net_edge_bps".into(), net.to_string());

    if q.pool_liquidity_usd < q.reference_amount_usd * cfg.min_liquidity_multiple {
        return mk(
            Decision::WatchOnly,
            None,
            "The pool is too small for this order size, so a follower could not trade it cleanly.",
            comps,
            window,
        );
    }
    if net <= 0 {
        return mk(
            Decision::Rejected,
            Some(0),
            "After the delay, price impact and fees, a follower would lose the leader's edge.",
            comps,
            window,
        );
    }
    // Value: net edge relative to the leader's edge (how much survives), 0..100.
    let kept = (Decimal::from(net) / Decimal::from(edge.max(1)))
        .max(Decimal::ZERO)
        .min(Decimal::ONE);
    let value = (kept * Decimal::from(100))
        .round()
        .to_string()
        .parse::<u8>()
        .ok();
    let decision = if kept >= Decimal::new(5, 1) {
        Decision::Eligible
    } else {
        Decision::WatchOnly
    };
    mk(decision, value, "Share of the leader's typical edge that would survive the follower's delay, price impact and fees. Higher is better. It is not a chance of profit.", comps, window)
}
