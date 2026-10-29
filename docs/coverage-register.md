# Protocol coverage register

Statuses: enabled (fixtures plus real-chain proof), partial, disabled (blocker named), unsupported.
Nothing below is enabled. This file changes in the same commit as the fixtures that justify it.

| Chain | Protocol | Status | Blocker |
|---|---|---|---|
| Solana | raydium_amm_v4 | disabled | Parser not verified against golden fixtures from real transactions |
| Solana | orca_whirlpool | unsupported | No adapter planned for MVP |
| Ethereum | uniswap_v2 | disabled | Parser not verified against golden fixtures |
| Ethereum | uniswap_v3 | unsupported | No adapter planned for MVP |
| Base | uniswap_v2_forks | disabled | Factory allowlist and fixtures required |
| BNB Chain | pancakeswap_v2 | disabled | Factory allowlist and fixtures required |

Watch-only monitoring accepts any syntactically valid address on all four chains. When a wallet's
activity cannot be parsed the API will report `partial_monitoring`, `unsupported_protocol` or
`waiting_for_activity`, never a fake alert.


## Update 2026-10-28
No protocol was enabled in milestone 3. Pure-logic crates were added and tested on synthetic data only.
