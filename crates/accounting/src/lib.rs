//! Wallet position accounting with explicit uncertainty.
//!
//! Method: FIFO lots per asset (documented in ADR 0002). Every swap is both a
//! disposal of `token_in` and an acquisition of `token_out`, valued in USD by a
//! price source at the swap time.
//!
//! Honesty rules enforced here and covered by tests:
//! * A disposal with no matching lot (inventory that arrived by transfer, or
//!   history we did not see) has *unresolved basis*; it never produces profit.
//! * A swap that cannot be priced is *unpriced*; its lots carry unknown basis
//!   and later disposals of them are unresolved.
//! * `realized_pnl_usd` is `None` when nothing could be resolved, never `0`.
//! * Reorged swaps are ignored. Tentative swaps are flagged on the result.
//! * `pnl_confidence` is a rule-based quality label from coverage and sample
//!   size. It is not a probability of profit.

use chrono::{DateTime, Utc};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use trace_domain::{AssetId, Attribution, BaseUnits, Finality, NormalizedSwap};

/// USD value of a quantity at a time, with where the number came from.
#[derive(Debug, Clone, PartialEq)]
pub struct PricedValue {
    pub usd: Decimal,
    pub source: String,
    /// True when the price comes from a quote asset (stablecoin or similar)
    /// whose value is not itself being discovered by this trade. Preferred
    /// over a market price of the traded token when valuing a swap.
    pub is_quote: bool,
}

pub trait PriceSource {
    fn usd_value(
        &self,
        asset: &AssetId,
        amount: BaseUnits,
        at: DateTime<Utc>,
    ) -> Option<PricedValue>;

    /// Stable quote assets carry no trading PnL. Disposals and acquisitions of
    /// them are neutral: they never create lots, closes or unresolved basis.
    fn is_stable(&self, _asset: &AssetId) -> bool {
        false
    }
}

/// Mark-to-market price for unrealized PnL, with its observation time.
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    pub usd_per_base_unit: Decimal,
    pub observed_at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PnlConfidence {
    High,
    Medium,
    Low,
    /// Too few reconstructable trades to say anything.
    Insufficient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Anomaly {
    UnresolvedBasis {
        tx_id: String,
        asset: String,
        unmatched_base_units: String,
    },
    Unpriced {
        tx_id: String,
    },
    /// Both sides of the swap were priced and disagree by more than 5%.
    PriceDivergence {
        tx_id: String,
    },
    AmountOutOfDecimalRange {
        tx_id: String,
    },
    /// Swap excluded because attribution is not Direct.
    NotCreditedToWallet {
        tx_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClosedLot {
    pub asset: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
    pub base_units: String,
    pub cost_usd: Decimal,
    pub proceeds_usd: Decimal,
    pub pnl_usd: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpenPosition {
    pub asset: String,
    pub base_units: String,
    /// Cost of the priced part of the position. `None` if any lot has unknown basis.
    pub cost_usd: Option<Decimal>,
    pub unrealized_pnl_usd: Option<Decimal>,
    pub mark_observed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WalletAccounting {
    pub total_swaps: usize,
    pub credited_swaps: usize,
    pub reconstructable_swaps: usize,
    pub unpriced_swaps: usize,
    pub unresolved_disposals: usize,
    pub realized_pnl_usd: Option<Decimal>,
    pub unrealized_pnl_usd: Option<Decimal>,
    pub winning_closes: usize,
    pub losing_closes: usize,
    /// `reconstructable_swaps / credited_swaps`, `None` with no credited swaps.
    pub coverage_ratio: Option<Decimal>,
    pub pnl_confidence: PnlConfidence,
    pub has_tentative: bool,
    pub closed_lots: Vec<ClosedLot>,
    pub open_positions: Vec<OpenPosition>,
    pub anomalies: Vec<Anomaly>,
}

/// Documented thresholds for the confidence label (see ADR 0002).
pub const MIN_RECONSTRUCTABLE_FOR_ANY_CONFIDENCE: usize = 5;
pub const HIGH_COVERAGE: i64 = 90; // percent
pub const MEDIUM_COVERAGE: i64 = 70; // percent

#[derive(Debug, Clone)]
struct Lot {
    opened_at: DateTime<Utc>,
    remaining: u128,
    original: u128,
    /// Cost of the original quantity. `None` means basis unknown.
    cost_usd: Option<Decimal>,
}

fn dec(v: u128) -> Option<Decimal> {
    Decimal::from_u128(v)
}

/// Compute accounting for one wallet from its swaps (any order).
pub fn compute(
    swaps: &[NormalizedSwap],
    prices: &dyn PriceSource,
    marks: &BTreeMap<AssetId, Mark>,
) -> WalletAccounting {
    let mut ordered: Vec<&NormalizedSwap> = swaps
        .iter()
        .filter(|s| s.finality != Finality::Reorged)
        .collect();
    ordered.sort_by(|a, b| {
        (a.observed_at, &a.tx_id, a.event_index).cmp(&(b.observed_at, &b.tx_id, b.event_index))
    });

    let mut lots: BTreeMap<AssetId, VecDeque<Lot>> = BTreeMap::new();
    let mut out = WalletAccounting {
        total_swaps: ordered.len(),
        credited_swaps: 0,
        reconstructable_swaps: 0,
        unpriced_swaps: 0,
        unresolved_disposals: 0,
        realized_pnl_usd: None,
        unrealized_pnl_usd: None,
        winning_closes: 0,
        losing_closes: 0,
        coverage_ratio: None,
        pnl_confidence: PnlConfidence::Insufficient,
        has_tentative: ordered.iter().any(|s| s.finality == Finality::Tentative),
        closed_lots: vec![],
        open_positions: vec![],
        anomalies: vec![],
    };

    for s in &ordered {
        if s.attribution != Attribution::Direct {
            out.anomalies.push(Anomaly::NotCreditedToWallet {
                tx_id: s.tx_id.clone(),
            });
            continue;
        }
        out.credited_swaps += 1;
        let mut reconstructable = true;

        // Value the swap. Prefer a quote-asset side; otherwise the paid side.
        let pin = prices.usd_value(&s.token_in, s.amount_in, s.observed_at);
        let pout = prices.usd_value(&s.token_out, s.amount_out, s.observed_at);
        if let (Some(i), Some(o)) = (&pin, &pout) {
            let hi = i.usd.max(o.usd);
            if hi > Decimal::ZERO && (i.usd - o.usd).abs() / hi > Decimal::new(5, 2) {
                out.anomalies.push(Anomaly::PriceDivergence {
                    tx_id: s.tx_id.clone(),
                });
            }
        }
        let value = match (pin, pout) {
            (Some(i), Some(o)) => Some(if o.is_quote && !i.is_quote { o } else { i }),
            (Some(i), None) => Some(i),
            (None, Some(o)) => Some(o),
            (None, None) => None,
        };
        let value_usd = value.map(|v| v.usd);
        let in_stable = prices.is_stable(&s.token_in);
        let out_stable = prices.is_stable(&s.token_out);
        if value_usd.is_none() {
            out.unpriced_swaps += 1;
            out.anomalies.push(Anomaly::Unpriced {
                tx_id: s.tx_id.clone(),
            });
            reconstructable = false;
        }

        // Disposal of token_in against FIFO lots.
        let queue = lots.entry(s.token_in.clone()).or_default();
        let mut to_dispose = if in_stable { 0 } else { s.amount_in.0 };
        let mut unmatched: u128 = 0;
        let total_disposed = s.amount_in.0;
        let mut disposal_unresolved = false;
        while to_dispose > 0 {
            let Some(front) = queue.front_mut() else {
                unmatched = to_dispose;
                break;
            };
            let take = front.remaining.min(to_dispose);
            let (Some(take_d), Some(orig_d), Some(total_d)) =
                (dec(take), dec(front.original), dec(total_disposed))
            else {
                out.anomalies.push(Anomaly::AmountOutOfDecimalRange {
                    tx_id: s.tx_id.clone(),
                });
                disposal_unresolved = true;
                front.remaining -= take;
                to_dispose -= take;
                if front.remaining == 0 {
                    queue.pop_front();
                }
                continue;
            };
            match (front.cost_usd, value_usd) {
                (Some(cost_total), Some(v)) => {
                    let cost = cost_total * take_d / orig_d;
                    let proceeds = v * take_d / total_d;
                    let pnl = proceeds - cost;
                    out.realized_pnl_usd =
                        Some(out.realized_pnl_usd.unwrap_or(Decimal::ZERO) + pnl);
                    if pnl > Decimal::ZERO {
                        out.winning_closes += 1;
                    } else if pnl < Decimal::ZERO {
                        out.losing_closes += 1;
                    }
                    out.closed_lots.push(ClosedLot {
                        asset: s.token_in.contract_or_mint.to_string(),
                        opened_at: front.opened_at,
                        closed_at: s.observed_at,
                        base_units: take.to_string(),
                        cost_usd: cost,
                        proceeds_usd: proceeds,
                        pnl_usd: pnl,
                    });
                }
                _ => disposal_unresolved = true,
            }
            front.remaining -= take;
            to_dispose -= take;
            if front.remaining == 0 {
                queue.pop_front();
            }
        }
        if unmatched > 0 {
            disposal_unresolved = true;
            out.anomalies.push(Anomaly::UnresolvedBasis {
                tx_id: s.tx_id.clone(),
                asset: s.token_in.contract_or_mint.to_string(),
                unmatched_base_units: unmatched.to_string(),
            });
        }
        if disposal_unresolved {
            out.unresolved_disposals += 1;
            reconstructable = false;
        }

        // Acquisition of token_out. Unknown value means unknown basis.
        if !out_stable {
            lots.entry(s.token_out.clone()).or_default().push_back(Lot {
                opened_at: s.observed_at,
                remaining: s.amount_out.0,
                original: s.amount_out.0,
                cost_usd: value_usd,
            });
        }

        if reconstructable {
            out.reconstructable_swaps += 1;
        }
    }

    // Open positions and unrealized PnL.
    let mut unrealized: Option<Decimal> = None;
    for (asset, queue) in &lots {
        if queue.is_empty() {
            continue;
        }
        let remaining: u128 = queue.iter().map(|l| l.remaining).sum();
        let mut cost = Some(Decimal::ZERO);
        for l in queue {
            let part = match (l.cost_usd, dec(l.remaining), dec(l.original)) {
                (Some(c), Some(r), Some(o)) => Some(c * r / o),
                _ => None,
            };
            cost = match (cost, part) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            };
        }
        let mark = marks.get(asset);
        let upnl = match (cost, mark, dec(remaining)) {
            (Some(c), Some(m), Some(q)) => Some(q * m.usd_per_base_unit - c),
            _ => None,
        };
        if let Some(u) = upnl {
            unrealized = Some(unrealized.unwrap_or(Decimal::ZERO) + u);
        }
        out.open_positions.push(OpenPosition {
            asset: asset.contract_or_mint.to_string(),
            base_units: remaining.to_string(),
            cost_usd: cost,
            unrealized_pnl_usd: upnl,
            mark_observed_at: mark.map(|m| m.observed_at),
        });
    }
    out.unrealized_pnl_usd = unrealized;

    if out.credited_swaps > 0 {
        out.coverage_ratio = Some(
            Decimal::from(out.reconstructable_swaps as u64)
                / Decimal::from(out.credited_swaps as u64),
        );
    }
    out.pnl_confidence = confidence(&out);
    out
}

fn confidence(a: &WalletAccounting) -> PnlConfidence {
    let Some(cov) = a.coverage_ratio else {
        return PnlConfidence::Insufficient;
    };
    if a.reconstructable_swaps < MIN_RECONSTRUCTABLE_FOR_ANY_CONFIDENCE {
        return PnlConfidence::Insufficient;
    }
    let pct = cov * Decimal::from(100);
    if pct >= Decimal::from(HIGH_COVERAGE) && !a.has_tentative {
        PnlConfidence::High
    } else if pct >= Decimal::from(MEDIUM_COVERAGE) {
        PnlConfidence::Medium
    } else {
        PnlConfidence::Low
    }
}
