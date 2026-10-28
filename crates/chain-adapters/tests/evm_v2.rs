//! SYNTHETIC fixtures. They prove decoding and attribution logic only and are
//! not real-chain proof; see docs/fixtures.md for the capture workflow.

use chrono::{TimeZone, Utc};
use std::collections::HashMap;
use trace_chain_adapters::evm_v2::*;
use trace_domain::{Address, Attribution, ChainId, Finality};

const TRADER: &str = "0x1111111111111111111111111111111111111111";
const ROUTER: &str = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d";
const BOT: &str = "0x2222222222222222222222222222222222222222";
const PAIR: &str = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";
const TOKEN0: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const TOKEN1: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

fn a(s: &str) -> Address {
    Address::parse(ChainId::Ethereum, s).unwrap()
}

fn pad_addr(s: &str) -> String {
    format!("0x{:0>64}", s.trim_start_matches("0x"))
}

fn words(w: [u128; 4]) -> String {
    let mut s = String::from("0x");
    for v in w {
        s.push_str(&format!("{:064x}", v));
    }
    s
}

fn log(pair: &str, idx: u32, recipient: &str, w: [u128; 4]) -> Log {
    Log {
        address: pair.into(),
        topics: vec![swap_topic(), pad_addr(ROUTER), pad_addr(recipient)],
        data: words(w),
        log_index: idx,
    }
}

fn tx(to: Option<&str>, logs: Vec<Log>, finalized: bool) -> TxContext {
    TxContext {
        tx_hash: "0xABCDEF0000000000000000000000000000000000000000000000000000000001".into(),
        from: TRADER.into(),
        to: to.map(str::to_string),
        block_number: 23_000_000,
        block_hash: "0xBEEF".into(),
        block_time: Utc.timestamp_opt(1_793_190_000, 0).unwrap(),
        logs,
        finalized,
    }
}

fn pairs() -> HashMap<Address, PairInfo> {
    HashMap::from([(
        a(PAIR),
        PairInfo {
            token0: a(TOKEN0),
            token1: a(TOKEN1),
            protocol: "uniswap_v2".into(),
        },
    )])
}

fn cfg(routers: &[Address]) -> Config<'_> {
    Config {
        chain: ChainId::Ethereum,
        routers,
    }
}

const USDC_IN: u128 = 3_000_000_000; // 3000 USDC (6 decimals)
const WETH_OUT: u128 = 1_000_000_000_000_000_000; // 1 WETH

#[test]
fn direct_swap_is_attributed_and_amounts_are_exact() {
    let t = tx(
        Some(PAIR),
        vec![log(PAIR, 5, TRADER, [USDC_IN, 0, 0, WETH_OUT])],
        true,
    );
    let out = parse_tx(&cfg(&[]), &t, &pairs());
    assert_eq!(out.swaps.len(), 1);
    let s = &out.swaps[0];
    assert_eq!(s.attribution, Attribution::Direct);
    assert_eq!(s.token_in.contract_or_mint, a(TOKEN0));
    assert_eq!(s.amount_in.0, USDC_IN);
    assert_eq!(s.token_out.contract_or_mint, a(TOKEN1));
    assert_eq!(s.amount_out.0, WETH_OUT);
    assert_eq!(s.finality, Finality::Final);
    assert_eq!(
        s.tx_id,
        "0xabcdef0000000000000000000000000000000000000000000000000000000001"
    );
    assert_eq!(s.event_index, 5);
}

#[test]
fn router_swap_with_sender_as_beneficiary_is_routed() {
    // recipient is the trader, executed via the allowlisted router
    let t = tx(
        Some(ROUTER),
        vec![log(PAIR, 3, TRADER, [0, WETH_OUT, USDC_IN, 0])],
        false,
    );
    let routers = [a(ROUTER)];
    let out = parse_tx(&cfg(&routers), &t, &pairs());
    assert_eq!(out.swaps[0].attribution, Attribution::Direct); // recipient == sender wins
    assert_eq!(out.swaps[0].token_in.contract_or_mint, a(TOKEN1));
    assert_eq!(out.swaps[0].finality, Finality::Tentative);
}

#[test]
fn swap_paid_to_someone_else_is_ambiguous_not_credited() {
    let t = tx(
        Some(ROUTER),
        vec![log(PAIR, 1, BOT, [USDC_IN, 0, 0, WETH_OUT])],
        true,
    );
    let routers: [Address; 0] = [];
    let out = parse_tx(&cfg(&routers), &t, &pairs());
    assert_eq!(out.swaps[0].attribution, Attribution::Ambiguous);
}

#[test]
fn unknown_pairs_are_reported_not_guessed() {
    let other = "0x9999999999999999999999999999999999999999";
    let t = tx(
        Some(other),
        vec![log(other, 2, TRADER, [USDC_IN, 0, 0, WETH_OUT])],
        true,
    );
    let out = parse_tx(&cfg(&[]), &t, &pairs());
    assert!(out.swaps.is_empty());
    assert_eq!(
        out.skipped,
        vec![Skipped::UnknownPair {
            pair: other.into(),
            log_index: 2
        }]
    );
}

#[test]
fn two_sided_input_is_not_a_clean_swap() {
    let t = tx(
        Some(PAIR),
        vec![log(PAIR, 0, TRADER, [USDC_IN, WETH_OUT, 0, 1])],
        true,
    );
    let out = parse_tx(&cfg(&[]), &t, &pairs());
    assert!(out.swaps.is_empty());
    assert_eq!(out.skipped, vec![Skipped::NotASimpleSwap { log_index: 0 }]);
}

#[test]
fn malformed_logs_are_reported() {
    let mut l = log(PAIR, 4, TRADER, [USDC_IN, 0, 0, WETH_OUT]);
    l.data = "0x1234".into();
    let out = parse_tx(&cfg(&[]), &tx(Some(PAIR), vec![l], true), &pairs());
    assert!(out.swaps.is_empty());
    assert!(matches!(
        out.skipped[0],
        Skipped::MalformedLog { log_index: 4, .. }
    ));
}

#[test]
fn amounts_above_u128_are_rejected_not_truncated() {
    let mut l = log(PAIR, 4, TRADER, [0, 0, 0, 0]);
    l.data = format!(
        "0x{}{}{}{}",
        "f".repeat(64),
        "0".repeat(64),
        "0".repeat(64),
        "0".repeat(64)
    );
    let out = parse_tx(&cfg(&[]), &tx(Some(PAIR), vec![l], true), &pairs());
    assert!(out.swaps.is_empty());
    assert!(matches!(out.skipped[0], Skipped::MalformedLog { .. }));
}

#[test]
fn multiple_swaps_in_one_tx_keep_distinct_event_indexes() {
    let logs = vec![
        log(PAIR, 2, TRADER, [USDC_IN, 0, 0, WETH_OUT]),
        log(PAIR, 9, TRADER, [0, WETH_OUT, USDC_IN, 0]),
    ];
    let out = parse_tx(&cfg(&[]), &tx(Some(PAIR), logs, true), &pairs());
    let idx: Vec<_> = out.swaps.iter().map(|s| s.event_index).collect();
    assert_eq!(idx, vec![2, 9]);
}

#[test]
fn non_swap_logs_are_ignored() {
    let mut l = log(PAIR, 1, TRADER, [USDC_IN, 0, 0, WETH_OUT]);
    l.topics[0] = format!("0x{}", "00".repeat(32));
    let out = parse_tx(&cfg(&[]), &tx(Some(PAIR), vec![l], true), &pairs());
    assert!(out.swaps.is_empty() && out.skipped.is_empty());
}
