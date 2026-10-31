//! Scheduling and running Solana ingestion as queue jobs. Same gating as EVM:
//! nothing runs unless the registry marks the protocol Enabled or Partial and
//! `TRACE_RPC_SOLANA` is set.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use trace_chain_adapters::registry::ProtocolCapability;
use trace_chain_adapters::solana::ProgramAllow;
use trace_domain::{ChainId, SupportStatus};
use trace_jobs::{Failure, Job};

use crate::ingest::IngestError;
use crate::ingest_job::{with_advisory_lock, Ctx};
use crate::solana_ingest::{step, SolanaIngestConfig, SolanaReport, SolanaSource};

pub const INGEST_SOLANA: &str = "ingest_solana";

pub struct SolanaDeployment {
    pub protocol: &'static str,
    pub poll_program: &'static str,
    pub programs: &'static [ProgramAllow],
}

static RAYDIUM_V4: [ProgramAllow; 1] = [ProgramAllow {
    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    protocol: "raydium_amm_v4",
}];

static DEPLOYMENTS: [SolanaDeployment; 1] = [SolanaDeployment {
    protocol: "raydium_amm_v4",
    poll_program: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    programs: &RAYDIUM_V4,
}];

/// Program ids must be verified against an explorer when a protocol is enabled.
pub fn deployments() -> &'static [SolanaDeployment] {
    &DEPLOYMENTS
}

pub type SolanaSourceFactory =
    Box<dyn Fn() -> Result<Box<dyn SolanaSource>, IngestError> + Send + Sync>;

fn status(protocols: &[ProtocolCapability], d: &SolanaDeployment) -> SupportStatus {
    protocols
        .iter()
        .find(|p| p.chain == ChainId::Solana && p.protocol == d.protocol)
        .map_or(SupportStatus::Unsupported, |p| p.status)
}

/// Deployments that may be ingested right now.
pub fn targets(ctx: &Ctx) -> Vec<&'static SolanaDeployment> {
    deployments()
        .iter()
        .filter(|d| {
            matches!(
                status(&ctx.protocols, d),
                SupportStatus::Enabled | SupportStatus::Partial
            ) && ctx.rpc_urls.contains_key("solana")
                && ctx.solana_source.is_some()
        })
        .collect()
}

pub async fn schedule_solana(
    pool: &PgPool,
    ctx: &Ctx,
    now: DateTime<Utc>,
) -> Result<usize, trace_jobs::JobError> {
    let bucket = now.timestamp() / 60;
    let mut n = 0;
    for d in targets(ctx) {
        let key = format!("{INGEST_SOLANA}:{}:{bucket}", d.protocol);
        trace_jobs::enqueue(
            pool,
            INGEST_SOLANA,
            Some(&key),
            serde_json::json!({ "protocol": d.protocol }),
            now,
            3,
        )
        .await?;
        n += 1;
    }
    Ok(n)
}

#[derive(Debug, PartialEq, Eq)]
pub enum SolanaRun {
    Stepped(SolanaReport),
    AlreadyRunning,
}

pub async fn run_solana(pool: &PgPool, ctx: &Ctx, job: &Job) -> Result<SolanaRun, Failure> {
    let protocol = job.payload["protocol"].as_str().unwrap_or_default();
    let d = deployments()
        .iter()
        .find(|d| d.protocol == protocol)
        .ok_or_else(|| Failure::Permanent(format!("unknown solana deployment '{protocol}'")))?;
    let st = status(&ctx.protocols, d);
    if !matches!(st, SupportStatus::Enabled | SupportStatus::Partial) {
        return Err(Failure::Permanent(format!(
            "solana:{protocol} is {st:?}; not ingesting"
        )));
    }
    let make = ctx.solana_source.as_ref().ok_or_else(|| {
        Failure::Permanent("solana ingestion is not configured on this worker".into())
    })?;
    let src = make().map_err(|e| Failure::Permanent(e.to_string()))?;
    let cfg = SolanaIngestConfig {
        protocol: d.protocol.into(),
        status: st,
        poll_program: d.poll_program,
        programs: d.programs,
        page: 200,
        max_backlog: 1000,
        max_per_step: 100,
    };
    let key = format!("ingest:solana:{protocol}");
    let out = with_advisory_lock(pool, &key, step(pool, src.as_ref(), &cfg)).await?;
    match out {
        None => Ok(SolanaRun::AlreadyRunning),
        Some(Ok(r)) => Ok(SolanaRun::Stepped(r)),
        Some(Err(IngestError::NotEnabled { .. })) => {
            Err(Failure::Permanent("protocol not enabled".into()))
        }
        Some(Err(e)) => Err(Failure::Transient(e.to_string())),
    }
}
