use actix_web::{http::StatusCode, test, web, App};
use jsonwebtoken::{encode, jwk::JwkSet, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use trace_api::{
    auth::{GoogleVerifier, IdTokenVerifier, VerifyError},
    configure, error,
    request_id::RequestId,
    AppState,
};

const CLIENT_ID: &str = "test-client.apps.googleusercontent.com";
const PRIVATE_PEM: &[u8] = include_bytes!("keys/test_rsa.pem");
const JWKS: &str = include_str!("keys/test_jwks.json");

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn verifier() -> Arc<dyn IdTokenVerifier> {
    let keys: JwkSet = serde_json::from_str(JWKS).unwrap();
    Arc::new(GoogleVerifier::with_static_keys(CLIENT_ID.into(), keys))
}

fn mint(claims: Value, kid: &str) -> String {
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some(kid.into());
    encode(
        &h,
        &claims,
        &EncodingKey::from_rsa_pem(PRIVATE_PEM).unwrap(),
    )
    .unwrap()
}

fn good_claims(sub: &str, email: &str) -> Value {
    json!({
        "iss": "https://accounts.google.com", "aud": CLIENT_ID, "sub": sub,
        "email": email, "email_verified": true, "name": "Test User",
        "iat": now(), "exp": now() + 600
    })
}

#[tokio::test]
async fn verifier_accepts_a_valid_token() {
    let id = verifier()
        .verify(&mint(good_claims("s1", "a@example.com"), "test-key-1"))
        .await
        .unwrap();
    assert_eq!(
        (id.provider, id.subject.as_str(), id.email.as_str()),
        ("google", "s1", "a@example.com")
    );
}

#[tokio::test]
async fn verifier_rejects_bad_tokens() {
    let v = verifier();
    let mut c = good_claims("s1", "a@example.com");
    c["aud"] = json!("someone-elses-client");
    assert!(
        matches!(
            v.verify(&mint(c, "test-key-1")).await,
            Err(VerifyError::Invalid(_))
        ),
        "wrong audience"
    );

    let mut c = good_claims("s1", "a@example.com");
    c["exp"] = json!(now() - 3600);
    assert!(
        matches!(
            v.verify(&mint(c, "test-key-1")).await,
            Err(VerifyError::Invalid(_))
        ),
        "expired"
    );

    let mut c = good_claims("s1", "a@example.com");
    c["iss"] = json!("https://evil.example.com");
    assert!(
        matches!(
            v.verify(&mint(c, "test-key-1")).await,
            Err(VerifyError::Invalid(_))
        ),
        "wrong issuer"
    );

    let mut c = good_claims("s1", "a@example.com");
    c["email_verified"] = json!(false);
    assert!(
        matches!(
            v.verify(&mint(c, "test-key-1")).await,
            Err(VerifyError::Invalid(_))
        ),
        "unverified email"
    );

    assert!(
        matches!(
            v.verify(&mint(good_claims("s1", "a@example.com"), "other-kid"))
                .await,
            Err(VerifyError::Invalid(_))
        ),
        "unknown key id"
    );
    assert!(matches!(
        v.verify("not.a.jwt").await,
        Err(VerifyError::Invalid(_))
    ));
}

#[tokio::test]
async fn verifier_rejects_hs256_algorithm_confusion() {
    // Attacker signs with HMAC using public key material as the secret.
    let jwks: Value = serde_json::from_str(JWKS).unwrap();
    let n = jwks["keys"][0]["n"].as_str().unwrap().as_bytes().to_vec();
    let mut h = Header::new(Algorithm::HS256);
    h.kid = Some("test-key-1".into());
    let forged = encode(
        &h,
        &good_claims("evil", "evil@example.com"),
        &EncodingKey::from_secret(&n),
    )
    .unwrap();
    assert!(matches!(
        verifier().verify(&forged).await,
        Err(VerifyError::Invalid(_))
    ));
}

macro_rules! app {
    ($pool:expr) => {
        test::init_service(
            App::new()
                .app_data(web::Data::new(AppState::with_verifier($pool, verifier())))
                .app_data(error::json_config())
                .wrap(RequestId)
                .configure(configure)
                .default_service(web::to(error::not_found)),
        )
        .await
    };
}

macro_rules! call {
    ($app:expr, $req:expr) => {{
        let res = test::call_service(&$app, $req.to_request()).await;
        let status = res.status();
        let body: Value = test::try_read_body_json(res).await.unwrap_or(Value::Null);
        (status, body)
    }};
}

async fn sign_in(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    sub: &str,
    email: &str,
) -> (String, Value) {
    let tok = mint(good_claims(sub, email), "test-key-1");
    let res = test::call_service(
        app,
        test::TestRequest::post()
            .uri("/v1/auth/google")
            .set_json(json!({ "id_token": tok }))
            .to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let b: Value = test::read_body_json(res).await;
    (b["access_token"].as_str().unwrap().to_string(), b)
}

fn auth(t: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {t}"))
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn unconfigured_sign_in_is_503_not_a_bypass(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(pool)))
            .app_data(error::json_config())
            .wrap(RequestId)
            .configure(configure),
    )
    .await;
    let (s, b) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/auth/google")
            .set_json(json!({ "id_token": "dev:x@y.z" }))
    );
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(b["code"], "dependency_unavailable");
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn session_lifecycle(pool: PgPool) {
    let app = app!(pool.clone());
    // no token
    let (s, b) = call!(app, test::TestRequest::get().uri("/v1/me"));
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(b["code"], "unauthorized");
    assert!(
        !b["request_id"].as_str().unwrap().is_empty(),
        "errors from handlers carry the request id"
    );
    // garbage token
    let (s, _) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/me")
            .insert_header(auth("trc_nope"))
    );
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (t, signin) = sign_in(&app, "g-1", "Ada@Example.com").await;
    assert!(t.starts_with("trc_"));
    assert_eq!(signin["user"]["email"], "Ada@Example.com");
    // only a hash is stored
    let stored: Vec<String> = sqlx::query_scalar("SELECT token_hash FROM app_sessions")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert_ne!(stored[0], t);
    assert_eq!(stored[0].len(), 64);

    let (s, me) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/me")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(me["wallets"], json!({ "watch_only": 0, "connected": 0 }));

    // same Google account signs in again: same user
    let (_, again) = sign_in(&app, "g-1", "ada@example.com").await;
    assert_eq!(again["user"]["id"], signin["user"]["id"]);

    let (s, w) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/me/wallets")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(w["connected"], json!([]));
    assert_eq!(w["embedded"]["available"], false);

    // logout revokes only that session
    let (s, _) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/auth/logout")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/me")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn expired_sessions_are_rejected(pool: PgPool) {
    let app = app!(pool.clone());
    let (t, _) = sign_in(&app, "g-1", "a@example.com").await;
    sqlx::query("UPDATE app_sessions SET expires_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    let (s, _) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/me")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn deleting_the_account_removes_user_data_and_access(pool: PgPool) {
    let app = app!(pool.clone());
    let (t, _) = sign_in(&app, "g-1", "a@example.com").await;
    let (s, _) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
            .set_json(
                json!({"chain":"ethereum","address":"0x1111111111111111111111111111111111111111"})
            )
    );
    assert_eq!(s, StatusCode::CREATED);
    let (s, _) = call!(
        app,
        test::TestRequest::delete()
            .uri("/v1/me")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::NO_CONTENT);
    for table in [
        "users",
        "oauth_identities",
        "app_sessions",
        "watchlist_wallets",
        "alert_preferences",
    ] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0, "{table} should be empty");
    }
    let audit: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE action='account_deleted'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audit, 1);
    let (s, _) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/me")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn watchlist_add_is_idempotent_and_validates_addresses(pool: PgPool) {
    let app = app!(pool.clone());
    let (t, _) = sign_in(&app, "g-1", "a@example.com").await;
    let mixed = "0xAbCdEf1111111111111111111111111111111111";
    let (s, a) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
            .set_json(json!({"chain":"ethereum","address":mixed,"alias":"whale"}))
    );
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(
        a["wallet"]["address"],
        mixed.to_lowercase(),
        "EVM addresses are lowercased"
    );
    assert_eq!(a["wallet"]["alias"], "whale");
    let (s, b) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
            .set_json(json!({"chain":"ethereum","address":mixed.to_lowercase()}))
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["id"], a["id"], "same wallet, same row");
    assert_eq!(
        b["wallet"]["alias"], "whale",
        "an omitted alias keeps the existing one"
    );

    let sol = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
    let (s, c) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
            .set_json(json!({"chain":"solana","address":sol}))
    );
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(c["wallet"]["address"], sol, "Solana case is preserved");

    for bad in [
        json!({"chain":"ethereum","address":"0x123"}),
        json!({"chain":"solana","address":"0x1111111111111111111111111111111111111111"}),
        json!({"chain":"dogecoin","address":"x"}),
        json!({"chain":"ethereum","address":mixed,"alias":"x".repeat(61)}),
    ] {
        let (s, e) = call!(
            app,
            test::TestRequest::post()
                .uri("/v1/watchlists")
                .insert_header(auth(&t))
                .set_json(bad)
        );
        assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");
    }

    let (s, list) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(list["items"].as_array().unwrap().len(), 2);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn watchlist_status_reflects_real_coverage_and_activity(pool: PgPool) {
    let app = app!(pool.clone());
    let (t, _) = sign_in(&app, "g-1", "a@example.com").await;
    let addr = "0x1111111111111111111111111111111111111111";
    let (_, a) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
            .set_json(json!({"chain":"ethereum","address":addr}))
    );
    assert_eq!(
        a["status"], "unsupported_protocol",
        "no protocol is enabled yet"
    );

    sqlx::query("INSERT INTO protocols (chain, protocol, support_status) VALUES ('ethereum','uniswap_v2','enabled')").execute(&pool).await.unwrap();
    let (_, l) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
    );
    assert_eq!(l["items"][0]["status"], "waiting_for_activity");

    let id = a["id"].as_str().unwrap();
    let (s, p) = call!(
        app,
        test::TestRequest::patch()
            .uri(&format!("/v1/watchlists/{id}"))
            .insert_header(auth(&t))
            .set_json(json!({"paused": true, "alias": "cold"}))
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(p["status"], "paused");
    assert_eq!(p["wallet"]["alias"], "cold");
    let (_, p) = call!(
        app,
        test::TestRequest::patch()
            .uri(&format!("/v1/watchlists/{id}"))
            .insert_header(auth(&t))
            .set_json(json!({"paused": false, "alias": ""}))
    );
    assert_eq!(
        p["wallet"]["alias"],
        Value::Null,
        "an empty alias clears it"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn users_cannot_touch_each_others_watchlists(pool: PgPool) {
    let app = app!(pool.clone());
    let (ta, _) = sign_in(&app, "g-1", "a@example.com").await;
    let (tb, _) = sign_in(&app, "g-2", "b@example.com").await;
    let (_, a) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&ta))
            .set_json(
                json!({"chain":"base","address":"0x2222222222222222222222222222222222222222"})
            )
    );
    let id = a["id"].as_str().unwrap();
    let (s, _) = call!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/v1/watchlists/{id}"))
            .insert_header(auth(&tb))
    );
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = call!(
        app,
        test::TestRequest::patch()
            .uri(&format!("/v1/watchlists/{id}"))
            .insert_header(auth(&tb))
            .set_json(json!({"paused":true}))
    );
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (_, lb) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/watchlists")
            .insert_header(auth(&tb))
    );
    assert_eq!(lb["items"], json!([]));
    let (s, _) = call!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/v1/watchlists/{id}"))
            .insert_header(auth(&ta))
    );
    assert_eq!(s, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn watchlist_has_a_size_limit(pool: PgPool) {
    let app = app!(pool.clone());
    let (t, _) = sign_in(&app, "g-1", "a@example.com").await;
    let uid: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    for i in 0..200 {
        let addr = format!("0x{i:040x}");
        sqlx::query("INSERT INTO wallets (chain,address) VALUES ('ethereum',$1)")
            .bind(&addr)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO watchlist_wallets (id,user_id,chain,address) VALUES ($1,$2,'ethereum',$3)",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(uid)
        .bind(&addr)
        .execute(&pool)
        .await
        .unwrap();
    }
    let (s, e) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
            .set_json(
                json!({"chain":"ethereum","address":"0xffffffffffffffffffffffffffffffffffffffff"})
            )
    );
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(e["code"], "limit_reached");
    // re-adding an existing one is still fine
    let (s, _) = call!(
        app,
        test::TestRequest::post()
            .uri("/v1/watchlists")
            .insert_header(auth(&t))
            .set_json(json!({"chain":"ethereum","address":format!("0x{:040x}", 5)}))
    );
    assert_eq!(s, StatusCode::OK);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn alerts_paginate_respect_preferences_and_isolate_users(pool: PgPool) {
    let app = app!(pool.clone());
    let (ta, sa) = sign_in(&app, "g-1", "a@example.com").await;
    let (tb, _) = sign_in(&app, "g-2", "b@example.com").await;
    let ua: uuid::Uuid = sa["user"]["id"].as_str().unwrap().parse().unwrap();
    for i in 0..5 {
        let ty = if i % 2 == 0 {
            "wallet_buy"
        } else {
            "risk_event"
        };
        sqlx::query("INSERT INTO alerts (id,user_id,type,title,body,evidence_tx,event_time,published_at,demo) VALUES ($1,$2,$3,$4,'b',ARRAY['0xabc'],now(),now() - ($5 || ' minutes')::interval,TRUE)")
            .bind(uuid::Uuid::new_v4()).bind(ua).bind(ty).bind(format!("alert {i}")).bind(i.to_string()).execute(&pool).await.unwrap();
    }
    let (s, p1) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/alerts?limit=2")
            .insert_header(auth(&ta))
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(p1["items"].as_array().unwrap().len(), 2);
    assert_eq!(p1["items"][0]["title"], "alert 0", "newest first");
    assert_eq!(p1["demo"], true);
    let c = p1["next_cursor"].as_str().unwrap();
    let (_, p2) = call!(
        app,
        test::TestRequest::get()
            .uri(&format!("/v1/alerts?limit=2&cursor={c}"))
            .insert_header(auth(&ta))
    );
    assert_eq!(p2["items"][0]["title"], "alert 2");
    let c2 = p2["next_cursor"].as_str().unwrap();
    let (_, p3) = call!(
        app,
        test::TestRequest::get()
            .uri(&format!("/v1/alerts?limit=2&cursor={c2}"))
            .insert_header(auth(&ta))
    );
    assert_eq!(p3["items"].as_array().unwrap().len(), 1);
    assert_eq!(p3["next_cursor"], Value::Null);

    let (s, e) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/alerts?cursor=zz")
            .insert_header(auth(&ta))
    );
    assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");

    // turning risk alerts off hides them
    let (s, pr) = call!(
        app,
        test::TestRequest::patch()
            .uri("/v1/alert-preferences")
            .insert_header(auth(&ta))
            .set_json(json!({"risk_event": false}))
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(pr["risk_event"], false);
    assert_eq!(pr["wallet_buy"], true);
    let (_, all) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/alerts?limit=50")
            .insert_header(auth(&ta))
    );
    assert_eq!(all["items"].as_array().unwrap().len(), 3);
    assert!(all["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|i| i["type"] == "wallet_buy"));

    let (_, other) = call!(
        app,
        test::TestRequest::get()
            .uri("/v1/alerts")
            .insert_header(auth(&tb))
    );
    assert_eq!(other["items"], json!([]));
}
