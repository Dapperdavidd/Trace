use actix_web::{http::StatusCode, test, web, App};
use serde_json::Value;
use trace_api::{
    error,
    rate_limit::{Limits, RateLimit},
    request_id::RequestId,
};

async fn ok() -> &'static str {
    "ok"
}

#[actix_rt::test]
async fn anonymous_limit_returns_standard_error_with_request_id() {
    let app = test::init_service(
        App::new()
            .app_data(error::json_config())
            .wrap(RateLimit::new(Limits {
                anonymous_ip_per_minute: 1,
                authenticated_ip_per_minute: 2,
                user_per_minute: 2,
                sign_in_ip_per_minute: 1,
            }))
            .wrap(RequestId)
            .route("/v1/test", web::get().to(ok)),
    )
    .await;
    let first =
        test::call_service(&app, test::TestRequest::get().uri("/v1/test").to_request()).await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/test")
            .insert_header(("x-request-id", "limit-test"))
            .to_request(),
    )
    .await;
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        second
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok()),
        Some("60")
    );
    let body: Value = test::read_body_json(second).await;
    assert_eq!(body["code"], "rate_limited");
    assert_eq!(body["request_id"], "limit-test");
}

#[actix_rt::test]
async fn sign_in_has_a_separate_stricter_bucket() {
    let app = test::init_service(
        App::new()
            .wrap(RateLimit::new(Limits {
                anonymous_ip_per_minute: 10,
                authenticated_ip_per_minute: 10,
                user_per_minute: 10,
                sign_in_ip_per_minute: 1,
            }))
            .wrap(RequestId)
            .route("/v1/auth/google", web::post().to(ok))
            .route("/v1/other", web::get().to(ok)),
    )
    .await;
    let sign_in = test::TestRequest::post()
        .uri("/v1/auth/google")
        .to_request();
    assert_eq!(
        test::call_service(&app, sign_in).await.status(),
        StatusCode::OK
    );
    let limited = test::TestRequest::post()
        .uri("/v1/auth/google")
        .to_request();
    assert_eq!(
        test::call_service(&app, limited).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    let other = test::TestRequest::get().uri("/v1/other").to_request();
    assert_eq!(
        test::call_service(&app, other).await.status(),
        StatusCode::OK
    );
}

#[actix_rt::test]
async fn authenticated_users_have_independent_buckets() {
    let app = test::init_service(
        App::new()
            .wrap(RateLimit::new(Limits {
                anonymous_ip_per_minute: 10,
                authenticated_ip_per_minute: 10,
                user_per_minute: 1,
                sign_in_ip_per_minute: 10,
            }))
            .route("/v1/test", web::get().to(ok)),
    )
    .await;
    let request = |token: &'static str| {
        test::TestRequest::get()
            .uri("/v1/test")
            .insert_header(("authorization", format!("Bearer {token}")))
            .to_request()
    };
    assert_eq!(
        test::call_service(&app, request("user-one")).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, request("user-one")).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        test::call_service(&app, request("user-two")).await.status(),
        StatusCode::OK
    );
}
