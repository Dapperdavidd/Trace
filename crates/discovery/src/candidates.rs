use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use trace_domain::{Address, Attribution, NormalizedSwap};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WalletActivity {
    pub wallet: String,
    pub trade_count: usize,
    pub distinct_tokens: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    /// Median seconds between consecutive trades. `None` with fewer than two trades.
    pub median_gap_seconds: Option<i64>,
    /// Swaps in the input that were not credited to this wallet (routed, ambiguous, infrastructure).
    pub non_direct_swaps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CandidateDecision {
    /// Eligible for deeper historical examination.
    Candidate,
    /// Known infrastructure. Never ranked.
    ExcludedInfrastructure,
    /// Behaviour resembles a bot. Kept, but marked uncertain and not promoted.
    UncertainBot { reason: String },
    /// Not enough observed activity yet.
    WaitingForActivity { have: usize, need: usize },
}

#[derive(Debug, Clone)]
pub struct CandidateConfig {
    pub min_trades: usize,
    pub bot_min_trades: usize,
    /// Median gap below this, with many trades, suggests automation.
    pub bot_median_gap_seconds: i64,
}

impl Default for CandidateConfig {
    fn default() -> Self {
        Self {
            min_trades: 5,
            bot_min_trades: 20,
            bot_median_gap_seconds: 3,
        }
    }
}

/// Group Direct swaps by wallet into activity summaries.
pub fn summarize(swaps: &[NormalizedSwap]) -> Vec<WalletActivity> {
    let mut by: BTreeMap<String, Vec<&NormalizedSwap>> = BTreeMap::new();
    let mut non_direct: BTreeMap<String, usize> = BTreeMap::new();
    for s in swaps {
        let w = s.wallet.address.to_string();
        if s.attribution == Attribution::Direct {
            by.entry(w).or_default().push(s);
        } else {
            *non_direct.entry(w).or_default() += 1;
        }
    }
    let mut out = vec![];
    for (w, mut v) in by {
        v.sort_by_key(|s| s.observed_at);
        let tokens: BTreeSet<_> = v
            .iter()
            .flat_map(|s| {
                [
                    s.token_in.contract_or_mint.as_str(),
                    s.token_out.contract_or_mint.as_str(),
                ]
            })
            .collect();
        let mut gaps: Vec<i64> = v
            .windows(2)
            .map(|p| (p[1].observed_at - p[0].observed_at).num_seconds())
            .collect();
        gaps.sort_unstable();
        out.push(WalletActivity {
            trade_count: v.len(),
            distinct_tokens: tokens.len(),
            first_seen: v.first().unwrap().observed_at,
            last_seen: v.last().unwrap().observed_at,
            median_gap_seconds: gaps.get(gaps.len() / 2).copied(),
            non_direct_swaps: non_direct.get(&w).copied().unwrap_or(0),
            wallet: w,
        });
    }
    out
}

pub fn classify(
    cfg: &CandidateConfig,
    a: &WalletActivity,
    known_infrastructure: &BTreeSet<Address>,
) -> CandidateDecision {
    if known_infrastructure.iter().any(|i| i.as_str() == a.wallet) {
        return CandidateDecision::ExcludedInfrastructure;
    }
    if a.trade_count >= cfg.bot_min_trades
        && a.median_gap_seconds
            .is_some_and(|g| g < cfg.bot_median_gap_seconds)
    {
        return CandidateDecision::UncertainBot {
            reason: format!(
                "{} trades with a median of {}s between trades",
                a.trade_count,
                a.median_gap_seconds.unwrap()
            ),
        };
    }
    if a.trade_count < cfg.min_trades {
        return CandidateDecision::WaitingForActivity {
            have: a.trade_count,
            need: cfg.min_trades,
        };
    }
    CandidateDecision::Candidate
}
