use actix_web::{http::StatusCode, test, web, App};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use trace_api::{configure, error, request_id::RequestId, AppState};

const TOKEN: &str = "a-long-random-ops-token-0123456789";

macro_rules! app {
    ($pool:expr, $token:expr) => {
        test::init_service(
            App::new()
                .app_data(web::Data::new(AppState::new($pool).with_ops_token($token)))
                .app_data(error::json_config())
                .wrap(RequestId)
                .configure(configure)
                .default_service(web::to(error::not_found)),
        )
        .await
    };
}

async fn get(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    auth: Option<&str>,
) -> (StatusCode, String) {
    let mut r = test::TestRequest::get().uri("/v1/ops/metrics");
    if let Some(a) = auth {
        r = r.insert_header(("authorization", a));
    }
    let res = test::call_service(app, r.to_request()).await;
    let status = res.status();
    (
        status,
        String::from_utf8(test::read_body(res).await.to_vec()).unwrap(),
    )
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn the_endpoint_does_not_exist_unless_a_token_is_configured(pool: PgPool) {
    let app = app!(pool, None);
    let (s, body) = get(&app, Some(&format!("Bearer {TOKEN}"))).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert!(!body.contains("trace_jobs"));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn it_requires_the_exact_bearer_token(pool: PgPool) {
    let app = app!(pool, Some(TOKEN.into()));
    for bad in [
        None,
        Some("Bearer wrong"),
        Some(TOKEN),
        Some("Bearer a-long-random-ops-token-012345678"),
        Some("bearer a-long-random-ops-token-0123456789"),
    ] {
        assert_eq!(get(&app, bad).await.0, StatusCode::UNAUTHORIZED, "{bad:?}");
    }
    assert_eq!(
        get(&app, Some(&format!("Bearer {TOKEN}"))).await.0,
        StatusCode::OK
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn metrics_are_read_from_the_database_and_exclude_demo_rows(pool: PgPool) {
    let now = Utc::now();
    sqlx::query("INSERT INTO worker_jobs (id, kind, status, run_after) VALUES (gen_random_uuid(), 'ingest_evm', 'dead', now())")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO worker_jobs (id, kind, status, run_after) VALUES (gen_random_uuid(), 'poll_prices', 'queued', $1)")
        .bind(now - Duration::seconds(120)).execute(&pool).await.unwrap();
    trace_database::coverage::upsert_protocol(
        &pool,
        "ethereum",
        "uniswap_v2",
        "enabled",
        &[],
        None,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE protocols SET last_successful_scan_at = $1 WHERE protocol = 'uniswap_v2' AND chain = 'ethereum'")
        .bind(now - Duration::seconds(90))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ingestion_cursors (chain, protocol, cursor, height) VALUES ('ethereum','uniswap_v2','0xabc',12345)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO audit_events (id, actor, action, subject, detail) VALUES (gen_random_uuid(),'indexer','ingest_gap','solana:x','{}')")
        .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO tokens (chain, contract_or_mint) VALUES ('ethereum','0x000000000000000000000000000000000000d001')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO price_points (chain, asset, observed_at, usd_per_base_unit, source, demo) VALUES ('ethereum','0x000000000000000000000000000000000000d001', now(), 1, 'demo_seed', TRUE)")
        .execute(&pool).await.unwrap();

    let app = app!(pool, Some(TOKEN.into()));
    let (s, m) = get(&app, Some(&format!("Bearer {TOKEN}"))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        m.contains("trace_jobs{kind=\"ingest_evm\",status=\"dead\"} 1"),
        "{m}"
    );
    assert!(m.contains("trace_jobs{kind=\"poll_prices\",status=\"queued\"} 1"));
    let age: f64 = m
        .lines()
        .find(|l| l.starts_with("trace_oldest_due_job_age_seconds "))
        .unwrap()
        .rsplit(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((115.0..180.0).contains(&age), "{age}");
    let scan: f64 = m
        .lines()
        .find(|l| {
            l.starts_with(
                "trace_ingest_last_scan_age_seconds{chain=\"ethereum\",protocol=\"uniswap_v2\"}",
            )
        })
        .unwrap()
        .rsplit(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((85.0..150.0).contains(&scan), "{scan}");
    assert!(
        m.contains("trace_ingest_cursor_height{chain=\"ethereum\",protocol=\"uniswap_v2\"} 12345")
    );
    assert!(m.contains("trace_ingest_gaps 1"));
    assert!(
        !m.lines()
            .any(|l| l.starts_with("trace_price_latest_age_seconds ")),
        "no real price yet, so the metric is absent, not zero"
    );
    assert!(m.contains("# TYPE trace_swaps gauge"));
}
