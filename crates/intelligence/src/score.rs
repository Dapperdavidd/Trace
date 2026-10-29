use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Eligible,
    WatchOnly,
    InsufficientEvidence,
    Rejected,
}

/// A score is never a win probability. `value` is `None` whenever the
/// decision is `InsufficientEvidence`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Score {
    pub decision: Decision,
    pub value: Option<u8>,
    pub version: &'static str,
    pub components: BTreeMap<String, String>,
    pub cohort: String,
    pub window: String,
    pub explanation: String,
}
