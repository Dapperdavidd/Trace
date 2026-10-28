//! SYNTHETIC fixtures shaped like `getTransaction` jsonParsed results.
//! They prove parser logic only. They are not real-chain proof; the protocol
//! stays Disabled until a captured real transaction passes (see docs/fixtures.md).

use serde_json::{json, Value};
use trace_chain_adapters::solana::*;
use trace_domain::{Attribution, Finality};

const PAYER: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const POOL: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
const RAYDIUM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const OTHER_PROGRAM: &str = "11111111111111111111111111111111";

fn bal(idx: usize, mint: &str, owner: &str, amount: u128) -> Value {
    json!({"accountIndex": idx, "mint": mint, "owner": owner, "uiTokenAmount": {"amount": amount.to_string(), "decimals": 6}})
}

fn tx(
    program: &str,
    err: Value,
    pre_sol: u64,
    post_sol: u64,
    pre: Vec<Value>,
    post: Vec<Value>,
) -> TxResult {
    serde_json::from_value(json!({
        "slot": 300_000_000u64,
        "blockTime": 1_793_190_000i64,
        "transaction": {
            "signatures": ["5synthSigAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"],
            "message": {
                "accountKeys": [{"pubkey": PAYER}, {"pubkey": POOL}],
                "instructions": [{"programId": program}]
            }
        },
        "meta": {"err": err, "fee": 5000, "preBalances": [pre_sol, 0], "postBalances": [post_sol, 0],
                 "preTokenBalances": pre, "postTokenBalances": post}
    })).unwrap()
}

fn cfg() -> (Vec<ProgramAllow>, bool) {
    (
        vec![ProgramAllow {
            program_id: RAYDIUM,
            protocol: "raydium_amm_v4",
        }],
        true,
    )
}

#[test]
fn native_sol_to_usdc_buy_is_one_clean_swap() {
    let t = tx(
        RAYDIUM,
        Value::Null,
        5_000_000_000,
        3_999_995_000,
        vec![bal(2, USDC, PAYER, 0), bal(3, USDC, POOL, 900_000_000)],
        vec![
            bal(2, USDC, PAYER, 150_000_000),
            bal(3, USDC, POOL, 750_000_000),
        ],
    );
    let (programs, finalized) = cfg();
    let s = parse_tx(
        &Config {
            programs: &programs,
            finalized,
        },
        &t,
    )
    .unwrap();
    assert_eq!(s.token_in.contract_or_mint.as_str(), WRAPPED_SOL);
    assert_eq!(s.amount_in.0, 1_000_000_000);
    assert_eq!(s.token_out.contract_or_mint.as_str(), USDC);
    assert_eq!(s.amount_out.0, 150_000_000);
    assert_eq!(s.pool, POOL);
    assert_eq!(s.protocol, "raydium_amm_v4");
    assert_eq!(s.finality, Finality::Final);
    assert_eq!(s.attribution, Attribution::Direct);
    assert!(!s.demo);
}

#[test]
fn fee_alone_is_not_a_trade() {
    let t = tx(
        RAYDIUM,
        Value::Null,
        5_000_000_000,
        4_999_995_000,
        vec![],
        vec![],
    );
    let (programs, finalized) = cfg();
    let e = parse_tx(
        &Config {
            programs: &programs,
            finalized,
        },
        &t,
    )
    .unwrap_err();
    assert!(matches!(e, Skipped::NotASimpleSwap { .. }));
}

#[test]
fn failed_transactions_are_skipped() {
    let t = tx(
        RAYDIUM,
        json!({"InstructionError": [0, "Custom"]}),
        5_000_000_000,
        4_999_995_000,
        vec![],
        vec![],
    );
    let (programs, finalized) = cfg();
    assert_eq!(
        parse_tx(
            &Config {
                programs: &programs,
                finalized
            },
            &t
        )
        .unwrap_err(),
        Skipped::FailedTransaction
    );
}

#[test]
fn unlisted_programs_are_not_classified() {
    let t = tx(
        OTHER_PROGRAM,
        Value::Null,
        5_000_000_000,
        3_999_995_000,
        vec![bal(2, USDC, PAYER, 0)],
        vec![bal(2, USDC, PAYER, 150_000_000)],
    );
    let (programs, finalized) = cfg();
    assert_eq!(
        parse_tx(
            &Config {
                programs: &programs,
                finalized
            },
            &t
        )
        .unwrap_err(),
        Skipped::NoAllowlistedProgram
    );
}

#[test]
fn two_assets_leaving_is_not_a_simple_swap() {
    // Looks like LP deposit: SOL and USDC both leave, LP token arrives.
    let lp = "So11111111111111111111111111111111111111111";
    let t = tx(
        RAYDIUM,
        Value::Null,
        5_000_000_000,
        3_999_995_000,
        vec![bal(2, USDC, PAYER, 200_000_000), bal(4, lp, PAYER, 0)],
        vec![bal(2, USDC, PAYER, 50_000_000), bal(4, lp, PAYER, 777)],
    );
    let (programs, finalized) = cfg();
    let e = parse_tx(
        &Config {
            programs: &programs,
            finalized,
        },
        &t,
    )
    .unwrap_err();
    assert_eq!(
        e,
        Skipped::NotASimpleSwap {
            detail: "2 mints out, 1 mints in".into()
        }
    );
}

#[test]
fn tentative_commitment_is_marked_tentative() {
    let t = tx(
        RAYDIUM,
        Value::Null,
        5_000_000_000,
        3_999_995_000,
        vec![bal(2, USDC, PAYER, 0)],
        vec![bal(2, USDC, PAYER, 150_000_000)],
    );
    let (programs, _) = cfg();
    let s = parse_tx(
        &Config {
            programs: &programs,
            finalized: false,
        },
        &t,
    )
    .unwrap();
    assert_eq!(s.finality, Finality::Tentative);
}

#[test]
fn replay_is_deterministic() {
    let t = tx(
        RAYDIUM,
        Value::Null,
        5_000_000_000,
        3_999_995_000,
        vec![bal(2, USDC, PAYER, 0)],
        vec![bal(2, USDC, PAYER, 150_000_000)],
    );
    let (programs, finalized) = cfg();
    let c = Config {
        programs: &programs,
        finalized,
    };
    assert_eq!(parse_tx(&c, &t).unwrap(), parse_tx(&c, &t).unwrap());
}
