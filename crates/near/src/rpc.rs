use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatus {
    pub final_execution_status: String,
    pub status: Value,
    pub transaction: Value,
}

impl TransactionStatus {
    pub fn succeeded_finally(&self) -> bool {
        self.final_execution_status == "FINAL"
            && self
                .status
                .as_object()
                .is_some_and(|status| status.contains_key("SuccessValue"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NearRpcError {
    #[error("NEAR RPC request failed")]
    Request(#[from] reqwest::Error),
    #[error("NEAR RPC rejected the request")]
    Rpc(Value),
    #[error("NEAR RPC returned no result")]
    MissingResult,
    #[error("NEAR RPC returned an invalid result")]
    InvalidResult(#[source] serde_json::Error),
}

#[derive(Clone)]
pub struct NearRpcClient {
    http: reqwest::Client,
    url: String,
}

impl NearRpcClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            url: url.into(),
        }
    }

    pub async fn transaction_status(
        &self,
        tx_hash: &str,
        sender_account_id: &str,
    ) -> Result<TransactionStatus, NearRpcError> {
        let response: Value = self
            .http
            .post(&self.url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": "trace-near-anchor",
                "method": "tx_status",
                "params": {
                    "tx_hash": tx_hash,
                    "sender_account_id": sender_account_id,
                    "wait_until": "FINAL"
                }
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(error) = response.get("error") {
            return Err(NearRpcError::Rpc(error.clone()));
        }
        serde_json::from_value(
            response
                .get("result")
                .cloned()
                .ok_or(NearRpcError::MissingResult)?,
        )
        .map_err(NearRpcError::InvalidResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_final_success() {
        let mut value = TransactionStatus {
            final_execution_status: "FINAL".into(),
            status: serde_json::json!({"SuccessValue": ""}),
            transaction: Value::Null,
        };
        assert!(value.succeeded_finally());
        value.final_execution_status = "EXECUTED_OPTIMISTIC".into();
        assert!(!value.succeeded_finally());
        value.final_execution_status = "FINAL".into();
        value.status = serde_json::json!({"Failure": {}});
        assert!(!value.succeeded_finally());
    }
}
