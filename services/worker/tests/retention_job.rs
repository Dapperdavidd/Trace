use chrono::{DateTime, Duration, TimeZone, Utc};
use sqlx::PgPool;
use trace_worker::retention_job::{prune, schedule, Policy, PRUNE_RETENTION};
use uuid::Uuid;

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

fn policy(batch_size: i64) -> Policy {
    Policy {
        raw_event_days: Some(30),
        job_attempt_days: Some(7),
        audit_event_days: Some(90),
        batch_size,
    }
}

async fn raw(pool: &PgPool, tx: &str, old: bool, fixture: bool) {
    let at = if old {
        now() - Duration::days(31)
    } else {
        now() - Duration::days(1)
    };
    sqlx::query(
        "INSERT INTO raw_chain_events
         (chain,tx_id,event_index,source,payload,payload_sha256,fixture,observed_at,ingested_at)
         VALUES ('ethereum',$1,0,'test','{}','hash',$2,$3,$3)",
    )
    .bind(tx)
    .bind(fixture)
    .bind(at)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn retention_is_opt_in_and_scheduled_once_per_day(pool: PgPool) {
    assert!(!schedule(&pool, None, now()).await.unwrap());
    assert!(schedule(&pool, Some(&policy(100)), now()).await.unwrap());
    assert!(!schedule(&pool, Some(&policy(100)), now()).await.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM worker_jobs WHERE kind=$1")
        .bind(PRUNE_RETENTION)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn prune_respects_age_fixture_and_finished_attempt_guards(pool: PgPool) {
    raw(&pool, "0xold", true, false).await;
    raw(&pool, "0xfixture", true, true).await;
    raw(&pool, "0xrecent", false, false).await;

    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO worker_jobs (id,kind,status,payload) VALUES ($1,'test','succeeded','{}')",
    )
    .bind(job_id)
    .execute(&pool)
    .await
    .unwrap();
    for (started, finished) in [
        (now() - Duration::days(9), Some(now() - Duration::days(8))),
        (now() - Duration::days(9), None),
        (now() - Duration::days(2), Some(now() - Duration::days(1))),
    ] {
        sqlx::query("INSERT INTO job_attempts (job_id,started_at,finished_at) VALUES ($1,$2,$3)")
            .bind(job_id)
            .bind(started)
            .bind(finished)
            .execute(&pool)
            .await
            .unwrap();
    }

    for (action, at) in [
        ("old", now() - Duration::days(91)),
        ("recent", now() - Duration::days(1)),
    ] {
        sqlx::query(
            "INSERT INTO audit_events (id,actor,action,created_at) VALUES ($1,'test',$2,$3)",
        )
        .bind(Uuid::new_v4())
        .bind(action)
        .bind(at)
        .execute(&pool)
        .await
        .unwrap();
    }

    let report = prune(&pool, &policy(100), now()).await.unwrap();
    assert_eq!(
        (report.raw_events, report.job_attempts, report.audit_events),
        (1, 1, 1)
    );
    let raw_left: Vec<(String, bool)> =
        sqlx::query_as("SELECT tx_id,fixture FROM raw_chain_events ORDER BY tx_id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        raw_left,
        vec![("0xfixture".into(), true), ("0xrecent".into(), false)]
    );
    let attempts_left: i64 = sqlx::query_scalar("SELECT count(*) FROM job_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts_left, 2);
    let audits_left: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_events ORDER BY action")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(audits_left, vec!["recent"]);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn prune_is_bounded_per_table(pool: PgPool) {
    for index in 0..3 {
        raw(&pool, &format!("0xold{index}"), true, false).await;
    }
    let report = prune(
        &pool,
        &Policy {
            raw_event_days: Some(30),
            job_attempt_days: None,
            audit_event_days: None,
            batch_size: 2,
        },
        now(),
    )
    .await
    .unwrap();
    assert_eq!(report.raw_events, 2);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM raw_chain_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 1);
}
