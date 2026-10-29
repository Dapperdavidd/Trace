use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use trace_chain_adapters::evm_v2::{parse_tx, swap_topic, Config, PairInfo, TxContext};
use trace_domain::{Address, ChainId};

use crate::{hex_u64, host_of, review, sha256_hex, ts, Fixture, SCHEMA, TOOL_VERSION};

/// `token0()` and `token1()` selectors of a Uniswap V2 style pair.
pub const TOKEN0_SELECTOR: &str = "0x0dfe1681";
pub const TOKEN1_SELECTOR: &str = "0xd21220a7";

/// Raw JSON-RPC results needed to build one EVM fixture.
pub struct Raw {
    pub receipt: Value,
    pub transaction: Value,
    pub block: Value,
    /// Result of `eth_getBlockByNumber("finalized")`. `None` when the chain/RPC has no such tag.
    pub finalized_block: Option<Value>,
    /// pair address (lowercase) -> (token0 call result, token1 call result)
    pub pair_calls: BTreeMap<String, (String, String)>,
}

/// Pair contracts that emitted a Swap event in this receipt.
pub fn swap_pairs(receipt: &Value) -> BTreeSet<String> {
    let topic = swap_topic();
    receipt["logs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| {
            l["topics"][0]
                .as_str()
                .is_some_and(|t| t.eq_ignore_ascii_case(&topic))
        })
        .filter_map(|l| l["address"].as_str().map(str::to_lowercase))
        .collect()
}

fn address_from_call(result: &str) -> Result<String> {
    let h = result.strip_prefix("0x").unwrap_or(result);
    if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("eth_call did not return one 32-byte word: {result}");
    }
    Ok(format!("0x{}", &h[24..]).to_lowercase())
}

pub fn assemble(
    chain: ChainId,
    protocol: &str,
    rpc_url: &str,
    routers: &[String],
    raw: &Raw,
) -> Result<Fixture> {
    if raw.receipt.is_null() {
        bail!("no receipt: the transaction is unknown to this RPC or still pending");
    }
    let tx_hash = raw.receipt["transactionHash"]
        .as_str()
        .context("receipt has no transactionHash")?
        .to_lowercase();
    let from = raw.transaction["from"]
        .as_str()
        .or(raw.receipt["from"].as_str())
        .context("no sender")?
        .to_lowercase();
    let to = raw.transaction["to"]
        .as_str()
        .or(raw.receipt["to"].as_str())
        .map(str::to_lowercase);
    let block_number = hex_u64(&raw.receipt["blockNumber"], "blockNumber")?;
    let block_hash = raw.receipt["blockHash"]
        .as_str()
        .context("receipt has no blockHash")?
        .to_lowercase();
    if hex_u64(&raw.block["number"], "block.number")? != block_number {
        bail!("block response does not match the receipt's block number");
    }
    let block_time = ts(hex_u64(&raw.block["timestamp"], "block.timestamp")? as i64)?;
    let finalized = match &raw.finalized_block {
        Some(b) if !b.is_null() => block_number <= hex_u64(&b["number"], "finalized.number")?,
        _ => false,
    };
    let logs: Vec<Value> = raw.receipt["logs"]
        .as_array()
        .context("receipt has no logs array")?
        .iter()
        .map(|l| -> Result<Value> {
            Ok(json!({
                "address": l["address"].as_str().context("log without address")?.to_lowercase(),
                "topics": l["topics"].as_array().context("log without topics")?.iter().filter_map(|t| t.as_str().map(str::to_lowercase)).collect::<Vec<_>>(),
                "data": l["data"].as_str().context("log without data")?.to_lowercase(),
                "logIndex": hex_u64(&l["logIndex"], "logIndex")?,
            }))
        })
        .collect::<Result<_>>()?;
    let payload = json!({
        "tx_hash": tx_hash, "from": from, "to": to, "block_number": block_number,
        "block_hash": block_hash, "block_time": block_time, "logs": logs, "finalized": finalized,
    });

    let mut pairs = BTreeMap::new();
    for pair in swap_pairs(&raw.receipt) {
        let (t0, t1) = raw
            .pair_calls
            .get(&pair)
            .with_context(|| format!("no token0/token1 result for pair {pair}"))?;
        pairs.insert(pair, json!({ "token0": address_from_call(t0)?, "token1": address_from_call(t1)?, "protocol": protocol }));
    }
    let draft = draft(chain, &payload, &pairs, routers)?;
    Ok(Fixture {
        schema: SCHEMA,
        kind: "evm_v2_tx",
        chain: chain.as_str().into(),
        protocol: protocol.into(),
        tx_id: tx_hash,
        captured_at: Utc::now(),
        rpc_host: host_of(rpc_url)?,
        tool_version: TOOL_VERSION,
        payload_sha256: sha256_hex(&payload),
        payload,
        pairs,
        routers: routers.iter().map(|r| r.to_lowercase()).collect(),
        program_ids: vec![],
        review: review(draft),
    })
}

/// Current parser output for a captured payload. A draft only.
pub fn draft(
    chain: ChainId,
    payload: &Value,
    pairs: &BTreeMap<String, Value>,
    routers: &[String],
) -> Result<Value> {
    let tx: TxContext =
        serde_json::from_value(payload.clone()).context("payload is not a TxContext")?;
    let mut map: HashMap<Address, PairInfo> = HashMap::new();
    for (k, v) in pairs {
        map.insert(
            Address::parse(chain, k)?,
            PairInfo {
                token0: Address::parse(chain, v["token0"].as_str().unwrap_or_default())?,
                token1: Address::parse(chain, v["token1"].as_str().unwrap_or_default())?,
                protocol: v["protocol"].as_str().unwrap_or_default().to_string(),
            },
        );
    }
    let routers: Vec<Address> = routers
        .iter()
        .filter_map(|r| Address::parse(chain, r).ok())
        .collect();
    let out = parse_tx(
        &Config {
            chain,
            routers: &routers,
        },
        &tx,
        &map,
    );
    Ok(json!({
        "swaps": out.swaps.iter().map(|s| json!({
            "wallet": s.wallet.address.as_str(),
            "token_in": s.token_in.contract_or_mint.as_str(),
            "amount_in": s.amount_in.to_string(),
            "token_out": s.token_out.contract_or_mint.as_str(),
            "amount_out": s.amount_out.to_string(),
            "attribution": serde_json::to_value(s.attribution).unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "skipped": out.skipped.len(),
    }))
}
