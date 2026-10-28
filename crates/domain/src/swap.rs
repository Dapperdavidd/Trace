use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    address::{AssetId, WalletId},
    amount::BaseUnits,
    chain::ChainId,
};

/// Finality of an observation. Tentative data may be reversed by a reorg;
/// derived positions and signals must carry this forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finality {
    Tentative,
    Final,
    Reorged,
}

impl Finality {
    pub fn as_str(self) -> &'static str {
        match self {
            Finality::Tentative => "tentative",
            Finality::Final => "final",
            Finality::Reorged => "reorged",
        }
    }
}

/// How confidently a swap can be credited to the wallet as its own trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribution {
    /// Trader is the transaction sender and recipient of the output.
    Direct,
    /// Intermediate leg of a router path (output went to another contract).
    /// Not credited as a trade by itself; multi-hop routes are only partly
    /// attributed (the final leg is Direct), which is a documented limitation.
    Routed,
    /// Unknown aggregator, bot or contract. Not credited as a normal trade.
    Ambiguous,
    /// Recognised infrastructure (router, MEV, LP action). Excluded from discovery.
    Infrastructure,
}

/// Canonical swap event. Identity is
/// `(chain, tx_id, event_index, finality_revision)` and is idempotent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSwap {
    pub chain: ChainId,
    pub tx_id: String,
    pub event_index: u32,
    pub finality_revision: u32,
    pub wallet: WalletId,
    pub protocol: String,
    pub pool: String,
    pub token_in: AssetId,
    pub amount_in: BaseUnits,
    pub token_out: AssetId,
    pub amount_out: BaseUnits,
    pub block_height: Option<u64>,
    pub block_hash: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub finality: Finality,
    pub attribution: Attribution,
    pub parser_version: String,
    /// Where the raw evidence lives (raw_chain_events natural key).
    pub raw_ref: String,
    /// True for seeded sample data. Never shown as live.
    pub demo: bool,
}
