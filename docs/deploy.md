# Deploying TRACE

```
 browser / app ──► TLS proxy ──► trace-api ──► PostgreSQL ◄── trace-worker ──► RPC nodes, price API
       │                            │    │                         │
 NEAR wallet ──► mandate contract ◄─┘    └──► NEAR Intents        │
                          /v1/ops/metrics ◄── Prometheus ◄─────────┘
```

Two processes share one PostgreSQL database: `trace-api` (HTTP) and `trace-worker` (queue: ingestion, prices, snapshots, alerts, signals, and outcomes). Migrations run when either starts. They are forward-only.

## Environment

| Variable | Process | Required | Notes |
|---|---|---|---|
| `DATABASE_URL` | both | yes | Use a strong password. The dev password is refused in production |
| `TRACE_ENV` | api | yes | Set `production` to enable the startup safety checks below |
| `TRACE_PASSKEY_RP_ID` | api | yes in prod | Stable passkey domain, such as `trace.example`; changing it invalidates credentials |
| `TRACE_PASSKEY_RP_ORIGIN` | api | yes in prod | Exact HTTPS web origin allowed to use the passkeys |
| `GOOGLE_CLIENT_ID` | api | optional | Alternative identity provider web client id used to verify ID tokens |
| `TRACE_CORS_ORIGINS` | api | yes | Comma separated, https only in production |
| `TRACE_BIND_ADDR` | api | no | Default `127.0.0.1:8080`; the image sets `0.0.0.0:8080` |
| `TRACE_DB_MAX_CONNECTIONS` | api | no | Default 10. The worker needs at least 2 |
| `TRACE_OPS_TOKEN` | api | no | 24+ characters. Without it `/v1/ops/metrics` returns 404 |
| `TRACE_NEAR_NETWORK` | api | no | Defaults to `testnet`; set `mainnet` only after review |
| `TRACE_NEAR_RPC_URL` | api | no | Defaults to the public RPC for the configured network; use a production provider for launch |
| `TRACE_NEAR_CONTRACT_ID` | api | for anchoring | Deployed TRACE mandate contract account |
| `TRACE_NEAR_AGENT_ACCOUNT_ID` | api | for mandates | Named agent account allowed by generated policies |
| `TRACE_NEAR_DELEGATION_ENABLED` | api | never before review | Enables delegated mandate drafts; defaults off |
| `TRACE_NEAR_INTENTS_BASE_URL` | api | no | Defaults to the official 1Click endpoint |
| `TRACE_NEAR_INTENTS_API_KEY` | api | no | Partner key for 1Click; keep in the secret store |
| `TRACE_RPC_ETHEREUM` | worker | for ingestion | RPC URL; treat as a secret |
| `TRACE_RPC_SOLANA` | worker | for ingestion | RPC URL; treat as a secret |
| `TRACE_PRICE_PROVIDER` | worker | for prices | `defillama`. Off when unset |
| `TRACE_PRICE_API_BASE` | worker | no | Override the provider host |
| `TRACE_DEV_AUTH` | api | never in prod | Accepts `dev:<email>` as sign-in |
| `TRACE_SEED_DEMO` | api | never in prod | Seeds labelled fake data |

With `TRACE_ENV=production` the api refuses to start if any of these hold: dev
auth on, demo seeding on, a localhost passkey RP, a non-HTTPS passkey origin, a CORS origin that is `*`,
localhost or not https, an ops token under 24 characters, the development
database password, or delegation enabled without both NEAR contract and agent
accounts. It lists every problem, not only the first.

## Order of work

1. Create PostgreSQL (managed if you can) and a strong password. Turn on automated backups and test one restore.
2. Build the images (`docker build --target api` and `--target worker`). CI builds both targets on every push and pull request.
3. Put the secrets in your host's secret store, not in the repo or the compose file.
4. Configure TLS 1.2 or newer, HSTS, request-size limits, and shared client-IP rate limits at ingress.
5. Start `api`, check `GET /v1/health/ready`, then start `worker`.
6. Ingestion stays idle until a protocol is Enabled in the registry **and** its RPC variable is set. Check the worker log for `ingestion is idle` or `ingestion active`.
7. Sign-in: set the final passkey RP domain and HTTPS origin. Publish Apple `webcredentials` and Android Digital Asset Links association files before native testing. Google can remain an optional alternative sign-in method.
8. Deploy `trace-mandates-contract` to a dedicated NEAR testnet account, set
   `TRACE_NEAR_CONTRACT_ID`, and verify the deployed WebAssembly hash.
9. Create a dedicated NEAR agent account. Keep delegation off while testing
   approval-required mandates and dry Intents quotes.

## Historical EVM backfill

Backfill is explicit and never starts automatically. The protocol must be Enabled or Partial and the worker must have its chain RPC configured. Request a finalized inclusive block range from the same release as the worker:

```bash
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-backfill -- \
  request ethereum uniswap_v2 19000000 19010000
```

The running worker processes at most 200 finalized blocks per job and stores progress separately from the live ingestion cursor. A failed range is retried without advancing progress. One request is limited to 2,000,000 blocks. Split larger work into reviewed ranges.

```bash
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-backfill -- status
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-backfill -- status RUN_ID
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-backfill -- cancel RUN_ID
```

Test the range in staging first. Compare inserted swaps, duplicate counts, skipped reasons, and raw evidence before requesting the production range.

## Historical price backfill

Historical prices are also explicit and never start automatically. The token must already exist with verified decimals, and `TRACE_PRICE_PROVIDER=defillama` must be configured on the worker. Times use RFC 3339 and the interval is between 300 and 86400 seconds.

```bash
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-price-backfill -- \
  request ethereum 0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2 \
  2026-10-01T00:00:00Z 2026-10-31T00:00:00Z 3600

DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-price-backfill -- status
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-price-backfill -- status RUN_ID
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-price-backfill -- cancel RUN_ID
```

Each worker job requests at most 40 points. Provider or database failures do not advance progress. Missing, low-confidence, or distant points are counted as rejected rather than invented. One run is limited to 10,000 requested points.

## Historical Solana backfill

Solana backfill uses two finalized transaction signatures as exclusive boundaries. `before-signature` is the newer boundary and `until-signature` is the older boundary. The protocol must be Enabled or Partial and `TRACE_RPC_SOLANA` must be configured. A Disabled protocol can be requested for planning, but the worker will not schedule it.

```bash
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-solana-backfill -- \
  request raydium_amm_v4 BEFORE_SIGNATURE UNTIL_SIGNATURE 10000

DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-solana-backfill -- status
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-solana-backfill -- status RUN_ID
DATABASE_URL="$DATABASE_URL" cargo run -p trace-worker --bin trace-solana-backfill -- cancel RUN_ID
```

The worker reads finalized signatures in pages of 100 and shares the live ingestion lock. Progress never moves the live cursor. If `max-signatures` is exhausted before the older boundary is proven reached, the run ends as `limit_reached`, not `completed`. Continue with a reviewed follow-up range starting at the reported `next_before` signature.

## Monitoring

Scrape `/v1/ops/metrics` with `Authorization: Bearer $TRACE_OPS_TOKEN` (Prometheus text format, read from the database).

```yaml
scrape_configs:
  - job_name: trace
    metrics_path: /v1/ops/metrics
    authorization: { type: Bearer, credentials_file: /etc/prometheus/trace-ops-token }
    static_configs: [{ targets: ["trace-api.internal:8080"] }]
```

Suggested alerts:

| Alert | Expression | Meaning |
|---|---|---|
| Worker stalled | `trace_oldest_due_job_age_seconds > 300` | Jobs are due but nothing runs them |
| Ingestion stale | `trace_ingest_last_scan_age_seconds > 600` | A protocol stopped scanning |
| Never scanned | `trace_ingest_enabled_never_scanned > 0` | Enabled but the worker or RPC is not working |
| Dead jobs | `trace_jobs{status="dead"} > 0` | A job exhausted its attempts; read `worker_jobs.last_error` |
| Data gap | `increase(trace_ingest_gaps[1h]) > 0` | A range was not read; coverage has a hole |
| Backfill stalled | `trace_backfill_oldest_update_age_seconds > 600` | A requested historical range is not progressing |
| Price backfill stalled | `trace_price_backfill_oldest_update_age_seconds > 600` | A requested historical price range is not progressing |
| Solana backfill stalled | `trace_solana_backfill_oldest_update_age_seconds > 600` | A requested finalized signature range is not progressing |
| Prices stale | `trace_price_latest_age_seconds > 1800` | Price polling stopped (only if prices are on) |
| Not ready | probe `/v1/health/ready` fails | Database unreachable |

Logs are JSON on stdout. Errors never contain RPC URLs or keys.

### Retention

Retention is disabled unless at least one policy setting is present. The worker runs one bounded batch per configured table each day. Fixture rows are never removed, unfinished job attempts are never removed, and invalid or overly aggressive settings stop the worker at startup.

| Setting | Minimum | Meaning |
|---|---:|---|
| `TRACE_RETENTION_RAW_EVENT_DAYS` | 30 | Age for non-fixture raw chain payloads |
| `TRACE_RETENTION_JOB_ATTEMPT_DAYS` | 7 | Age for finished job-attempt records |
| `TRACE_RETENTION_AUDIT_EVENT_DAYS` | 90 | Age for audit records |
| `TRACE_RETENTION_BATCH_SIZE` | 1 | Rows removed from each configured table per daily run, default 1000 and maximum 10000 |

Set only the tables covered by the approved production policy. Keep raw events long enough to replay every supported parser window. Backups can retain deleted rows separately, so their expiry policy must be configured with the selected database host.

Incident diagnosis and recovery procedures are in [runbook.md](runbook.md).

## Not done

* No log shipping, tracing or paging integration. The alert rules above are a starting point.
* The api has per-peer and per-session in-process rate limits. Production still needs a shared proxy or platform limiter because counters are not coordinated across replicas and only the proxy knows the original client IP.
* The web app is deployed separately (for example Vercel). Set its API base URL to the api's public https origin.
