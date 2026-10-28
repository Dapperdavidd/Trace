use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

use crate::error::DomainError;

/// Supported target networks. Support for a chain does not imply support for
/// every protocol on it: see the coverage registry in `trace-chain-adapters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainId {
    Solana,
    Ethereum,
    Base,
    Bnb,
}

/// Address family a chain uses, which decides validation and canonicalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmFamily {
    Svm,
    Evm,
}

impl ChainId {
    pub const ALL: [ChainId; 4] = [
        ChainId::Solana,
        ChainId::Ethereum,
        ChainId::Base,
        ChainId::Bnb,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ChainId::Solana => "solana",
            ChainId::Ethereum => "ethereum",
            ChainId::Base => "base",
            ChainId::Bnb => "bnb",
        }
    }

    pub fn family(self) -> VmFamily {
        match self {
            ChainId::Solana => VmFamily::Svm,
            _ => VmFamily::Evm,
        }
    }

    /// EVM chain id. `None` for Solana, which uses a cluster identifier instead.
    pub fn evm_chain_id(self) -> Option<u64> {
        match self {
            ChainId::Ethereum => Some(1),
            ChainId::Base => Some(8453),
            ChainId::Bnb => Some(56),
            ChainId::Solana => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ChainId::Solana => "Solana",
            ChainId::Ethereum => "Ethereum",
            ChainId::Base => "Base",
            ChainId::Bnb => "BNB Chain",
        }
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChainId {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "solana" | "sol" => Ok(ChainId::Solana),
            "ethereum" | "eth" => Ok(ChainId::Ethereum),
            "base" => Ok(ChainId::Base),
            "bnb" | "bsc" => Ok(ChainId::Bnb),
            other => Err(DomainError::UnknownChain(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aliases_case_insensitively() {
        assert_eq!("SOL".parse::<ChainId>().unwrap(), ChainId::Solana);
        assert_eq!("bsc".parse::<ChainId>().unwrap(), ChainId::Bnb);
        assert!("dogechain".parse::<ChainId>().is_err());
    }

    #[test]
    fn evm_chain_ids_are_distinct() {
        assert_eq!(ChainId::Ethereum.evm_chain_id(), Some(1));
        assert_eq!(ChainId::Base.evm_chain_id(), Some(8453));
        assert_eq!(ChainId::Bnb.evm_chain_id(), Some(56));
        assert_eq!(ChainId::Solana.evm_chain_id(), None);
    }
}
