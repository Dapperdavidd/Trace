use actix_web::{http::StatusCode, test, web, App};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use trace_api::{auth::create_session, configure, error, request_id::RequestId, AppState};
use uuid::Uuid;

async fn app_session(pool: &PgPool) -> String {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id,email,name) VALUES ($1,$2,$3)")
        .bind(user_id)
        .bind(format!("{user_id}@example.test"))
        .bind("Dapper")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO alert_preferences (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    create_session(pool, user_id, "Test", Utc::now())
        .await
        .unwrap()
        .0
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn agent_credentials_are_scoped_hashed_and_revocable(pool: PgPool) {
    let session = app_session(&pool).await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(pool.clone())))
            .app_data(error::json_config())
            .wrap(RequestId)
            .configure(configure),
    )
    .await;

    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/agent-connections")
            .insert_header(("authorization", format!("Bearer {session}")))
            .set_json(json!({
                "name": "Research copilot",
                "scopes": ["observe"],
                "expires_in_days": 7
            }))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: Value = test::read_body_json(created).await;
    let id = created["id"].as_str().unwrap();
    let token = created["token"].as_str().unwrap();
    assert!(token.starts_with("trca_"));
    assert_eq!(created["execution_authority"], false);
    let stored: String = sqlx::query_scalar("SELECT token_hash FROM agent_connections WHERE id=$1")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, token);

    let context = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/agent/context")
            .insert_header(("authorization", format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(context.status(), StatusCode::OK);
    let context: Value = test::read_body_json(context).await;
    assert_eq!(context["near_is_control_plane"], true);
    assert_eq!(context["capabilities"]["observe"], true);
    assert_eq!(context["capabilities"]["propose"], false);
    assert_eq!(context["capabilities"]["execute"], false);

    let forbidden = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/agent/intents/preview")
            .insert_header(("authorization", format!("Bearer {token}")))
            .set_json(json!({
                "mandate_id": Uuid::new_v4(),
                "quote": {
                    "originAsset": "nep141:wrap.near",
                    "destinationAsset": "nep141:eth.omft.near",
                    "amount": "1",
                    "slippageTolerance": 100,
                    "recipient": "alice.near",
                    "refundTo": "alice.near",
                    "deadline": "2099-01-01T00:00:00Z"
                }
            }))
            .to_request(),
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let listed = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/agent-connections")
            .insert_header(("authorization", format!("Bearer {session}")))
            .to_request(),
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed: Value = test::read_body_json(listed).await;
    assert!(listed["items"][0].get("token").is_none());
    assert_eq!(listed["items"][0]["token_hint"].as_str().unwrap().len(), 6);

    let revoked = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri(&format!("/v1/agent-connections/{id}"))
            .insert_header(("authorization", format!("Bearer {session}")))
            .to_request(),
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    let rejected = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/agent/context")
            .insert_header(("authorization", format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn agent_connection_input_is_bounded(pool: PgPool) {
    let session = app_session(&pool).await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(pool)))
            .app_data(error::json_config())
            .wrap(RequestId)
            .configure(configure),
    )
    .await;
    for body in [
        json!({"name":"","scopes":["observe"]}),
        json!({"name":"Agent","scopes":["execute"]}),
        json!({"name":"Agent","scopes":["observe"],"expires_in_days":91}),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/v1/agent-connections")
                .insert_header(("authorization", format!("Bearer {session}")))
                .set_json(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
