//! Wallet and trade intelligence: three independent score dimensions and
//! explainable Trading DNA. Everything is deterministic, versioned and
//! configurable. Numeric weights and thresholds here are defaults that have
//! NOT been validated against profitability (see ADR 0003).

pub mod dna;
pub mod score;
pub mod tcq;
pub mod wcs;
pub mod wqs;

pub use score::{Decision, Score};
