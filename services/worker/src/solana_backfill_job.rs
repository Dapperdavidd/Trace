//! Bounded and resumable finalized Solana signature backfill.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use trace_domain::SupportStatus;
use trace_jobs::{Failure, Job};
use uuid::Uuid;

use crate::ingest::IngestError;
use crate::ingest_job::{with_advisory_lock, Ctx};
use crate::solana_ingest::{process_signatures, SolanaIngestConfig, SolanaReport};
use crate::solana_job::{deployments, status};

pub const BACKFILL_SOLANA: &str = "backfill_solana";
pub const PAGE: usize = 100;
pub const MAX_SIGNATURES: i64 = 1_000_000;

#[derive(Debug, Clone, FromRow)]
pub struct SolanaBackfillRun {
    pub id: Uuid,
    pub protocol: String,
    pub before_signature: String,
    pub until_signature: String,
    pub next_before: String,
    pub max_signatures: i64,
    pub processed: i64,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SolanaBackfillReport {
    pub signatures: usize,
    pub ingestion: SolanaReport,
    pub completed: bool,
    pub limit_reached: bool,
    pub already_running: bool,
}

pub async fn request(
    pool: &PgPool,
    protocol: &str,
    before_signature: &str,
    until_signature: &str,
    max_signatures: i64,
) -> Result<Uuid, anyhow::Error> {
    if before_signature.is_empty() || until_signature.is_empty() {
        anyhow::bail!("both signature boundaries are required");
    }
    if before_signature == until_signature {
        anyhow::bail!("signature boundaries must differ");
    }
    if !(1..=MAX_SIGNATURES).contains(&max_signatures) {
        anyhow::bail!("max signatures must be between 1 and {MAX_SIGNATURES}");
    }
    if !deployments().iter().any(|d| d.protocol == protocol) {
        anyhow::bail!("unknown Solana protocol {protocol}");
    }
    let id = Uuid::new_v4();
    let stored: Uuid = sqlx::query_scalar(
        "INSERT INTO solana_backfill_runs
         (id,protocol,before_signature,until_signature,next_before,max_signatures,status)
         VALUES ($1,$2,$3,$4,$3,$5,'queued')
         ON CONFLICT (protocol,before_signature,until_signature)
         DO UPDATE SET updated_at=solana_backfill_runs.updated_at RETURNING id",
    )
    .bind(id)
    .bind(protocol)
    .bind(before_signature)
    .bind(until_signature)
    .bind(max_signatures)
    .fetch_one(pool)
    .await?;
    Ok(stored)
}

pub async fn list(pool: &PgPool, id: Option<Uuid>) -> Result<Vec<SolanaBackfillRun>, sqlx::Error> {
    let select = "SELECT id,protocol,before_signature,until_signature,next_before,
                  max_signatures,processed,status,last_error FROM solana_backfill_runs";
    if let Some(id) = id {
        sqlx::query_as(&format!("{select} WHERE id=$1"))
            .bind(id)
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as(&format!("{select} ORDER BY created_at DESC LIMIT 100"))
            .fetch_all(pool)
            .await
    }
}

pub async fn cancel(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query(
        "UPDATE solana_backfill_runs SET status='cancelled',updated_at=now()
         WHERE id=$1 AND status IN ('queued','running')",
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

pub async fn schedule(
    pool: &PgPool,
    ctx: &Ctx,
    now: DateTime<Utc>,
) -> Result<usize, trace_jobs::JobError> {
    let active: Vec<&str> = crate::solana_job::targets(ctx)
        .into_iter()
        .map(|d| d.protocol)
        .collect();
    if active.is_empty() {
        return Ok(0);
    }
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id,protocol,next_before FROM solana_backfill_runs
         WHERE status IN ('queued','running') ORDER BY created_at LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    let mut count = 0;
    for (id, protocol, next) in rows {
        if !active.contains(&protocol.as_str()) {
            continue;
        }
        let key = format!("{BACKFILL_SOLANA}:{id}:{next}:{}", now.timestamp() / 60);
        if trace_jobs::enqueue(
            pool,
            BACKFILL_SOLANA,
            Some(&key),
            serde_json::json!({ "run_id": id }),
            now,
            5,
        )
        .await?
        .is_some()
        {
            count += 1;
        }
    }
    Ok(count)
}

pub async fn run(pool: &PgPool, ctx: &Ctx, job: &Job) -> Result<SolanaBackfillReport, Failure> {
    let id = job.payload["run_id"]
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| Failure::Permanent("Solana backfill job has no valid run_id".into()))?;
    let run = list(pool, Some(id))
        .await
        .map_err(|error| Failure::Transient(error.to_string()))?
        .into_iter()
        .next()
        .ok_or_else(|| Failure::Permanent(format!("Solana backfill run {id} does not exist")))?;
    if matches!(
        run.status.as_str(),
        "completed" | "limit_reached" | "cancelled"
    ) {
        return Ok(SolanaBackfillReport {
            completed: run.status == "completed",
            limit_reached: run.status == "limit_reached",
            ..Default::default()
        });
    }
    let deployment = deployments()
        .iter()
        .find(|d| d.protocol == run.protocol)
        .ok_or_else(|| Failure::Permanent(format!("unknown protocol {}", run.protocol)))?;
    let support = status(&ctx.protocols, deployment);
    if !matches!(support, SupportStatus::Enabled | SupportStatus::Partial) {
        return Err(Failure::Permanent(format!(
            "solana:{} is {support:?}; backfill is paused",
            run.protocol
        )));
    }
    let make = ctx
        .solana_source
        .as_ref()
        .ok_or_else(|| Failure::Permanent("Solana RPC is not configured".into()))?;
    let source = make().map_err(|error| Failure::Permanent(error.to_string()))?;
    let cfg = SolanaIngestConfig {
        protocol: deployment.protocol.into(),
        status: support,
        poll_program: deployment.poll_program,
        programs: deployment.programs,
        page: PAGE,
        max_backlog: PAGE,
        max_per_step: PAGE,
    };
    let key = format!("ingest:solana:{}", run.protocol);
    let Some(result) =
        with_advisory_lock(pool, &key, run_locked(pool, source.as_ref(), &cfg, &run)).await?
    else {
        return Ok(SolanaBackfillReport {
            already_running: true,
            ..Default::default()
        });
    };
    match result {
        Ok(report) => Ok(report),
        Err(error) => {
            let message = error.to_string();
            let _ = sqlx::query(
                "UPDATE solana_backfill_runs SET last_error=$2,updated_at=now() WHERE id=$1",
            )
            .bind(id)
            .bind(&message)
            .execute(pool)
            .await;
            Err(Failure::Transient(message))
        }
    }
}

async fn run_locked(
    pool: &PgPool,
    source: &dyn crate::solana_ingest::SolanaSource,
    cfg: &SolanaIngestConfig,
    run: &SolanaBackfillRun,
) -> Result<SolanaBackfillReport, IngestError> {
    let remaining = (run.max_signatures - run.processed).max(0) as usize;
    if remaining == 0 {
        set_status(pool, run.id, "limit_reached").await?;
        return Ok(SolanaBackfillReport {
            limit_reached: true,
            ..Default::default()
        });
    }
    let request_limit = PAGE.min(remaining.saturating_add(1));
    let mut signatures = source
        .signatures(
            cfg.poll_program,
            Some(&run.next_before),
            Some(&run.until_signature),
            request_limit,
        )
        .await?;
    if signatures.is_empty() {
        set_status(pool, run.id, "completed").await?;
        return Ok(SolanaBackfillReport {
            completed: true,
            ..Default::default()
        });
    }
    let exceeded = signatures.len() > remaining;
    signatures.truncate(remaining);
    let next_before = signatures.last().unwrap().signature.clone();
    let reached_boundary = signatures.len() < request_limit && !exceeded;
    let mut ordered = signatures.clone();
    ordered.reverse();
    let ingestion = process_signatures(pool, source, cfg, &ordered).await?;
    let processed = i64::try_from(signatures.len()).unwrap_or(i64::MAX);
    let new_processed = run.processed.saturating_add(processed);
    let limit_reached = exceeded || new_processed >= run.max_signatures && !reached_boundary;
    let completed = reached_boundary && !limit_reached;
    let state = if limit_reached {
        "limit_reached"
    } else if completed {
        "completed"
    } else {
        "running"
    };
    sqlx::query(
        "UPDATE solana_backfill_runs SET next_before=$2,processed=$3,status=$4,
         completed_at=CASE WHEN $4 IN ('completed','limit_reached') THEN now() ELSE NULL END,
         last_error=NULL,updated_at=now() WHERE id=$1",
    )
    .bind(run.id)
    .bind(next_before)
    .bind(new_processed)
    .bind(state)
    .execute(pool)
    .await
    .map_err(trace_database::DbError::from)?;
    Ok(SolanaBackfillReport {
        signatures: signatures.len(),
        ingestion,
        completed,
        limit_reached,
        already_running: false,
    })
}

async fn set_status(pool: &PgPool, id: Uuid, status: &str) -> Result<(), IngestError> {
    sqlx::query(
        "UPDATE solana_backfill_runs SET status=$2,completed_at=now(),
         last_error=NULL,updated_at=now() WHERE id=$1",
    )
    .bind(id)
    .bind(status)
    .execute(pool)
    .await
    .map_err(trace_database::DbError::from)?;
    Ok(())
}
