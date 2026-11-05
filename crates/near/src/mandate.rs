use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateMode {
    Paper,
    ApprovalRequired,
    Delegated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateStatus {
    Draft,
    Active,
    Paused,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AssetPermission {
    pub chain: String,
    /// NEAR Intents asset identifier, for example `nep141:wrap.near`.
    pub asset_id: String,
}

/// Off-chain representation of the policy stored by the TRACE NEAR contract.
/// Monetary limits remain decimal strings in JSON and are never floats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMandate {
    pub id: Uuid,
    pub owner_near_account: String,
    pub agent_near_account: String,
    pub strategy_id: Uuid,
    pub mode: MandateMode,
    pub status: MandateStatus,
    pub allowed_chains: BTreeSet<String>,
    pub allowed_assets: BTreeSet<AssetPermission>,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_trade_usd: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_daily_usd: Decimal,
    pub max_slippage_bps: u16,
    pub expires_at: DateTime<Utc>,
    pub nonce: u64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MandateError {
    #[error("owner_near_account must be a plausible NEAR account ID")]
    InvalidOwner,
    #[error("agent_near_account must be a plausible NEAR account ID")]
    InvalidAgent,
    #[error("at least one allowed chain is required")]
    NoChains,
    #[error("at least one allowed asset is required outside paper mode")]
    NoAssets,
    #[error("money limits must be positive and the daily limit cannot be below the trade limit")]
    InvalidLimits,
    #[error("max_slippage_bps must be at most 2,000")]
    InvalidSlippage,
    #[error("the mandate must expire in the future")]
    InvalidExpiry,
    #[error("an allowed asset must belong to an allowed chain")]
    AssetChainMismatch,
    #[error("delegated mandates must use a non-zero nonce")]
    MissingDelegationNonce,
    #[error("mandate serialization failed")]
    Serialization,
}

impl AgentMandate {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), MandateError> {
        if !plausible_near_account(&self.owner_near_account) {
            return Err(MandateError::InvalidOwner);
        }
        if !plausible_near_account(&self.agent_near_account) {
            return Err(MandateError::InvalidAgent);
        }
        if self.allowed_chains.is_empty() {
            return Err(MandateError::NoChains);
        }
        if self.mode != MandateMode::Paper && self.allowed_assets.is_empty() {
            return Err(MandateError::NoAssets);
        }
        if self.max_trade_usd <= Decimal::ZERO || self.max_daily_usd < self.max_trade_usd {
            return Err(MandateError::InvalidLimits);
        }
        if self.max_slippage_bps > 2_000 {
            return Err(MandateError::InvalidSlippage);
        }
        if self.expires_at <= now {
            return Err(MandateError::InvalidExpiry);
        }
        if self
            .allowed_assets
            .iter()
            .any(|asset| !self.allowed_chains.contains(&asset.chain))
        {
            return Err(MandateError::AssetChainMismatch);
        }
        if self.mode == MandateMode::Delegated && self.nonce == 0 {
            return Err(MandateError::MissingDelegationNonce);
        }
        Ok(())
    }

    /// Stable SHA-256 commitment used to compare API state with NEAR state.
    pub fn commitment(&self) -> Result<String, MandateError> {
        let bytes = serde_json::to_vec(self).map_err(|_| MandateError::Serialization)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

fn plausible_near_account(value: &str) -> bool {
    let len = value.len();
    (2..=64).contains(&len)
        && !value.starts_with(['.', '-'])
        && !value.ends_with(['.', '-'])
        && value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn mandate() -> AgentMandate {
        AgentMandate {
            id: Uuid::nil(),
            owner_near_account: "alice.near".into(),
            agent_near_account: "trace-agent.near".into(),
            strategy_id: Uuid::nil(),
            mode: MandateMode::ApprovalRequired,
            status: MandateStatus::Draft,
            allowed_chains: ["eth".into(), "sol".into()].into(),
            allowed_assets: [AssetPermission {
                chain: "eth".into(),
                asset_id: "nep141:eth.omft.near".into(),
            }]
            .into(),
            max_trade_usd: Decimal::new(10000, 2),
            max_daily_usd: Decimal::new(50000, 2),
            max_slippage_bps: 100,
            expires_at: Utc::now() + Duration::days(7),
            nonce: 0,
        }
    }

    #[test]
    fn validates_a_scoped_approval_mandate() {
        let value = mandate();
        assert_eq!(value.validate(Utc::now()), Ok(()));
    }

    #[test]
    fn refuses_an_asset_from_an_unapproved_chain() {
        let mut value = mandate();
        value.allowed_assets.insert(AssetPermission {
            chain: "btc".into(),
            asset_id: "nep141:btc.omft.near".into(),
        });
        assert_eq!(
            value.validate(Utc::now()),
            Err(MandateError::AssetChainMismatch)
        );
    }

    #[test]
    fn commitment_is_stable_and_sensitive_to_policy_changes() {
        let value = mandate();
        let first = value.commitment().unwrap();
        assert_eq!(first, value.commitment().unwrap());
        let mut changed = value;
        changed.max_slippage_bps += 1;
        assert_ne!(first, changed.commitment().unwrap());
    }
}
