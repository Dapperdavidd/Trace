use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub db_max_connections: u32,
    pub cors_origins: Vec<String>,
    pub google_client_id: Option<String>,
    /// Local development only. Accepts `dev:<email>` as an ID token.
    pub dev_auth: bool,
    /// Seed and refresh clearly-labelled demo data. Off by default.
    pub seed_demo: bool,
    /// `production` turns on startup safety checks (see `validate`).
    pub env: String,
    /// Bearer token for `/v1/ops/metrics`. The endpoint is off when unset.
    pub ops_token: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            bind_addr: std::env::var("TRACE_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into()),
            db_max_connections: std::env::var("TRACE_DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            cors_origins: std::env::var("TRACE_CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            google_client_id: std::env::var("GOOGLE_CLIENT_ID")
                .ok()
                .filter(|v| !v.is_empty()),
            dev_auth: std::env::var("TRACE_DEV_AUTH").is_ok_and(|v| v == "true"),
            seed_demo: std::env::var("TRACE_SEED_DEMO").is_ok_and(|v| v == "true"),
            env: std::env::var("TRACE_ENV").unwrap_or_else(|_| "development".into()),
            ops_token: std::env::var("TRACE_OPS_TOKEN")
                .ok()
                .filter(|v| !v.is_empty()),
        })
    }

    pub fn is_production(&self) -> bool {
        self.env.eq_ignore_ascii_case("production")
    }

    /// Settings that must never reach production. Returns every problem, not just the first.
    /// Outside production this returns no problems.
    pub fn production_problems(&self) -> Vec<String> {
        if !self.is_production() {
            return vec![];
        }
        let mut p = vec![];
        if self.dev_auth {
            p.push("TRACE_DEV_AUTH=true accepts dev:<email> tokens as sign-in; it is an authentication bypass".into());
        }
        if self.seed_demo {
            p.push("TRACE_SEED_DEMO=true would seed fake data into a production database".into());
        }
        if self.google_client_id.is_none() {
            p.push("GOOGLE_CLIENT_ID is not set, so nobody can sign in".into());
        }
        if self.cors_origins.is_empty() {
            p.push("TRACE_CORS_ORIGINS is empty".into());
        }
        for o in &self.cors_origins {
            let l = o.to_ascii_lowercase();
            if o == "*" || l.contains("localhost") || l.contains("127.0.0.1") {
                p.push(format!(
                    "TRACE_CORS_ORIGINS contains '{o}', which is not a production origin"
                ));
            } else if !l.starts_with("https://") {
                p.push(format!(
                    "TRACE_CORS_ORIGINS contains '{o}', which is not https"
                ));
            }
        }
        if self.ops_token.as_deref().is_some_and(|t| t.len() < 24) {
            p.push("TRACE_OPS_TOKEN is shorter than 24 characters".into());
        }
        if self.database_url.contains("trace:trace@") {
            p.push("DATABASE_URL uses the development password".into());
        }
        p
    }
}
