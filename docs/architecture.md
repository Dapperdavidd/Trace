# TRACE architecture

## System shape

```text
web and mobile
      |
      v
  trace-api -------------------- PostgreSQL
                                    ^
                                    |
                              trace-worker
                              /    |     \
                           RPC   prices   jobs
```

The API and worker share one PostgreSQL database and run the same forward-only migrations. API requests read stored evidence and analyses. They do not call chain RPCs or price providers directly.

## Evidence flow

1. Enabled chain adapters read bounded ranges from configured RPC nodes.
2. Raw provider payloads are stored for replay with content hashes.
3. Parsers create chain-aware normalized swaps with explicit finality and attribution.
4. Accepted price quotes are stored as exact USD-per-base-unit decimals.
5. The market job derives partial price and volume snapshots from real swaps and accepted prices.
6. Accounting, discovery, risk, wallet intelligence, Shadow, signals, and alerts read the stored evidence.
7. Analytic API responses report whether their data is live, stale, partial, unavailable, or demo.

## Worker jobs

| Job | Input | Output | Safety gate |
| --- | --- | --- | --- |
| EVM ingestion | Enabled deployment and RPC | Raw events and normalized swaps | Registry status and reviewed fixtures |
| EVM historical backfill | Explicit finalized block range | Raw events and normalized swaps | Separate resumable progress, 200 block chunks, shared ingestion lock |
| Solana ingestion | Enabled program and RPC | Raw events and normalized swaps | Registry status and reviewed fixtures |
| Solana historical backfill | Explicit finalized signature boundaries | Raw events and normalized swaps | Registry status, separate progress, and configured signature limit |
| Price polling | Recent real tokens and provider | `price_points` | Freshness, confidence, decimals, and positive exact values |
| Historical price backfill | Explicit asset, time range, and interval | `price_points` | Verified decimals, provider timestamps, confidence, and 30 minute tolerance |
| EVM token metadata | Incomplete tokens and configured RPCs | Decimals, symbols, and names | Direct contract reads, validation, and retry state |
| Market snapshots | Final swaps and accepted prices | Price and volume windows | Missing prices skip the snapshot; missing liquidity stays null |
| Watchlist alerts | Final direct swaps | In-app buy and sell alerts | Stable-pair direction, preferences, pause, and deduplication |
| Research signals | Three or more distinct finalized direct buyers | Sealed receipt and append-only lifecycle | Fresh snapshot, reviewed risk inputs, one-hour cooldown, unknown risk is watch-only |
| Signal outcomes | Published sealed receipts | Fixed-horizon outcomes | First valid price after each horizon; missing liquidity is untradeable |

Jobs use leases, bounded retries, deduplication keys, and dead-job states. Ingestion also uses PostgreSQL advisory locks so only one worker handles a protocol at a time.

## Identity and money

- Wallets and assets are always identified by chain plus address or mint.
- Token amounts are integer base units.
- Prices and USD values use exact decimals.
- Missing values remain unavailable. They are never converted to zero.
- Stable assets come from a reviewed chain-specific registry.
- Native Solana is normalized to wrapped SOL for token accounting. Uniswap V2 pool legs use WETH, not an invented native ETH token.

## Truthfulness boundaries

- Demo rows are stored with `demo = true` and labelled in every client.
- Stale data never becomes live because a page was refreshed.
- Partial market snapshots cannot enter Top Movers or Shadow without reviewed liquidity.
- Unsupported and ambiguous transactions are skipped and counted.
- Shadow is paper-only. No backend path signs or broadcasts a trade.
- Signal receipts are sealed at publication and later outcomes cannot replace an earlier result.

## API and clients

The contract lives in [api-contract.md](api-contract.md) and [openapi.snapshot.json](openapi.snapshot.json). Snapshot tests prevent silent drift. Web and mobile format money from strings and provide loading, empty, error, offline, freshness, and data-status states.

## Operations

The API and worker container targets share one build stage and run as a non-root user. CI checks Rust, web, mobile, and both production container targets. Production configuration, monitoring, backups, restoration, retention, rate limiting, and secret rotation are described or tracked in [deploy.md](deploy.md).

## Known gaps

- Live liquidity and token-risk providers are not selected.
- Solana ingestion stores consistent mint decimals from finalized transaction balances and audits conflicts. Names and symbols still need a real chain reader.
- EVM block, Solana signature, and historical price backfill are available.
- Push and email alert delivery are not implemented.
- Production hosting, Google OAuth credentials, app-store accounts, and release operations require owner decisions or credentials.
