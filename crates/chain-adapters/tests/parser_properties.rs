use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use std::collections::HashMap;
use trace_chain_adapters::{
    evm_v2::{self, Config as EvmConfig, Log, PairInfo, TxContext},
    solana::{self, Config as SolanaConfig, ProgramAllow, TxResult},
};
use trace_domain::{Address, ChainId};

const PAIR: &str = "0x1111111111111111111111111111111111111111";
const TOKEN0: &str = "0x2222222222222222222222222222222222222222";
const TOKEN1: &str = "0x3333333333333333333333333333333333333333";
const SENDER: &str = "0x4444444444444444444444444444444444444444";
const PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const PAYER: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const BONK: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

fn topic_address(address: &str) -> String {
    format!("0x{:0>64}", address.trim_start_matches("0x"))
}

proptest! {
    #[test]
    fn evm_decoder_never_panics_on_arbitrary_log_data(data in prop::collection::vec(any::<u8>(), 0..300)) {
        let pair = Address::parse(ChainId::Ethereum, PAIR).unwrap();
        let pairs = HashMap::from([(
            pair,
            PairInfo {
                token0: Address::parse(ChainId::Ethereum, TOKEN0).unwrap(),
                token1: Address::parse(ChainId::Ethereum, TOKEN1).unwrap(),
                protocol: "uniswap_v2".into(),
            },
        )]);
        let tx = TxContext {
            tx_hash: format!("0x{}", "ab".repeat(32)),
            from: SENDER.into(),
            to: Some(SENDER.into()),
            block_number: 1,
            block_hash: format!("0x{}", "cd".repeat(32)),
            block_time: Utc.timestamp_opt(1_793_000_000, 0).unwrap(),
            logs: vec![Log {
                address: PAIR.into(),
                topics: vec![evm_v2::swap_topic(), topic_address(SENDER), topic_address(SENDER)],
                data: format!("0x{}", hex::encode(data)),
                log_index: 0,
            }],
            finalized: true,
        };
        let out = evm_v2::parse_tx(&EvmConfig { chain: ChainId::Ethereum, routers: &[] }, &tx, &pairs);
        for swap in out.swaps {
            prop_assert!(swap.amount_in.0 > 0);
            prop_assert!(swap.amount_out.0 > 0);
            prop_assert_ne!(swap.token_in, swap.token_out);
        }
    }

    #[test]
    fn solana_decoder_rejects_or_normalizes_arbitrary_amount_strings(
        amount_in in ".{0,80}",
        amount_out in ".{0,80}",
    ) {
        let raw = serde_json::json!({
            "slot": 1, "blockTime": 1_793_000_000i64,
            "transaction": {
                "signatures": ["signature"],
                "message": { "accountKeys": [PAYER], "instructions": [{ "programId": PROGRAM }] }
            },
            "meta": {
                "err": null, "fee": 0,
                "preBalances": [1], "postBalances": [1],
                "preTokenBalances": [
                    { "accountIndex": 1, "mint": USDC, "owner": PAYER,
                      "uiTokenAmount": { "amount": amount_in, "decimals": 6 } },
                    { "accountIndex": 2, "mint": BONK, "owner": PAYER,
                      "uiTokenAmount": { "amount": "0", "decimals": 5 } }
                ],
                "postTokenBalances": [
                    { "accountIndex": 1, "mint": USDC, "owner": PAYER,
                      "uiTokenAmount": { "amount": "0", "decimals": 6 } },
                    { "accountIndex": 2, "mint": BONK, "owner": PAYER,
                      "uiTokenAmount": { "amount": amount_out, "decimals": 5 } }
                ]
            }
        });
        let tx: TxResult = serde_json::from_value(raw).unwrap();
        let programs = [ProgramAllow { program_id: PROGRAM, protocol: "raydium_amm_v4" }];
        if let Ok(swap) = solana::parse_tx(&SolanaConfig { programs: &programs, finalized: true }, &tx) {
            prop_assert!(swap.amount_in.0 > 0);
            prop_assert!(swap.amount_out.0 > 0);
            prop_assert_ne!(swap.token_in, swap.token_out);
        }
    }
}

#[test]
fn solana_amount_above_i128_is_rejected_not_wrapped() {
    let too_large = (i128::MAX as u128 + 1).to_string();
    let raw = serde_json::json!({
        "slot": 1, "blockTime": 1_793_000_000i64,
        "transaction": {
            "signatures": ["signature"],
            "message": { "accountKeys": [PAYER], "instructions": [{ "programId": PROGRAM }] }
        },
        "meta": {
            "err": null, "fee": 0,
            "preBalances": [1], "postBalances": [1],
            "preTokenBalances": [{ "accountIndex": 1, "mint": USDC, "owner": PAYER,
                "uiTokenAmount": { "amount": too_large, "decimals": 6 } }],
            "postTokenBalances": []
        }
    });
    let tx: TxResult = serde_json::from_value(raw).unwrap();
    let programs = [ProgramAllow {
        program_id: PROGRAM,
        protocol: "raydium_amm_v4",
    }];
    assert!(matches!(
        solana::parse_tx(
            &SolanaConfig {
                programs: &programs,
                finalized: true,
            },
            &tx
        ),
        Err(solana::Skipped::Malformed(_))
    ));
}
