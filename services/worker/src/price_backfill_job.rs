//! Bounded and resumable historical token-price intake.

use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};
use std::collections::HashSet;
use trace_domain::{AssetId, ChainId};
use trace_jobs::{Failure, Job};
use trace_pricing::{accept_historical, HistoricalRequest};
use uuid::Uuid;

use crate::ingest_job::with_advisory_lock;
use crate::ingest_job::Ctx;

pub const BACKFILL_PRICES: &str = "backfill_prices";
pub const POINTS_PER_JOB: usize = 40;
pub const MAX_POINTS_PER_RUN: i64 = 10_000;

#[derive(Debug, Clone, FromRow)]
pub struct PriceBackfillRun {
    pub id: Uuid,
    pub chain: String,
    pub asset: String,
    pub from_time: DateTime<Utc>,
    pub to_time: DateTime<Utc>,
    pub next_time: DateTime<Utc>,
    pub interval_seconds: i64,
    pub status: String,
    pub stored_points: i64,
    pub rejected_points: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PriceBackfillReport {
    pub requested: usize,
    pub stored: usize,
    pub duplicates: usize,
    pub rejected: usize,
    pub completed: bool,
}

pub async fn request(
    pool: &PgPool,
    chain: &str,
    asset: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    interval_seconds: i64,
) -> Result<Uuid, anyhow::Error> {
    if from > to {
        anyhow::bail!("from time must not exceed to time");
    }
    if !(300..=86_400).contains(&interval_seconds) {
        anyhow::bail!("interval must be between 300 and 86400 seconds");
    }
    let span = (to - from).num_seconds();
    let points = span / interval_seconds + 1;
    if points > MAX_POINTS_PER_RUN {
        anyhow::bail!(
            "one price backfill may contain at most {MAX_POINTS_PER_RUN} points; split this range"
        );
    }
    let chain_id: ChainId = chain.parse()?;
    let chain = chain_id.as_str();
    let parsed = AssetId::parse(chain_id, asset)?;
    let decimals: Option<i16> =
        sqlx::query_scalar("SELECT decimals FROM tokens WHERE chain=$1 AND contract_or_mint=$2")
            .bind(chain)
            .bind(parsed.contract_or_mint.as_str())
            .fetch_optional(pool)
            .await?
            .flatten();
    if decimals.is_none() {
        anyhow::bail!("token must exist with verified decimals before price backfill");
    }

    let id = Uuid::new_v4();
    let stored: Uuid = sqlx::query_scalar(
        "INSERT INTO price_backfill_runs
         (id, chain, asset, from_time, to_time, next_time, interval_seconds, status)
         VALUES ($1,$2,$3,$4,$5,$4,$6,'queued')
         ON CONFLICT (chain, asset, from_time, to_time, interval_seconds)
         DO UPDATE SET updated_at=price_backfill_runs.updated_at RETURNING id",
    )
    .bind(id)
    .bind(chain)
    .bind(parsed.contract_or_mint.as_str())
    .bind(from)
    .bind(to)
    .bind(interval_seconds)
    .fetch_one(pool)
    .await?;
    Ok(stored)
}

pub async fn list(pool: &PgPool, id: Option<Uuid>) -> Result<Vec<PriceBackfillRun>, sqlx::Error> {
    let select = "SELECT id, chain, asset, from_time, to_time, next_time, interval_seconds,
                  status, stored_points, rejected_points, last_error FROM price_backfill_runs";
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
        "UPDATE price_backfill_runs SET status='cancelled', updated_at=now()
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
    enabled: bool,
    now: DateTime<Utc>,
) -> Result<usize, trace_jobs::JobError> {
    if !enabled {
        return Ok(0);
    }
    let rows: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, next_time FROM price_backfill_runs
         WHERE status IN ('queued','running') ORDER BY created_at LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    let mut count = 0;
    for (id, next) in rows {
        let key = format!(
            "{BACKFILL_PRICES}:{id}:{}:{}",
            next.timestamp(),
            now.timestamp() / 60
        );
        if trace_jobs::enqueue(
            pool,
            BACKFILL_PRICES,
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

pub async fn run(pool: &PgPool, ctx: &Ctx, job: &Job) -> Result<PriceBackfillReport, Failure> {
    let id = job.payload["run_id"]
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| Failure::Permanent("price backfill job has no valid run_id".into()))?;
    let lock = format!("price-backfill:{id}");
    let Some(result) = with_advisory_lock(pool, &lock, run_id(pool, ctx, id)).await? else {
        return Ok(PriceBackfillReport::default());
    };
    result
}

async fn run_id(pool: &PgPool, ctx: &Ctx, id: Uuid) -> Result<PriceBackfillReport, Failure> {
    let run = list(pool, Some(id))
        .await
        .map_err(|error| Failure::Transient(error.to_string()))?
        .into_iter()
        .next()
        .ok_or_else(|| Failure::Permanent(format!("price backfill run {id} does not exist")))?;
    if matches!(run.status.as_str(), "completed" | "cancelled") {
        return Ok(PriceBackfillReport {
            completed: true,
            ..Default::default()
        });
    }
    let provider = ctx
        .prices
        .as_ref()
        .and_then(|prices| prices.historical.as_deref())
        .ok_or_else(|| Failure::Permanent("historical price provider is not configured".into()))?;
    let chain: ChainId = run
        .chain
        .parse()
        .map_err(|error| Failure::Permanent(format!("invalid stored chain: {error}")))?;
    let asset = AssetId::parse(chain, &run.asset)
        .map_err(|error| Failure::Permanent(format!("invalid stored asset: {error}")))?;
    let decimals: Option<i16> =
        sqlx::query_scalar("SELECT decimals FROM tokens WHERE chain=$1 AND contract_or_mint=$2")
            .bind(&run.chain)
            .bind(&run.asset)
            .fetch_one(pool)
            .await
            .map_err(|error| Failure::Transient(error.to_string()))?;
    let decimals = decimals
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| Failure::Permanent("token decimals are not verified".into()))?;

    let interval = Duration::seconds(run.interval_seconds);
    let mut at = run.next_time;
    let mut requests = Vec::new();
    while at <= run.to_time && requests.len() < POINTS_PER_JOB {
        requests.push(HistoricalRequest {
            asset: asset.clone(),
            at,
        });
        at += interval;
    }
    if requests.is_empty() {
        finish(pool, id).await?;
        return Ok(PriceBackfillReport {
            completed: true,
            ..Default::default()
        });
    }
    let batch = match provider.fetch_historical(&requests).await {
        Ok(batch) => batch,
        Err(error) => {
            let message = error.to_string();
            let _ = sqlx::query(
                "UPDATE price_backfill_runs SET last_error=$2, updated_at=now() WHERE id=$1",
            )
            .bind(id)
            .bind(&message)
            .execute(pool)
            .await;
            return Err(Failure::Transient(message));
        }
    };
    let mut report = PriceBackfillReport {
        requested: requests.len(),
        rejected: batch.rejected.len(),
        ..Default::default()
    };
    let mut matched = HashSet::new();
    for quote in batch.quotes {
        let Some((index, requested)) = requests
            .iter()
            .enumerate()
            .filter(|(index, _)| !matched.contains(index))
            .min_by_key(|(_, request)| {
                (quote.observed_at - request.at)
                    .num_seconds()
                    .unsigned_abs()
            })
        else {
            report.rejected += 1;
            continue;
        };
        matched.insert(index);
        let accepted = match accept_historical(&quote, Some(decimals), requested.at) {
            Ok(accepted) => accepted,
            Err(_) => {
                report.rejected += 1;
                continue;
            }
        };
        let result = sqlx::query(
            "INSERT INTO price_points
             (chain, asset, observed_at, usd_per_base_unit, source, demo)
             VALUES ($1,$2,$3,$4,$5,FALSE) ON CONFLICT DO NOTHING",
        )
        .bind(accepted.asset.chain.as_str())
        .bind(accepted.asset.contract_or_mint.as_str())
        .bind(accepted.observed_at)
        .bind(accepted.usd_per_base_unit)
        .bind(provider.name())
        .execute(pool)
        .await
        .map_err(|error| Failure::Transient(error.to_string()))?;
        if result.rows_affected() == 1 {
            report.stored += 1;
        } else {
            report.duplicates += 1;
        }
    }
    report.rejected += requests
        .len()
        .saturating_sub(matched.len() + batch.rejected.len());
    report.completed = at > run.to_time;
    sqlx::query(
        "UPDATE price_backfill_runs SET next_time=$2,
         status=CASE WHEN $3 THEN 'completed' ELSE 'running' END,
         stored_points=stored_points+$4, rejected_points=rejected_points+$5,
         completed_at=CASE WHEN $3 THEN now() ELSE NULL END,
         last_error=NULL, updated_at=now() WHERE id=$1",
    )
    .bind(id)
    .bind(at)
    .bind(report.completed)
    .bind(i64::try_from(report.stored).unwrap_or(i64::MAX))
    .bind(i64::try_from(report.rejected).unwrap_or(i64::MAX))
    .execute(pool)
    .await
    .map_err(|error| Failure::Transient(error.to_string()))?;
    Ok(report)
}

async fn finish(pool: &PgPool, id: Uuid) -> Result<(), Failure> {
    sqlx::query(
        "UPDATE price_backfill_runs SET status='completed', completed_at=now(),
         last_error=NULL, updated_at=now() WHERE id=$1",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|error| Failure::Transient(error.to_string()))?;
    Ok(())
}
