use actix_web::{http::StatusCode, test, web, App};
use serde_json::Value;
use sqlx::PgPool;
use trace_api::{configure, error, request_id::RequestId, AppState};

macro_rules! app {
    ($pool:expr) => {
        test::init_service(
            App::new()
                .app_data(web::Data::new(AppState::new($pool)))
                .app_data(error::json_config())
                .wrap(RequestId)
                .configure(configure)
                .default_service(web::to(error::not_found)),
        )
        .await
    };
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn live_and_ready_are_ok(pool: PgPool) {
    let app = app!(pool);
    let live = test::call_service(
        &app,
        test::TestRequest::get().uri("/v1/health/live").to_request(),
    )
    .await;
    assert_eq!(live.status(), StatusCode::OK);
    let ready = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/health/ready")
            .to_request(),
    )
    .await;
    assert_eq!(ready.status(), StatusCode::OK);
    let body: Value = test::read_body_json(ready).await;
    assert_eq!(body["database"], "ok");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn ready_reports_503_with_envelope_when_database_is_down(pool: PgPool) {
    let app = app!(pool.clone());
    pool.close().await;
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/health/ready")
            .to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "dependency_unavailable");
    assert!(!body["request_id"].as_str().unwrap().is_empty());
    assert!(body.get("stack").is_none());
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn chains_never_claims_coverage_it_has_not_earned(pool: PgPool) {
    let app = app!(pool);
    let res = test::call_service(
        &app,
        test::TestRequest::get().uri("/v1/chains").to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = test::read_body_json(res).await;
    let chains = body["chains"].as_array().unwrap();
    assert_eq!(chains.len(), 4);
    assert!(chains.iter().all(|c| c["watch_only_supported"] == true));
    assert!(chains.iter().all(|c| c["status"] != "enabled"));
    assert_eq!(body["data_status"], "unavailable");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn unknown_routes_use_error_envelope_and_echo_request_id(pool: PgPool) {
    let app = app!(pool);
    let req = test::TestRequest::get()
        .uri("/v1/nope")
        .insert_header(("x-request-id", "abc-123"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(res.headers().get("x-request-id").unwrap(), "abc-123");
    let body: Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "not_found");
    assert_eq!(body["request_id"], "abc-123");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn hostile_request_ids_are_replaced(pool: PgPool) {
    let app = app!(pool);
    let req = test::TestRequest::get()
        .uri("/v1/health/live")
        .insert_header(("x-request-id", "bad id; drop table users"))
        .to_request();
    let res = test::call_service(&app, req).await;
    let id = res.headers().get("x-request-id").unwrap().to_str().unwrap();
    assert_ne!(id, "bad id; drop table users");
    assert_eq!(id.len(), 36);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn openapi_documents_every_route(pool: PgPool) {
    let app = app!(pool);
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/openapi.json")
            .to_request(),
    )
    .await;
    let body: Value = test::read_body_json(res).await;
    let paths = body["paths"].as_object().unwrap();
    for p in ["/v1/health/live", "/v1/health/ready", "/v1/chains"] {
        assert!(paths.contains_key(p), "missing {p}");
    }
}

/// Every documented route must exist: a request must never fall through to the
/// "No such route." default, whatever the method and auth state.
#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn every_documented_route_is_registered(pool: PgPool) {
    let app = app!(pool);
    for (method, path, _, _) in trace_api::routes::openapi::ROUTES {
        let uri = path
            .replace("{chain}", "ethereum")
            .replace("{address}", "0x1111111111111111111111111111111111111111")
            .replace("{id}", "00000000-0000-0000-0000-000000000001");
        let req = match *method {
            "get" => test::TestRequest::get(),
            "post" => test::TestRequest::post(),
            "patch" => test::TestRequest::patch(),
            "delete" => test::TestRequest::delete(),
            m => panic!("unexpected method {m}"),
        }
        .uri(&uri)
        .set_json(serde_json::json!({}))
        .to_request();
        let res = test::call_service(&app, req).await;
        let status = res.status();
        let body: Value = test::try_read_body_json(res).await.unwrap_or(Value::Null);
        assert!(
            body["message"] != "No such route.",
            "{method} {path} is documented but not registered (status {status})"
        );
    }
}
