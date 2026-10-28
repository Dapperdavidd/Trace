//! Chain adapters: the capability registry plus protocol parsers that turn raw
//! chain payloads into canonical swaps.
//!
//! Parsers are pure functions over already-fetched payloads. Fetching is done
//! by the indexer so that every parser can be replayed from stored raw events.

pub mod evm_v2;
pub mod registry;
pub mod solana;

pub use registry::*;
