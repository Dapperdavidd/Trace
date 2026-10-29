//! Shadow Wallet simulation (paper only). Replays a leader's trades as a
//! follower with a delay, size rule, slippage, fees and an exit rule.
//!
//! The follower enters at the FIRST FEASIBLE post-signal quote. When no quote
//! exists, liquidity is too thin or the budget is exhausted, the trade is
//! logged as `skipped_unexecutable`. The leader's ideal execution is never
//! substituted. This module never signs or broadcasts anything.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct LeaderTrade {
    pub tx_id: String,
    pub token: String,
    pub side: Side,
    pub detected_at: DateTime<Utc>,
    pub leader_price_usd: Decimal,
}

#[derive(Debug, Clone, Copy)]
pub enum SizeRule {
    FixedUsd(Decimal),
    /// Percent of the starting budget per trade (0 to 100).
    PercentOfBudget(Decimal),
}

#[derive(Debug, Clone, Copy)]
pub enum ExitRule {
    /// Close the whole position when the leader sells.
    MirrorSell,
    /// Close after a fixed number of seconds from entry.
    FixedHorizon(i64),
}

#[derive(Debug, Clone)]
pub struct Strategy {
    pub budget_usd: Decimal,
    pub size_rule: SizeRule,
    pub delay_seconds: i64,
    pub slippage_bps: u32,
    pub fee_bps: u32,
    pub exit_rule: ExitRule,
    /// How long after the delay we keep looking for a first feasible quote.
    pub max_entry_wait_seconds: i64,
    /// An order may not exceed this fraction of pool liquidity (e.g. 0.02).
    pub max_liquidity_fraction: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub at: DateTime<Utc>,
    pub price_usd: Decimal,
    pub liquidity_usd: Decimal,
}

pub trait QuoteProvider {
    /// First available quote at or after `t`.
    fn first_at_or_after(&self, token: &str, t: DateTime<Utc>) -> Option<Quote>;
    /// Latest quote at or before `t`, used to mark open positions.
    fn latest_at_or_before(&self, token: &str, t: DateTime<Utc>) -> Option<Quote>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    NoQuote,
    InsufficientLiquidity,
    InsufficientBudget,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Skipped {
    pub tx_id: String,
    pub token: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClosedTrade {
    pub token: String,
    pub entered_at: DateTime<Utc>,
    pub exited_at: DateTime<Utc>,
    pub cost_usd: Decimal,
    pub proceeds_usd: Decimal,
    pub pnl_usd: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Assumptions {
    pub delay_seconds: i64,
    pub slippage_bps: u32,
    pub fee_bps: u32,
    pub max_entry_wait_seconds: i64,
    pub max_liquidity_fraction: Decimal,
    pub entry_rule: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Baseline {
    /// Same trades at the leader's exact prices with no delay, fees or slippage.
    /// Shown for comparison only. It is never used as the follower result.
    pub ideal_leader_execution_return_pct: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulationResult {
    pub starting_usd: Decimal,
    pub ending_usd: Decimal,
    pub return_pct: Decimal,
    pub filled_entries: usize,
    pub closed: Vec<ClosedTrade>,
    pub wins: usize,
    pub losses: usize,
    pub skipped: Vec<Skipped>,
    pub open_positions: usize,
    /// Open positions with no mark price are excluded from ending value.
    pub unmarked_open_positions: usize,
    pub fees_and_slippage_usd: Decimal,
    pub baseline: Baseline,
    pub assumptions: Assumptions,
}

#[derive(Debug, Clone)]
struct Position {
    qty: Decimal,
    cost_usd: Decimal,
    entered_at: DateTime<Utc>,
}

fn pct(n: Decimal, d: Decimal) -> Decimal {
    if d.is_zero() {
        Decimal::ZERO
    } else {
        n / d * Decimal::from(100)
    }
}

pub fn simulate(
    strategy: &Strategy,
    trades: &[LeaderTrade],
    quotes: &dyn QuoteProvider,
    end: DateTime<Utc>,
) -> SimulationResult {
    let mut ordered: Vec<&LeaderTrade> = trades.iter().collect();
    ordered.sort_by(|a, b| (a.detected_at, &a.tx_id).cmp(&(b.detected_at, &b.tx_id)));

    let bps = Decimal::from(10_000);
    let slip = Decimal::from(strategy.slippage_bps) / bps;
    let fee = Decimal::from(strategy.fee_bps) / bps;

    let mut cash = strategy.budget_usd;
    let mut ideal_cash = strategy.budget_usd;
    let mut ideal_pos: BTreeMap<String, (Decimal, DateTime<Utc>)> = BTreeMap::new(); // token -> (qty, entry)
    let mut pos: BTreeMap<String, Position> = BTreeMap::new();
    let mut costs = Decimal::ZERO;
    let mut out = SimulationResult {
        starting_usd: strategy.budget_usd,
        ending_usd: Decimal::ZERO,
        return_pct: Decimal::ZERO,
        filled_entries: 0,
        closed: vec![],
        wins: 0,
        losses: 0,
        skipped: vec![],
        open_positions: 0,
        unmarked_open_positions: 0,
        fees_and_slippage_usd: Decimal::ZERO,
        baseline: Baseline {
            ideal_leader_execution_return_pct: None,
        },
        assumptions: Assumptions {
            delay_seconds: strategy.delay_seconds,
            slippage_bps: strategy.slippage_bps,
            fee_bps: strategy.fee_bps,
            max_entry_wait_seconds: strategy.max_entry_wait_seconds,
            max_liquidity_fraction: strategy.max_liquidity_fraction,
            entry_rule: "first feasible quote after detection plus delay",
        },
    };

    let order_size = |_: ()| match strategy.size_rule {
        SizeRule::FixedUsd(v) => v,
        SizeRule::PercentOfBudget(p) => strategy.budget_usd * p / Decimal::from(100),
    };

    let close_position = |token: &str,
                          at: DateTime<Utc>,
                          q: &Quote,
                          pos: &mut BTreeMap<String, Position>,
                          cash: &mut Decimal,
                          costs: &mut Decimal,
                          out: &mut SimulationResult| {
        if let Some(p) = pos.remove(token) {
            let fill = q.price_usd * (Decimal::ONE - slip);
            let gross = p.qty * fill;
            let fee_usd = gross * fee;
            let proceeds = gross - fee_usd;
            *costs += fee_usd + p.qty * (q.price_usd - fill);
            *cash += proceeds;
            let pnl = proceeds - p.cost_usd;
            if pnl > Decimal::ZERO {
                out.wins += 1;
            } else if pnl < Decimal::ZERO {
                out.losses += 1;
            }
            out.closed.push(ClosedTrade {
                token: token.to_string(),
                entered_at: p.entered_at,
                exited_at: at,
                cost_usd: p.cost_usd,
                proceeds_usd: proceeds,
                pnl_usd: pnl,
            });
        }
    };

    for t in ordered {
        match t.side {
            Side::Buy => {
                // Ideal baseline: exact leader price, no frictions, same size.
                let size = order_size(()).min(ideal_cash);
                if size > Decimal::ZERO
                    && t.leader_price_usd > Decimal::ZERO
                    && !ideal_pos.contains_key(&t.token)
                {
                    ideal_cash -= size;
                    ideal_pos.insert(t.token.clone(), (size / t.leader_price_usd, t.detected_at));
                }

                if pos.contains_key(&t.token) {
                    continue; // one position per token in this simple model
                }
                let want = order_size(());
                if want > cash || want <= Decimal::ZERO {
                    out.skipped.push(Skipped {
                        tx_id: t.tx_id.clone(),
                        token: t.token.clone(),
                        reason: SkipReason::InsufficientBudget,
                    });
                    continue;
                }
                let earliest = t.detected_at + Duration::seconds(strategy.delay_seconds);
                let latest = earliest + Duration::seconds(strategy.max_entry_wait_seconds);
                let Some(q) = quotes
                    .first_at_or_after(&t.token, earliest)
                    .filter(|q| q.at <= latest && q.price_usd > Decimal::ZERO)
                else {
                    out.skipped.push(Skipped {
                        tx_id: t.tx_id.clone(),
                        token: t.token.clone(),
                        reason: SkipReason::NoQuote,
                    });
                    continue;
                };
                if want > q.liquidity_usd * strategy.max_liquidity_fraction {
                    out.skipped.push(Skipped {
                        tx_id: t.tx_id.clone(),
                        token: t.token.clone(),
                        reason: SkipReason::InsufficientLiquidity,
                    });
                    continue;
                }
                let fill = q.price_usd * (Decimal::ONE + slip);
                let fee_usd = want * fee;
                let qty = (want - fee_usd) / fill;
                costs += fee_usd + qty * (fill - q.price_usd);
                cash -= want;
                pos.insert(
                    t.token.clone(),
                    Position {
                        qty,
                        cost_usd: want,
                        entered_at: q.at,
                    },
                );
                out.filled_entries += 1;

                if let ExitRule::FixedHorizon(h) = strategy.exit_rule {
                    let exit_t = q.at + Duration::seconds(h);
                    if exit_t <= end {
                        if let Some(xq) = quotes
                            .first_at_or_after(&t.token, exit_t)
                            .filter(|x| x.at <= end)
                        {
                            close_position(
                                &t.token, xq.at, &xq, &mut pos, &mut cash, &mut costs, &mut out,
                            );
                        }
                    }
                }
            }
            Side::Sell => {
                if let (Some((qty, _)), true) = (
                    ideal_pos.remove(&t.token),
                    t.leader_price_usd > Decimal::ZERO,
                ) {
                    ideal_cash += qty * t.leader_price_usd;
                }
                if matches!(strategy.exit_rule, ExitRule::MirrorSell) && pos.contains_key(&t.token)
                {
                    let earliest = t.detected_at + Duration::seconds(strategy.delay_seconds);
                    let latest = earliest + Duration::seconds(strategy.max_entry_wait_seconds);
                    match quotes
                        .first_at_or_after(&t.token, earliest)
                        .filter(|q| q.at <= latest && q.price_usd > Decimal::ZERO)
                    {
                        Some(q) => close_position(
                            &t.token, q.at, &q, &mut pos, &mut cash, &mut costs, &mut out,
                        ),
                        None => out.skipped.push(Skipped {
                            tx_id: t.tx_id.clone(),
                            token: t.token.clone(),
                            reason: SkipReason::NoQuote,
                        }),
                    }
                }
            }
        }
    }

    // Mark remaining open positions at the latest known price.
    let mut value = cash;
    for (token, p) in &pos {
        out.open_positions += 1;
        match quotes.latest_at_or_before(token, end) {
            Some(q) => value += p.qty * q.price_usd,
            None => out.unmarked_open_positions += 1,
        }
    }
    out.ending_usd = value;
    out.return_pct = pct(value - strategy.budget_usd, strategy.budget_usd);
    out.fees_and_slippage_usd = costs;

    // Ideal baseline marks open ideal positions at the latest quote too.
    let mut ideal_value = ideal_cash;
    let mut ideal_ok = true;
    for (token, (qty, _)) in &ideal_pos {
        match quotes.latest_at_or_before(token, end) {
            Some(q) => ideal_value += *qty * q.price_usd,
            None => ideal_ok = false,
        }
    }
    if ideal_ok {
        out.baseline.ideal_leader_execution_return_pct =
            Some(pct(ideal_value - strategy.budget_usd, strategy.budget_usd));
    }
    out
}
