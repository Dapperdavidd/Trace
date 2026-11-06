//! NEAR is TRACE's authorization and cross-chain execution control plane.
//!
//! This crate deliberately separates quote preparation from signing. TRACE may
//! request dry NEAR Intents quotes, but a wallet must authorize every live
//! action unless an on-chain mandate explicitly grants narrow delegated access.

mod intents;
mod mandate;
mod rpc;

pub use intents::{Asset, IntentsClient, IntentsError, QuotePreview, QuoteRequest, QuoteResponse};
pub use mandate::{AgentMandate, AssetPermission, MandateError, MandateMode, MandateStatus};
pub use rpc::{NearRpcClient, NearRpcError, TransactionStatus};
