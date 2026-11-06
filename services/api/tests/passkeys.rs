use actix_web::{http::StatusCode, test, web, App};
use serde_json::{json, Value};
use sqlx::PgPool;
use trace_api::{configure, error, request_id::RequestId, AppState};
use uuid::Uuid;

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn registration_options_are_server_held_and_short_lived(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(pool.clone())))
            .app_data(error::json_config())
            .wrap(RequestId)
            .configure(configure),
    )
    .await;
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/auth/passkeys/register/options")
            .set_json(json!({"username":"dapper","display_name":"Dapper"}))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert!(body["ceremony_id"].is_string());
    assert_eq!(body["options"]["rp"]["id"], "localhost");
    assert_eq!(body["options"]["user"]["name"], "dapper");

    let row: (String, bool, bool) = sqlx::query_as(
        "SELECT kind, state <> '{}'::jsonb, expires_at <= created_at + interval '5 minutes'
         FROM passkey_ceremonies WHERE id=$1",
    )
    .bind(Uuid::parse_str(body["ceremony_id"].as_str().unwrap()).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, ("register".into(), true, true));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn passkey_entry_points_validate_and_reject_unknown_accounts(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(pool)))
            .app_data(error::json_config())
            .wrap(RequestId)
            .configure(configure),
    )
    .await;
    let invalid = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/auth/passkeys/register/options")
            .set_json(json!({"username":"Bad Name!","display_name":"Dapper"}))
            .to_request(),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let unknown = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/auth/passkeys/authenticate/options")
            .set_json(json!({"username":"nobody"}))
            .to_request(),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(unknown).await;
    assert_eq!(body["code"], "unauthorized");
    assert!(body["request_id"].as_str().is_some_and(|id| !id.is_empty()));
}
