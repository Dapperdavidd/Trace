//! capture-fixture: save a real transaction as an unreviewed golden fixture.
//!
//!   RPC_URL=https://... capture-fixture evm --chain ethereum --protocol uniswap_v2 --tx 0x... [--router 0x...]
//!   RPC_URL=https://... capture-fixture solana --protocol <name> --program <program id> --tx <signature>
//!
//! The RPC URL is read from the environment only (so it stays out of shell
//! history and is never stored). Run from the repository root.

use anyhow::{bail, Context, Result};
use fixture_capture::{evm, path_for, rpc::Rpc, solana};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::str::FromStr;
use trace_domain::ChainId;

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn flags(args: &[String], name: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter(|(_, a)| *a == name)
        .filter_map(|(i, _)| args.get(i + 1).cloned())
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(kind) = args.first().map(String::as_str) else {
        bail!("usage: capture-fixture <evm|solana> --protocol P --tx TX [...]  (RPC_URL in the environment)");
    };
    let url = std::env::var("RPC_URL").context("set RPC_URL in the environment")?;
    let rpc = Rpc::new(url.clone())?;
    let protocol = flag(&args, "--protocol").context("--protocol is required")?;
    let tx = flag(&args, "--tx").context("--tx is required")?;

    let fixture = match kind {
        "evm" => {
            let chain = ChainId::from_str(&flag(&args, "--chain").context("--chain is required")?)?;
            if chain.family() != trace_domain::chain::VmFamily::Evm {
                bail!("{chain} is not an EVM chain");
            }
            let routers = flags(&args, "--router");
            let receipt = rpc.call("eth_getTransactionReceipt", json!([tx])).await?;
            let transaction = rpc.call("eth_getTransactionByHash", json!([tx])).await?;
            let bn = receipt["blockNumber"].clone();
            let block = rpc.call("eth_getBlockByNumber", json!([bn, false])).await?;
            // Not every chain/provider supports the "finalized" tag; absence means "not proven final".
            let finalized_block = rpc
                .call("eth_getBlockByNumber", json!(["finalized", false]))
                .await
                .ok();
            let mut pair_calls = BTreeMap::new();
            for pair in evm::swap_pairs(&receipt) {
                let call = |sel: &str| json!([{ "to": pair, "data": sel }, "latest"]);
                let t0 = rpc.call("eth_call", call(evm::TOKEN0_SELECTOR)).await?;
                let t1 = rpc.call("eth_call", call(evm::TOKEN1_SELECTOR)).await?;
                pair_calls.insert(
                    pair.clone(),
                    (
                        t0.as_str().unwrap_or_default().to_string(),
                        t1.as_str().unwrap_or_default().to_string(),
                    ),
                );
            }
            evm::assemble(
                chain,
                &protocol,
                &url,
                &routers,
                &evm::Raw {
                    receipt,
                    transaction,
                    block,
                    finalized_block,
                    pair_calls,
                },
            )?
        }
        "solana" => {
            let programs = flags(&args, "--program");
            if programs.is_empty() {
                bail!("--program is required at least once");
            }
            let result: Value = rpc
                .call("getTransaction", json!([tx, { "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0, "commitment": "finalized" }]))
                .await?;
            solana::assemble(&protocol, &url, &tx, &programs, &result)?
        }
        other => bail!("unknown kind '{other}'"),
    };

    let path = path_for(&fixture.chain, &fixture.tx_id);
    if std::path::Path::new(&path).exists() {
        bail!("{path} already exists. Delete it deliberately if you want to recapture.");
    }
    std::fs::create_dir_all(std::path::Path::new(&path).parent().context("path")?)?;
    std::fs::write(&path, serde_json::to_string_pretty(&fixture)? + "\n")?;
    println!("wrote {path}");
    println!("status: unreviewed. Verify it against a block explorer, then fill in 'expected' and set status to 'reviewed'.");
    println!(
        "draft from the current parser (not evidence): {}",
        fixture.review.draft
    );
    Ok(())
}
