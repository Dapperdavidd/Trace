use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use trace_signals::*;

fn t(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000 + secs, 0).unwrap()
}

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn receipt() -> Receipt {
    Receipt {
        chain: "ethereum".into(),
        token: "0x00000000000000000000000000000000000000aa".into(),
        source_tx_ids: vec!["0xabc".into()],
        wallet_evidence_ids: vec!["w1".into()],
        block_time: t(0),
        detected_at: t(5),
        published_at: t(10),
        reference_price_usd: d("1.00"),
        reference_amount_usd: d("500"),
        fee_bps: 30,
        slippage_bps: 50,
        risk_verdict: "pass".into(),
        security_findings: vec![],
        wqs: Some(72),
        wcs: Some(65),
        tcq: Some(80),
        score_versions: vec!["wqs/1".into(), "wcs/1".into(), "tcq/1".into()],
        rules_version: RULES_VERSION.into(),
        exit_policy: "fixed_horizon_first_available_price".into(),
        horizons_secs: HORIZONS_SECS.to_vec(),
    }
}

fn published() -> Signal {
    let mut s = Signal::new("sig1", receipt(), t(5));
    s.advance(SignalState::Evaluated, t(6), "scored").unwrap();
    s.advance(SignalState::PublishedAsResearch, t(10), "published")
        .unwrap();
    s
}

#[test]
fn lifecycle_is_append_only_and_rejects_illegal_jumps() {
    let mut s = Signal::new("s", receipt(), t(0));
    assert_eq!(
        s.advance(SignalState::PublishedAsResearch, t(1), "skip"),
        Err(SignalError::IllegalTransition {
            from: SignalState::Detected,
            to: SignalState::PublishedAsResearch
        })
    );
    s.advance(SignalState::Evaluated, t(1), "ok").unwrap();
    s.advance(SignalState::Rejected, t(2), "unsafe").unwrap();
    assert!(
        s.advance(SignalState::PublishedAsResearch, t(3), "undo")
            .is_err(),
        "a rejected signal cannot be revived"
    );
    assert_eq!(s.events().len(), 3);
}

#[test]
fn receipt_is_sealed_and_tampering_is_detected() {
    let s = published();
    assert!(s.verify().is_ok());
    assert_eq!(s.receipt_hash().len(), 64);
    let mut forged = s.clone();
    // Simulate a corrupted store: swap in a flattering receipt but keep the old hash.
    let json = serde_json::to_string(&forged)
        .unwrap()
        .replace("\"wqs\":72", "\"wqs\":99");
    forged = serde_json::from_str(&json).unwrap();
    assert_eq!(forged.verify(), Err(SignalError::ReceiptTampered));
}

#[test]
fn identical_receipts_hash_identically() {
    assert_eq!(receipt().seal(), receipt().seal());
    let mut r = receipt();
    r.fee_bps = 31;
    assert_ne!(receipt().seal(), r.seal());
}

#[test]
fn horizons_become_due_only_after_they_elapse() {
    let s = published();
    assert!(s.due_horizons(t(10 + 3_599)).is_empty());
    assert_eq!(s.due_horizons(t(10 + 3_600)), vec![3_600]);
    assert_eq!(s.due_horizons(t(10 + 86_400)), vec![3_600, 86_400]);
}

#[test]
fn outcome_uses_first_available_price_not_a_later_peak() {
    let s = published();
    let target = t(10 + 3_600);
    let prices = vec![
        PricePoint {
            at: target + Duration::seconds(30),
            price_usd: d("0.90"),
            exit_liquid: true,
        },
        PricePoint {
            at: target + Duration::hours(2),
            price_usd: d("3.00"),
            exit_liquid: true,
        }, // later peak must be ignored
    ];
    let o = evaluate_horizon(s.receipt(), 3_600, &prices, t(10 + 3_600 + 7_200));
    // return = -10% = -1000 bps, minus costs (2*30 + 50) = -1110
    assert_eq!(
        o.kind,
        OutcomeKind::Observed {
            return_bps: -1_110,
            exit_price_usd: d("0.90")
        }
    );
}

#[test]
fn missing_stale_or_illiquid_prices_are_recorded_not_dropped() {
    let s = published();
    let target = t(10 + 3_600);
    // not available yet
    assert_eq!(
        evaluate_horizon(s.receipt(), 3_600, &[], target + Duration::minutes(5)).kind,
        OutcomeKind::DataUnavailable
    );
    // never became available
    assert_eq!(
        evaluate_horizon(s.receipt(), 3_600, &[], target + Duration::hours(12)).kind,
        OutcomeKind::Unpriced
    );
    // only a price long after the allowed lag
    let late = vec![PricePoint {
        at: target + Duration::hours(10),
        price_usd: d("2"),
        exit_liquid: true,
    }];
    assert_eq!(
        evaluate_horizon(s.receipt(), 3_600, &late, target + Duration::hours(11)).kind,
        OutcomeKind::Expired
    );
    // price exists but exit was not tradeable
    let illiquid = vec![PricePoint {
        at: target,
        price_usd: d("2"),
        exit_liquid: false,
    }];
    assert_eq!(
        evaluate_horizon(s.receipt(), 3_600, &illiquid, target + Duration::hours(1)).kind,
        OutcomeKind::Untradeable
    );
}

#[test]
fn a_recorded_outcome_cannot_be_replaced_by_a_better_one() {
    let mut s = published();
    let bad = Outcome {
        horizon_secs: 3_600,
        recorded_at: t(4_000),
        kind: OutcomeKind::Observed {
            return_bps: -2_000,
            exit_price_usd: d("0.8"),
        },
    };
    let good = Outcome {
        horizon_secs: 3_600,
        recorded_at: t(5_000),
        kind: OutcomeKind::Observed {
            return_bps: 5_000,
            exit_price_usd: d("1.5"),
        },
    };
    assert!(s.record_outcome(bad.clone()));
    assert!(!s.record_outcome(good));
    assert_eq!(s.outcomes(), &[bad]);
    assert!(s.due_horizons(t(10 + 3_700)).is_empty());
}

#[test]
fn summary_counts_losers_and_failures_alongside_winners() {
    let s = published();
    let outs = [
        Outcome {
            horizon_secs: 1,
            recorded_at: t(0),
            kind: OutcomeKind::Observed {
                return_bps: 300,
                exit_price_usd: d("1"),
            },
        },
        Outcome {
            horizon_secs: 2,
            recorded_at: t(0),
            kind: OutcomeKind::Observed {
                return_bps: -900,
                exit_price_usd: d("1"),
            },
        },
        Outcome {
            horizon_secs: 3,
            recorded_at: t(0),
            kind: OutcomeKind::Unpriced,
        },
        Outcome {
            horizon_secs: 4,
            recorded_at: t(0),
            kind: OutcomeKind::Untradeable,
        },
        Outcome {
            horizon_secs: 5,
            recorded_at: t(0),
            kind: OutcomeKind::Error {
                message: "x".into(),
            },
        },
    ];
    let sum = summarize(outs.iter());
    assert_eq!(
        (
            sum.total,
            sum.gains,
            sum.losses,
            sum.unpriced,
            sum.untradeable,
            sum.errors
        ),
        (5, 1, 1, 1, 1, 1)
    );
    let _ = s;
}

#[test]
fn invalidated_is_reachable_from_published_states() {
    let mut s = published();
    s.advance(SignalState::Invalidated, t(100), "risk changed")
        .unwrap();
    assert_eq!(s.state(), SignalState::Invalidated);
}
