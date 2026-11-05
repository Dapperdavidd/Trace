use actix_web::{http::StatusCode, test, web, App};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use trace_api::{
    auth::DevVerifier, configure, error, request_id::RequestId, state::NearState, AppState,
};
use trace_near::IntentsClient;

const WALLET: &str = "0x0000000000000000000000000000000000000001";

fn near_state() -> NearState {
    NearState {
        network: Arc::from("testnet"),
        contract_id: Some(Arc::from("mandates.trace.testnet")),
        agent_account_id: Some(Arc::from("agent.trace.testnet")),
        delegation_enabled: false,
        intents: Arc::new(IntentsClient::new(None)),
    }
}

macro_rules! sign_in {
    ($app:expr) => {{
        let request = test::TestRequest::post()
            .uri("/v1/auth/google")
            .set_json(json!({"id_token": "dev:near@example.com"}))
            .to_request();
        let response = test::call_service(&$app, request).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value = test::read_body_json(response).await;
        body["access_token"].as_str().unwrap().to_string()
    }};
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn near_status_never_claims_an_unconfigured_deployment(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(pool)))
            .app_data(error::json_config())
            .wrap(RequestId)
            .configure(configure),
    )
    .await;
    let response =
        test::call_service(&app, test::TestRequest::get().uri("/v1/near").to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["near_is_control_plane"], true);
    assert_eq!(body["data_status"], "unavailable");
    assert_eq!(body["contract"]["configured"], false);
    assert_eq!(body["contract"]["deployment_verified"], false);
    assert_eq!(body["intents"]["server_side_signing"], false);
    assert_eq!(body["intents"]["server_side_submission"], false);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn mandate_is_a_wallet_signable_draft_and_delegation_stays_off(pool: PgPool) {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let state = AppState::with_verifier(pool, Arc::new(DevVerifier)).with_near(near_state());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .app_data(error::json_config())
            .wrap(RequestId)
            .configure(configure),
    )
    .await;
    let access_token = sign_in!(app);

    let strategy_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/shadow-strategies")
            .insert_header(("authorization", format!("Bearer {access_token}")))
            .set_json(json!({
                "wallet": {"chain": "ethereum", "address": WALLET},
                "budget_usd": "500",
                "size_rule": "fixed_usd",
                "delay_seconds": 30,
                "slippage_bps": 100,
                "exit_rule": "mirror_sells"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(strategy_response.status(), StatusCode::CREATED);
    let strategy: Value = test::read_body_json(strategy_response).await;

    let mandate_body = json!({
        "shadow_strategy_id": strategy["id"],
        "owner_near_account": "alice.testnet",
        "agent_near_account": "agent.trace.testnet",
        "mode": "approval_required",
        "allowed_chains": ["ethereum"],
        "allowed_assets": [
            {"chain": "ethereum", "asset_id": "nep141:eth.omft.near"},
            {"chain": "ethereum", "asset_id": "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near"}
        ],
        "max_trade_usd": "100.00",
        "max_daily_usd": "250.00",
        "max_slippage_bps": 100,
        "expires_at": Utc::now() + Duration::days(7)
    });
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/near/mandates")
            .insert_header(("authorization", format!("Bearer {access_token}")))
            .set_json(&mandate_body)
            .to_request(),
    )
    .await;
    let response_status = response.status();
    let body: Value = test::read_body_json(response).await;
    assert_eq!(response_status, StatusCode::CREATED, "{body}");
    assert_eq!(body["status"], "draft");
    assert_eq!(body["owner_signature_required"], true);
    assert_eq!(
        body["near_transaction"]["receiver_id"],
        "mandates.trace.testnet"
    );
    assert_eq!(
        body["near_transaction"]["actions"][0]["params"]["method_name"],
        "create_mandate"
    );
    assert_eq!(
        body["near_transaction"]["actions"][0]["params"]["args"]["mandate"]["max_trade_usd_micros"],
        "100000000"
    );

    let list = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/near/mandates")
            .insert_header(("authorization", format!("Bearer {access_token}")))
            .to_request(),
    )
    .await;
    let list_body: Value = test::read_body_json(list).await;
    assert_eq!(list_body["items"].as_array().unwrap().len(), 1);
    assert_eq!(list_body["items"][0]["status"], "draft");
    assert_eq!(list_body["items"][0]["anchor_tx_hash"], Value::Null);

    let mut delegated = mandate_body;
    delegated["mode"] = json!("delegated");
    delegated["shadow_strategy_id"] = json!(strategy["id"]);
    let denied = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/near/mandates")
            .insert_header(("authorization", format!("Bearer {access_token}")))
            .set_json(delegated)
            .to_request(),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::SERVICE_UNAVAILABLE);
}
