use trace_api::config::Config;

const TOKEN: &str = "a-long-random-ops-token-0123456789";

fn prod() -> Config {
    Config {
        database_url: "postgres://trace:s3cret-prod@db:5432/trace".into(),
        bind_addr: "0.0.0.0:8080".into(),
        db_max_connections: 10,
        cors_origins: vec!["https://app.example.com".into()],
        google_client_id: Some("id.apps.googleusercontent.com".into()),
        dev_auth: false,
        seed_demo: false,
        env: "production".into(),
        ops_token: Some(TOKEN.into()),
    }
}

#[test]
fn a_correct_production_config_has_no_problems() {
    assert!(prod().production_problems().is_empty());
}

#[test]
fn production_refuses_every_development_shortcut() {
    let mut c = prod();
    c.dev_auth = true;
    c.seed_demo = true;
    c.google_client_id = None;
    c.cors_origins = vec![
        "*".into(),
        "http://localhost:3000".into(),
        "http://app.example.com".into(),
    ];
    c.ops_token = Some("short".into());
    c.database_url = "postgres://trace:trace@db/trace".into();
    let p = c.production_problems().join("\n");
    for needle in [
        "TRACE_DEV_AUTH",
        "TRACE_SEED_DEMO",
        "GOOGLE_CLIENT_ID",
        "'*'",
        "localhost",
        "not https",
        "24 characters",
        "development password",
    ] {
        assert!(p.contains(needle), "missing '{needle}' in:\n{p}");
    }
}

#[test]
fn development_is_never_blocked() {
    let mut c = prod();
    c.env = "development".into();
    c.dev_auth = true;
    c.seed_demo = true;
    c.google_client_id = None;
    c.cors_origins = vec!["http://localhost:3000".into()];
    assert!(c.production_problems().is_empty());
}
