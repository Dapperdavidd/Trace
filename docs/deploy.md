# Deploying TRACE

```
 browser / app ──► TLS proxy ──► trace-api ──► PostgreSQL ◄── trace-worker ──► RPC nodes, price API
                                    │                              │
                          /v1/ops/metrics (token) ◄── Prometheus ◄─┘ (reads the same database)
```

Two processes share one PostgreSQL database: `trace-api` (HTTP) and `trace-worker` (queue: outcomes, ingestion, prices). Migrations run when either starts. They are forward-only.

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
2. Build the images (`docker build --target api` and `--target worker`). The Dockerfile has not been built in CI yet; do one build before relying on it.
3. Put the secrets in your host's secret store, not in the repo or the compose file.
4. Start `api`, check `GET /v1/health/ready`, then start `worker`.
5. Ingestion stays idle until a protocol is Enabled in the registry **and** its RPC variable is set. Check the worker log for `ingestion is idle` or `ingestion active`.
6. Sign-in: create a Google OAuth web client, authorise your web origin, set `GOOGLE_CLIENT_ID`, and sign in once end to end.

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
| Prices stale | `trace_price_latest_age_seconds > 1800` | Price polling stopped (only if prices are on) |
| Not ready | probe `/v1/health/ready` fails | Database unreachable |

Logs are JSON on stdout. Errors never contain RPC URLs or keys.

## Not done

* The Docker images have not been built or run in CI.
* No log shipping, tracing or paging integration. The alert rules above are a starting point.
* No rate limiting on the api. Put it behind a proxy or platform limiter before opening sign-in to the public.
* The web app is deployed separately (for example Vercel). Set its API base URL to the api's public https origin.
