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
