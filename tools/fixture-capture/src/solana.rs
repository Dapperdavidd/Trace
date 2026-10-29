use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use trace_chain_adapters::solana::{parse_tx, Config, ProgramAllow, TxResult};

use crate::{host_of, review, sha256_hex, Fixture, SCHEMA, TOOL_VERSION};

/// `result` of `getTransaction` with `encoding: "jsonParsed"`,
/// `maxSupportedTransactionVersion: 0` and `commitment: "finalized"`.
pub fn assemble(
    protocol: &str,
    rpc_url: &str,
    signature: &str,
    program_ids: &[String],
    result: &Value,
) -> Result<Fixture> {
    if result.is_null() {
        bail!("transaction not found at finalized commitment (unknown signature, or not yet finalized)");
    }
    let sigs = result["transaction"]["signatures"]
        .as_array()
        .context("no signatures in result")?;
    if !sigs.iter().any(|s| s.as_str() == Some(signature)) {
        bail!("the RPC returned a different transaction than the one requested");
    }
    let draft = draft(protocol, program_ids, result)?;
    Ok(Fixture {
        schema: SCHEMA,
        kind: "solana_tx",
        chain: "solana".into(),
        protocol: protocol.into(),
        tx_id: signature.into(),
        captured_at: Utc::now(),
        rpc_host: host_of(rpc_url)?,
        tool_version: TOOL_VERSION,
        payload_sha256: sha256_hex(result),
        payload: result.clone(),
        pairs: Default::default(),
        routers: vec![],
        program_ids: program_ids.to_vec(),
        review: review(draft),
    })
}

pub fn draft(protocol: &str, program_ids: &[String], result: &Value) -> Result<Value> {
    let tx: TxResult = serde_json::from_value(result.clone())
        .context("result is not a jsonParsed getTransaction result")?;
    let allow: Vec<ProgramAllow> = program_ids
        .iter()
        .map(|p| ProgramAllow {
            program_id: Box::leak(p.clone().into_boxed_str()),
            protocol: Box::leak(protocol.to_string().into_boxed_str()),
        })
        .collect();
    Ok(
        match parse_tx(
            &Config {
                programs: &allow,
                finalized: true,
            },
            &tx,
        ) {
            Ok(s) => json!({ "swap": {
                "wallet": s.wallet.address.as_str(),
                "token_in": s.token_in.contract_or_mint.as_str(),
                "amount_in": s.amount_in.to_string(),
                "token_out": s.token_out.contract_or_mint.as_str(),
                "amount_out": s.amount_out.to_string(),
            }}),
            Err(e) => json!({ "skipped": format!("{e:?}") }),
        },
    )
}
