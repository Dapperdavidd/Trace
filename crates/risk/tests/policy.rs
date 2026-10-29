use chrono::Utc;
use rust_decimal::Decimal;
use trace_risk::*;

fn good() -> RiskInputs {
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
        pool_age_hours: Some(500),
        quote_age_seconds: Some(20),
        abnormal_activity: Some(false),
        vendor_says_safe: None,
    }
}

fn state(r: &RiskReport, k: CheckKey) -> RiskState {
    r.checks.iter().find(|c| c.key == k).unwrap().state
}

#[test]
fn clean_token_passes_and_is_actionable() {
    let r = assess(&RiskPolicy::default(), &good(), Utc::now());
    assert_eq!(r.overall, RiskState::Pass);
    assert!(r.actionable);
}

#[test]
fn honeypot_blocks_everything() {
    let mut i = good();
    i.honeypot = Some(true);
    let r = assess(&RiskPolicy::default(), &i, Utc::now());
    assert_eq!(r.overall, RiskState::Block);
    assert!(!r.actionable);
}

#[test]
fn extreme_sell_tax_blocks_even_without_honeypot_flag() {
    let mut i = good();
    i.sell_tax_bps = Some(6_000);
    assert_eq!(
        assess(&RiskPolicy::default(), &i, Utc::now()).overall,
        RiskState::Block
    );
}

#[test]
fn missing_required_data_is_unknown_not_pass() {
    let r = assess(&RiskPolicy::default(), &RiskInputs::default(), Utc::now());
    assert_eq!(r.overall, RiskState::Unknown);
    assert!(!r.actionable);
    assert!(r.checks.iter().all(|c| c.state == RiskState::Unknown));
}

#[test]
fn one_missing_check_downgrades_an_otherwise_clean_token() {
    let mut i = good();
    i.top10_holder_pct = None;
    let r = assess(&RiskPolicy::default(), &i, Utc::now());
    assert_eq!(r.overall, RiskState::Unknown);
    assert_eq!(state(&r, CheckKey::HolderConcentration), RiskState::Unknown);
}

#[test]
fn stale_quote_cannot_support_actionable_status() {
    let mut i = good();
    i.quote_age_seconds = Some(3_600);
    let r = assess(&RiskPolicy::default(), &i, Utc::now());
    assert_eq!(state(&r, CheckKey::QuoteFreshness), RiskState::Unknown);
    assert!(!r.actionable);
}

#[test]
fn vendor_safe_flag_never_overrides_a_block() {
    let mut i = good();
    i.vendor_says_safe = Some(true);
    i.honeypot = Some(true);
    assert_eq!(
        assess(&RiskPolicy::default(), &i, Utc::now()).overall,
        RiskState::Block
    );
}

#[test]
fn vendor_safe_flag_does_not_fill_in_missing_contract_data() {
    let mut i = good();
    i.mint_authority_active = None;
    i.freeze_authority_active = None;
    i.blacklist_function = None;
    i.vendor_says_safe = Some(true);
    let r = assess(&RiskPolicy::default(), &i, Utc::now());
    assert_eq!(state(&r, CheckKey::ContractSafety), RiskState::Unknown);
}

#[test]
fn thin_liquidity_blocks_and_limited_liquidity_warns() {
    let mut i = good();
    i.liquidity_usd = Some(Decimal::from(5_000));
    assert_eq!(
        state(
            &assess(&RiskPolicy::default(), &i, Utc::now()),
            CheckKey::Liquidity
        ),
        RiskState::Block
    );
    i.liquidity_usd = Some(Decimal::from(30_000));
    assert_eq!(
        state(
            &assess(&RiskPolicy::default(), &i, Utc::now()),
            CheckKey::Liquidity
        ),
        RiskState::Warn
    );
}

#[test]
fn large_price_impact_blocks_despite_deep_headline_liquidity() {
    let mut i = good();
    i.price_impact_bps = Some(2_000);
    assert_eq!(
        state(
            &assess(&RiskPolicy::default(), &i, Utc::now()),
            CheckKey::Liquidity
        ),
        RiskState::Block
    );
}

#[test]
fn creator_privileges_warn() {
    let mut i = good();
    i.mint_authority_active = Some(true);
    let r = assess(&RiskPolicy::default(), &i, Utc::now());
    assert_eq!(state(&r, CheckKey::ContractSafety), RiskState::Warn);
    assert_eq!(r.overall, RiskState::Warn);
    assert!(r.actionable);
}
