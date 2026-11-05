use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://1click.chaindefuser.com";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub asset_id: String,
    pub decimals: u8,
    pub blockchain: String,
    pub symbol: String,
    pub contract_address: Option<String>,
}

/// Parameters accepted from TRACE clients. The server always sends a dry
/// request, so retrieving a preview can never initiate a swap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub origin_asset: String,
    pub destination_asset: String,
    /// Integer base units only.
    pub amount: String,
    pub slippage_tolerance: u16,
    pub recipient: String,
    pub refund_to: String,
    pub deadline: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotePreview {
    pub amount_in: String,
    pub amount_in_formatted: String,
    pub amount_in_usd: String,
    pub min_amount_in: String,
    pub amount_out: String,
    pub amount_out_formatted: String,
    pub amount_out_usd: String,
    pub min_amount_out: String,
    pub time_estimate: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub correlation_id: String,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub quote: QuotePreview,
}

#[derive(Debug, thiserror::Error)]
pub enum IntentsError {
    #[error("amount must be a positive integer string in base units")]
    InvalidAmount,
    #[error("slippage tolerance must be at most 2,000 basis points")]
    InvalidSlippage,
    #[error("quote deadline must be in the future")]
    InvalidDeadline,
    #[error("NEAR Intents request failed")]
    Request(#[from] reqwest::Error),
    #[error("NEAR Intents returned HTTP {status}: {message}")]
    Upstream { status: u16, message: String },
}

#[derive(Clone)]
pub struct IntentsClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl IntentsClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(DEFAULT_BASE_URL, api_key)
    }

    pub fn with_base_url(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
        }
    }

    pub async fn assets(&self) -> Result<Vec<Asset>, IntentsError> {
        let response = self
            .http
            .get(format!("{}/v0/tokens", self.base_url))
            .send()
            .await?;
        decode(response).await
    }

    /// Request a non-executable preview. TRACE never exposes a server-side
    /// endpoint that submits deposits or signed intents.
    pub async fn preview(&self, request: &QuoteRequest) -> Result<QuoteResponse, IntentsError> {
        validate_request(request)?;
        let payload = serde_json::json!({
            "dry": true,
            "swapType": "EXACT_INPUT",
            "slippageTolerance": request.slippage_tolerance,
            "originAsset": request.origin_asset,
            "depositType": "ORIGIN_CHAIN",
            "destinationAsset": request.destination_asset,
            "amount": request.amount,
            "recipient": request.recipient,
            "recipientType": "DESTINATION_CHAIN",
            "refundTo": request.refund_to,
            "refundType": "ORIGIN_CHAIN",
            "deadline": request.deadline,
        });
        let mut call = self
            .http
            .post(format!("{}/v0/quote", self.base_url))
            .json(&payload);
        if let Some(key) = &self.api_key {
            call = call.header("x-api-key", key);
        }
        decode(call.send().await?).await
    }
}

fn validate_request(request: &QuoteRequest) -> Result<(), IntentsError> {
    if request.amount.is_empty()
        || request.amount == "0"
        || !request.amount.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(IntentsError::InvalidAmount);
    }
    if request.slippage_tolerance > 2_000 {
        return Err(IntentsError::InvalidSlippage);
    }
    if request.deadline <= Utc::now() {
        return Err(IntentsError::InvalidDeadline);
    }
    Ok(())
}

async fn decode<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, IntentsError> {
    let status = response.status();
    if !status.is_success() {
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "unreadable response".into());
        return Err(IntentsError::Upstream {
            status: status.as_u16(),
            message: message.chars().take(300).collect(),
        });
    }
    Ok(response.json().await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn request() -> QuoteRequest {
        QuoteRequest {
            origin_asset: "nep141:wrap.near".into(),
            destination_asset: "nep141:eth.omft.near".into(),
            amount: "1000000000000000000000000".into(),
            slippage_tolerance: 100,
            recipient: "0x0000000000000000000000000000000000000001".into(),
            refund_to: "alice.near".into(),
            deadline: Utc::now() + Duration::minutes(5),
        }
    }

    #[test]
    fn rejects_decimal_and_zero_amounts() {
        let mut value = request();
        value.amount = "0.1".into();
        assert!(matches!(
            validate_request(&value),
            Err(IntentsError::InvalidAmount)
        ));
        value.amount = "0".into();
        assert!(matches!(
            validate_request(&value),
            Err(IntentsError::InvalidAmount)
        ));
    }

    #[test]
    fn validates_preview_request() {
        assert!(validate_request(&request()).is_ok());
    }
}
