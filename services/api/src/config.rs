use anyhow::Context;

/// Base58 SHA-256 of `trace_mandates_contract.wasm` built from `contracts/trace-mandates`
/// (cargo build --release --target wasm32-unknown-unknown). Update it with every reviewed
/// contract release; `TRACE_NEAR_CONTRACT_CODE_HASH` overrides it.
pub const REVIEWED_CONTRACT_CODE_HASH: &str = "7Wo7p8xeSgYLs8svdUm4xDMNqeqkmMWuD2oeWn5TwvsR";

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub db_max_connections: u32,
    pub cors_origins: Vec<String>,
    pub google_client_id: Option<String>,
    /// Stable WebAuthn identity. Changing the RP id invalidates every passkey.
    pub passkey_rp_id: String,
    pub passkey_rp_origin: String,
    /// Local development only. Accepts `dev:<email>` as an ID token.
    pub dev_auth: bool,
    /// Seed and refresh clearly-labelled demo data. Off by default.
    pub seed_demo: bool,
    /// `production` turns on startup safety checks (see `validate`).
    pub env: String,
    /// Bearer token for `/v1/ops/metrics`. The endpoint is off when unset.
    pub ops_token: Option<String>,
    /// NEAR is the control plane. Contract and agent IDs remain optional until deployment.
    pub near_network: String,
    pub near_rpc_url: String,
    pub near_contract_id: Option<String>,
    pub near_contract_code_hash: String,
    pub near_agent_account_id: Option<String>,
    pub near_delegation_enabled: bool,
    pub near_intents_base_url: String,
    pub near_intents_api_key: Option<String>,
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
            passkey_rp_id: std::env::var("TRACE_PASSKEY_RP_ID")
                .unwrap_or_else(|_| "localhost".into()),
            passkey_rp_origin: std::env::var("TRACE_PASSKEY_RP_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            dev_auth: std::env::var("TRACE_DEV_AUTH").is_ok_and(|v| v == "true"),
            seed_demo: std::env::var("TRACE_SEED_DEMO").is_ok_and(|v| v == "true"),
            env: std::env::var("TRACE_ENV").unwrap_or_else(|_| "development".into()),
            ops_token: std::env::var("TRACE_OPS_TOKEN")
                .ok()
                .filter(|v| !v.is_empty()),
            near_network: std::env::var("TRACE_NEAR_NETWORK").unwrap_or_else(|_| "testnet".into()),
            near_rpc_url: std::env::var("TRACE_NEAR_RPC_URL").unwrap_or_else(|_| {
                if std::env::var("TRACE_NEAR_NETWORK").as_deref() == Ok("mainnet") {
                    "https://rpc.mainnet.near.org".into()
                } else {
                    "https://rpc.testnet.near.org".into()
                }
            }),
            near_contract_code_hash: std::env::var("TRACE_NEAR_CONTRACT_CODE_HASH")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| REVIEWED_CONTRACT_CODE_HASH.into()),
            near_contract_id: std::env::var("TRACE_NEAR_CONTRACT_ID")
                .ok()
                .filter(|v| !v.is_empty()),
            near_agent_account_id: std::env::var("TRACE_NEAR_AGENT_ACCOUNT_ID")
                .ok()
                .filter(|v| !v.is_empty()),
            near_delegation_enabled: std::env::var("TRACE_NEAR_DELEGATION_ENABLED")
                .is_ok_and(|v| v == "true"),
            near_intents_base_url: std::env::var("TRACE_NEAR_INTENTS_BASE_URL")
                .unwrap_or_else(|_| "https://1click.chaindefuser.com".into()),
            near_intents_api_key: std::env::var("TRACE_NEAR_INTENTS_API_KEY")
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
        if self.passkey_rp_id == "localhost" {
            p.push("TRACE_PASSKEY_RP_ID still uses localhost".into());
        }
        if !self
            .passkey_rp_origin
            .to_ascii_lowercase()
            .starts_with("https://")
        {
            p.push("TRACE_PASSKEY_RP_ORIGIN must be https in production".into());
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
        if self.near_delegation_enabled {
            if self.near_contract_id.is_none() {
                p.push("TRACE_NEAR_DELEGATION_ENABLED=true without TRACE_NEAR_CONTRACT_ID".into());
            }
            if self.near_agent_account_id.is_none() {
                p.push(
                    "TRACE_NEAR_DELEGATION_ENABLED=true without TRACE_NEAR_AGENT_ACCOUNT_ID".into(),
                );
            }
        }
        p
    }
}
