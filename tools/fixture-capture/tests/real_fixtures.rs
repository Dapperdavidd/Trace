//! Gate for REAL fixtures.
//!
//! * `reviewed` fixtures must reproduce their human-verified `expected`.
//! * `unreviewed` fixtures are listed, never counted.
//! * A protocol may be Partial or Enabled only with enough reviewed fixtures.
//!
//! Synthetic fixtures in crates/chain-adapters/tests/*.rs do not count here.

use fixture_capture::{evm, sha256_hex, solana, FIXTURE_ROOT, MIN_REVIEWED_FIXTURES};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use trace_chain_adapters::registry::protocols;
use trace_domain::{ChainId, SupportStatus};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(FIXTURE_ROOT)
}

fn load() -> Vec<(PathBuf, Value)> {
    let mut out = vec![];
    let mut stack = vec![root()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json") {
                let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap())
                    .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                out.push((p, v));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn is_reviewed(v: &Value) -> bool {
    v["review"]["status"] == "reviewed"
}

fn redo_draft(v: &Value) -> Value {
    let protocol = v["protocol"].as_str().unwrap();
    match v["kind"].as_str().unwrap() {
        "evm_v2_tx" => {
            let chain: ChainId = v["chain"].as_str().unwrap().parse().unwrap();
            let routers: Vec<String> = v["routers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|r| r.as_str().map(String::from))
                .collect();
            let pairs: BTreeMap<String, Value> = v["pairs"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(k, x)| (k.clone(), x.clone()))
                .collect();
            evm::draft(chain, &v["payload"], &pairs, &routers).unwrap()
        }
        "solana_tx" => {
            let programs: Vec<String> = v["program_ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|r| r.as_str().map(String::from))
                .collect();
            solana::draft(protocol, &programs, &v["payload"]).unwrap()
        }
        k => panic!("unknown fixture kind {k}"),
    }
}

#[test]
fn every_real_fixture_is_well_formed_and_untampered() {
    for (p, v) in load() {
        let name = p.display().to_string();
        assert_eq!(v["schema"], 1, "{name}");
        for k in [
            "chain",
            "protocol",
            "tx_id",
            "captured_at",
            "rpc_host",
            "payload_sha256",
        ] {
            assert!(
                v[k].as_str().is_some_and(|s| !s.is_empty()),
                "{name}: missing {k}"
            );
        }
        assert_eq!(
            v["payload_sha256"].as_str().unwrap(),
            sha256_hex(&v["payload"]),
            "{name}: payload was edited after capture"
        );
        let host = v["rpc_host"].as_str().unwrap();
        assert!(
            !host.contains(['/', '?', '@', ':']),
            "{name}: rpc_host must be a bare host"
        );
    }
}

#[test]
fn reviewed_fixtures_reproduce_their_verified_expectations() {
    for (p, v) in load().into_iter().filter(|(_, v)| is_reviewed(v)) {
        let name = p.display().to_string();
        assert!(
            !v["review"]["expected"].is_null(),
            "{name}: reviewed but expected is empty"
        );
        assert_eq!(
            redo_draft(&v),
            v["review"]["expected"],
            "{name}: parser output differs from the verified expectation"
        );
    }
}

#[test]
fn unreviewed_fixtures_are_reported_and_never_counted() {
    let pending: Vec<String> = load()
        .into_iter()
        .filter(|(_, v)| !is_reviewed(v))
        .map(|(p, _)| p.display().to_string())
        .collect();
    if !pending.is_empty() {
        eprintln!(
            "{} fixture(s) await human review:\n  {}",
            pending.len(),
            pending.join("\n  ")
        );
    }
}

#[test]
fn nothing_is_partial_or_enabled_without_reviewed_real_fixtures() {
    let mut have: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for (_, v) in load().into_iter().filter(|(_, v)| is_reviewed(v)) {
        have.entry((
            v["chain"].as_str().unwrap().into(),
            v["protocol"].as_str().unwrap().into(),
        ))
        .or_default()
        .insert(v["tx_id"].as_str().unwrap().into());
    }
    for cap in protocols() {
        if matches!(cap.status, SupportStatus::Enabled | SupportStatus::Partial) {
            let n = have
                .get(&(cap.chain.as_str().into(), cap.protocol.into()))
                .map_or(0, BTreeSet::len);
            assert!(
                n >= MIN_REVIEWED_FIXTURES,
                "{}:{} is {:?} with {n} reviewed real fixture(s); needs {MIN_REVIEWED_FIXTURES} distinct transactions",
                cap.chain.as_str(),
                cap.protocol,
                cap.status
            );
        }
    }
}
