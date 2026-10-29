//! TCQ: Trade Quality Score for one specific current trade.

use rust_decimal::Decimal;
use std::collections::BTreeMap;
use trace_risk::{RiskReport, RiskState};

use crate::score::{Decision, Score};

pub const VERSION: &str = "tcq/1";

#[derive(Debug, Clone)]
pub struct TradeInputs {
    pub risk: RiskReport,
    /// Price move since the leader's entry, in percent. `None` if unknown.
    pub move_since_entry_pct: Option<Decimal>,
    pub independent_buyers: Option<u32>,
    pub signal_age_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct TcqConfig {
    pub max_signal_age_seconds: i64,
    /// Beyond this move the entry has largely been missed.
    pub max_move_since_entry_pct: Decimal,
    pub min_independent_buyers: u32,
}

impl Default for TcqConfig {
    fn default() -> Self {
        Self {
            max_signal_age_seconds: 3_600,
            max_move_since_entry_pct: Decimal::from(25),
            min_independent_buyers: 2,
        }
    }
}

pub fn score(cfg: &TcqConfig, t: &TradeInputs, cohort: &str) -> Score {
    let mut comps: BTreeMap<String, String> = BTreeMap::new();
    comps.insert(
        "risk_overall".into(),
        format!("{:?}", t.risk.overall).to_lowercase(),
    );
    comps.insert(
        "signal_age_seconds".into(),
        t.signal_age_seconds.to_string(),
    );
    if let Some(m) = t.move_since_entry_pct {
        comps.insert("move_since_entry_pct".into(), m.round_dp(2).to_string());
    }
    if let Some(b) = t.independent_buyers {
        comps.insert("independent_buyers".into(), b.to_string());
    }
    let mk = |decision, value, explanation: &str, comps| Score {
        decision,
        value,
        version: VERSION,
        components: comps,
        cohort: cohort.into(),
        window: "current".into(),
        explanation: explanation.into(),
    };

    match t.risk.overall {
        RiskState::Block => {
            return mk(
                Decision::Rejected,
                Some(0),
                "A critical risk was found, so this trade is not actionable.",
                comps,
            )
        }
        RiskState::Unknown => {
            return mk(
                Decision::InsufficientEvidence,
                None,
                "Required risk data is missing, so this trade can only be observed.",
                comps,
            )
        }
        _ => {}
    }
    if t.signal_age_seconds > cfg.max_signal_age_seconds {
        return mk(
            Decision::WatchOnly,
            None,
            "This signal is old. The opportunity may already be gone.",
            comps,
        );
    }
    let Some(buyers) = t.independent_buyers else {
        return mk(
            Decision::InsufficientEvidence,
            None,
            "We cannot tell how many independent wallets are buying.",
            comps,
        );
    };
    let Some(mv) = t.move_since_entry_pct else {
        return mk(
            Decision::InsufficientEvidence,
            None,
            "The price move since the leader's entry is unknown.",
            comps,
        );
    };
    if mv > cfg.max_move_since_entry_pct {
        return mk(
            Decision::WatchOnly,
            Some(10),
            "The price has already moved a lot since the leader bought.",
            comps,
        );
    }
    let risk_pts: i64 = if t.risk.overall == RiskState::Pass {
        40
    } else {
        25
    };
    let buyer_pts = (buyers.min(5) as i64) * 8;
    let fresh_pts = 20 - (t.signal_age_seconds * 20 / cfg.max_signal_age_seconds.max(1)).min(20);
    let move_pts = ((cfg.max_move_since_entry_pct - mv.max(Decimal::ZERO))
        / cfg.max_move_since_entry_pct
        * Decimal::from(20))
    .round()
    .to_string()
    .parse::<i64>()
    .unwrap_or(0);
    let value = (risk_pts + buyer_pts + fresh_pts + move_pts).clamp(0, 100) as u8;
    let decision = if buyers >= cfg.min_independent_buyers {
        Decision::Eligible
    } else {
        Decision::WatchOnly
    };
    mk(decision, Some(value), "Combines risk checks, independent buyers, signal freshness and how much of the move is already gone. It is not a chance of profit.", comps)
}
