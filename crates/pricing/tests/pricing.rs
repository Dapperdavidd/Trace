//! Synthetic provider bodies in the documented DefiLlama shape.

use chrono::{Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;
use trace_domain::{AssetId, ChainId};
use trace_pricing::*;

const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const MINT: &str = "So11111111111111111111111111111111111111112";

fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

fn eth(a: &str) -> AssetId {
    AssetId::parse(ChainId::Ethereum, a).unwrap()
}

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn quote(price: &str, decimals: Option<u8>, age_min: i64, conf: Option<f64>) -> Quote {
    Quote {
        asset: eth(WETH),
        usd_per_token: d(price),
        decimals,
        symbol: Some("WETH".into()),
        observed_at: now() - Duration::minutes(age_min),
        confidence: conf,
    }
}

#[test]
fn a_fresh_confident_quote_converts_to_a_per_base_unit_price() {
    let a = accept(&quote("3000.5", Some(18), 1, Some(0.99)), Some(18), now()).unwrap();
    assert_eq!(a.usd_per_base_unit, d("3000.5") / d("1000000000000000000"));
    assert_eq!(
        a.observed_at,
        now() - Duration::minutes(1),
        "the quote's own time, not the poll time"
    );
    assert_eq!(a.learned_decimals, None);
}

#[test]
fn stale_future_and_low_confidence_quotes_are_rejected() {
    assert_eq!(
        accept(&quote("1", Some(18), 16, None), Some(18), now()),
        Err(Reason::Stale)
    );
    assert_eq!(
        accept(&quote("1", Some(18), -5, None), Some(18), now()),
        Err(Reason::Stale)
    );
    assert_eq!(
        accept(&quote("1", Some(18), 1, Some(0.5)), Some(18), now()),
        Err(Reason::LowConfidence)
    );
    assert_eq!(
        accept(&quote("1", Some(18), 1, Some(1.5)), Some(18), now()),
        Err(Reason::LowConfidence)
    );
    assert!(
        accept(&quote("1", Some(18), 1, None), Some(18), now()).is_ok(),
        "no confidence field is not a rejection"
    );
}

#[test]
fn zero_and_negative_prices_are_rejected() {
    for p in ["0", "-1"] {
        assert_eq!(
            accept(&quote(p, Some(18), 1, None), Some(18), now()),
            Err(Reason::BadPrice)
        );
    }
}

#[test]
fn decimals_are_cross_checked_never_overridden() {
    assert_eq!(
        accept(&quote("1", Some(6), 1, None), Some(18), now()),
        Err(Reason::DecimalsMismatch)
    );
    let learned = accept(&quote("1", Some(6), 1, None), None, now()).unwrap();
    assert_eq!(
        learned.learned_decimals,
        Some(6),
        "provider decimals are used only when we had none, and flagged"
    );
    assert_eq!(
        accept(&quote("1", None, 1, None), None, now()),
        Err(Reason::DecimalsUnknown)
    );
    assert!(
        accept(&quote("1", None, 1, None), Some(18), now()).is_ok(),
        "our stored decimals are enough"
    );
}

#[test]
fn a_price_that_rounds_to_zero_per_base_unit_is_rejected_not_stored_as_zero() {
    assert_eq!(
        accept(&quote("0.000000000001", Some(18), 1, None), Some(18), now()),
        Err(Reason::Underflow)
    );
}

#[test]
fn parsing_reads_price_text_exactly_and_matches_evm_keys_case_insensitively() {
    let body = json!({ "coins": {
        "ethereum:0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2": {
            "decimals": 18, "symbol": "WETH", "price": 3012.123456789, "timestamp": 1_792_999_990u64, "confidence": 0.99
        }
    }});
    let b = parse_defillama(&body, &[eth(WETH)]);
    assert!(b.rejected.is_empty());
    assert_eq!(b.quotes[0].usd_per_token, d("3012.123456789"));
    assert_eq!(b.quotes[0].decimals, Some(18));
    assert_eq!(b.quotes[0].observed_at.timestamp(), 1_792_999_990);
}

#[test]
fn solana_mints_match_exactly_and_unknown_assets_are_reported_missing() {
    let sol = AssetId::parse(ChainId::Solana, MINT).unwrap();
    let body = json!({ "coins": {
        "solana:so11111111111111111111111111111111111111112": { "price": 150.0, "timestamp": 1_792_999_990u64, "decimals": 9 }
    }});
    let b = parse_defillama(&body, &[sol.clone(), eth(USDC)]);
    assert!(
        b.quotes.is_empty(),
        "a differently-cased base58 string is a different mint"
    );
    assert_eq!(b.rejected.len(), 2);
    assert!(b.rejected.iter().all(|(_, r)| *r == Reason::Missing));
}

#[test]
fn malformed_entries_are_rejected_not_coerced() {
    let body = json!({ "coins": {
        "ethereum:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2": { "price": "3000", "timestamp": 1_792_999_990u64 }
    }});
    let b = parse_defillama(&body, &[eth(WETH)]);
    assert_eq!(
        b.rejected,
        vec![(eth(WETH), Reason::BadPrice)],
        "a string price is not a number"
    );
    assert!(parse_defillama(&json!({}), &[eth(WETH)]).quotes.is_empty());
}

#[test]
fn coin_keys_use_the_providers_chain_names() {
    assert_eq!(coin_key(&eth(WETH)), format!("ethereum:{WETH}"));
    assert_eq!(
        coin_key(&AssetId::parse(ChainId::Bnb, WETH).unwrap()),
        format!("bsc:{WETH}")
    );
}
