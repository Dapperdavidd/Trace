//! Core domain types for TRACE: chain-aware identities, exact amounts,
//! data-status honesty flags and the API error model.
//!
//! Invariants (see docs/adr/0001-architecture.md):
//! * every wallet is `(chain, address)` and every asset is `(chain, contract_or_mint)`;
//! * token amounts are integer base units, prices and USD values are exact decimals;
//! * missing data is a first-class state, never silently defaulted.

pub mod address;
pub mod amount;
pub mod chain;
pub mod error;
pub mod status;
pub mod swap;

pub use address::{Address, AssetId, WalletId};
pub use amount::BaseUnits;
pub use chain::ChainId;
pub use error::{ApiErrorBody, DomainError};
pub use status::{DataStatus, Provenance, SupportStatus};
pub use swap::{Attribution, Finality, NormalizedSwap};
