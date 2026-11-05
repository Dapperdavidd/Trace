//! Bounded and resumable historical EVM ingestion.
//!
//! Backfill progress lives outside the live ingestion cursor. Only finalized
//! blocks are processed, and each job advances at most `CHUNK_SIZE` blocks.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use trace_domain::{Address, SupportStatus};
use trace_jobs::{Failure, Job};
use uuid::Uuid;

use crate::ingest::{process_range, EvmIngestConfig, EvmSource, IngestError, StepReport};
use crate::ingest_job::{deployments, with_advisory_lock, Ctx};

pub const BACKFILL_EVM: &str = "backfill_evm";
pub const CHUNK_SIZE: u64 = 200;
pub const MAX_RUN_BLOCKS: u64 = 2_000_000;

#[derive(Debug, Clone, FromRow)]
pub struct BackfillRun {
    pub id: Uuid,
    pub chain: String,
    pub protocol: String,
    pub from_height: i64,
    pub to_height: i64,
    pub next_height: i64,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BackfillResult {
    Stepped(StepReport),
    WaitingForFinality,
    AlreadyRunning,
    Finished,
}

pub async fn request(
    pool: &PgPool,
    chain: &str,
    protocol: &str,
    from: u64,
    to: u64,
) -> Result<Uuid, anyhow::Error> {
    if from > to {
        anyhow::bail!("from height must not exceed to height");
    }
    let count = to
        .checked_sub(from)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| anyhow::anyhow!("block range overflow"))?;
    if count > MAX_RUN_BLOCKS {
        anyhow::bail!(
            "one backfill run may contain at most {MAX_RUN_BLOCKS} blocks; split this range"
        );
    }
    if !deployments()
        .iter()
        .any(|d| d.chain.as_str() == chain && d.protocol == protocol)
    {
        anyhow::bail!("unknown EVM deployment {chain}:{protocol}");
    }

    let id = Uuid::new_v4();
    let stored: Uuid = sqlx::query_scalar(
        "INSERT INTO backfill_runs
         (id, chain, protocol, from_height, to_height, next_height, status)
         VALUES ($1,$2,$3,$4,$5,$4,'queued')
         ON CONFLICT (chain, protocol, from_height, to_height)
         DO UPDATE SET updated_at=backfill_runs.updated_at
         RETURNING id",
    )
    .bind(id)
    .bind(chain)
    .bind(protocol)
    .bind(i64::try_from(from)?)
    .bind(i64::try_from(to)?)
    .fetch_one(pool)
    .await?;
    Ok(stored)
}

pub async fn list(pool: &PgPool, id: Option<Uuid>) -> Result<Vec<BackfillRun>, sqlx::Error> {
    if let Some(id) = id {
        sqlx::query_as(
            "SELECT id, chain, protocol, from_height, to_height, next_height, status, last_error
             FROM backfill_runs WHERE id=$1",
        )
        .bind(id)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as(
            "SELECT id, chain, protocol, from_height, to_height, next_height, status, last_error
             FROM backfill_runs ORDER BY created_at DESC LIMIT 100",
        )
        .fetch_all(pool)
        .await
    }
}

pub async fn cancel(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let changed = sqlx::query(
        "UPDATE backfill_runs SET status='cancelled', updated_at=now()
         WHERE id=$1 AND status IN ('queued','running')",
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(changed == 1)
}

pub async fn schedule(
    pool: &PgPool,
    ctx: &Ctx,
    now: DateTime<Utc>,
) -> Result<usize, trace_jobs::JobError> {
    let rows: Vec<(Uuid, String, String, i64)> = sqlx::query_as(
        "SELECT id, chain, protocol, next_height FROM backfill_runs
         WHERE status IN ('queued','running') ORDER BY created_at LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(trace_jobs::JobError::Db)?;
    let bucket = now.timestamp() / 60;
    let targets = ctx.targets();
    let mut scheduled = 0;
    for (id, chain, protocol, next) in rows {
        if !targets
            .iter()
            .any(|d| d.chain.as_str() == chain && d.protocol == protocol)
        {
            continue;
        }
        trace_jobs::enqueue(
            pool,
            BACKFILL_EVM,
            Some(&format!("{BACKFILL_EVM}:{id}:{next}:{bucket}")),
            serde_json::json!({ "run_id": id }),
            now,
            5,
        )
        .await?;
        scheduled += 1;
    }
    Ok(scheduled)
}

pub async fn run(pool: &PgPool, ctx: &Ctx, job: &Job) -> Result<BackfillResult, Failure> {
    let id = job.payload["run_id"]
        .as_str()
        .and_then(|v| Uuid::parse_str(v).ok())
        .ok_or_else(|| Failure::Permanent("backfill job has no valid run_id".into()))?;
    let run: BackfillRun = sqlx::query_as(
        "SELECT id, chain, protocol, from_height, to_height, next_height, status, last_error
         FROM backfill_runs WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Failure::Transient(e.to_string()))?
    .ok_or_else(|| Failure::Permanent(format!("backfill run {id} does not exist")))?;

    if matches!(run.status.as_str(), "completed" | "cancelled") {
        return Ok(BackfillResult::Finished);
    }
    let d = deployments()
        .iter()
        .find(|d| d.chain.as_str() == run.chain && d.protocol == run.protocol)
        .ok_or_else(|| {
            Failure::Permanent(format!("unknown deployment {}:{}", run.chain, run.protocol))
        })?;
    let status = ctx
        .protocols
        .iter()
        .find(|p| p.chain == d.chain && p.protocol == d.protocol)
        .map_or(SupportStatus::Unsupported, |p| p.status);
    if !matches!(status, SupportStatus::Enabled | SupportStatus::Partial) {
        return Err(Failure::Permanent(format!(
            "{}:{} is {status:?}; backfill is paused",
            run.chain, run.protocol
        )));
    }
    let make = ctx
        .source
        .as_ref()
        .ok_or_else(|| Failure::Permanent("EVM ingestion is not configured".into()))?;
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
        max_range: CHUNK_SIZE,
        reorg_window: 64,
        start_height: None,
    };
    let lock = format!("ingest:{}:{}", run.chain, run.protocol);
    let Some(result) =
        with_advisory_lock(pool, &lock, run_locked(pool, src.as_ref(), &cfg, &run)).await?
    else {
        return Ok(BackfillResult::AlreadyRunning);
    };
    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            let message = error.to_string();
            let _ =
                sqlx::query("UPDATE backfill_runs SET last_error=$2, updated_at=now() WHERE id=$1")
                    .bind(id)
                    .bind(&message)
                    .execute(pool)
                    .await;
            match error {
                IngestError::NotEnabled { .. } => Err(Failure::Permanent(message)),
                _ => Err(Failure::Transient(message)),
            }
        }
    }
}

async fn run_locked(
    pool: &PgPool,
    src: &dyn EvmSource,
    cfg: &EvmIngestConfig,
    run: &BackfillRun,
) -> Result<BackfillResult, IngestError> {
    if run.next_height > run.to_height {
        finish(pool, run.id).await?;
        return Ok(BackfillResult::Finished);
    }
    let head = src.head().await?;
    let Some(finalized) = head.finalized else {
        mark_running(pool, run.id).await?;
        return Ok(BackfillResult::WaitingForFinality);
    };
    let from = run.next_height as u64;
    if from > finalized {
        mark_running(pool, run.id).await?;
        return Ok(BackfillResult::WaitingForFinality);
    }
    let target = run.to_height as u64;
    let to = target.min(finalized).min(from + CHUNK_SIZE - 1);
    let report = process_range(pool, src, cfg, head, from, to).await?;
    let next = i64::try_from(to + 1)
        .map_err(|e| IngestError::Source(format!("backfill height overflow: {e}")))?;
    let completed = next > run.to_height;
    sqlx::query(
        "UPDATE backfill_runs SET next_height=$2,
         status=CASE WHEN $3 THEN 'completed' ELSE 'running' END,
         completed_at=CASE WHEN $3 THEN now() ELSE NULL END,
         last_error=NULL, updated_at=now() WHERE id=$1",
    )
    .bind(run.id)
    .bind(next)
    .bind(completed)
    .execute(pool)
    .await
    .map_err(trace_database::DbError::Sqlx)?;
    Ok(BackfillResult::Stepped(report))
}

async fn mark_running(pool: &PgPool, id: Uuid) -> Result<(), IngestError> {
    sqlx::query(
        "UPDATE backfill_runs SET status='running', last_error=NULL, updated_at=now() WHERE id=$1",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(trace_database::DbError::Sqlx)?;
    Ok(())
}

async fn finish(pool: &PgPool, id: Uuid) -> Result<(), IngestError> {
    sqlx::query(
        "UPDATE backfill_runs SET status='completed', completed_at=now(),
         last_error=NULL, updated_at=now() WHERE id=$1",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(trace_database::DbError::Sqlx)?;
    Ok(())
}
