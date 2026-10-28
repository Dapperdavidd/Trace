//! Solana swap extraction from `getTransaction` (jsonParsed) results.
//!
//! Method: for an allowlisted DEX program invocation in a successful
//! transaction, compute the fee payer's net token deltas per mint from
//! pre/post token balances (native SOL is merged into the wrapped-SOL mint,
//! excluding the fee). A clean trade has exactly one mint going down and one
//! going up. Anything else is reported as skipped, never guessed. Not every
//! SPL transfer is a buy.

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::collections::BTreeMap;
use trace_domain::{
    Address, AssetId, Attribution, BaseUnits, ChainId, Finality, NormalizedSwap, WalletId,
};

pub const PARSER_VERSION: &str = "solana_delta/1";
pub const WRAPPED_SOL: &str = "So11111111111111111111111111111111111111112";

#[derive(Debug, Clone, Deserialize)]
pub struct UiTokenAmount {
    pub amount: String,
    #[allow(dead_code)]
    pub decimals: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenBalance {
    #[serde(rename = "accountIndex")]
    pub account_index: usize,
    pub mint: String,
    pub owner: Option<String>,
    #[serde(rename = "uiTokenAmount")]
    pub ui_token_amount: UiTokenAmount,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub err: Option<serde_json::Value>,
    pub fee: u64,
    #[serde(rename = "preBalances")]
    pub pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    pub post_balances: Vec<u64>,
    #[serde(rename = "preTokenBalances", default)]
    pub pre_token_balances: Vec<TokenBalance>,
    #[serde(rename = "postTokenBalances", default)]
    pub post_token_balances: Vec<TokenBalance>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Instruction {
    #[serde(rename = "programId")]
    pub program_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InnerInstructions {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    #[serde(rename = "accountKeys")]
    pub account_keys: Vec<AccountKey>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AccountKey {
    Plain(String),
    Detailed { pubkey: String },
}

impl AccountKey {
    pub fn pubkey(&self) -> &str {
        match self {
            AccountKey::Plain(s) => s,
            AccountKey::Detailed { pubkey } => pubkey,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Transaction {
    pub message: Message,
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TxResult {
    pub slot: u64,
    #[serde(rename = "blockTime")]
    pub block_time: Option<i64>,
    pub transaction: Transaction,
    pub meta: Meta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skipped {
    FailedTransaction,
    NoAllowlistedProgram,
    MissingBlockTime,
    /// Net deltas were not exactly one mint out and one mint in.
    NotASimpleSwap {
        detail: String,
    },
    Malformed(String),
}

#[derive(Debug, Clone)]
pub struct ProgramAllow {
    pub program_id: &'static str,
    pub protocol: &'static str,
}

pub struct Config<'a> {
    pub programs: &'a [ProgramAllow],
    /// `finalized` commitment was used to fetch this transaction.
    pub finalized: bool,
}

fn parse_amount(s: &str) -> Result<i128, Skipped> {
    s.parse::<u128>()
        .map(|v| v as i128)
        .map_err(|_| Skipped::Malformed(format!("bad token amount '{s}'")))
}

/// Parse one transaction. `Ok(None)` is not used: every outcome is either a
/// swap or an explicit skip reason so coverage gaps stay visible.
pub fn parse_tx(cfg: &Config<'_>, tx: &TxResult) -> Result<NormalizedSwap, Skipped> {
    if tx.meta.err.is_some() {
        return Err(Skipped::FailedTransaction);
    }
    let keys: Vec<&str> = tx
        .transaction
        .message
        .account_keys
        .iter()
        .map(|k| k.pubkey())
        .collect();
    let payer = *keys
        .first()
        .ok_or_else(|| Skipped::Malformed("no account keys".into()))?;
    let program = tx
        .transaction
        .message
        .instructions
        .iter()
        .find_map(|i| cfg.programs.iter().find(|p| p.program_id == i.program_id))
        .ok_or(Skipped::NoAllowlistedProgram)?;
    let block_time = tx.block_time.ok_or(Skipped::MissingBlockTime)?;
    let observed_at: DateTime<Utc> = Utc
        .timestamp_opt(block_time, 0)
        .single()
        .ok_or_else(|| Skipped::Malformed("bad blockTime".into()))?;

    // Net token deltas for the payer, by mint.
    let mut delta: BTreeMap<String, i128> = BTreeMap::new();
    for b in &tx.meta.pre_token_balances {
        if b.owner.as_deref() == Some(payer) {
            *delta.entry(b.mint.clone()).or_default() -= parse_amount(&b.ui_token_amount.amount)?;
        }
    }
    for b in &tx.meta.post_token_balances {
        if b.owner.as_deref() == Some(payer) {
            *delta.entry(b.mint.clone()).or_default() += parse_amount(&b.ui_token_amount.amount)?;
        }
    }
    // Native SOL movement of the payer, net of the fee, counts as wrapped SOL.
    let pre = *tx
        .meta
        .pre_balances
        .first()
        .ok_or_else(|| Skipped::Malformed("no preBalances".into()))? as i128;
    let post = *tx
        .meta
        .post_balances
        .first()
        .ok_or_else(|| Skipped::Malformed("no postBalances".into()))? as i128;
    let native = post - pre + tx.meta.fee as i128;
    if native != 0 {
        *delta.entry(WRAPPED_SOL.to_string()).or_default() += native;
    }
    delta.retain(|_, v| *v != 0);

    let outs: Vec<_> = delta.iter().filter(|(_, v)| **v < 0).collect();
    let ins: Vec<_> = delta.iter().filter(|(_, v)| **v > 0).collect();
    if outs.len() != 1 || ins.len() != 1 {
        return Err(Skipped::NotASimpleSwap {
            detail: format!("{} mints out, {} mints in", outs.len(), ins.len()),
        });
    }
    let (mint_out, d_out) = outs[0];
    let (mint_in, d_in) = ins[0];

    let to_asset =
        |m: &str| AssetId::parse(ChainId::Solana, m).map_err(|e| Skipped::Malformed(e.to_string()));
    // Pool: the non-payer owner whose balance of the received mint fell the most.
    let pool = pool_owner(tx, payer, mint_in).unwrap_or_else(|| "unknown".to_string());
    let sig = tx
        .transaction
        .signatures
        .first()
        .ok_or_else(|| Skipped::Malformed("no signature".into()))?;

    Ok(NormalizedSwap {
        chain: ChainId::Solana,
        tx_id: sig.clone(),
        event_index: 0,
        finality_revision: 0,
        wallet: WalletId::parse(ChainId::Solana, payer)
            .map_err(|e| Skipped::Malformed(e.to_string()))?,
        protocol: program.protocol.to_string(),
        pool,
        // The wallet pays `mint_out` (its balance fell) and receives `mint_in`.
        token_in: to_asset(mint_out)?,
        amount_in: BaseUnits((-*d_out) as u128),
        token_out: to_asset(mint_in)?,
        amount_out: BaseUnits(*d_in as u128),
        block_height: Some(tx.slot),
        block_hash: None,
        observed_at,
        finality: if cfg.finalized {
            Finality::Final
        } else {
            Finality::Tentative
        },
        // Fee payer swapping directly through an allowlisted program with clean deltas.
        attribution: Attribution::Direct,
        parser_version: PARSER_VERSION.into(),
        raw_ref: format!("solana:{sig}:0"),
        demo: false,
    })
}

fn pool_owner(tx: &TxResult, payer: &str, mint: &str) -> Option<String> {
    let mut d: BTreeMap<String, i128> = BTreeMap::new();
    for b in tx
        .meta
        .pre_token_balances
        .iter()
        .filter(|b| b.mint == mint && b.owner.as_deref() != Some(payer))
    {
        *d.entry(b.owner.clone()?).or_default() -=
            b.ui_token_amount.amount.parse::<u128>().ok()? as i128;
    }
    for b in tx
        .meta
        .post_token_balances
        .iter()
        .filter(|b| b.mint == mint && b.owner.as_deref() != Some(payer))
    {
        *d.entry(b.owner.clone()?).or_default() +=
            b.ui_token_amount.amount.parse::<u128>().ok()? as i128;
    }
    d.into_iter()
        .filter(|(_, v)| *v < 0)
        .min_by_key(|(_, v)| *v)
        .map(|(k, _)| k)
}

pub fn address(s: &str) -> Option<Address> {
    Address::parse(ChainId::Solana, s).ok()
}
