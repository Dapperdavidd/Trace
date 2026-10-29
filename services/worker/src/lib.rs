pub mod outcomes;

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
    match job.kind.as_str() {
        EVALUATE_OUTCOMES => outcomes::evaluate_due(pool, now)
            .await
            .map(|r| tracing::info!(?r, "signal outcomes evaluated"))
            .map_err(|e| Failure::Transient(e.to_string())),
        other => Err(Failure::Permanent(format!("unknown job kind '{other}'"))),
    }
}

/// Lease and run due jobs until none remain. Returns how many ran.
pub async fn drain(pool: &PgPool, now: DateTime<Utc>) -> Result<usize, trace_jobs::JobError> {
    let mut n = 0;
    while let Some(job) = trace_jobs::lease(pool, now, Duration::minutes(5), KINDS).await? {
        n += 1;
        match run_job(pool, &job, now).await {
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
