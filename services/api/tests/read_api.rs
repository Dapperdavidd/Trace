use actix_web::{http::StatusCode, test, web, App};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, jwk::JwkSet, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use trace_api::{auth::GoogleVerifier, configure, demo, error, request_id::RequestId, AppState};

const CLIENT_ID: &str = "test-client.apps.googleusercontent.com";
const WALLET_A: &str = "0x00000000000000000000000000000000000a0001";
const WALLET_C: &str = "0x00000000000000000000000000000000000a0003";
const NOVA: &str = "0x000000000000000000000000000000000000d001";
const PRSM: &str = "0x000000000000000000000000000000000000d002";
const HNYP: &str = "0x000000000000000000000000000000000000d005";

macro_rules! app {
    ($pool:expr) => {{
        let keys: JwkSet = serde_json::from_str(include_str!("keys/test_jwks.json")).unwrap();
        let v = Arc::new(GoogleVerifier::with_static_keys(CLIENT_ID.into(), keys));
        test::init_service(
            App::new()
                .app_data(web::Data::new(
                    AppState::with_verifier($pool, v).with_demo(true),
                ))
                .app_data(error::json_config())
                .wrap(RequestId)
                .configure(configure)
                .default_service(web::to(error::not_found)),
        )
        .await
    }};
}

macro_rules! get {
    ($app:expr, $uri:expr) => {{
        let res = test::call_service(&$app, test::TestRequest::get().uri($uri).to_request()).await;
        let s = res.status();
        let b: Value = test::try_read_body_json(res).await.unwrap_or(Value::Null);
        (s, b)
    }};
}

fn tok(n: i64) -> String {
    let now = Utc::now().timestamp();
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("test-key-1".into());
    encode(
        &h,
        &json!({"iss":"https://accounts.google.com","aud":CLIENT_ID,"sub":format!("s{n}"),"email":format!("u{n}@example.com"),"email_verified":true,"iat":now,"exp":now+600}),
        &EncodingKey::from_rsa_pem(include_bytes!("keys/test_rsa.pem")).unwrap(),
    )
    .unwrap()
}

macro_rules! sign_in {
    ($app:expr, $n:expr) => {{
        let res = test::call_service(&$app, test::TestRequest::post().uri("/v1/auth/google").set_json(json!({"id_token": tok($n)})).to_request()).await;
        assert_eq!(res.status(), StatusCode::OK);
        let b: Value = test::read_body_json(res).await;
        b["access_token"].as_str().unwrap().to_string()
    }};
}

fn names(v: &Value) -> Vec<String> {
    v["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["token"]["symbol"].as_str().unwrap().to_string())
        .collect()
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn without_data_every_read_says_unavailable(pool: PgPool) {
    let app = app!(pool);
    for uri in [
        "/v1/discovery/movers",
        "/v1/discovery/wallets",
        "/v1/briefings",
        "/v1/signals",
    ] {
        let (s, b) = get!(app, uri);
        assert_eq!(s, StatusCode::OK, "{uri}");
        assert_eq!(b["data_status"], "unavailable", "{uri}");
        assert_eq!(b["demo"], false, "{uri}");
    }
    let (_, b) = get!(app, &format!("/v1/wallets/ethereum/{WALLET_A}"));
    assert_eq!(b["data_status"], "unavailable");
    assert_eq!(
        b["metrics"]["realized_pnl_usd"],
        Value::Null,
        "no history means no PnL, not zero"
    );
    let (s, _) = get!(app, &format!("/v1/tokens/ethereum/{NOVA}"));
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn movers_exclude_thin_liquidity_and_blocked_tokens_and_say_so(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool);
    let (s, b) = get!(app, "/v1/discovery/movers?tab=trending&window=24h");
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["data_status"], "demo");
    assert_eq!(b["demo"], true);
    let n = names(&b);
    assert!(n.contains(&"NOVA".to_string()));
    assert!(
        !n.contains(&"ORBT".to_string()),
        "thin liquidity is excluded: {n:?}"
    );
    assert!(
        !n.contains(&"HNYP".to_string()),
        "a honeypot is never ranked: {n:?}"
    );
    let note = b["coverage_note"].as_str().unwrap();
    assert!(
        note.contains("thin liquidity") && note.contains("blocked"),
        "{note}"
    );
    let first = &b["items"][0];
    assert_eq!(first["rank"], 1);
    assert!(first["reason"]["label"].as_str().is_some());
    assert!(first["sparkline"].as_array().unwrap().len() > 5);
    assert!(first["source_timestamp"].as_str().is_some());

    let (_, smart) = get!(app, "/v1/discovery/movers?tab=smart_money");
    assert_eq!(names(&smart), vec!["NOVA"]);
    let (_, newp) = get!(app, "/v1/discovery/movers?tab=new");
    assert_eq!(names(&newp), vec!["KELP"]);
    let (s, _) = get!(app, "/v1/discovery/movers?tab=hot");
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = get!(app, "/v1/discovery/movers?window=1y");
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = get!(app, "/v1/discovery/movers?chain=dogecoin");
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (_, base) = get!(app, "/v1/discovery/movers?chain=base");
    assert!(names(&base).iter().all(|s| s != "NOVA"));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn stale_snapshots_are_not_ranked(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    sqlx::query("UPDATE market_snapshots SET last_updated = $1")
        .bind(Utc::now() - Duration::hours(2))
        .execute(&pool)
        .await
        .unwrap();
    let app = app!(pool);
    let (_, b) = get!(app, "/v1/discovery/movers");
    assert_eq!(b["items"], json!([]));
    assert!(b["coverage_note"].as_str().unwrap().contains("stale"));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn seeding_twice_does_not_duplicate_and_refresh_only_touches_demo(pool: PgPool) {
    assert!(demo::seed(&pool, Utc::now()).await.unwrap());
    let swaps: i64 = sqlx::query_scalar("SELECT count(*) FROM normalized_swaps")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!demo::seed(&pool, Utc::now()).await.unwrap());
    let again: i64 = sqlx::query_scalar("SELECT count(*) FROM normalized_swaps")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(swaps, again);
    assert!(swaps > 0);
    let real: i64 = sqlx::query_scalar("SELECT count(*) FROM normalized_swaps WHERE NOT demo")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(real, 0, "every seeded swap is flagged demo");
    let real_snaps: i64 =
        sqlx::query_scalar("SELECT count(*) FROM market_snapshots WHERE NOT demo")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(real_snaps, 0);
    let real_prices: i64 = sqlx::query_scalar("SELECT count(*) FROM price_points WHERE NOT demo")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(real_prices, 0);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn discovery_wallets_rank_only_scored_wallets(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool);
    let (s, b) = get!(app, "/v1/discovery/wallets?tab=top");
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["demo"], true);
    let addrs: Vec<&str> = b["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["wallet"]["address"].as_str().unwrap())
        .collect();
    assert!(addrs.contains(&WALLET_A));
    assert!(
        !addrs.contains(&WALLET_C),
        "a wallet with too little history is not ranked: {addrs:?}"
    );
    let a = b["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["wallet"]["address"] == WALLET_A)
        .unwrap();
    assert!(a["sample_size"].as_u64().unwrap() >= 10);
    assert!(a["realized_return_pct"].is_string());
    // the New tab lists the sparse wallet too, with an honest decision
    let (_, n) = get!(app, "/v1/discovery/wallets?tab=new");
    let c = n["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["wallet"]["address"] == WALLET_C)
        .expect("new tab lists it");
    assert_eq!(c["decision"], "insufficient_evidence");
    let (s, _) = get!(app, "/v1/discovery/wallets?tab=best");
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = get!(app, "/v1/discovery/wallets?cursor=abc");
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn wallet_research_endpoints(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool);
    let (s, o) = get!(app, &format!("/v1/wallets/ethereum/{WALLET_A}"));
    assert_eq!(s, StatusCode::OK);
    assert_eq!(o["data_status"], "demo");
    assert!(o["first_seen"].is_string() && o["last_seen"].is_string());
    assert_eq!(
        o["monitoring"], "unsupported_protocol",
        "no real protocol is enabled in this database"
    );

    let (_, m) = get!(app, &format!("/v1/wallets/ethereum/{WALLET_A}/metrics"));
    assert!(m["realized_pnl_usd"].is_string());
    assert!(
        ["high", "medium", "low", "insufficient"].contains(&m["pnl_confidence"].as_str().unwrap())
    );
    assert!(m["wqs"]["version"].as_str().unwrap().starts_with("wqs/"));
    assert_eq!(m["wcs"], Value::Null);
    assert!(m["wqs"]["explanation"].as_str().unwrap().len() > 10);

    let (_, d) = get!(app, &format!("/v1/wallets/ethereum/{WALLET_A}/dna"));
    assert_eq!(d["state"], "ok");
    assert!(d["rules_version"].as_str().is_some());
    let (_, dc) = get!(app, &format!("/v1/wallets/ethereum/{WALLET_C}/dna"));
    assert_eq!(dc["state"], "insufficient_history");
    assert_eq!(dc["traits"], json!([]));

    let (_, t1) = get!(
        app,
        &format!("/v1/wallets/ethereum/{WALLET_A}/trades?limit=5")
    );
    let items = t1["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    assert!(items
        .iter()
        .all(|i| ["buy", "sell"].contains(&i["side"].as_str().unwrap())
            && i["token"]["symbol"].is_string()
            && i["finality"] == "final"));
    assert!(
        items
            .windows(2)
            .all(|w| w[0]["event_time"].as_str() >= w[1]["event_time"].as_str()),
        "newest first"
    );
    let c = t1["next_cursor"].as_str().unwrap();
    let (_, t2) = get!(
        app,
        &format!("/v1/wallets/ethereum/{WALLET_A}/trades?limit=5&cursor={c}")
    );
    assert_ne!(t2["items"][0]["tx_id"], t1["items"][0]["tx_id"]);

    // a valid but unknown wallet is watchable and simply has no data
    let (s, u) = get!(
        app,
        "/v1/wallets/ethereum/0x9999999999999999999999999999999999999999/metrics"
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(u["data_status"], "unavailable");
    assert_eq!(u["wqs"]["decision"], "insufficient_evidence");
    assert_eq!(u["wqs"]["value"], Value::Null);
    // invalid input
    for bad in [
        "/v1/wallets/ethereum/0x123",
        "/v1/wallets/dogecoin/0x1",
        "/v1/wallets/solana/0x1111111111111111111111111111111111111111/trades",
    ] {
        let (s, e) = get!(app, bad);
        assert_eq!(s, StatusCode::BAD_REQUEST, "{bad} {e}");
    }
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn token_pages_show_facts_hypotheses_and_risk(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool);
    let (s, t) = get!(app, &format!("/v1/tokens/ethereum/{NOVA}"));
    assert_eq!(s, StatusCode::OK);
    assert_eq!(t["token"]["symbol"], "NOVA");
    assert_eq!(t["risk"]["overall"], "pass");
    let why = t["why_moving"].as_array().unwrap();
    assert!(why.iter().any(|w| w["kind"] == "fact"));
    let hyp = why
        .iter()
        .find(|w| w["kind"] == "hypothesis")
        .expect("smart buyers produce a labelled hypothesis");
    assert!(hyp["text"].as_str().unwrap().contains("hypothesis"));
    assert!(t["price_usd"].is_string());

    let (_, h) = get!(app, &format!("/v1/tokens/ethereum/{HNYP}"));
    assert_eq!(
        h["risk"]["overall"], "block",
        "a honeypot is blocked even with smart buyers"
    );
    let (s, _) = get!(
        app,
        "/v1/tokens/ethereum/0x9999999999999999999999999999999999999999"
    );
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = get!(app, "/v1/tokens/ethereum/xyz");
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (s, tl) = get!(app, &format!("/v1/tokens/ethereum/{NOVA}/timeline"));
    assert_eq!(s, StatusCode::OK);
    let kinds: Vec<&str> = tl["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"hypothesis"));
    assert!(
        tl["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["kind"] == "fact")
            .all(|i| !i["evidence_ids"].as_array().unwrap().is_empty()),
        "facts cite evidence"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn compare_validates_its_input(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool);
    let (s, c) = get!(
        app,
        &format!("/v1/tokens/compare?a=ethereum:{NOVA}&b=ethereum:{PRSM}")
    );
    assert_eq!(s, StatusCode::OK);
    assert_eq!(c["items"].as_array().unwrap().len(), 2);
    assert!(c["verdict_note"]
        .as_str()
        .unwrap()
        .contains("not a recommendation"));
    assert!(c["items"][0]["change_pct_24h"].is_string());
    for bad in [
        format!("/v1/tokens/compare?a=ethereum:{NOVA}"),
        format!("/v1/tokens/compare?a=ethereum:{NOVA}&b=ethereum:{NOVA}"),
        format!("/v1/tokens/compare?a={NOVA}&b=ethereum:{PRSM}"),
    ] {
        let (s, _) = get!(app, &bad);
        assert_eq!(s, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (s, _) = get!(app, &format!("/v1/tokens/compare?a=ethereum:{NOVA}&b=ethereum:0x9999999999999999999999999999999999999999"));
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn signals_show_receipts_and_every_outcome_including_losses(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool.clone());
    let (_, l) = get!(app, "/v1/signals");
    assert_eq!(l["items"].as_array().unwrap().len(), 4);
    assert_eq!(l["data_status"], "demo");
    let (_, only) = get!(app, "/v1/signals?state=observed");
    assert_eq!(only["items"].as_array().unwrap().len(), 1);
    let (s, _) = get!(app, "/v1/signals?state=bogus");
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let id = Uuid::from_u128(0xde30_0000_0000_0000_0000_0000_0000_0003);
    let (s, d) = get!(app, &format!("/v1/signals/{id}"));
    assert_eq!(s, StatusCode::OK);
    assert!(d["receipt_hash"].as_str().unwrap().len() == 64);
    assert!(d["receipt"]["exit_policy"]
        .as_str()
        .unwrap()
        .contains("first available"));
    assert_eq!(d["events"].as_array().unwrap().len(), 4);

    let (_, o) = get!(app, &format!("/v1/signals/{id}/outcomes"));
    let items = o["items"].as_array().unwrap();
    assert_eq!(items.len(), 4, "all four horizons are always listed");
    assert!(
        items.iter().any(|i| i["kind"] == "untradeable"),
        "untradeable is shown"
    );
    assert!(
        o["summary"]["losses"].as_u64().unwrap() >= 1,
        "losers are counted: {o}"
    );
    assert!(o["summary"]["gains"].as_u64().unwrap() >= 1);

    // a young signal lists later horizons as pending, not as missing
    let young = Uuid::from_u128(0xde30_0000_0000_0000_0000_0000_0000_0001);
    let (_, y) = get!(app, &format!("/v1/signals/{young}/outcomes"));
    assert!(y["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["status"] == "pending"));

    let (s, _) = get!(app, &format!("/v1/signals/{}", Uuid::new_v4()));
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = get!(app, "/v1/signals/not-a-uuid");
    assert!(s == StatusCode::NOT_FOUND || s == StatusCode::BAD_REQUEST);

    // tampering with a stored receipt is detected and its content withheld
    sqlx::query("UPDATE signals SET receipt = jsonb_set(receipt, '{reference_price_usd}', '\"0.0001\"') WHERE id = $1").bind(id).execute(&pool).await.unwrap();
    let (s, _) = get!(app, &format!("/v1/signals/{id}"));
    assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
    let (_, l2) = get!(app, "/v1/signals");
    assert_eq!(
        l2["items"].as_array().unwrap().len(),
        4,
        "one bad receipt does not hide the rest"
    );
    assert!(l2["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["integrity"] == "failed"));
}

use uuid::Uuid;

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn shadow_strategies_are_validated_owned_and_paper_only(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool);
    let body = json!({"wallet":{"chain":"ethereum","address":WALLET_A},"budget_usd":"10000","size_rule":"fixed_pct","delay_seconds":60,"slippage_bps":100,"exit_rule":"mirror_sells"});
    let res = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/shadow-strategies")
            .set_json(&body)
            .to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let t1 = sign_in!(app, 1);
    let t2 = sign_in!(app, 2);
    let auth = |t: &str| ("Authorization", format!("Bearer {t}"));
    let post = |t: &str, b: Value| {
        test::TestRequest::post()
            .uri("/v1/shadow-strategies")
            .insert_header(auth(t))
            .set_json(b)
            .to_request()
    };

    for (field, bad) in [
        ("size_rule", json!("proportional")),
        ("exit_rule", json!("stop_loss_20")),
        ("budget_usd", json!("0")),
        ("budget_usd", json!("-5")),
        ("budget_usd", json!("abc")),
        ("budget_usd", json!("99999999")),
        ("delay_seconds", json!(99999)),
        ("slippage_bps", json!(5000)),
    ] {
        let mut b = body.clone();
        b[field] = bad;
        let res = test::call_service(&app, post(&t1, b)).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "{field}");
    }
    let mut b = body.clone();
    b["wallet"]["address"] = json!("0x12");
    assert_eq!(
        test::call_service(&app, post(&t1, b)).await.status(),
        StatusCode::BAD_REQUEST
    );

    let res = test::call_service(&app, post(&t1, body.clone())).await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let id = test::read_body_json::<Value, _>(res).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    // a numeric budget is accepted too
    let mut nb = body.clone();
    nb["budget_usd"] = json!(5000);
    assert_eq!(
        test::call_service(&app, post(&t1, nb)).await.status(),
        StatusCode::CREATED
    );

    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/v1/shadow-strategies/{id}/results"))
            .insert_header(auth(&t1))
            .to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let r: Value = test::read_body_json(res).await;
    assert_eq!(r["data_status"], "demo");
    assert_eq!(r["starting_usd"], "10000");
    assert!(r["ending_usd"].is_string() && r["return_pct"].is_string());
    assert!(r["coverage_note"]
        .as_str()
        .unwrap()
        .contains("Paper simulation"));
    assert!(r["baseline"]["note"]
        .as_str()
        .unwrap()
        .contains("Comparison only"));
    assert_eq!(r["assumptions"]["delay_seconds"], 60);
    assert_eq!(r["assumptions"]["fee_bps"], 30);
    let filled = r["trades"].as_u64().unwrap();
    let skipped = r["skipped_unexecutable"].as_u64().unwrap();
    assert!(
        filled + skipped > 0,
        "the simulation looked at the leader's trades"
    );
    assert_eq!(r["skipped"].as_array().unwrap().len() as u64, skipped);

    // someone else's strategy is not visible
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/v1/shadow-strategies/{id}/results"))
            .insert_header(auth(&t2))
            .to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/shadow-strategies")
            .insert_header(auth(&t2))
            .to_request(),
    )
    .await;
    assert_eq!(
        test::read_body_json::<Value, _>(res).await["items"],
        json!([])
    );
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/shadow-strategies")
            .insert_header(auth(&t1))
            .to_request(),
    )
    .await;
    assert_eq!(
        test::read_body_json::<Value, _>(res).await["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn shadow_on_a_wallet_without_history_reports_nothing_to_simulate(pool: PgPool) {
    let app = app!(pool);
    let t = sign_in!(app, 1);
    let body = json!({"wallet":{"chain":"base","address":"0x7777777777777777777777777777777777777777"},"budget_usd":"1000","size_rule":"fixed_usd","delay_seconds":0,"slippage_bps":50,"exit_rule":"time_24h"});
    let res = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/v1/shadow-strategies")
            .insert_header(("Authorization", format!("Bearer {t}")))
            .set_json(&body)
            .to_request(),
    )
    .await;
    let id = test::read_body_json::<Value, _>(res).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/v1/shadow-strategies/{id}/results"))
            .insert_header(("Authorization", format!("Bearer {t}")))
            .to_request(),
    )
    .await;
    let r: Value = test::read_body_json(res).await;
    assert_eq!(r["data_status"], "unavailable");
    assert_eq!(r["return_pct"], "0");
    assert_eq!(r["trades"], 0);
    assert!(r["coverage_note"]
        .as_str()
        .unwrap()
        .contains("nothing to simulate"));
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn briefing_is_built_from_stored_data_only(pool: PgPool) {
    demo::seed(&pool, Utc::now()).await.unwrap();
    let app = app!(pool);
    let (s, b) = get!(app, "/v1/briefings");
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["data_status"], "demo");
    let titles: Vec<&str> = b["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["title"].as_str().unwrap())
        .collect();
    for t in ["Top movers", "Risk watch", "Signals"] {
        assert!(titles.contains(&t), "{titles:?}");
    }
    let text = b.to_string();
    assert!(
        !text.contains("guarantee") && !text.contains("will go up"),
        "no promises in generated copy"
    );
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn demo_accounts_get_labelled_starter_alerts(pool: PgPool) {
    let app = app!(pool);
    let t = sign_in!(app, 1);
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/alerts?limit=50")
            .insert_header(("Authorization", format!("Bearer {t}")))
            .to_request(),
    )
    .await;
    let b: Value = test::read_body_json(res).await;
    assert_eq!(b["items"].as_array().unwrap().len(), 4);
    assert_eq!(b["demo"], true);
    assert!(b["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|i| i["demo"] == true && i["body"].as_str().unwrap().contains("demo")));
    // signing in again does not add more
    let _ = sign_in!(app, 1);
    let res = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/v1/alerts?limit=50")
            .insert_header(("Authorization", format!("Bearer {t}")))
            .to_request(),
    )
    .await;
    assert_eq!(
        test::read_body_json::<Value, _>(res).await["items"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}
