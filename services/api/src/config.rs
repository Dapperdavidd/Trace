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
        })
    }
}
