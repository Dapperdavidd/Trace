# Protocol coverage register

Statuses: enabled (fixtures plus real-chain proof), partial, disabled (blocker named), unsupported.
This file changes in the same commit as the fixtures that justify an enabled status.

| Chain | Protocol | Status | Blocker |
|---|---|---|---|
| Solana | raydium_amm_v4 | disabled | Parser not verified against golden fixtures from real transactions |
| Solana | orca_whirlpool | unsupported | No adapter planned for MVP |
| Ethereum | uniswap_v2 | enabled | None |
| Ethereum | uniswap_v3 | unsupported | No adapter planned for MVP |
| Base | uniswap_v2_forks | disabled | Factory allowlist and fixtures required |
| BNB Chain | pancakeswap_v2 | disabled | Factory allowlist and fixtures required |

Watch-only monitoring accepts any syntactically valid address on all four chains. When a wallet's
activity cannot be parsed the API will report `partial_monitoring`, `unsupported_protocol` or
`waiting_for_activity`, never a fake alert.


## Update 2026-10-28
No protocol was enabled in milestone 3. Pure-logic crates were added and tested on synthetic data only.

## Update 2026-10-30
Uniswap V2 on Ethereum is enabled after four Ethereum mainnet fixtures were captured and
independently checked against Etherscan: a direct swap, a multi-hop swap, a
fee-on-transfer swap, and a non-swap liquidity removal. The verified contracts are the
Uniswap V2 factory `0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f` and Router02
`0x7a250d5630b4cf539739df2c5dacb4c659f2488d`.
