# ADR 0004: Reviewed stable asset registry

Status: accepted, 2026-11-02

## Context

Accounting treats stable assets differently when a historical market price is unavailable. The schema had an `is_stable` flag, but only demo seeding set it. Real USDC, USDT and DAI legs were therefore counted as unpriced.

Token symbols are not identities. The registry must use a chain and canonical contract or mint address. Incorrectly marking an asset stable can hide real price risk, so entries need an issuer or protocol source and a code review.

## Decision

The forward-only database migration installs this initial reviewed registry:

| Chain | Asset | Contract or mint | Decimals | Source |
| --- | --- | --- | ---: | --- |
| Ethereum | USDC | `0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48` | 6 | Circle USDC contract addresses |
| Ethereum | USDT | `0xdac17f958d2ee523a2206206994597c13d831ec7` | 6 | Tether supported protocols |
| Ethereum | DAI | `0x6b175474e89094c44da98b954eedeac495271d0f` | 18 | Sky ecosystem DAI guide |
| Base | USDC | `0x833589fcd6edb6e08f4c7c32d4f71b54bda02913` | 6 | Circle USDC contract addresses |
| Solana | USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | 6 | Circle USDC contract addresses |
| Solana | USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` | 6 | Tether supported protocols |

Sources reviewed on 2026-11-02:

- <https://developers.circle.com/stablecoins/usdc-contract-addresses>
- <https://tether.to/en/supported-protocols/>
- <https://github.com/sky-ecosystem/developerguides/blob/master/dai/dai-token/dai-token.md>

EVM addresses are stored lowercase because normalized swap assets use lowercase addresses. The migration sets reviewed metadata and `is_stable = TRUE` even when ingestion saw the token first.

BNB Chain assets are intentionally omitted. The common Binance-Peg tokens need a separate issuer and custody review before TRACE treats them as stable.

## Consequences

Real accounting can recognize these assets without demo seeding. Adding or removing an entry requires a new migration, a source review, and an updated database test. A stable classification does not mean risk-free and does not replace live pricing when a valid price is available.
