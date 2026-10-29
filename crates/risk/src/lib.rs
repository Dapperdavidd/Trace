//! Token risk policy. Pure and versioned.
//!
//! Hard safety behaviour (from the v3 instructions):
//! * a known critical risk means no actionable status;
//! * unavailable required risk data means unknown / observe-only;
//! * a vendor "safe" flag is an input, never proof of safety;
//! * stale quotes cannot support an executable claim.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const RISK_POLICY_VERSION: &str = "risk/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskState {
    Pass,
    Warn,
    Block,
    Unknown,
}

impl RiskState {
    fn worst(self, other: RiskState) -> RiskState {
        use RiskState::*;
        // Block > Unknown > Warn > Pass
        let rank = |s: RiskState| match s {
            Block => 3,
            Unknown => 2,
            Warn => 1,
            Pass => 0,
        };
        if rank(other) > rank(self) {
            other
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKey {
    Liquidity,
    Honeypot,
    HolderConcentration,
    ContractSafety,
    SuspiciousActivity,
    QuoteFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Check {
    pub key: CheckKey,
    pub state: RiskState,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RiskReport {
    pub policy_version: &'static str,
    pub overall: RiskState,
    pub checks: Vec<Check>,
    pub fetched_at: DateTime<Utc>,
    /// An actionable (executable or published-as-research) status is only
    /// allowed when overall is Pass or Warn.
    pub actionable: bool,
}

/// Everything is optional on purpose: missing data is a first-class state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RiskInputs {
    pub honeypot: Option<bool>,
    pub sell_tax_bps: Option<u32>,
    pub mint_authority_active: Option<bool>,
    pub freeze_authority_active: Option<bool>,
    pub blacklist_function: Option<bool>,
    pub liquidity_usd: Option<Decimal>,
    /// Estimated price impact in bps for the reference trade size.
    pub price_impact_bps: Option<u32>,
    pub reference_size_usd: Option<Decimal>,
    pub top10_holder_pct: Option<Decimal>,
    pub pool_age_hours: Option<u32>,
    pub quote_age_seconds: Option<u64>,
    pub abnormal_activity: Option<bool>,
    /// Third-party "safe" verdict. Recorded, never trusted on its own.
    pub vendor_says_safe: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct RiskPolicy {
    pub min_liquidity_usd: Decimal,
    pub warn_liquidity_usd: Decimal,
    pub block_sell_tax_bps: u32,
    pub warn_sell_tax_bps: u32,
    pub block_price_impact_bps: u32,
    pub warn_price_impact_bps: u32,
    pub block_top10_pct: Decimal,
    pub warn_top10_pct: Decimal,
    pub max_quote_age_seconds: u64,
}

impl Default for RiskPolicy {
    /// Defaults are conservative starting values, not validated thresholds.
    fn default() -> Self {
        Self {
            min_liquidity_usd: Decimal::from(10_000),
            warn_liquidity_usd: Decimal::from(50_000),
            block_sell_tax_bps: 5_000,
            warn_sell_tax_bps: 1_000,
            block_price_impact_bps: 1_500,
            warn_price_impact_bps: 300,
            block_top10_pct: Decimal::from(90),
            warn_top10_pct: Decimal::from(60),
            max_quote_age_seconds: 300,
        }
    }
}

fn check(key: CheckKey, state: RiskState, note: impl Into<String>) -> Check {
    Check {
        key,
        state,
        note: note.into(),
    }
}

pub fn assess(policy: &RiskPolicy, i: &RiskInputs, now: DateTime<Utc>) -> RiskReport {
    use CheckKey::*;
    use RiskState::*;
    let mut checks = Vec::new();

    // Liquidity depth and exit impact.
    checks.push(match (i.liquidity_usd, i.price_impact_bps) {
        (None, _) => check(Liquidity, Unknown, "Liquidity data is unavailable."),
        (Some(l), _) if l < policy.min_liquidity_usd => {
            check(Liquidity, Block, "Liquidity is too thin to trade safely.")
        }
        (Some(_), Some(imp)) if imp >= policy.block_price_impact_bps => check(
            Liquidity,
            Block,
            "Trading this size would move the price a lot.",
        ),
        (Some(l), imp)
            if l < policy.warn_liquidity_usd
                || imp.is_some_and(|x| x >= policy.warn_price_impact_bps) =>
        {
            check(
                Liquidity,
                Warn,
                "Liquidity is limited. Larger trades may move the price.",
            )
        }
        (Some(_), None) => check(
            Liquidity,
            Unknown,
            "Price impact for the reference size is unavailable.",
        ),
        (Some(_), Some(_)) => check(
            Liquidity,
            Pass,
            "Liquidity looks adequate for the reference size.",
        ),
    });

    // Honeypot and transfer taxes.
    checks.push(match (i.honeypot, i.sell_tax_bps) {
        (Some(true), _) => check(Honeypot, Block, "Selling appears to be blocked (honeypot)."),
        (_, Some(t)) if t >= policy.block_sell_tax_bps => {
            check(Honeypot, Block, "Sell tax is extreme.")
        }
        (None, _) => check(Honeypot, Unknown, "Honeypot check is unavailable."),
        (Some(false), Some(t)) if t >= policy.warn_sell_tax_bps => {
            check(Honeypot, Warn, "Sell tax is high.")
        }
        (Some(false), Some(_)) => check(Honeypot, Pass, "No honeypot behaviour detected."),
        (Some(false), None) => check(
            Honeypot,
            Warn,
            "No honeypot detected, but the sell tax is unknown.",
        ),
    });

    // Holder concentration.
    checks.push(match i.top10_holder_pct {
        None => check(HolderConcentration, Unknown, "Holder data is unavailable."),
        Some(p) if p >= policy.block_top10_pct => check(
            HolderConcentration,
            Block,
            "A few wallets hold almost all supply.",
        ),
        Some(p) if p >= policy.warn_top10_pct => check(
            HolderConcentration,
            Warn,
            "A few wallets hold a large share of supply.",
        ),
        Some(_) => check(
            HolderConcentration,
            Pass,
            "Supply is reasonably spread out.",
        ),
    });

    // Contract privileges.
    let mint = i.mint_authority_active;
    let freeze = i.freeze_authority_active;
    let blacklist = i.blacklist_function;
    checks.push(if mint.is_none() && freeze.is_none() && blacklist.is_none() {
        check(ContractSafety, Unknown, "Contract checks are unavailable.")
    } else if mint == Some(true) || freeze == Some(true) {
        check(ContractSafety, Warn, "The creator can still mint or freeze tokens.")
    } else if blacklist == Some(true) {
        check(ContractSafety, Warn, "The contract can block specific wallets.")
    } else if i.vendor_says_safe == Some(true) {
        // Recorded as context only; a vendor flag never upgrades the state.
        check(ContractSafety, Pass, "No risky privileges found. A security vendor also reports it as safe, which is not proof.")
    } else {
        check(ContractSafety, Pass, "No risky privileges found.")
    });

    checks.push(match i.abnormal_activity {
        None => check(
            SuspiciousActivity,
            Unknown,
            "Activity screening is unavailable.",
        ),
        Some(true) => check(
            SuspiciousActivity,
            Warn,
            "Unusual trading patterns were detected.",
        ),
        Some(false) => check(SuspiciousActivity, Pass, "No unusual patterns detected."),
    });

    checks.push(match i.quote_age_seconds {
        None => check(QuoteFreshness, Unknown, "No price quote age is available."),
        Some(a) if a > policy.max_quote_age_seconds => check(
            QuoteFreshness,
            Unknown,
            "The latest quote is too old to rely on.",
        ),
        Some(_) => check(QuoteFreshness, Pass, "The quote is fresh."),
    });

    let overall = checks.iter().fold(Pass, |acc, c| acc.worst(c.state));
    RiskReport {
        policy_version: RISK_POLICY_VERSION,
        overall,
        actionable: matches!(overall, Pass | Warn),
        checks,
        fetched_at: now,
    }
}
