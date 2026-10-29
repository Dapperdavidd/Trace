//! Discovery: candidate wallets from observed market activity, infrastructure
//! and bot screening, independent-buyer counting and Top Movers.
//!
//! Candidates originate from observed eligible swaps, never from a hard-coded
//! list of famous wallets. Uncertain classifications stay uncertain.

pub mod buyers;
pub mod candidates;
pub mod movers;
