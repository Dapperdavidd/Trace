//! Signals: an append-only lifecycle, an immutable receipt frozen at
//! publication, and fixed-horizon outcome journaling that includes losers,
//! unpriced and failed observations.
//!
//! Lifecycle: detected -> evaluated -> published_as_research | rejected | watch_only
//! -> observation_due -> observed | unpriced | invalidated.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const RULES_VERSION: &str = "signals/1";
/// Predeclared outcome horizons: 1 hour, 24 hours, 7 days, 30 days.
pub const HORIZONS_SECS: [i64; 4] = [3_600, 86_400, 604_800, 2_592_000];
/// Longest delay after a horizon during which the first available price still counts.
pub const MAX_PRICE_LAG_SECS: i64 = 6 * 3_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalState {
    Detected,
    Evaluated,
    PublishedAsResearch,
    Rejected,
    WatchOnly,
    ObservationDue,
    Observed,
    Unpriced,
    Invalidated,
}

impl SignalState {
    pub fn can_become(self, next: SignalState) -> bool {
        use SignalState::*;
        matches!(
            (self, next),
            (Detected, Evaluated)
                | (Evaluated, PublishedAsResearch)
                | (Evaluated, Rejected)
                | (Evaluated, WatchOnly)
                | (PublishedAsResearch, ObservationDue)
                | (ObservationDue, Observed)
                | (ObservationDue, Unpriced)
                | (PublishedAsResearch, Invalidated)
                | (ObservationDue, Invalidated)
                | (Observed, Invalidated)
        )
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SignalError {
    #[error("illegal transition {from:?} -> {to:?}")]
    IllegalTransition { from: SignalState, to: SignalState },
    #[error("receipt does not match its sealed hash")]
    ReceiptTampered,
}

/// Everything the user and auditors need, frozen at publication. Later
/// performance derives from `exit_policy`, never from hindsight peaks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub chain: String,
    pub token: String,
    pub source_tx_ids: Vec<String>,
    pub wallet_evidence_ids: Vec<String>,
    pub block_time: DateTime<Utc>,
    pub detected_at: DateTime<Utc>,
    pub published_at: DateTime<Utc>,
    pub reference_price_usd: Decimal,
    pub reference_amount_usd: Decimal,
    pub fee_bps: u32,
    pub slippage_bps: u32,
    pub risk_verdict: String,
    pub security_findings: Vec<String>,
    pub wqs: Option<u8>,
    pub wcs: Option<u8>,
    pub tcq: Option<u8>,
    pub score_versions: Vec<String>,
    pub rules_version: String,
    pub exit_policy: String,
    pub horizons_secs: Vec<i64>,
}

impl Receipt {
    /// Canonical hash. Struct field order is fixed, so serialization is stable.
    pub fn seal(&self) -> String {
        let json = serde_json::to_vec(self).expect("receipt serializes");
        hex::encode(Sha256::digest(json))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalEvent {
    pub state: SignalState,
    pub at: DateTime<Utc>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutcomeKind {
    Observed {
        return_bps: i64,
        exit_price_usd: Decimal,
    },
    /// No price observation was available for the horizon.
    Unpriced,
    /// A price existed but liquidity or risk made the exit untradeable.
    Untradeable,
    DataUnavailable,
    Expired,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub horizon_secs: i64,
    pub recorded_at: DateTime<Utc>,
    pub kind: OutcomeKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub id: String,
    receipt: Receipt,
    receipt_hash: String,
    events: Vec<SignalEvent>,
    outcomes: Vec<Outcome>,
}

impl Signal {
    /// Create a signal in `Detected` with its receipt frozen and hashed.
    pub fn new(id: impl Into<String>, receipt: Receipt, now: DateTime<Utc>) -> Self {
        let receipt_hash = receipt.seal();
        Self {
            id: id.into(),
            receipt,
            receipt_hash,
            events: vec![SignalEvent {
                state: SignalState::Detected,
                at: now,
                note: "detected".into(),
            }],
            outcomes: vec![],
        }
    }

    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
    pub fn receipt_hash(&self) -> &str {
        &self.receipt_hash
    }
    pub fn events(&self) -> &[SignalEvent] {
        &self.events
    }
    pub fn outcomes(&self) -> &[Outcome] {
        &self.outcomes
    }
    pub fn state(&self) -> SignalState {
        self.events.last().expect("always has Detected").state
    }

    /// Append-only transition. History is never edited or removed.
    pub fn advance(
        &mut self,
        next: SignalState,
        at: DateTime<Utc>,
        note: impl Into<String>,
    ) -> Result<(), SignalError> {
        let from = self.state();
        if !from.can_become(next) {
            return Err(SignalError::IllegalTransition { from, to: next });
        }
        self.events.push(SignalEvent {
            state: next,
            at,
            note: note.into(),
        });
        Ok(())
    }

    /// Verify the frozen receipt still matches its hash.
    pub fn verify(&self) -> Result<(), SignalError> {
        if self.receipt.seal() == self.receipt_hash {
            Ok(())
        } else {
            Err(SignalError::ReceiptTampered)
        }
    }

    /// Horizons that have elapsed and have no recorded outcome yet.
    pub fn due_horizons(&self, now: DateTime<Utc>) -> Vec<i64> {
        self.receipt
            .horizons_secs
            .iter()
            .copied()
            .filter(|h| now >= self.receipt.published_at + Duration::seconds(*h))
            .filter(|h| !self.outcomes.iter().any(|o| o.horizon_secs == *h))
            .collect()
    }

    /// Record one outcome per horizon. Re-recording a horizon is rejected so
    /// a bad result cannot be replaced by a better one later.
    pub fn record_outcome(&mut self, outcome: Outcome) -> bool {
        if self
            .outcomes
            .iter()
            .any(|o| o.horizon_secs == outcome.horizon_secs)
        {
            return false;
        }
        self.outcomes.push(outcome);
        true
    }
}

/// A price observation at a point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct PricePoint {
    pub at: DateTime<Utc>,
    pub price_usd: Decimal,
    pub exit_liquid: bool,
}

/// Evaluate one horizon using the predeclared exit: the FIRST available price
/// at or after `published_at + horizon`, within the allowed lag. Never a later
/// favourable peak.
pub fn evaluate_horizon(
    receipt: &Receipt,
    horizon_secs: i64,
    prices: &[PricePoint],
    now: DateTime<Utc>,
) -> Outcome {
    let target = receipt.published_at + Duration::seconds(horizon_secs);
    let deadline = target + Duration::seconds(MAX_PRICE_LAG_SECS);
    let first = prices
        .iter()
        .filter(|p| p.at >= target)
        .min_by_key(|p| p.at);
    let kind = match first {
        Some(p) if p.at <= deadline => {
            if !p.exit_liquid {
                OutcomeKind::Untradeable
            } else if receipt.reference_price_usd <= Decimal::ZERO {
                OutcomeKind::Error {
                    message: "reference price is not positive".into(),
                }
            } else {
                let ret = (p.price_usd - receipt.reference_price_usd) / receipt.reference_price_usd
                    * Decimal::from(10_000);
                let cost = Decimal::from(2 * receipt.fee_bps as i64 + receipt.slippage_bps as i64);
                let net = (ret - cost).round().to_string().parse::<i64>().unwrap_or(0);
                OutcomeKind::Observed {
                    return_bps: net,
                    exit_price_usd: p.price_usd,
                }
            }
        }
        Some(_) => OutcomeKind::Expired,
        None if now > deadline => OutcomeKind::Unpriced,
        None => OutcomeKind::DataUnavailable,
    };
    Outcome {
        horizon_secs,
        recorded_at: now,
        kind,
    }
}

/// Counts that include every eligible signal. Losers, unpriced and failures
/// are reported, never dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OutcomeSummary {
    pub total: usize,
    pub gains: usize,
    pub losses: usize,
    pub flat: usize,
    pub unpriced: usize,
    pub untradeable: usize,
    pub data_unavailable: usize,
    pub expired: usize,
    pub errors: usize,
}

pub fn summarize<'a>(outcomes: impl IntoIterator<Item = &'a Outcome>) -> OutcomeSummary {
    let mut s = OutcomeSummary::default();
    for o in outcomes {
        s.total += 1;
        match &o.kind {
            OutcomeKind::Observed { return_bps, .. } if *return_bps > 0 => s.gains += 1,
            OutcomeKind::Observed { return_bps, .. } if *return_bps < 0 => s.losses += 1,
            OutcomeKind::Observed { .. } => s.flat += 1,
            OutcomeKind::Unpriced => s.unpriced += 1,
            OutcomeKind::Untradeable => s.untradeable += 1,
            OutcomeKind::DataUnavailable => s.data_unavailable += 1,
            OutcomeKind::Expired => s.expired += 1,
            OutcomeKind::Error { .. } => s.errors += 1,
        }
    }
    s
}
