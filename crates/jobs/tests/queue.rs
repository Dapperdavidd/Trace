use chrono::{Duration, TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use trace_jobs::*;

fn t0() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

async fn status_of(pool: &PgPool, id: uuid::Uuid) -> String {
    sqlx::query_scalar("SELECT status FROM worker_jobs WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn enqueue_is_idempotent_on_the_dedupe_key(pool: PgPool) {
    let a = enqueue(&pool, "k", Some("same"), json!({}), t0(), 3)
        .await
        .unwrap();
    let b = enqueue(&pool, "k", Some("same"), json!({}), t0(), 3)
        .await
        .unwrap();
    assert!(a.is_some() && b.is_none());
    // no key means no dedupe
    assert!(enqueue(&pool, "k", None, json!({}), t0(), 3)
        .await
        .unwrap()
        .is_some());
    assert!(enqueue(&pool, "k", None, json!({}), t0(), 3)
        .await
        .unwrap()
        .is_some());
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn lease_respects_run_after_kind_and_order(pool: PgPool) {
    let later = enqueue(
        &pool,
        "k",
        Some("later"),
        json!({"n": 2}),
        t0() + Duration::minutes(5),
        3,
    )
    .await
    .unwrap()
    .unwrap();
    let first = enqueue(&pool, "k", Some("first"), json!({"n": 1}), t0(), 3)
        .await
        .unwrap()
        .unwrap();
    enqueue(&pool, "other", Some("o"), json!({}), t0(), 3)
        .await
        .unwrap();
    assert!(
        lease(
            &pool,
            t0() - Duration::seconds(1),
            Duration::minutes(1),
            &["k"]
        )
        .await
        .unwrap()
        .is_none(),
        "nothing is due yet"
    );
    let j = lease(&pool, t0(), Duration::minutes(1), &["k"])
        .await
        .unwrap()
        .unwrap();
    assert_eq!((j.id, j.attempts), (first, 1));
    assert_eq!(j.payload, json!({"n": 1}));
    succeed(&pool, &j, t0()).await.unwrap();
    assert!(
        lease(&pool, t0(), Duration::minutes(1), &["k"])
            .await
            .unwrap()
            .is_none(),
        "the later job is not due and the first is leased"
    );
    let j2 = lease(
        &pool,
        t0() + Duration::minutes(6),
        Duration::minutes(1),
        &["k"],
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(j2.id, later);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn concurrent_workers_never_share_a_job(pool: PgPool) {
    for i in 0..20 {
        enqueue(&pool, "k", Some(&format!("j{i}")), json!({}), t0(), 3)
            .await
            .unwrap();
    }
    let mut handles = vec![];
    for _ in 0..8 {
        let p = pool.clone();
        handles.push(tokio::spawn(async move {
            let mut got = vec![];
            while let Some(j) = lease(&p, t0(), Duration::minutes(1), &["k"]).await.unwrap() {
                got.push(j.id);
            }
            got
        }));
    }
    let mut all = vec![];
    for h in handles {
        all.extend(h.await.unwrap());
    }
    let n = all.len();
    all.sort();
    all.dedup();
    assert_eq!(n, 20);
    assert_eq!(all.len(), 20, "no job was handed out twice");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn success_is_recorded(pool: PgPool) {
    let id = enqueue(&pool, "k", None, json!({}), t0(), 3)
        .await
        .unwrap()
        .unwrap();
    let j = lease(&pool, t0(), Duration::minutes(1), &["k"])
        .await
        .unwrap()
        .unwrap();
    succeed(&pool, &j, t0()).await.unwrap();
    assert_eq!(status_of(&pool, id).await, "succeeded");
    assert!(lease(
        &pool,
        t0() + Duration::days(1),
        Duration::minutes(1),
        &["k"]
    )
    .await
    .unwrap()
    .is_none());
    let outcome: String = sqlx::query_scalar("SELECT outcome FROM job_attempts WHERE job_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outcome, "succeeded");
}

#[test]
fn backoff_doubles_and_is_capped() {
    let secs: Vec<i64> = (1..=10).map(|a| backoff(a).num_seconds()).collect();
    assert_eq!(&secs[..4], &[30, 60, 120, 240]);
    assert_eq!(*secs.last().unwrap(), 3_600);
    assert_eq!(backoff(1000).num_seconds(), 3_600, "no overflow");
    assert_eq!(backoff(0).num_seconds(), 30);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn transient_failures_retry_with_backoff_then_die(pool: PgPool) {
    let id = enqueue(&pool, "k", None, json!({}), t0(), 3)
        .await
        .unwrap()
        .unwrap();
    let mut now = t0();
    for attempt in 1..=3 {
        let j = lease(&pool, now, Duration::minutes(1), &["k"])
            .await
            .unwrap()
            .expect("due");
        assert_eq!(j.attempts, attempt);
        let out = fail(&pool, &j, &Failure::Transient("rpc timeout".into()), now)
            .await
            .unwrap();
        if attempt < 3 {
            let FailOutcome::Retrying { run_after } = out else {
                panic!("expected retry")
            };
            assert_eq!(run_after, now + backoff(attempt));
            assert_eq!(status_of(&pool, id).await, "queued");
            assert!(
                lease(&pool, now, Duration::minutes(1), &["k"])
                    .await
                    .unwrap()
                    .is_none(),
                "not due before the backoff ends"
            );
            now = run_after;
        } else {
            assert_eq!(out, FailOutcome::Dead);
        }
    }
    assert_eq!(status_of(&pool, id).await, "dead");
    let err: String = sqlx::query_scalar("SELECT last_error FROM worker_jobs WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(err, "rpc timeout", "the reason is kept");
    let attempts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM job_attempts WHERE job_id=$1 AND failure_class='transient'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(attempts, 3);
    assert!(
        lease(&pool, now + Duration::days(1), Duration::minutes(1), &["k"])
            .await
            .unwrap()
            .is_none(),
        "dead jobs stay dead"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn permanent_failures_are_dead_at_once(pool: PgPool) {
    let id = enqueue(&pool, "k", None, json!({}), t0(), 5)
        .await
        .unwrap()
        .unwrap();
    let j = lease(&pool, t0(), Duration::minutes(1), &["k"])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fail(&pool, &j, &Failure::Permanent("bad payload".into()), t0())
            .await
            .unwrap(),
        FailOutcome::Dead
    );
    assert_eq!(status_of(&pool, id).await, "dead");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_crashed_workers_job_is_recovered_after_its_lease_expires(pool: PgPool) {
    let id = enqueue(&pool, "k", None, json!({}), t0(), 3)
        .await
        .unwrap()
        .unwrap();
    let j = lease(&pool, t0(), Duration::minutes(1), &["k"])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(j.attempts, 1);
    // worker vanishes without reporting
    assert!(
        lease(
            &pool,
            t0() + Duration::seconds(30),
            Duration::minutes(1),
            &["k"]
        )
        .await
        .unwrap()
        .is_none(),
        "still leased"
    );
    let again = lease(
        &pool,
        t0() + Duration::minutes(2),
        Duration::minutes(1),
        &["k"],
    )
    .await
    .unwrap()
    .expect("recovered");
    assert_eq!((again.id, again.attempts), (id, 2));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn a_job_that_keeps_crashing_is_not_run_forever(pool: PgPool) {
    let id = enqueue(&pool, "k", None, json!({}), t0(), 2)
        .await
        .unwrap()
        .unwrap();
    let mut now = t0();
    for _ in 0..2 {
        assert!(lease(&pool, now, Duration::minutes(1), &["k"])
            .await
            .unwrap()
            .is_some());
        now += Duration::minutes(2);
    }
    assert!(lease(&pool, now, Duration::minutes(1), &["k"])
        .await
        .unwrap()
        .is_none());
    assert_eq!(status_of(&pool, id).await, "dead");
    let counts = counts(&pool).await.unwrap();
    assert_eq!(counts, vec![("dead".to_string(), 1)]);
}
