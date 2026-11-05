//! Bounded retention for high-growth operational tables.
//!
//! Retention is opt-in. No row is deleted unless at least one explicit
//! `TRACE_RETENTION_*_DAYS` setting is present.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use trace_jobs::{Failure, Job};

pub const PRUNE_RETENTION: &str = "prune_retention";
const MAX_DAYS: i64 = 3_650;
const DEFAULT_BATCH_SIZE: i64 = 1_000;
const MAX_BATCH_SIZE: i64 = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    pub raw_event_days: Option<i64>,
    pub job_attempt_days: Option<i64>,
    pub audit_event_days: Option<i64>,
    pub batch_size: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub raw_events: u64,
    pub job_attempts: u64,
    pub audit_events: u64,
}

impl Policy {
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let raw_event_days = env_days("TRACE_RETENTION_RAW_EVENT_DAYS", 30)?;
        let job_attempt_days = env_days("TRACE_RETENTION_JOB_ATTEMPT_DAYS", 7)?;
        let audit_event_days = env_days("TRACE_RETENTION_AUDIT_EVENT_DAYS", 90)?;
        if raw_event_days.is_none() && job_attempt_days.is_none() && audit_event_days.is_none() {
            return Ok(None);
        }
        let batch_size = match std::env::var("TRACE_RETENTION_BATCH_SIZE") {
            Ok(value) => value
                .parse::<i64>()
                .map_err(|_| anyhow::anyhow!("TRACE_RETENTION_BATCH_SIZE must be an integer"))?,
            Err(std::env::VarError::NotPresent) => DEFAULT_BATCH_SIZE,
            Err(error) => return Err(error.into()),
        };
        if !(1..=MAX_BATCH_SIZE).contains(&batch_size) {
            anyhow::bail!("TRACE_RETENTION_BATCH_SIZE must be between 1 and {MAX_BATCH_SIZE}");
        }
        Ok(Some(Self {
            raw_event_days,
            job_attempt_days,
            audit_event_days,
            batch_size,
        }))
    }

    fn payload(&self) -> serde_json::Value {
        serde_json::json!({
            "raw_event_days": self.raw_event_days,
            "job_attempt_days": self.job_attempt_days,
            "audit_event_days": self.audit_event_days,
            "batch_size": self.batch_size,
        })
    }

    fn from_job(job: &Job) -> Result<Self, Failure> {
        let optional_days = |name: &str, minimum: i64| -> Result<Option<i64>, Failure> {
            match job.payload.get(name) {
                Some(serde_json::Value::Null) | None => Ok(None),
                Some(value) => {
                    let days = value.as_i64().ok_or_else(|| {
                        Failure::Permanent(format!("{name} must be an integer or null"))
                    })?;
                    validate_days(name, days, minimum).map(Some)
                }
            }
        };
        let batch_size = job.payload["batch_size"]
            .as_i64()
            .ok_or_else(|| Failure::Permanent("batch_size must be an integer".into()))?;
        if !(1..=MAX_BATCH_SIZE).contains(&batch_size) {
            return Err(Failure::Permanent(format!(
                "batch_size must be between 1 and {MAX_BATCH_SIZE}"
            )));
        }
        Ok(Self {
            raw_event_days: optional_days("raw_event_days", 30)?,
            job_attempt_days: optional_days("job_attempt_days", 7)?,
            audit_event_days: optional_days("audit_event_days", 90)?,
            batch_size,
        })
    }
}

fn env_days(name: &str, minimum: i64) -> anyhow::Result<Option<i64>> {
    let value = match std::env::var(name) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let days = value
        .parse::<i64>()
        .map_err(|_| anyhow::anyhow!("{name} must be an integer"))?;
    validate_days(name, days, minimum)
        .map(Some)
        .map_err(|error| {
            anyhow::anyhow!(match error {
                Failure::Permanent(message) | Failure::Transient(message) => message,
            })
        })
}

fn validate_days(name: &str, days: i64, minimum: i64) -> Result<i64, Failure> {
    if !(minimum..=MAX_DAYS).contains(&days) {
        return Err(Failure::Permanent(format!(
            "{name} must be between {minimum} and {MAX_DAYS} days"
        )));
    }
    Ok(days)
}

pub async fn schedule(
    pool: &PgPool,
    policy: Option<&Policy>,
    now: DateTime<Utc>,
) -> Result<bool, trace_jobs::JobError> {
    let Some(policy) = policy else {
        return Ok(false);
    };
    let day = now.timestamp() / 86_400;
    Ok(trace_jobs::enqueue(
        pool,
        PRUNE_RETENTION,
        Some(&format!("{PRUNE_RETENTION}:{day}")),
        policy.payload(),
        now,
        3,
    )
    .await?
    .is_some())
}

pub async fn run(pool: &PgPool, job: &Job, now: DateTime<Utc>) -> Result<Report, Failure> {
    let policy = Policy::from_job(job)?;
    prune(pool, &policy, now)
        .await
        .map_err(|error| Failure::Transient(error.to_string()))
}

pub async fn prune(
    pool: &PgPool,
    policy: &Policy,
    now: DateTime<Utc>,
) -> Result<Report, sqlx::Error> {
    let mut report = Report::default();
    if let Some(days) = policy.raw_event_days {
        report.raw_events = sqlx::query(
            "WITH doomed AS (
               SELECT id FROM raw_chain_events
               WHERE fixture=FALSE AND ingested_at < $1
               ORDER BY ingested_at, id LIMIT $2
             )
             DELETE FROM raw_chain_events AS target USING doomed
             WHERE target.id=doomed.id",
        )
        .bind(now - Duration::days(days))
        .bind(policy.batch_size)
        .execute(pool)
        .await?
        .rows_affected();
    }
    if let Some(days) = policy.job_attempt_days {
        report.job_attempts = sqlx::query(
            "WITH doomed AS (
               SELECT id FROM job_attempts
               WHERE finished_at IS NOT NULL AND finished_at < $1
               ORDER BY finished_at, id LIMIT $2
             )
             DELETE FROM job_attempts AS target USING doomed
             WHERE target.id=doomed.id",
        )
        .bind(now - Duration::days(days))
        .bind(policy.batch_size)
        .execute(pool)
        .await?
        .rows_affected();
    }
    if let Some(days) = policy.audit_event_days {
        report.audit_events = sqlx::query(
            "WITH doomed AS (
               SELECT id FROM audit_events
               WHERE created_at < $1
               ORDER BY created_at, id LIMIT $2
             )
             DELETE FROM audit_events AS target USING doomed
             WHERE target.id=doomed.id",
        )
        .bind(now - Duration::days(days))
        .bind(policy.batch_size)
        .execute(pool)
        .await?
        .rows_affected();
    }
    Ok(report)
}
