use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{
    chain::{ChainId, VmFamily},
    error::DomainError,
};

/// A syntactically valid, canonicalized public address for one chain family.
///
/// EVM addresses are canonicalized to lowercase hex (checksum casing is not
/// verified: a mixed-case address is accepted and lowercased). Solana
/// addresses are base58 strings decoding to exactly 32 bytes and are kept
/// verbatim because base58 is case-sensitive.
///
/// A valid address is only a monitoring key. It grants no signing rights.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Address(String);

impl Address {
    pub fn parse(chain: ChainId, raw: &str) -> Result<Self, DomainError> {
        let raw = raw.trim();
        match chain.family() {
            VmFamily::Evm => parse_evm(chain, raw),
            VmFamily::Svm => parse_solana(chain, raw),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn invalid(chain: ChainId, reason: &str) -> DomainError {
    DomainError::InvalidAddress {
        chain,
        reason: reason.to_string(),
    }
}

fn parse_evm(chain: ChainId, raw: &str) -> Result<Address, DomainError> {
    let hex = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .ok_or_else(|| invalid(chain, "EVM addresses must start with 0x"))?;
    if hex.len() != 40 {
        return Err(invalid(chain, "EVM addresses must have 40 hex characters"));
    }
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid(
            chain,
            "EVM addresses may only contain hex characters",
        ));
    }
    Ok(Address(format!("0x{}", hex.to_ascii_lowercase())))
}

fn parse_solana(chain: ChainId, raw: &str) -> Result<Address, DomainError> {
    if raw.len() < 32 || raw.len() > 44 {
        return Err(invalid(
            chain,
            "Solana addresses are 32 to 44 base58 characters",
        ));
    }
    let bytes = bs58::decode(raw)
        .into_vec()
        .map_err(|_| invalid(chain, "not valid base58"))?;
    if bytes.len() != 32 {
        return Err(invalid(chain, "Solana addresses must decode to 32 bytes"));
    }
    Ok(Address(raw.to_string()))
}

/// A wallet is always identified by chain and address together.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WalletId {
    pub chain: ChainId,
    pub address: Address,
}

impl WalletId {
    pub fn parse(chain: ChainId, raw: &str) -> Result<Self, DomainError> {
        Ok(Self {
            chain,
            address: Address::parse(chain, raw)?,
        })
    }
}

/// An asset is always `(chain, contract_or_mint)`. Identical symbols on
/// different chains, or wrapped variants, are different assets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AssetId {
    pub chain: ChainId,
    pub contract_or_mint: Address,
}

impl AssetId {
    pub fn parse(chain: ChainId, raw: &str) -> Result<Self, DomainError> {
        Ok(Self {
            chain,
            contract_or_mint: Address::parse(chain, raw)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOL_WSOL: &str = "So11111111111111111111111111111111111111112";
    const EVM_MIXED: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

    #[test]
    fn evm_is_lowercased() {
        let a = Address::parse(ChainId::Ethereum, EVM_MIXED).unwrap();
        assert_eq!(a.as_str(), "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
    }

    #[test]
    fn evm_rejects_bad_shapes() {
        assert!(Address::parse(ChainId::Base, "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").is_err());
        assert!(Address::parse(ChainId::Base, "0x1234").is_err());
        assert!(
            Address::parse(ChainId::Base, "0xZZb86991c6218b36c1d19D4a2e9Eb0cE3606eB48").is_err()
        );
    }

    #[test]
    fn solana_accepts_real_mint_and_preserves_case() {
        let a = Address::parse(ChainId::Solana, SOL_WSOL).unwrap();
        assert_eq!(a.as_str(), SOL_WSOL);
    }

    #[test]
    fn solana_rejects_evm_and_garbage() {
        assert!(Address::parse(ChainId::Solana, EVM_MIXED).is_err());
        assert!(Address::parse(ChainId::Solana, "short").is_err());
        assert!(Address::parse(ChainId::Solana, "0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl").is_err());
    }

    #[test]
    fn same_address_on_two_evm_chains_is_two_wallets() {
        let a = WalletId::parse(ChainId::Ethereum, EVM_MIXED).unwrap();
        let b = WalletId::parse(ChainId::Base, EVM_MIXED).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.address, b.address);
    }
}
