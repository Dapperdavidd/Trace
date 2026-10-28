use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// How trustworthy and fresh a piece of analytic output is. Every analytic
/// API response carries one. `Demo` data must always be conspicuously flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataStatus {
    /// Derived from observed events within the freshness budget.
    Live,
    /// Derived from observed events, but older than the freshness budget.
    Stale,
    /// Required inputs were unavailable; no value is asserted.
    Unavailable,
    /// Only part of the relevant history or protocols was covered.
    Partial,
    /// Seeded sample data. Never shown as live.
    Demo,
}

/// Whether a chain, protocol or feature is actually supported right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    /// Parsed against fixtures and verified against real chain data.
    Enabled,
    /// Works for a subset; limitations are listed.
    Partial,
    /// Adapter exists but is switched off; the blocker is documented.
    Disabled,
    /// No adapter. Activity here is never classified or guessed.
    Unsupported,
}

/// Where a datum came from and when, so clients can show source timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub observed_at: DateTime<Utc>,
    pub ingested_at: Option<DateTime<Utc>>,
}
