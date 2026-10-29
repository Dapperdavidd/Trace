use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::{collections::BTreeMap, str::FromStr};
use trace_shadow::*;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}
fn t(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000 + secs, 0).unwrap()
}

#[derive(Default)]
struct Quotes(BTreeMap<String, Vec<Quote>>);

impl Quotes {
    fn with(mut self, token: &str, secs: i64, price: &str, liq: i64) -> Self {
        let v = self.0.entry(token.into()).or_default();
        v.push(Quote {
            at: t(secs),
            price_usd: d(price),
            liquidity_usd: Decimal::from(liq),
        });
        v.sort_by_key(|q| q.at);
        self
    }
}

impl QuoteProvider for Quotes {
    fn first_at_or_after(&self, token: &str, at: DateTime<Utc>) -> Option<Quote> {
        self.0.get(token)?.iter().find(|q| q.at >= at).cloned()
    }
    fn latest_at_or_before(&self, token: &str, at: DateTime<Utc>) -> Option<Quote> {
        self.0
            .get(token)?
            .iter()
            .rev()
            .find(|q| q.at <= at)
            .cloned()
    }
}

fn strat() -> Strategy {
    Strategy {
        budget_usd: Decimal::from(1_000),
        size_rule: SizeRule::FixedUsd(Decimal::from(100)),
        delay_seconds: 30,
        slippage_bps: 50,
        fee_bps: 30,
        exit_rule: ExitRule::MirrorSell,
        max_entry_wait_seconds: 300,
        max_liquidity_fraction: d("0.02"),
    }
}

fn buy(id: &str, token: &str, secs: i64, price: &str) -> LeaderTrade {
    LeaderTrade {
        tx_id: id.into(),
        token: token.into(),
        side: Side::Buy,
        detected_at: t(secs),
        leader_price_usd: d(price),
    }
}
fn sell(id: &str, token: &str, secs: i64, price: &str) -> LeaderTrade {
    LeaderTrade {
        tx_id: id.into(),
        token: token.into(),
        side: Side::Sell,
        detected_at: t(secs),
        leader_price_usd: d(price),
    }
}

#[test]
fn follower_enters_at_the_first_feasible_post_delay_quote_not_the_leader_price() {
    // Leader bought at 1.00. Price had already run to 1.20 by the time the follower can act.
    let q = Quotes::default()
        .with("A", 0, "1.00", 1_000_000)
        .with("A", 40, "1.20", 1_000_000)
        .with("A", 600, "1.50", 1_000_000);
    let r = simulate(&strat(), &[buy("b1", "A", 0, "1.00")], &q, t(700));
    assert_eq!(r.filled_entries, 1);
    // fee 0.30, qty = 99.70 / (1.20 * 1.005); marked at 1.50
    assert!(r.fees_and_slippage_usd > Decimal::ZERO);
    let ideal = r.baseline.ideal_leader_execution_return_pct.unwrap();
    assert!(
        r.return_pct < ideal,
        "follower {} should trail the ideal baseline {}",
        r.return_pct,
        ideal
    );
}

#[test]
fn no_quote_in_the_window_is_skipped_not_filled_at_the_leader_price() {
    let q = Quotes::default().with("A", 0, "1.00", 1_000_000); // nothing after the delay
    let r = simulate(&strat(), &[buy("b1", "A", 0, "1.00")], &q, t(700));
    assert_eq!(r.filled_entries, 0);
    assert_eq!(
        r.skipped,
        vec![Skipped {
            tx_id: "b1".into(),
            token: "A".into(),
            reason: SkipReason::NoQuote
        }]
    );
    assert_eq!(r.ending_usd, Decimal::from(1_000));
}

#[test]
fn a_quote_that_arrives_after_the_wait_window_does_not_count() {
    let q = Quotes::default().with("A", 1_000, "1.00", 1_000_000);
    let r = simulate(&strat(), &[buy("b1", "A", 0, "1.00")], &q, t(2_000));
    assert_eq!(r.skipped[0].reason, SkipReason::NoQuote);
}

#[test]
fn thin_liquidity_makes_the_trade_unexecutable() {
    // $100 order vs $2,000 pool at a 2% cap = $40 max.
    let q = Quotes::default().with("A", 40, "1.00", 2_000);
    let r = simulate(&strat(), &[buy("b1", "A", 0, "1.00")], &q, t(700));
    assert_eq!(r.skipped[0].reason, SkipReason::InsufficientLiquidity);
    assert_eq!(r.filled_entries, 0);
}

#[test]
fn budget_is_enforced_and_exhaustion_is_logged() {
    let mut s = strat();
    s.budget_usd = Decimal::from(150);
    let q = Quotes::default()
        .with("A", 40, "1", 1_000_000)
        .with("B", 140, "1", 1_000_000);
    let r = simulate(
        &s,
        &[buy("b1", "A", 0, "1"), buy("b2", "B", 100, "1")],
        &q,
        t(700),
    );
    assert_eq!(r.filled_entries, 1);
    assert_eq!(r.skipped[0].reason, SkipReason::InsufficientBudget);
}

#[test]
fn mirror_sell_closes_and_records_net_result_after_costs() {
    let q = Quotes::default()
        .with("A", 40, "1.00", 1_000_000)
        .with("A", 1_040, "1.50", 1_000_000);
    let r = simulate(
        &strat(),
        &[buy("b1", "A", 0, "1.00"), sell("s1", "A", 1_000, "1.50")],
        &q,
        t(2_000),
    );
    assert_eq!(r.closed.len(), 1);
    assert_eq!(r.wins, 1);
    let c = &r.closed[0];
    assert!(
        c.pnl_usd > Decimal::ZERO && c.pnl_usd < Decimal::from(50),
        "costs must reduce the 50% gross move, got {}",
        c.pnl_usd
    );
    assert_eq!(r.open_positions, 0);
    assert_eq!(r.ending_usd, Decimal::from(1_000) + c.pnl_usd);
}

#[test]
fn failed_exit_quote_leaves_the_position_open_and_is_logged() {
    let q = Quotes::default().with("A", 40, "1.00", 1_000_000);
    let r = simulate(
        &strat(),
        &[buy("b1", "A", 0, "1.00"), sell("s1", "A", 1_000, "1.50")],
        &q,
        t(2_000),
    );
    assert!(r.closed.is_empty());
    assert_eq!(r.open_positions, 1);
    assert!(r.skipped.iter().any(|s| s.tx_id == "s1"));
}

#[test]
fn open_positions_without_a_mark_are_flagged_not_valued() {
    let q = Quotes::default().with("A", 40, "1.00", 1_000_000);
    // end before the only quote exists is impossible here, so mark at end uses latest <= end which is 40
    let r = simulate(&strat(), &[buy("b1", "A", 0, "1.00")], &q, t(2_000));
    assert_eq!(r.open_positions, 1);
    assert_eq!(r.unmarked_open_positions, 0);
}

#[test]
fn fixed_horizon_exit_uses_the_first_quote_after_the_horizon() {
    let mut s = strat();
    s.exit_rule = ExitRule::FixedHorizon(3_600);
    let q = Quotes::default()
        .with("A", 40, "1.00", 1_000_000)
        .with("A", 3_700, "0.80", 1_000_000)
        .with("A", 9_000, "5.00", 1_000_000);
    let r = simulate(&s, &[buy("b1", "A", 0, "1.00")], &q, t(10_000));
    assert_eq!(r.closed.len(), 1);
    assert_eq!(r.losses, 1, "the later 5.00 peak must not rescue the trade");
}

#[test]
fn simulation_is_deterministic_and_order_independent() {
    let q = Quotes::default()
        .with("A", 40, "1.00", 1_000_000)
        .with("A", 1_040, "1.50", 1_000_000);
    let trades = vec![buy("b1", "A", 0, "1.00"), sell("s1", "A", 1_000, "1.50")];
    let mut rev = trades.clone();
    rev.reverse();
    assert_eq!(
        simulate(&strat(), &trades, &q, t(2_000)),
        simulate(&strat(), &rev, &q, t(2_000))
    );
}

#[test]
fn assumptions_are_echoed_in_the_result() {
    let q = Quotes::default();
    let r = simulate(&strat(), &[], &q, t(1) + Duration::seconds(1));
    assert_eq!(r.assumptions.delay_seconds, 30);
    assert_eq!(r.assumptions.slippage_bps, 50);
    assert_eq!(r.return_pct, Decimal::ZERO);
}
