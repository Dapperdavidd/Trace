# Changelog

## 0.1.0 (milestone 1: foundation, 2026-10-28)
- Cargo workspace, Actix Web API skeleton, request ids, error envelope, CORS.
- `/v1/health/live`, `/v1/health/ready`, `/v1/chains`, `/v1/openapi.json`.
- Domain types: ChainId, Address, WalletId, AssetId, BaseUnits, DataStatus, SupportStatus.
- Forward-only migration for chains, protocols, coverage snapshots, provider quotas, audit events.
- Coverage registry: every protocol disabled or unsupported until fixtures prove it.
- Docker Compose for PostgreSQL, `.env.example`, ADR 0001, coverage register, provider matrix.
