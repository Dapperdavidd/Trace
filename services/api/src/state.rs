use crate::auth::{Disabled, IdTokenVerifier};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub verifier: Arc<dyn IdTokenVerifier>,
    /// True when demo data is enabled. New accounts then receive starter demo alerts.
    pub demo: bool,
}

impl AppState {
    /// State with Google sign-in unconfigured. Sign-in answers 503 until a
    /// verifier is supplied; nothing else depends on it.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            verifier: Arc::new(Disabled),
            demo: false,
        }
    }

    pub fn with_demo(mut self, demo: bool) -> Self {
        self.demo = demo;
        self
    }

    pub fn with_verifier(pool: PgPool, verifier: Arc<dyn IdTokenVerifier>) -> Self {
        Self {
            pool,
            verifier,
            demo: false,
        }
    }
}
