//! Scheduling and running ingestion as queue jobs.
//!
//! Nothing is scheduled unless the registry marks the protocol Enabled or
//! Partial and an RPC URL is configured.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use trace_chain_adapters::registry::{self, ProtocolCapability};
use trace_domain::{Address, ChainId, SupportStatus};
use trace_jobs::{Failure, Job};

use crate::ingest::{step, EvmIngestConfig, EvmSource, IngestError, StepReport};
use crate::rpc_source::EvmRpcSource;

pub const INGEST_EVM: &str = "ingest_evm";

/// A known protocol deployment. Addresses must be verified against a block
/// explorer when the protocol is enabled (see docs/fixtures.md).
pub struct Deployment {
    pub chain: ChainId,
    pub protocol: &'static str,
    pub factory: &'static str,
    pub routers: &'static [&'static str],
}

pub fn deployments() -> &'static [Deployment] {
    &[Deployment {
        chain: ChainId::Ethereum,
        protocol: "uniswap_v2",
        factory: "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f",
        routers: &["0x7a250d5630b4cf539739df2c5dacb4c659f2488d"],
    }]
}

pub type SourceFactory =
    Box<dyn Fn(&Deployment) -> Result<Box<dyn EvmSource>, IngestError> + Send + Sync>;

/// Everything an ingest job needs from outside the database.
pub struct Ctx {
    pub protocols: Vec<ProtocolCapability>,
    pub rpc_urls: HashMap<String, String>,
    /// `None` disables ingest jobs entirely (they are not even leased).
    pub source: Option<SourceFactory>,
    /// `None` disables the price poll job (it is not even leased).
    pub prices: Option<crate::price_job::PriceCtx>,
}

impl Ctx {
    pub fn empty() -> Self {
        Self {
            protocols: registry::protocols(),
            rpc_urls: HashMap::new(),
            source: None,
            prices: None,
        }
    }

    /// RPC URLs come from `TRACE_RPC_<CHAIN>` (for example `TRACE_RPC_ETHEREUM`).
    pub fn from_env() -> Self {
        let rpc_urls: HashMap<String, String> = deployments()
            .iter()
            .filter_map(|d| {
                let key = format!("TRACE_RPC_{}", d.chain.as_str().to_uppercase());
                std::env::var(key)
                    .ok()
                    .filter(|v| !v.is_empty())
                    .map(|v| (d.chain.as_str().to_string(), v))
            })
            .collect();
        let urls = rpc_urls.clone();
        let source: SourceFactory = Box::new(move |d| {
            let url = urls.get(d.chain.as_str()).cloned().ok_or_else(|| {
                IngestError::Source(format!("no RPC URL configured for {}", d.chain))
            })?;
            Ok(Box::new(EvmRpcSource::new(
                url, d.chain, d.protocol, d.factory,
            )?))
        });
        Self {
            protocols: registry::protocols(),
            rpc_urls,
            source: Some(source),
            prices: prices_from_env(),
        }
    }

    fn status(&self, d: &Deployment) -> SupportStatus {
        self.protocols
            .iter()
            .find(|p| p.chain == d.chain && p.protocol == d.protocol)
            .map_or(SupportStatus::Unsupported, |p| p.status)
    }

    /// Deployments that may be ingested right now.
    pub fn targets(&self) -> Vec<&'static Deployment> {
        deployments()
            .iter()
            .filter(|d| {
                matches!(
                    self.status(d),
                    SupportStatus::Enabled | SupportStatus::Partial
                ) && self.rpc_urls.contains_key(d.chain.as_str())
            })
            .collect()
    }
}

pub async fn schedule_ingest(
    pool: &PgPool,
    ctx: &Ctx,
    now: DateTime<Utc>,
) -> Result<usize, trace_jobs::JobError> {
    let bucket = now.timestamp() / 60;
    let mut n = 0;
    for d in ctx.targets() {
        let key = format!("{INGEST_EVM}:{}:{}:{bucket}", d.chain, d.protocol);
        trace_jobs::enqueue(
            pool,
            INGEST_EVM,
            Some(&key),
            serde_json::json!({ "chain": d.chain.as_str(), "protocol": d.protocol }),
            now,
            3,
        )
        .await?;
        n += 1;
    }
    Ok(n)
}

#[derive(Debug, PartialEq, Eq)]
pub enum IngestRun {
    Stepped(StepReport),
    /// Another worker holds the lock for this protocol. Nothing was done.
    AlreadyRunning,
}

pub async fn run_ingest(pool: &PgPool, ctx: &Ctx, job: &Job) -> Result<IngestRun, Failure> {
    let chain = job.payload["chain"].as_str().unwrap_or_default();
    let protocol = job.payload["protocol"].as_str().unwrap_or_default();
    let d = deployments()
        .iter()
        .find(|d| d.chain.as_str() == chain && d.protocol == protocol)
        .ok_or_else(|| Failure::Permanent(format!("unknown deployment {chain}:{protocol}")))?;
    // Re-check at run time: a job queued before a protocol was disabled must not run.
    let status = ctx.status(d);
    if !matches!(status, SupportStatus::Enabled | SupportStatus::Partial) {
        return Err(Failure::Permanent(format!(
            "{chain}:{protocol} is {status:?}; not ingesting"
        )));
    }
    let make = ctx
        .source
        .as_ref()
        .ok_or_else(|| Failure::Permanent("ingestion is not configured on this worker".into()))?;
    let src = make(d).map_err(|e| Failure::Permanent(e.to_string()))?;
    let routers = d
        .routers
        .iter()
        .map(|r| Address::parse(d.chain, r))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| Failure::Permanent(e.to_string()))?;
    let cfg = EvmIngestConfig {
        chain: d.chain,
        protocol: d.protocol.into(),
        status,
        routers,
        max_range: 200,
        reorg_window: 64,
        start_height: None,
    };

    // One worker per protocol at a time. The lock is released even if the step fails.
    let lock = format!("ingest:{chain}:{protocol}");
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| Failure::Transient(e.to_string()))?;
    let got: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtextextended($1, 0))")
        .bind(&lock)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| Failure::Transient(e.to_string()))?;
    if !got {
        return Ok(IngestRun::AlreadyRunning);
    }
    let result = step(pool, src.as_ref(), &cfg).await;
    let _ = sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
        .bind(&lock)
        .fetch_one(&mut *conn)
        .await;
    match result {
        Ok(r) => Ok(IngestRun::Stepped(r)),
        Err(IngestError::NotEnabled { .. }) => {
            Err(Failure::Permanent("protocol not enabled".into()))
        }
        Err(e) => Err(Failure::Transient(e.to_string())),
    }
}

/// `TRACE_PRICE_PROVIDER=defillama` turns price polling on. `TRACE_PRICE_API_BASE` overrides the host.
fn prices_from_env() -> Option<crate::price_job::PriceCtx> {
    match std::env::var("TRACE_PRICE_PROVIDER").ok().as_deref() {
        Some("defillama") => {
            let base = std::env::var("TRACE_PRICE_API_BASE")
                .unwrap_or_else(|_| trace_pricing::defillama::DEFAULT_BASE.to_string());
            match trace_pricing::defillama::DefiLlama::new(&base) {
                Ok(p) => Some(crate::price_job::PriceCtx {
                    provider: std::sync::Arc::new(p),
                }),
                Err(e) => {
                    tracing::error!(error = %e, "price provider could not start");
                    None
                }
            }
        }
        Some(other) if !other.is_empty() => {
            tracing::error!(
                provider = other,
                "unknown TRACE_PRICE_PROVIDER; price polling is off"
            );
            None
        }
        _ => None,
    }
}
