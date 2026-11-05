# Deploying TRACE

```
 browser / app ──► TLS proxy ──► trace-api ──► PostgreSQL ◄── trace-worker ──► RPC nodes, price API
                                    │                              │
                          /v1/ops/metrics (token) ◄── Prometheus ◄─┘ (reads the same database)
```

Two processes share one PostgreSQL database: `trace-api` (HTTP) and `trace-worker` (queue: ingestion, prices, snapshots, alerts, signals, and outcomes). Migrations run when either starts. They are forward-only.

## Environment

| Variable | Process | Required | Notes |
|---|---|---|---|
| `DATABASE_URL` | both | yes | Use a strong password. The dev password is refused in production |
| `TRACE_ENV` | api | yes | Set `production` to enable the startup safety checks below |
| `GOOGLE_CLIENT_ID` | api | yes in prod | Web client id used to verify ID tokens (audience check) |
| `TRACE_CORS_ORIGINS` | api | yes | Comma separated, https only in production |
| `TRACE_BIND_ADDR` | api | no | Default `127.0.0.1:8080`; the image sets `0.0.0.0:8080` |
| `TRACE_DB_MAX_CONNECTIONS` | api | no | Default 10. The worker needs at least 2 |
| `TRACE_OPS_TOKEN` | api | no | 24+ characters. Without it `/v1/ops/metrics` returns 404 |
| `TRACE_RPC_ETHEREUM` | worker | for ingestion | RPC URL; treat as a secret |
| `TRACE_RPC_SOLANA` | worker | for ingestion | RPC URL; treat as a secret |
| `TRACE_PRICE_PROVIDER` | worker | for prices | `defillama`. Off when unset |
| `TRACE_PRICE_API_BASE` | worker | no | Override the provider host |
| `TRACE_DEV_AUTH` | api | never in prod | Accepts `dev:<email>` as sign-in |
| `TRACE_SEED_DEMO` | api | never in prod | Seeds labelled fake data |

With `TRACE_ENV=production` the api refuses to start if any of these hold: dev auth on, demo seeding on, no `GOOGLE_CLIENT_ID`, a CORS origin that is `*`, localhost or not https, an ops token under 24 characters, or the development database password. It lists every problem, not only the first.

## Order of work

1. Create PostgreSQL (managed if you can) and a strong password. Turn on automated backups and test one restore.
2. Build the images (`docker build --target api` and `--target worker`). CI builds both targets on every push and pull request.
3. Put the secrets in your host's secret store, not in the repo or the compose file.
4. Configure TLS 1.2 or newer, HSTS, request-size limits, and shared client-IP rate limits at ingress.
5. Start `api`, check `GET /v1/health/ready`, then start `worker`.
6. Ingestion stays idle until a protocol is Enabled in the registry **and** its RPC variable is set. Check the worker log for `ingestion is idle` or `ingestion active`.
7. Sign-in: create a Google OAuth web client, authorise your web origin, set `GOOGLE_CLIENT_ID`, and sign in once end to end.

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
