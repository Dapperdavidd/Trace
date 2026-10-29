//! Trading DNA: observable behavioural features from confirmed closed trades.
//! Labels come from versioned, explainable rules. Never infers off-chain
//! personality or identity. Sparse data yields `insufficient_history`.

use chrono::Duration;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
use trace_accounting::ClosedLot;

pub const RULES_VERSION: &str = "dna/1";
pub const MIN_CLOSES: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Trait {
    pub key: String,
    pub label: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DnaState {
    Ok,
    InsufficientHistory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dna {
    pub state: DnaState,
    pub rules_version: &'static str,
    pub sample_size: usize,
    pub traits: Vec<Trait>,
}

fn median_secs(mut v: Vec<i64>) -> i64 {
    v.sort_unstable();
    v[v.len() / 2]
}

fn human(secs: i64) -> String {
    let d = Duration::seconds(secs);
    if d.num_days() >= 1 {
        format!("{} days", d.num_days())
    } else if d.num_hours() >= 1 {
        format!("{} hours", d.num_hours())
    } else {
        format!("{} minutes", d.num_minutes().max(1))
    }
}

pub fn analyze(closes: &[ClosedLot]) -> Dna {
    let n = closes.len();
    if n < MIN_CLOSES {
        return Dna {
            state: DnaState::InsufficientHistory,
            rules_version: RULES_VERSION,
            sample_size: n,
            traits: vec![],
        };
    }
    let mut sorted: Vec<&ClosedLot> = closes.iter().collect();
    sorted.sort_by_key(|c| c.closed_at);
    let holds: Vec<i64> = sorted
        .iter()
        .map(|c| (c.closed_at - c.opened_at).num_seconds().max(0))
        .collect();
    let med = median_secs(holds.clone());
    let mut traits = vec![];

    let (key, label) = if med < 6 * 3600 {
        ("fast_flipper", "Fast Flipper")
    } else if med < 7 * 86_400 {
        ("swing_trader", "Swing Trader")
    } else {
        ("position_holder", "Position Holder")
    };
    traits.push(Trait {
        key: key.into(),
        label: label.into(),
        evidence: format!(
            "Median holding time is {} across {} closed trades.",
            human(med),
            n
        ),
    });

    // Asset concentration by cost.
    let mut by_asset: BTreeMap<&str, Decimal> = BTreeMap::new();
    for c in closes {
        *by_asset.entry(c.asset.as_str()).or_default() += c.cost_usd;
    }
    let total: Decimal = by_asset.values().copied().sum();
    if total > Decimal::ZERO {
        let top = by_asset.values().copied().max().unwrap_or_default();
        let share = top / total;
        let pct = (share * Decimal::from(100)).round();
        if share > Decimal::new(5, 1) {
            traits.push(Trait {
                key: "concentrated".into(),
                label: "Concentrated".into(),
                evidence: format!(
                    "{pct}% of capital deployed went into one token ({n} closed trades)."
                ),
            });
        } else {
            traits.push(Trait {
                key: "diversified".into(),
                label: "Diversified".into(),
                evidence: format!(
                    "The largest single token is {pct}% of capital deployed ({} tokens).",
                    by_asset.len()
                ),
            });
        }
    }

    // Typical position size band by median cost.
    let mut costs: Vec<Decimal> = closes.iter().map(|c| c.cost_usd).collect();
    costs.sort();
    let med_cost = costs[costs.len() / 2];
    let band = if med_cost < Decimal::from(500) {
        ("small_size", "Small positions", "under $500")
    } else if med_cost < Decimal::from(5_000) {
        ("medium_size", "Medium positions", "$500 to $5,000")
    } else {
        ("large_size", "Large positions", "over $5,000")
    };
    traits.push(Trait {
        key: band.0.into(),
        label: band.1.into(),
        evidence: format!(
            "Median position size is {} (${}).",
            band.2,
            med_cost.round()
        ),
    });

    // Style drift: median hold in the first half vs the second half.
    if n >= 2 * MIN_CLOSES {
        let half = holds.len() / 2;
        let first = median_secs(holds[..half].to_vec()).max(1);
        let second = median_secs(holds[half..].to_vec()).max(1);
        let ratio = if second > first {
            second as f64 / first as f64
        } else {
            first as f64 / second as f64
        };
        if ratio >= 2.0 {
            traits.push(Trait {
                key: "style_drift".into(),
                label: "Style drift".into(),
                evidence: format!("Median holding time changed from {} to {} between the earlier and later half of its history.", human(first), human(second)),
            });
        }
    }

    // Active window: most common UTC hour block of the day (4 hour blocks).
    let mut blocks = [0usize; 6];
    for c in closes {
        use chrono::Timelike;
        blocks[(c.opened_at.hour() / 4) as usize] += 1;
    }
    let (bi, bc) = blocks
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| **c)
        .map(|(i, c)| (i, *c))
        .unwrap();
    if bc * 100 / n >= 40 {
        traits.push(Trait {
            key: "active_window".into(),
            label: "Active window".into(),
            evidence: format!(
                "{}% of entries happen between {:02}:00 and {:02}:00 UTC.",
                bc * 100 / n,
                bi * 4,
                bi * 4 + 4
            ),
        });
    }

    Dna {
        state: DnaState::Ok,
        rules_version: RULES_VERSION,
        sample_size: n,
        traits,
    }
}
