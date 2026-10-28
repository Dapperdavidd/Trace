# ADR 0001: Modular Rust workspace, watch-only MVP

Status: accepted, 2026-10-28

## Decisions
- One Cargo workspace. Shared libraries in `crates/`, binaries in `services/`.
- Actix Web 4 for HTTP (Axum is explicitly excluded by the v3 instructions), Tokio, SQLx, PostgreSQL.
- Runtime SQLx queries rather than compile-time macros so the repo builds without a live database.
- Chain-aware identities: wallet is `(chain, address)`, asset is `(chain, contract_or_mint)`.
  EVM addresses are lowercased, Solana base58 is case-preserved and must decode to 32 bytes.
- Money: token amounts are integer base units (`u128`, serialized as strings). Prices and USD values
  are exact decimals. Floats are never used for balances or PnL.
- Coverage is declared in code (`trace-chain-adapters`), mirrored into `protocols`, and exposed at
  `/v1/chains`. A protocol becomes `enabled` only together with golden fixtures and evidence.
- Error envelope: `code`, `message`, optional `details`, `request_id`. No stack traces.
- Auth is Google OIDC for app identity only. A signed-in account is not a wallet and has no signing rights.
- MVP is watch-only. No custody, signing, live execution or automatic trading.

## Consequences
- The API can start and report honest "unavailable" states before any chain parser is proven.
- Adding a protocol is a deliberate, tested change to the registry, the fixtures and the coverage register.
