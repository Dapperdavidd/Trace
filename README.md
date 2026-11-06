# TRACE

TRACE is a NEAR-native agentic trading intelligence product. It discovers
on-chain activity, studies wallets and tokens, proves strategies with Shadow,
and expresses user-owned agent authority through NEAR mandates and NEAR
Intents.

It has a Rust backend, PostgreSQL, a deployable NEAR contract, a Next.js web
app, and a Flutter mobile app. TRACE has no custody and holds no wallet keys.
The API can prepare a wallet-signable mandate call and dry quote, but cannot
sign, deposit or submit funds. Live execution stays unavailable until the NEAR
deployment is verified and independently reviewed.

## Current coverage

| Capability | Status |
| --- | --- |
| NEAR agent mandates | Contract builds to WebAssembly; deployment account required |
| NEAR Intents | Live asset discovery and dry quote adapter; no server submission |
| NEAR agent delegation | Implemented behind a production-off safety switch pending deployment and review |
| External agent gateway | Revocable observe and propose credentials; no execute scope; NEAR mandate remains authoritative |
| MCP agent bridge | Compatible agents can read context and request dry NEAR-governed proposals |
| Ethereum Uniswap V2 ingestion | Enabled with reviewed real fixtures |
| Solana Raydium AMM v4 | Disabled pending reviewed real fixtures |
| Base and BNB protocols | Disabled pending deployments and fixtures |
| Real token prices | DefiLlama adapter available, off until configured |
| Market snapshots | Real prices and finalized-swap volume; liquidity unavailable pending a reviewed source |
| Watchlist alerts | Real finalized direct swaps, in-app only |
| Signals | Real finalized buys can publish sealed research signals when liquidity and risk inputs are complete; outcomes use fixed horizons |
| Shadow | Paper-only and gated by price and liquidity evidence |
| Passkey sign-in | Implemented on web and mobile; production domain association and physical-device checks remain |
| Google sign-in | Optional alternative; production credentials and an end-to-end live check remain |

NEAR is the authorization, evidence and cross-chain execution control plane.
Ethereum and Solana are markets TRACE observes. See
[`docs/near-architecture.md`](docs/near-architecture.md).

MCP-compatible agents can connect through the least-privilege bridge in
[`integrations/trace-mcp`](integrations/trace-mcp). The bridge has no execute
tool and never receives wallet keys.

The code registry and [coverage register](docs/coverage-register.md) are the source of truth for protocol support. Disabled or unsupported activity is never guessed.

## Run locally

Start PostgreSQL:

```bash
docker compose -f infra/docker-compose.yml up -d
export DATABASE_URL=postgres://trace:trace@localhost:5432/trace
```

Start the API with clearly labelled demo data and development sign-in:

```bash
TRACE_SEED_DEMO=true TRACE_DEV_AUTH=true \
TRACE_CORS_ORIGINS=http://127.0.0.1:3000,http://localhost:3000 \
cargo run -p trace-api
```

Start the worker separately. Ingestion and pricing stay off unless their configuration is present:

```bash
TRACE_RPC_ETHEREUM='your secret RPC URL' \
TRACE_PRICE_PROVIDER=defillama \
cargo run -p trace-worker
```

Historical EVM ranges can be queued without moving the live cursor:

```bash
cargo run -p trace-worker --bin trace-backfill -- \
  request ethereum uniswap_v2 FROM_BLOCK TO_BLOCK
```

The worker processes only finalized blocks and resumes from persisted progress. See `docs/deploy.md` for status and cancellation commands.

Historical token prices can be queued after token decimals are verified:

```bash
cargo run -p trace-worker --bin trace-price-backfill -- \
  request CHAIN ASSET FROM_RFC3339 TO_RFC3339 INTERVAL_SECONDS
```

Finalized Solana history can be queued between explicit signature boundaries after the protocol is enabled:

```bash
cargo run -p trace-worker --bin trace-solana-backfill -- \
  request PROTOCOL BEFORE_SIGNATURE UNTIL_SIGNATURE MAX_SIGNATURES
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

The chain-adapter suite includes randomized property tests for arbitrary EVM log bytes and Solana token amount strings. Parser boundaries must reject malformed or oversized values without panicking or wrapping.

## Repository layout

- `crates/` contains domain, database, adapters, accounting, intelligence, risk, pricing, signals, discovery, Shadow, and job-queue libraries.
- `services/api` exposes the HTTP API.
- `services/worker` runs ingestion, token metadata, pricing, market snapshots, alerts, and signal outcome jobs.
- `migrations/` contains forward-only SQLx migrations.
- `apps/web` and `apps/mobile` are the user clients.
- `docs/` contains the API contract, architecture, ADRs, deployment notes, operations runbook, fixture policy, and coverage register.

Start with [architecture.md](docs/architecture.md), [api-contract.md](docs/api-contract.md), and [deploy.md](docs/deploy.md).
