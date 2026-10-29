# Changelog

## 0.1.0 (milestone 1: foundation, 2026-10-28)
- Cargo workspace, Actix Web API skeleton, request ids, error envelope, CORS.
- `/v1/health/live`, `/v1/health/ready`, `/v1/chains`, `/v1/openapi.json`.
- Domain types: ChainId, Address, WalletId, AssetId, BaseUnits, DataStatus, SupportStatus.
- Forward-only migration for chains, protocols, coverage snapshots, provider quotas, audit events.
- Coverage registry: every protocol disabled or unsupported until fixtures prove it.
- Docker Compose for PostgreSQL, `.env.example`, ADR 0001, coverage register, provider matrix.

## Milestone 3 (2026-10-28)
- Added `risk` (token risk policy), `intelligence` (WQS, WCS, TCQ, Trading DNA), `signals` (lifecycle, sealed receipts, outcomes), `shadow` (paper simulation) and `discovery` (candidates, independent buyers, Top Movers).
- Added `docs/api-contract.md` and ADR 0003.
- All score defaults are unvalidated. Parsers remain disabled until real fixtures exist.

## Milestone 5 (2026-10-29)
- Product schema migration: users, sessions, watchlists, alerts, price points, market snapshots, signals, outcomes, paper strategies.
- Auth: Google ID token verification (RS256 only, audience, issuer, expiry, verified email), opaque hashed sessions, account deletion.
- API routes for account, watchlist, alerts, discovery, wallet research, tokens, signals, paper strategies and briefings.
- Request ids now appear in errors raised inside handlers.
- Demo seeder, all rows flagged demo.
- Shadow rules settled: unsupported rules are refused rather than approximated. Web and mobile option lists updated.

## Milestone 6 (2026-10-29)
- `trace-jobs`: durable PostgreSQL job queue (idempotent enqueue, SKIP LOCKED leasing, crash recovery, transient and permanent failures, capped backoff, dead jobs kept).
- `trace-worker`: signal outcome evaluation using the first available price at or after each horizon, append-only outcomes, lifecycle events, tamper detection. Periodic scheduling and a drain loop.

## Milestone 7: real-fixture capture and gate

- `tools/fixture-capture`: `capture-fixture evm|solana` saves a real transaction as an unreviewed fixture. The RPC URL comes from the environment and only its host is stored.
- Fixtures carry provenance and a payload hash. The tool never fills `expected`; a human verifies against a block explorer.
- Gate tests: reviewed fixtures must reproduce `expected`, tampering fails, and no protocol may be Partial or Enabled without 3 distinct reviewed real transactions.
- `docs/fixtures.md` documents capture and review. No real fixtures exist yet, so every protocol stays Disabled.
