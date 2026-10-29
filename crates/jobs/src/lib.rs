//! Durable job queue on PostgreSQL.
//!
//! * `enqueue` is idempotent on `dedupe_key`.
//! * `lease` uses `FOR UPDATE SKIP LOCKED`, so concurrent workers never share a job.
//! * A job whose lease expired (worker crashed) is picked up again, until it
//!   runs out of attempts, then it is marked `dead` and kept for inspection.
//! * Failures are classed transient (retry with capped exponential backoff) or
//!   permanent (dead immediately). Nothing is ever deleted.

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub id: Uuid,
    pub kind: String,
    pub payload: Value,
    pub attempts: i32,
    pub max_attempts: i32,
    attempt_row: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// Worth retrying: timeouts, rate limits, upstream 5xx.
    Transient(String),
    /// Retrying cannot help: bad payload, unsupported input.
    Permanent(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailOutcome {
    Retrying { run_after: DateTime<Utc> },
    Dead,
}

pub const BACKOFF_BASE_SECS: i64 = 30;
pub const BACKOFF_CAP_SECS: i64 = 3_600;

/// 30s, 60s, 120s ... capped at one hour.
pub fn backoff(attempts: i32) -> Duration {
    let exp = attempts.clamp(1, 20) as u32 - 1;
    Duration::seconds((BACKOFF_BASE_SECS.saturating_mul(1i64 << exp)).min(BACKOFF_CAP_SECS))
}

/// Returns `None` when a job with the same `dedupe_key` already exists.
pub async fn enqueue(
    pool: &PgPool,
    kind: &str,
    dedupe_key: Option<&str>,
    payload: Value,
    run_after: DateTime<Utc>,
    max_attempts: i32,
) -> Result<Option<Uuid>, JobError> {
    let id = Uuid::new_v4();
    let r = sqlx::query(
        r#"INSERT INTO worker_jobs (id, kind, dedupe_key, payload, status, max_attempts, run_after)
           VALUES ($1,$2,$3,$4,'queued',$5,$6) ON CONFLICT (dedupe_key) DO NOTHING"#,
    )
    .bind(id)
    .bind(kind)
    .bind(dedupe_key)
    .bind(payload)
    .bind(max_attempts.max(1))
    .bind(run_after)
    .execute(pool)
    .await?;
    Ok((r.rows_affected() == 1).then_some(id))
}

/// Take the next due job of one of `kinds`, or `None`.
pub async fn lease(
    pool: &PgPool,
    now: DateTime<Utc>,
    lease_for: Duration,
    kinds: &[&str],
) -> Result<Option<Job>, JobError> {
    let kinds: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
    loop {
        let mut tx = pool.begin().await?;
        let row = sqlx::query(
            r#"SELECT id, kind, payload, attempts, max_attempts, status FROM worker_jobs
               WHERE kind = ANY($1)
                 AND ((status = 'queued' AND run_after <= $2)
                   OR (status = 'running' AND locked_until < $2))
               ORDER BY run_after, created_at LIMIT 1 FOR UPDATE SKIP LOCKED"#,
        )
        .bind(&kinds)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let id: Uuid = row.get("id");
        let attempts: i32 = row.get("attempts");
        let max: i32 = row.get("max_attempts");
        if attempts >= max {
            // A crashed worker used up the last attempt. Keep it, do not run it again.
            sqlx::query("UPDATE worker_jobs SET status='dead', locked_until=NULL, last_error=COALESCE(last_error,'lease expired after the final attempt'), updated_at=$2 WHERE id=$1")
                .bind(id)
                .bind(now)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            continue;
        }
        sqlx::query("UPDATE worker_jobs SET status='running', attempts=attempts+1, locked_until=$2, updated_at=$3 WHERE id=$1")
            .bind(id)
            .bind(now + lease_for)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        let attempt_row: i64 = sqlx::query_scalar(
            "INSERT INTO job_attempts (job_id, started_at) VALUES ($1,$2) RETURNING id",
        )
        .bind(id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(Some(Job {
            id,
            kind: row.get("kind"),
            payload: row.get("payload"),
            attempts: attempts + 1,
            max_attempts: max,
            attempt_row,
        }));
    }
}

pub async fn succeed(pool: &PgPool, job: &Job, now: DateTime<Utc>) -> Result<(), JobError> {
    sqlx::query("UPDATE worker_jobs SET status='succeeded', locked_until=NULL, last_error=NULL, updated_at=$2 WHERE id=$1")
        .bind(job.id)
        .bind(now)
        .execute(pool)
        .await?;
    sqlx::query("UPDATE job_attempts SET finished_at=$2, outcome='succeeded' WHERE id=$1")
        .bind(job.attempt_row)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn fail(
    pool: &PgPool,
    job: &Job,
    failure: &Failure,
    now: DateTime<Utc>,
) -> Result<FailOutcome, JobError> {
    let (class, msg) = match failure {
        Failure::Transient(m) => ("transient", m),
        Failure::Permanent(m) => ("permanent", m),
    };
    let msg: String = msg.chars().take(500).collect();
    sqlx::query("UPDATE job_attempts SET finished_at=$2, outcome='failed', failure_class=$3, error=$4 WHERE id=$1")
        .bind(job.attempt_row)
        .bind(now)
        .bind(class)
        .bind(&msg)
        .execute(pool)
        .await?;
    let dead = class == "permanent" || job.attempts >= job.max_attempts;
    if dead {
        sqlx::query("UPDATE worker_jobs SET status='dead', locked_until=NULL, last_error=$2, updated_at=$3 WHERE id=$1")
            .bind(job.id)
            .bind(&msg)
            .bind(now)
            .execute(pool)
            .await?;
        return Ok(FailOutcome::Dead);
    }
    let run_after = now + backoff(job.attempts);
    sqlx::query("UPDATE worker_jobs SET status='queued', locked_until=NULL, run_after=$2, last_error=$3, updated_at=$4 WHERE id=$1")
        .bind(job.id)
        .bind(run_after)
        .bind(&msg)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(FailOutcome::Retrying { run_after })
}

/// Counts by status, for operators and health output.
pub async fn counts(pool: &PgPool) -> Result<Vec<(String, i64)>, JobError> {
    Ok(
        sqlx::query(
            "SELECT status, count(*) AS n FROM worker_jobs GROUP BY status ORDER BY status",
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| (r.get("status"), r.get("n")))
        .collect(),
    )
}
