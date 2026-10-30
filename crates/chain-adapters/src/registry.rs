//! Chain and protocol capability registry.
//!
//! The registry is the single, code-defined statement of what TRACE can parse.
//! A protocol is only `Enabled` after its parser passes golden fixtures built
//! from real transactions. Everything else is explicitly `Disabled` (with the
//! blocker named) or `Unsupported`; activity on such protocols is never
//! classified or guessed.

use serde::Serialize;
use trace_domain::{ChainId, SupportStatus};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProtocolCapability {
    pub chain: ChainId,
    pub protocol: &'static str,
    pub family: &'static str,
    pub status: SupportStatus,
    pub limitations: Vec<&'static str>,
    /// What must happen before this protocol can be enabled.
    pub blocker: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChainCapability {
    pub chain: ChainId,
    pub display_name: &'static str,
    /// Chain-level status: the best status among its protocols.
    pub status: SupportStatus,
    pub watch_only_supported: bool,
    pub protocols: Vec<ProtocolCapability>,
}

const NO_FIXTURE: &str =
    "Parser not yet verified against golden fixtures from real transactions; not enabled.";

fn cap(
    chain: ChainId,
    protocol: &'static str,
    family: &'static str,
    status: SupportStatus,
    limitations: Vec<&'static str>,
    blocker: Option<&'static str>,
) -> ProtocolCapability {
    ProtocolCapability {
        chain,
        protocol,
        family,
        status,
        limitations,
        blocker,
    }
}

/// All declared protocols. Statuses are updated only alongside the fixtures
/// and evidence that justify them (see docs/coverage-register.md).
pub fn protocols() -> Vec<ProtocolCapability> {
    use ChainId::*;
    use SupportStatus::*;
    vec![
        cap(
            Solana,
            "raydium_amm_v4",
            "constant-product AMM",
            Disabled,
            vec![
                "instruction and inner-instruction decoding required",
                "wrapped SOL handling required",
            ],
            Some(NO_FIXTURE),
        ),
        cap(
            Solana,
            "orca_whirlpool",
            "concentrated liquidity",
            Unsupported,
            vec![],
            Some("No adapter planned for MVP."),
        ),
        cap(
            Ethereum,
            "uniswap_v2",
            "constant-product AMM",
            Enabled,
            vec!["Swap log + Transfer context required"],
            None,
        ),
        cap(
            Ethereum,
            "uniswap_v3",
            "concentrated liquidity",
            Unsupported,
            vec![],
            Some("No adapter planned for MVP."),
        ),
        cap(
            Base,
            "uniswap_v2_forks",
            "constant-product AMM",
            Disabled,
            vec!["factory allowlist required"],
            Some(NO_FIXTURE),
        ),
        cap(
            Bnb,
            "pancakeswap_v2",
            "constant-product AMM",
            Disabled,
            vec!["factory allowlist required"],
            Some(NO_FIXTURE),
        ),
    ]
}

pub fn chains() -> Vec<ChainCapability> {
    let all = protocols();
    ChainId::ALL
        .iter()
        .map(|&chain| {
            let ps: Vec<_> = all.iter().filter(|p| p.chain == chain).cloned().collect();
            ChainCapability {
                chain,
                display_name: chain.display_name(),
                status: best_status(&ps),
                // Any syntactically valid address can be watched regardless of parser support.
                watch_only_supported: true,
                protocols: ps,
            }
        })
        .collect()
}

fn best_status(ps: &[ProtocolCapability]) -> SupportStatus {
    use SupportStatus::*;
    if ps.iter().any(|p| p.status == Enabled) {
        if ps.iter().all(|p| p.status == Enabled) {
            Enabled
        } else {
            Partial
        }
    } else if ps.iter().any(|p| p.status == Partial) {
        Partial
    } else if ps.iter().any(|p| p.status == Disabled) {
        Disabled
    } else {
        Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_target_chain_has_a_capability_entry() {
        let c = chains();
        assert_eq!(c.len(), 4);
        assert!(c.iter().all(|c| c.watch_only_supported));
    }

    #[test]
    fn only_fixture_backed_protocols_are_enabled() {
        let enabled: Vec<_> = protocols()
            .into_iter()
            .filter(|p| p.status == SupportStatus::Enabled)
            .map(|p| (p.chain, p.protocol))
            .collect();
        assert_eq!(enabled, vec![(ChainId::Ethereum, "uniswap_v2")]);
    }

    #[test]
    fn disabled_protocols_name_their_blocker() {
        for p in protocols() {
            if matches!(
                p.status,
                SupportStatus::Disabled | SupportStatus::Unsupported
            ) {
                assert!(p.blocker.is_some(), "{} {}", p.chain, p.protocol);
            }
        }
    }
}
