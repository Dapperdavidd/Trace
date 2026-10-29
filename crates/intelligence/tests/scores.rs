use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use trace_accounting::ClosedLot;
use trace_intelligence::{dna, tcq, wcs, wqs, Decision};
use trace_risk::{assess, RiskInputs, RiskPolicy};

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn t0() -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

fn close(day: i64, hold_hours: i64, cost: &str, pnl: &str, asset: &str) -> ClosedLot {
    let opened = t0() + Duration::days(day);
    let closed = opened + Duration::hours(hold_hours);
    let cost = d(cost);
    let pnl = d(pnl);
    ClosedLot {
        asset: asset.into(),
        opened_at: opened,
        closed_at: closed,
        base_units: "1".into(),
        cost_usd: cost,
        proceeds_usd: cost + pnl,
        pnl_usd: pnl,
    }
}

fn steady_winner(n: i64) -> Vec<ClosedLot> {
    (0..n)
        .map(|i| {
            close(
                i * 3,
                10,
                "1000",
                if i % 4 == 3 { "-50" } else { "120" },
                &format!("tok{}", i % 5),
            )
        })
        .collect()
}

fn now() -> DateTime<Utc> {
    t0() + Duration::days(40)
}

// ----- WQS -----

#[test]
fn one_big_win_never_produces_a_high_score() {
    let one = vec![close(0, 5, "100", "10000", "tok")];
    let s = wqs::score(
        &wqs::WqsConfig::default(),
        &one,
        Some(Decimal::ONE),
        "eth/large",
        now(),
    );
    assert_eq!(s.decision, Decision::InsufficientEvidence);
    assert_eq!(s.value, None);
}

#[test]
fn low_coverage_or_unknown_coverage_abstains() {
    let c = steady_winner(20);
    let low = wqs::score(&wqs::WqsConfig::default(), &c, Some(d("0.2")), "x", now());
    assert_eq!(low.decision, Decision::InsufficientEvidence);
    let unknown = wqs::score(&wqs::WqsConfig::default(), &c, None, "x", now());
    assert_eq!(unknown.decision, Decision::InsufficientEvidence);
}

#[test]
fn steady_winner_is_eligible_with_components_and_cohort() {
    let c = steady_winner(24);
    let s = wqs::score(
        &wqs::WqsConfig::default(),
        &c,
        Some(Decimal::ONE),
        "ethereum/medium/swing",
        t0() + Duration::days(72),
    );
    assert_eq!(s.decision, Decision::Eligible, "{:?}", s.components);
    assert!(s.value.unwrap() > 40);
    assert_eq!(s.cohort, "ethereum/medium/swing");
    assert!(s.components.contains_key("shrunk_expectancy"));
    assert_eq!(s.version, "wqs/1");
}

#[test]
fn consistent_loser_is_rejected() {
    let c: Vec<_> = (0..20)
        .map(|i| {
            close(
                i * 3,
                10,
                "1000",
                if i % 5 == 0 { "40" } else { "-80" },
                "tok",
            )
        })
        .collect();
    let s = wqs::score(
        &wqs::WqsConfig::default(),
        &c,
        Some(Decimal::ONE),
        "x",
        now(),
    );
    assert_eq!(s.decision, Decision::Rejected);
}

#[test]
fn outlier_is_capped_so_it_cannot_carry_a_mediocre_wallet() {
    let mut c: Vec<_> = (0..14)
        .map(|i| close(i * 3, 10, "1000", "-40", "tok"))
        .collect();
    c.push(close(50, 10, "10", "100000", "moon"));
    let s = wqs::score(
        &wqs::WqsConfig::default(),
        &c,
        Some(Decimal::ONE),
        "x",
        t0() + Duration::days(55),
    );
    assert_ne!(s.decision, Decision::Eligible);
}

#[test]
fn quiet_wallet_downgrades_to_watch_only() {
    let c = steady_winner(24);
    let s = wqs::score(
        &wqs::WqsConfig::default(),
        &c,
        Some(Decimal::ONE),
        "x",
        t0() + Duration::days(400),
    );
    assert_eq!(s.decision, Decision::WatchOnly);
}

#[test]
fn scoring_is_deterministic() {
    let c = steady_winner(24);
    let a = wqs::score(
        &wqs::WqsConfig::default(),
        &c,
        Some(Decimal::ONE),
        "x",
        now(),
    );
    let b = wqs::score(
        &wqs::WqsConfig::default(),
        &c,
        Some(Decimal::ONE),
        "x",
        now(),
    );
    assert_eq!(a, b);
}

// ----- WCS -----

fn quote(amount: i64, delay: u32, deterioration: i64, impact: u32, liq: i64) -> wcs::FollowerQuote {
    wcs::FollowerQuote {
        reference_amount_usd: Decimal::from(amount),
        delay_seconds: delay,
        quote_time: now(),
        entry_deterioration_bps: deterioration,
        price_impact_bps: impact,
        fee_bps: 30,
        pool_liquidity_usd: Decimal::from(liq),
    }
}

#[test]
fn wcs_changes_with_amount_and_delay_and_stores_assumptions() {
    let cfg = wcs::WcsConfig::default();
    let small_fast = wcs::score(
        &cfg,
        Some(1_500),
        Some(&quote(500, 5, 100, 20, 1_000_000)),
        "x",
        now(),
    );
    let big_slow = wcs::score(
        &cfg,
        Some(1_500),
        Some(&quote(500_000, 120, 900, 700, 1_000_000)),
        "x",
        now(),
    );
    assert_eq!(small_fast.decision, Decision::Eligible);
    assert!(small_fast.value.unwrap() > big_slow.value.unwrap_or(0));
    assert_eq!(small_fast.components["reference_amount_usd"], "500");
    assert_eq!(small_fast.components["delay_assumption_seconds"], "5");
    assert!(small_fast.components.contains_key("quote_time"));
}

#[test]
fn high_wallet_quality_does_not_imply_copyability() {
    // Leader edge is large but follower costs eat all of it.
    let s = wcs::score(
        &wcs::WcsConfig::default(),
        Some(400),
        Some(&quote(500, 60, 600, 100, 1_000_000)),
        "x",
        now(),
    );
    assert_eq!(s.decision, Decision::Rejected);
}

#[test]
fn missing_or_stale_quote_forces_observation_only() {
    let cfg = wcs::WcsConfig::default();
    assert_eq!(
        wcs::score(&cfg, Some(1_000), None, "x", now()).decision,
        Decision::WatchOnly
    );
    let mut q = quote(500, 5, 0, 10, 1_000_000);
    q.quote_time = now() - Duration::hours(2);
    assert_eq!(
        wcs::score(&cfg, Some(1_000), Some(&q), "x", now()).decision,
        Decision::WatchOnly
    );
}

#[test]
fn a_pool_too_small_for_the_order_is_watch_only() {
    let s = wcs::score(
        &wcs::WcsConfig::default(),
        Some(1_500),
        Some(&quote(50_000, 5, 0, 10, 100_000)),
        "x",
        now(),
    );
    assert_eq!(s.decision, Decision::WatchOnly);
}

// ----- TCQ -----

fn risk(i: RiskInputs) -> trace_risk::RiskReport {
    assess(&RiskPolicy::default(), &i, now())
}

fn good_inputs() -> RiskInputs {
    RiskInputs {
        honeypot: Some(false),
        sell_tax_bps: Some(0),
        mint_authority_active: Some(false),
        freeze_authority_active: Some(false),
        blacklist_function: Some(false),
        liquidity_usd: Some(Decimal::from(500_000)),
        price_impact_bps: Some(40),
        reference_size_usd: Some(Decimal::from(1_000)),
        top10_holder_pct: Some(Decimal::from(25)),
        pool_age_hours: Some(100),
        quote_age_seconds: Some(10),
        abnormal_activity: Some(false),
        vendor_says_safe: None,
    }
}

#[test]
fn risky_or_missing_data_trades_are_never_actionable() {
    let mut bad = good_inputs();
    bad.honeypot = Some(true);
    let r = tcq::score(
        &tcq::TcqConfig::default(),
        &tcq::TradeInputs {
            risk: risk(bad),
            move_since_entry_pct: Some(d("1")),
            independent_buyers: Some(5),
            signal_age_seconds: 10,
        },
        "x",
    );
    assert_eq!(r.decision, Decision::Rejected);
    let u = tcq::score(
        &tcq::TcqConfig::default(),
        &tcq::TradeInputs {
            risk: risk(RiskInputs::default()),
            move_since_entry_pct: Some(d("1")),
            independent_buyers: Some(5),
            signal_age_seconds: 10,
        },
        "x",
    );
    assert_eq!(u.decision, Decision::InsufficientEvidence);
    assert_eq!(u.value, None);
}

#[test]
fn fresh_clean_trade_with_several_independent_buyers_is_eligible() {
    let s = tcq::score(
        &tcq::TcqConfig::default(),
        &tcq::TradeInputs {
            risk: risk(good_inputs()),
            move_since_entry_pct: Some(d("3")),
            independent_buyers: Some(3),
            signal_age_seconds: 120,
        },
        "x",
    );
    assert_eq!(s.decision, Decision::Eligible);
    assert!(s.value.unwrap() >= 70);
}

#[test]
fn old_signal_or_big_move_since_entry_is_watch_only() {
    let cfg = tcq::TcqConfig::default();
    let old = tcq::score(
        &cfg,
        &tcq::TradeInputs {
            risk: risk(good_inputs()),
            move_since_entry_pct: Some(d("3")),
            independent_buyers: Some(3),
            signal_age_seconds: 99_999,
        },
        "x",
    );
    assert_eq!(old.decision, Decision::WatchOnly);
    let moved = tcq::score(
        &cfg,
        &tcq::TradeInputs {
            risk: risk(good_inputs()),
            move_since_entry_pct: Some(d("80")),
            independent_buyers: Some(3),
            signal_age_seconds: 60,
        },
        "x",
    );
    assert_eq!(moved.decision, Decision::WatchOnly);
}

// ----- DNA -----

#[test]
fn dna_needs_enough_history() {
    let few: Vec<_> = steady_winner(3);
    let out = dna::analyze(&few);
    assert_eq!(out.state, dna::DnaState::InsufficientHistory);
    assert!(out.traits.is_empty());
    assert_eq!(dna::analyze(&[]).sample_size, 0);
}

#[test]
fn dna_labels_come_with_evidence_and_sample_size() {
    let out = dna::analyze(&steady_winner(12));
    assert_eq!(out.state, dna::DnaState::Ok);
    assert_eq!(out.rules_version, "dna/1");
    let hold = out
        .traits
        .iter()
        .find(|t| t.key == "swing_trader")
        .expect("10h median hold is a swing trade");
    assert!(hold.evidence.contains("12 closed trades"));
    assert!(out.traits.iter().any(|t| t.key == "diversified"));
    assert!(out.traits.iter().any(|t| t.key == "medium_size"));
}

#[test]
fn dna_detects_concentration_and_style_drift() {
    let mut c: Vec<_> = (0..6)
        .map(|i| close(i * 2, 2, "1000", "50", "one"))
        .collect();
    c.extend((0..6).map(|i| close(20 + i * 2, 24 * 9, "1000", "50", "one")));
    let out = dna::analyze(&c);
    assert!(out.traits.iter().any(|t| t.key == "concentrated"));
    assert!(out.traits.iter().any(|t| t.key == "style_drift"));
}
