//! Capture real transactions as golden fixtures.
//!
//! The network is a thin shell (`rpc`). Everything that decides what ends up
//! in a fixture is pure and tested here with synthetic RPC responses.
//!
//! Rules:
//! * The RPC URL is never stored. It usually carries an API key. Only the host
//!   name is recorded.
//! * The tool never writes `expected`. It may write a `draft` taken from the
//!   current parser, which is a starting point for a human reviewer to compare
//!   against a block explorer. A draft is not proof; the parser cannot grade itself.

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub mod evm;
pub mod rpc;
pub mod solana;

pub const SCHEMA: u32 = 1;
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
pub struct Fixture {
    pub schema: u32,
    pub kind: &'static str,
    pub chain: String,
    pub protocol: String,
    pub tx_id: String,
    pub captured_at: DateTime<Utc>,
    /// Host name only. Never the URL, path or query.
    pub rpc_host: String,
    pub tool_version: &'static str,
    pub payload_sha256: String,
    pub payload: Value,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub pairs: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub routers: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub program_ids: Vec<String>,
    pub review: Review,
}

#[derive(Debug, Clone, Serialize)]
pub struct Review {
    /// Always "unreviewed" when written by this tool.
    pub status: &'static str,
    /// What the human reviewer must do before this fixture counts.
    pub instructions: &'static str,
    /// Parser output at capture time. Not evidence.
    pub draft: Value,
    /// Always null here. A reviewer fills it in after comparing with a block explorer.
    pub expected: Value,
}

pub const REVIEW_INSTRUCTIONS: &str =
    "Open this transaction in a block explorer. Check wallet, tokens and amounts by hand. \
Copy the draft into 'expected' only for values you verified, then set status to 'reviewed'.";

pub fn sha256_hex(v: &Value) -> String {
    // serde_json sorts keys only for maps built from BTreeMap; here the payload is
    // written once and hashed from its compact serialization.
    hex::encode(Sha256::digest(serde_json::to_vec(v).unwrap_or_default()))
}

/// Host name of an RPC URL, or an error for anything that is not http(s).
pub fn host_of(url: &str) -> anyhow::Result<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| anyhow::anyhow!("RPC URL must start with http:// or https://"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // drop user:pass@ and :port
    let host = authority
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default();
    if host.is_empty() {
        anyhow::bail!("RPC URL has no host");
    }
    Ok(host.to_lowercase())
}

pub fn hex_u64(v: &Value, what: &str) -> anyhow::Result<u64> {
    let s = v
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("{what} is missing"))?;
    u64::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|_| anyhow::anyhow!("{what} is not a hex number: {s}"))
}

pub fn ts(secs: i64) -> anyhow::Result<DateTime<Utc>> {
    Utc.timestamp_opt(secs, 0)
        .single()
        .ok_or_else(|| anyhow::anyhow!("bad timestamp {secs}"))
}

pub fn review(draft: Value) -> Review {
    Review {
        status: "unreviewed",
        instructions: REVIEW_INSTRUCTIONS,
        draft,
        expected: Value::Null,
    }
}

pub fn path_for(chain: &str, tx_id: &str) -> String {
    let safe: String = tx_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(88)
        .collect();
    format!("crates/chain-adapters/tests/fixtures/real/{chain}/{safe}.json")
}

/// Minimum reviewed real fixtures a protocol needs before it may be `Partial` or `Enabled`.
pub const MIN_REVIEWED_FIXTURES: usize = 3;

pub const FIXTURE_ROOT: &str = "crates/chain-adapters/tests/fixtures/real";
