# TRACE

TRACE is a watch-only crypto research product for discovering on-chain activity, studying wallets and tokens, and testing Shadow strategies with paper money.

It has a Rust backend, PostgreSQL, a Next.js web app, and a Flutter mobile app. TRACE has no custody, signing, live trade execution, real-money trade mirroring, Send, Buy, or Swap. Those surfaces are labelled Coming later.

## Current coverage

| Capability | Status |
| --- | --- |
| Ethereum Uniswap V2 ingestion | Enabled with reviewed real fixtures |
| Solana Raydium AMM v4 | Disabled pending reviewed real fixtures |
| Base and BNB protocols | Disabled pending deployments and fixtures |
| Real token prices | DefiLlama adapter available, off until configured |
| Market snapshots | Real prices and finalized-swap volume; liquidity unavailable pending a reviewed source |
| Watchlist alerts | Real finalized direct swaps, in-app only |
| Signals | Outcome evaluation exists; real signal publishing is not built |
| Shadow | Paper-only and gated by price and liquidity evidence |
| Google sign-in | Implemented; production credentials and an end-to-end live check are still required |

The code registry and [coverage register](docs/coverage-register.md) are the source of truth for protocol support. Disabled or unsupported activity is never guessed.

## Run locally

Start PostgreSQL:

```bash
docker compose -f infra/docker-compose.yml up -d
export DATABASE_URL=postgres://trace:trace@localhost:5432/trace
```

Start the API with clearly labelled demo data and development sign-in:

```bash
TRACE_SEED_DEMO=true TRACE_DEV_AUTH=true cargo run -p trace-api
```

Start the worker separately. Ingestion and pricing stay off unless their configuration is present:

```bash
TRACE_RPC_ETHEREUM='your secret RPC URL' \
TRACE_PRICE_PROVIDER=defillama \
cargo run -p trace-worker
```

Run the web app:

```bash
cd apps/web
npm ci
npm run dev
```

Run the mobile app:

```bash
cd apps/mobile
flutter pub get
flutter run
```

Never commit RPC URLs, OAuth credentials, or other secrets.

## Quality checks

```bash
export DATABASE_URL=postgres://trace:trace@localhost:5432/trace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Web checks are `npm run typecheck`, `npm run lint`, `npm test`, and `npm run build`. Mobile checks are `flutter analyze` and `flutter test`. GitHub Actions also builds the API and worker container targets.

## Repository layout

- `crates/` contains domain, database, adapters, accounting, intelligence, risk, pricing, signals, discovery, Shadow, and job-queue libraries.
- `services/api` exposes the HTTP API.
- `services/worker` runs ingestion, token metadata, pricing, market snapshots, alerts, and signal outcome jobs.
- `migrations/` contains forward-only SQLx migrations.
- `apps/web` and `apps/mobile` are the user clients.
- `docs/` contains the API contract, architecture, ADRs, deployment notes, fixture policy, and coverage register.

Start with [architecture.md](docs/architecture.md), [api-contract.md](docs/api-contract.md), and [deploy.md](docs/deploy.md).
