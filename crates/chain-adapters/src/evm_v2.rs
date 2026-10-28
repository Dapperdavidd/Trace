//! Uniswap V2 style constant-product pair `Swap` log decoding (Ethereum, Base,
//! BNB Chain forks). Attribution is conservative: only swaps whose recipient is
//! the transaction sender (`Direct`) are credited to a wallet. Intermediate
//! router legs are `Routed` (not credited); everything else is `Ambiguous`.
//! Both are excluded from discovery.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;
use trace_domain::{
    Address, AssetId, Attribution, BaseUnits, ChainId, Finality, NormalizedSwap, WalletId,
};

pub const PARSER_VERSION: &str = "evm_v2/1";

/// topic0 of `Swap(address,uint256,uint256,uint256,uint256,address)`.
pub fn swap_topic() -> String {
    format!(
        "0x{}",
        hex::encode(Keccak256::digest(
            b"Swap(address,uint256,uint256,uint256,uint256,address)"
        ))
    )
}

#[derive(Debug, Clone, Deserialize)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    #[serde(rename = "logIndex")]
    pub log_index: u32,
}

/// The parts of a transaction receipt plus block context the parser needs.
#[derive(Debug, Clone, Deserialize)]
pub struct TxContext {
    pub tx_hash: String,
    pub from: String,
    pub to: Option<String>,
    pub block_number: u64,
    pub block_hash: String,
    pub block_time: DateTime<Utc>,
    pub logs: Vec<Log>,
    /// Whether the block is at or below the chain's finalized height.
    pub finalized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairInfo {
    pub token0: Address,
    pub token1: Address,
    pub protocol: String,
}

/// Resolves a pair contract to its tokens. Production code backs this with a
/// cached `eth_call`; tests use a map. Unknown pairs are skipped and reported,
/// never guessed.
pub trait PairResolver {
    fn resolve(&self, pair: &Address) -> Option<PairInfo>;
}

impl PairResolver for HashMap<Address, PairInfo> {
    fn resolve(&self, pair: &Address) -> Option<PairInfo> {
        self.get(pair).cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skipped {
    UnknownPair {
        pair: String,
        log_index: u32,
    },
    MalformedLog {
        log_index: u32,
        reason: String,
    },
    /// Swap with no net token movement or two-sided input; not a clean trade.
    NotASimpleSwap {
        log_index: u32,
    },
}

#[derive(Debug, Default)]
pub struct ParseOutcome {
    pub swaps: Vec<NormalizedSwap>,
    pub skipped: Vec<Skipped>,
}

pub struct Config<'a> {
    pub chain: ChainId,
    /// Router contracts known to forward trades for the sender.
    pub routers: &'a [Address],
}

fn word(data: &[u8], i: usize) -> Option<u128> {
    let w = data.get(i * 32..(i + 1) * 32)?;
    // values above u128 cannot be real token amounts here; treat as malformed
    if w[..16].iter().any(|b| *b != 0) {
        return None;
    }
    let mut a = [0u8; 16];
    a.copy_from_slice(&w[16..]);
    Some(u128::from_be_bytes(a))
}

fn topic_address(topic: &str) -> Option<String> {
    let t = topic.strip_prefix("0x")?;
    if t.len() != 64 {
        return None;
    }
    Some(format!("0x{}", &t[24..]))
}

pub fn parse_tx(cfg: &Config<'_>, tx: &TxContext, pairs: &dyn PairResolver) -> ParseOutcome {
    let mut out = ParseOutcome::default();
    let topic = swap_topic();
    let Ok(sender) = Address::parse(cfg.chain, &tx.from) else {
        return out;
    };
    let tx_to = tx
        .to
        .as_deref()
        .and_then(|t| Address::parse(cfg.chain, t).ok());

    for log in tx.logs.iter().filter(|l| {
        l.topics
            .first()
            .map(|t| t.eq_ignore_ascii_case(&topic))
            .unwrap_or(false)
    }) {
        if log.topics.len() != 3 {
            out.skipped.push(Skipped::MalformedLog {
                log_index: log.log_index,
                reason: "expected 3 topics".into(),
            });
            continue;
        }
        let Ok(pair) = Address::parse(cfg.chain, &log.address) else {
            out.skipped.push(Skipped::MalformedLog {
                log_index: log.log_index,
                reason: "bad pair address".into(),
            });
            continue;
        };
        let Some(info) = pairs.resolve(&pair) else {
            out.skipped.push(Skipped::UnknownPair {
                pair: pair.to_string(),
                log_index: log.log_index,
            });
            continue;
        };
        let data = match hex::decode(log.data.trim_start_matches("0x")) {
            Ok(d) if d.len() == 128 => d,
            _ => {
                out.skipped.push(Skipped::MalformedLog {
                    log_index: log.log_index,
                    reason: "data must be 4 words".into(),
                });
                continue;
            }
        };
        let (Some(a0in), Some(a1in), Some(a0out), Some(a1out)) = (
            word(&data, 0),
            word(&data, 1),
            word(&data, 2),
            word(&data, 3),
        ) else {
            out.skipped.push(Skipped::MalformedLog {
                log_index: log.log_index,
                reason: "amount exceeds u128".into(),
            });
            continue;
        };
        // Exactly one side in and the other side out, otherwise it is not a clean swap.
        let (in_is_0, amt_in, amt_out) = match (a0in > 0, a1in > 0, a0out > 0, a1out > 0) {
            (true, false, false, true) => (true, a0in, a1out),
            (false, true, true, false) => (false, a1in, a0out),
            _ => {
                out.skipped.push(Skipped::NotASimpleSwap {
                    log_index: log.log_index,
                });
                continue;
            }
        };
        let (tin, tout) = if in_is_0 {
            (&info.token0, &info.token1)
        } else {
            (&info.token1, &info.token0)
        };

        let recipient =
            topic_address(&log.topics[2]).and_then(|a| Address::parse(cfg.chain, &a).ok());
        let attribution = match (&recipient, &tx_to) {
            (Some(r), _) if *r == sender => Attribution::Direct,
            (Some(_), Some(to)) if cfg.routers.contains(to) => Attribution::Routed,
            _ => Attribution::Ambiguous,
        };

        out.swaps.push(NormalizedSwap {
            chain: cfg.chain,
            tx_id: tx.tx_hash.to_ascii_lowercase(),
            event_index: log.log_index,
            finality_revision: 0,
            wallet: WalletId {
                chain: cfg.chain,
                address: sender.clone(),
            },
            protocol: info.protocol.clone(),
            pool: pair.to_string(),
            token_in: AssetId {
                chain: cfg.chain,
                contract_or_mint: tin.clone(),
            },
            amount_in: BaseUnits(amt_in),
            token_out: AssetId {
                chain: cfg.chain,
                contract_or_mint: tout.clone(),
            },
            amount_out: BaseUnits(amt_out),
            block_height: Some(tx.block_number),
            block_hash: Some(tx.block_hash.to_ascii_lowercase()),
            observed_at: tx.block_time,
            finality: if tx.finalized {
                Finality::Final
            } else {
                Finality::Tentative
            },
            attribution,
            parser_version: PARSER_VERSION.into(),
            raw_ref: format!(
                "{}:{}:{}",
                cfg.chain,
                tx.tx_hash.to_ascii_lowercase(),
                log.log_index
            ),
            demo: false,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_topic_matches_uniswap_v2_event_signature() {
        assert_eq!(
            swap_topic(),
            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
        );
    }
}
