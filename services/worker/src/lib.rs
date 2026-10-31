pub mod ingest;
pub mod ingest_job;
pub mod outcomes;
pub mod price_job;
pub mod rpc_source;
pub mod solana_ingest;
pub mod solana_job;
pub mod solana_rpc;

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use trace_jobs::{Failure, Job};

pub const EVALUATE_OUTCOMES: &str = "evaluate_signal_outcomes";
pub const KINDS: &[&str] = &[EVALUATE_OUTCOMES];

/// Queue the periodic jobs. Keys are bucketed so repeated calls in the same
/// window enqueue one job.
pub async fn schedule_periodic(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<(), trace_jobs::JobError> {
    let bucket = now.timestamp() / 300;
    trace_jobs::enqueue(
        pool,
        EVALUATE_OUTCOMES,
        Some(&format!("{EVALUATE_OUTCOMES}:{bucket}")),
        serde_json::json!({}),
        now,
        3,
    )
    .await?;
    Ok(())
}

pub async fn run_job(pool: &PgPool, job: &Job, now: DateTime<Utc>) -> Result<(), Failure> {
    run_job_with(pool, job, now, &ingest_job::Ctx::empty()).await
}

pub async fn run_job_with(
    pool: &PgPool,
    job: &Job,
    now: DateTime<Utc>,
    ctx: &ingest_job::Ctx,
) -> Result<(), Failure> {
    match job.kind.as_str() {
        solana_job::INGEST_SOLANA => solana_job::run_solana(pool, ctx, job)
            .await
            .map(|r| tracing::info!(?r, "solana ingest step")),
        price_job::POLL_PRICES => match &ctx.prices {
            Some(p) => price_job::poll(pool, p.provider.as_ref(), now)
                .await
                .map(|r| tracing::info!(?r, "prices polled")),
            None => Err(Failure::Permanent(
                "price provider is not configured".into(),
            )),
        },
        ingest_job::INGEST_EVM => ingest_job::run_ingest(pool, ctx, job)
            .await
            .map(|r| tracing::info!(?r, "ingest step")),
        EVALUATE_OUTCOMES => outcomes::evaluate_due(pool, now)
            .await
            .map(|r| tracing::info!(?r, "signal outcomes evaluated"))
            .map_err(|e| Failure::Transient(e.to_string())),
        other => Err(Failure::Permanent(format!("unknown job kind '{other}'"))),
    }
}

/// Lease and run due jobs until none remain. Returns how many ran.
pub async fn drain(pool: &PgPool, now: DateTime<Utc>) -> Result<usize, trace_jobs::JobError> {
    drain_with(pool, now, &ingest_job::Ctx::empty()).await
}

pub async fn drain_with(
    pool: &PgPool,
    now: DateTime<Utc>,
    ctx: &ingest_job::Ctx,
) -> Result<usize, trace_jobs::JobError> {
    // Ingest jobs are leased only by a worker that can run them.
    let mut kinds: Vec<&str> = KINDS.to_vec();
    if ctx.source.is_some() {
        kinds.push(ingest_job::INGEST_EVM);
    }
    if ctx.solana_source.is_some() {
        kinds.push(solana_job::INGEST_SOLANA);
    }
    if ctx.prices.is_some() {
        kinds.push(price_job::POLL_PRICES);
    }
    let mut n = 0;
    while let Some(job) = trace_jobs::lease(pool, now, Duration::minutes(5), &kinds).await? {
        n += 1;
        match run_job_with(pool, &job, now, ctx).await {
            Ok(()) => trace_jobs::succeed(pool, &job, now).await?,
            Err(f) => {
                tracing::warn!(job = %job.id, kind = %job.kind, failure = ?f, "job failed");
                trace_jobs::fail(pool, &job, &f, now).await?;
            }
        }
        if n >= 100 {
            break;
        }
    }
    Ok(n)
}
