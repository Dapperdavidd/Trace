use serde::{Deserialize, Serialize};

use crate::chain::ChainId;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("unknown chain '{0}'")]
    UnknownChain(String),
    #[error("invalid {chain} address: {reason}")]
    InvalidAddress { chain: ChainId, reason: String },
    #[error("invalid amount '{0}'")]
    InvalidAmount(String),
}

/// Consistent API error envelope. Never carries stack traces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub request_id: String,
}
