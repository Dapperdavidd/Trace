use crate::auth::{Disabled, IdTokenVerifier};
use sqlx::PgPool;
use std::sync::Arc;
use trace_near::{IntentsClient, NearRpcClient};

#[derive(Clone)]
pub struct NearState {
    pub network: Arc<str>,
    pub contract_id: Option<Arc<str>>,
    pub agent_account_id: Option<Arc<str>>,
    pub delegation_enabled: bool,
    pub intents: Arc<IntentsClient>,
    pub rpc: Arc<NearRpcClient>,
}

impl Default for NearState {
    fn default() -> Self {
        Self {
            network: Arc::from("testnet"),
            contract_id: None,
            agent_account_id: None,
            delegation_enabled: false,
            intents: Arc::new(IntentsClient::new(None)),
            rpc: Arc::new(NearRpcClient::new("https://rpc.testnet.near.org")),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub verifier: Arc<dyn IdTokenVerifier>,
    /// True when demo data is enabled. New accounts then receive starter demo alerts.
    pub demo: bool,
    /// Bearer token for the ops endpoints. `None` disables them.
    pub ops_token: Option<Arc<str>>,
    pub near: NearState,
}

impl AppState {
    /// State with Google sign-in unconfigured. Sign-in answers 503 until a
    /// verifier is supplied; nothing else depends on it.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            verifier: Arc::new(Disabled),
            demo: false,
            ops_token: None,
            near: NearState::default(),
        }
    }

    pub fn with_demo(mut self, demo: bool) -> Self {
        self.demo = demo;
        self
    }

    pub fn with_ops_token(mut self, token: Option<String>) -> Self {
        self.ops_token = token.map(Into::into);
        self
    }

    pub fn with_verifier(pool: PgPool, verifier: Arc<dyn IdTokenVerifier>) -> Self {
        Self {
            pool,
            verifier,
            demo: false,
            ops_token: None,
            near: NearState::default(),
        }
    }

    pub fn with_near(mut self, near: NearState) -> Self {
        self.near = near;
        self
    }
}
