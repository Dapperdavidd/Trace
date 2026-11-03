use actix_web::{http::StatusCode, test, web, App};
use trace_api::security_headers;

async fn ok() -> &'static str {
    "ok"
}

#[actix_rt::test]
async fn api_responses_set_browser_security_headers() {
    let app = test::init_service(
        App::new()
            .wrap(security_headers())
            .route("/", web::get().to(ok)),
    )
    .await;
    let response = test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    let value = |name| headers.get(name).and_then(|header| header.to_str().ok());
    assert_eq!(value("x-content-type-options"), Some("nosniff"));
    assert_eq!(value("x-frame-options"), Some("DENY"));
    assert_eq!(value("referrer-policy"), Some("no-referrer"));
    assert_eq!(
        value("permissions-policy"),
        Some("camera=(), microphone=(), geolocation=(), payment=()")
    );
    assert!(
        !headers.contains_key("strict-transport-security"),
        "the app does not know whether its ingress connection was HTTPS"
    );
}
