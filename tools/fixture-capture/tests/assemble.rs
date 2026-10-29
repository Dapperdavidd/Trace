//! Fixture assembly with synthetic RPC responses. Proves the tool's logic
//! (shape checks, redaction, hashing, draft) and nothing about the chain.

use fixture_capture::{evm, host_of, sha256_hex, solana};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use trace_chain_adapters::evm_v2::swap_topic;
use trace_domain::ChainId;

const PAIR: &str = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";
const TOKEN0: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const TOKEN1: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const ROUTER: &str = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d";
const TRADER: &str = "0x1111111111111111111111111111111111111111";
const URL: &str = "https://eth-mainnet.example.io/v2/SECRET-API-KEY?token=abc";

fn pad(a: &str) -> String {
    format!("0x{:0>64}", a.trim_start_matches("0x"))
}

fn word_call(a: &str) -> String {
    pad(a)
}

fn raw(finalized_number: Option<u64>) -> evm::Raw {
    let data = format!(
        "0x{:064x}{:064x}{:064x}{:064x}",
        1_000_000u128, 0u128, 0u128, 5_000_000_000_000_000u128
    );
    let tx_hash = format!("0x{}", "AB".repeat(32));
    evm::Raw {
        receipt: json!({
            "transactionHash": tx_hash, "blockNumber": "0x10", "blockHash": "0xBEEF", "from": TRADER, "to": ROUTER,
            "logs": [{ "address": PAIR.to_uppercase().replace("0X", "0x"), "topics": [swap_topic(), pad(ROUTER), pad(TRADER)], "data": data, "logIndex": "0x2" }]
        }),
        transaction: json!({ "from": TRADER, "to": ROUTER }),
        block: json!({ "number": "0x10", "timestamp": "0x6553f100" }),
        finalized_block: finalized_number.map(|n| json!({ "number": format!("0x{n:x}") })),
        pair_calls: BTreeMap::from([(PAIR.to_string(), (word_call(TOKEN0), word_call(TOKEN1)))]),
    }
}

#[test]
fn host_never_includes_path_key_or_credentials() {
    assert_eq!(host_of(URL).unwrap(), "eth-mainnet.example.io");
    assert_eq!(
        host_of("http://user:pw@Node.Example.com:8545/x").unwrap(),
        "node.example.com"
    );
    assert!(host_of("ftp://x").is_err());
    assert!(host_of("https:///path").is_err());
}

#[test]
fn evm_fixture_never_contains_the_rpc_url_or_key() {
    let f = evm::assemble(
        ChainId::Ethereum,
        "uniswap_v2",
        URL,
        &[ROUTER.into()],
        &raw(Some(20)),
    )
    .unwrap();
    let text = serde_json::to_string(&f).unwrap();
    assert!(
        !text.contains("SECRET-API-KEY") && !text.contains("token=abc") && !text.contains("/v2/")
    );
    assert_eq!(f.rpc_host, "eth-mainnet.example.io");
}

#[test]
fn evm_fixture_is_normalised_hashed_and_unreviewed() {
    let f = evm::assemble(
        ChainId::Ethereum,
        "uniswap_v2",
        URL,
        &[ROUTER.into()],
        &raw(Some(20)),
    )
    .unwrap();
    assert_eq!(f.review.status, "unreviewed");
    assert!(
        f.review.expected.is_null(),
        "the tool must never fill expected"
    );
    assert_eq!(f.tx_id, format!("0x{}", "ab".repeat(32)));
    assert_eq!(f.payload["logs"][0]["address"], PAIR);
    assert_eq!(f.payload["logs"][0]["logIndex"], 2);
    assert_eq!(f.payload_sha256, sha256_hex(&f.payload));
    assert_eq!(f.pairs[PAIR]["token0"], TOKEN0);
    assert_eq!(f.pairs[PAIR]["token1"], TOKEN1);
}

#[test]
fn finality_is_proven_not_assumed() {
    let fin = evm::assemble(ChainId::Ethereum, "uniswap_v2", URL, &[], &raw(Some(16))).unwrap();
    assert_eq!(fin.payload["finalized"], true);
    let behind = evm::assemble(ChainId::Ethereum, "uniswap_v2", URL, &[], &raw(Some(15))).unwrap();
    assert_eq!(behind.payload["finalized"], false);
    let unknown = evm::assemble(ChainId::Ethereum, "uniswap_v2", URL, &[], &raw(None)).unwrap();
    assert_eq!(unknown.payload["finalized"], false);
}

#[test]
fn draft_comes_from_the_parser_and_is_labelled_not_evidence() {
    let f = evm::assemble(
        ChainId::Ethereum,
        "uniswap_v2",
        URL,
        &[ROUTER.into()],
        &raw(Some(20)),
    )
    .unwrap();
    assert_eq!(f.review.draft["swaps"].as_array().unwrap().len(), 1);
    assert_eq!(f.review.draft["swaps"][0]["wallet"], TRADER);
    assert!(f.review.instructions.contains("block explorer"));
}

#[test]
fn bad_evm_inputs_are_refused() {
    let mut r = raw(Some(20));
    r.receipt = Value::Null;
    assert!(evm::assemble(ChainId::Ethereum, "uniswap_v2", URL, &[], &r).is_err());

    let mut r = raw(Some(20));
    r.block = json!({ "number": "0x11", "timestamp": "0x1" });
    assert!(
        evm::assemble(ChainId::Ethereum, "uniswap_v2", URL, &[], &r).is_err(),
        "block must match receipt"
    );

    let mut r = raw(Some(20));
    r.pair_calls.clear();
    assert!(
        evm::assemble(ChainId::Ethereum, "uniswap_v2", URL, &[], &r).is_err(),
        "missing pair tokens"
    );

    let mut r = raw(Some(20));
    r.pair_calls
        .insert(PAIR.into(), ("0x1234".into(), word_call(TOKEN1)));
    assert!(
        evm::assemble(ChainId::Ethereum, "uniswap_v2", URL, &[], &r).is_err(),
        "short eth_call result"
    );
}

#[test]
fn solana_fixture_checks_signature_and_null() {
    assert!(solana::assemble("p", URL, "SIG", &["Prog".into()], &Value::Null).is_err());
    let other = json!({ "transaction": { "signatures": ["OTHER"] } });
    assert!(solana::assemble("p", URL, "SIG", &["Prog".into()], &other).is_err());
}
