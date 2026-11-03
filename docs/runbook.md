# TRACE operations runbook

This runbook assumes `trace-api`, `trace-worker`, and PostgreSQL are deployed as separate services. Preserve evidence before changing state. Back up PostgreSQL before migrations or manual recovery.

## First checks

1. Check `GET /v1/health/live`. Failure means the API process is down.
2. Check `GET /v1/health/ready`. A 503 with `dependency_unavailable` means PostgreSQL is unreachable from the API.
3. Check the worker process and its latest JSON logs.
4. Check `/v1/ops/metrics` with the operations bearer token.
5. Record the incident start time, affected chains and protocols, request IDs, and the deployed commit SHA.

Never paste database URLs, RPC URLs, bearer tokens, or OAuth credentials into tickets or chat.

## Worker stalled

Symptoms:

- `trace_oldest_due_job_age_seconds > 300`
- no recent `jobs run` log
- enabled protocol scan age continues increasing

Read-only checks:

```sql
SELECT status, count(*) FROM worker_jobs GROUP BY status ORDER BY status;

SELECT id, kind, status, attempts, max_attempts, run_after, locked_until,
       updated_at, last_error
FROM worker_jobs
WHERE status IN ('queued','running','failed','dead')
ORDER BY run_after
LIMIT 100;
```

Actions:

1. Confirm PostgreSQL and the configured RPC or price provider are reachable from the worker host.
2. Confirm the worker has the same release and environment configuration as the API.
3. Restart one worker instance. Expired leases are recovered by the queue; do not edit lock timestamps manually.
4. Watch for `trace-worker started`, the expected `ingestion active` lines, and then `jobs run`.
5. If the queue remains stuck, preserve the job rows and logs and investigate the oldest job's failure before retrying anything.

## Dead jobs

Symptoms:

- `trace_jobs{status="dead"} > 0`
- a job exhausted `max_attempts`

Inspect the job and every attempt:

```sql
SELECT id, kind, payload, status, attempts, max_attempts, created_at,
       updated_at, last_error
FROM worker_jobs
WHERE status = 'dead'
ORDER BY updated_at DESC;

SELECT job_id, started_at, finished_at, outcome, failure_class, error
FROM job_attempts
WHERE job_id = '<job uuid>'
ORDER BY started_at;
```

Classify the failure before acting:

- Transient provider or network failure: restore the dependency first. Use a new queue job through the normal scheduler after the cause is fixed.
- Permanent configuration failure: correct the registry or environment. Do not repeatedly retry the same payload.
- Parser or integrity failure: stop the affected protocol and follow the parser rollback section.

There is no administrative retry endpoint yet. Do not rewrite a dead job to queued by hand because that destroys the attempt history and can bypass current gating.

## Ingestion stale or never scanned

Symptoms:

- `trace_ingest_last_scan_age_seconds > 600`
- `trace_ingest_enabled_never_scanned > 0`

Checks:

```sql
SELECT chain, protocol, support_status, last_successful_scan_at,
       lookback_blocks, limitations, blocker
FROM protocols
ORDER BY chain, protocol;

SELECT chain, protocol, cursor, height, updated_at
FROM ingestion_cursors
ORDER BY chain, protocol;
```

Confirm all three gates agree:

1. The code registry marks the protocol Enabled or Partial.
2. The deployment exists in the worker.
3. The matching `TRACE_RPC_<CHAIN>` variable is configured.

If any gate is absent, ingestion is intentionally idle. Do not mark coverage live to silence the alert.

## Ingestion gap

Symptoms:

- `increase(trace_ingest_gaps[1h]) > 0`
- an `audit_events.action = 'ingest_gap'` row

```sql
SELECT created_at, subject, detail
FROM audit_events
WHERE action = 'ingest_gap'
ORDER BY created_at DESC
LIMIT 50;
```

Actions:

1. Record the exact missing range or signature boundary from `detail`.
2. Keep the UI coverage partial or stale for the affected protocol.
3. Do not move the cursor backwards on a running production worker.
4. Build or run a bounded, idempotent backfill against the recorded range in staging first.
5. Compare inserted swaps, duplicates, skipped reasons, and raw-event hashes before production replay.
6. Clear the incident only after the range is covered or the limitation is documented in the coverage register and UI.

Historical backfill is not yet implemented as a production job, so a gap currently requires engineering work. It must never be hidden by changing the audit row.

## Reorg audit

EVM ingestion records reorg evidence in `audit_events` and keeps finality revisions in raw and normalized rows.

```sql
SELECT created_at, subject, detail
FROM audit_events
WHERE action = 'reorg'
ORDER BY created_at DESC
LIMIT 50;

SELECT chain, tx_id, event_index, finality_revision, finality,
       block_height, block_hash, observed_at
FROM normalized_swaps
WHERE chain = '<chain>' AND tx_id = '<transaction>'
ORDER BY finality_revision;
```

Confirm that the old revision is reorged, the replacement revision points to the canonical block, and downstream analysis does not treat the reorged row as final. Escalate repeated reorgs beyond the configured window because the finality policy may be too weak.

Solana ingestion requests finalized transactions only. A finalized Solana reorg is outside the current operating assumption and must be treated as a critical integrity incident.

## Bad parser release

Symptoms include a sudden change in skipped reasons, impossible amounts, receipt-hash failures, or widespread metadata conflicts after a deployment.

1. Disable the affected protocol in the code registry and deploy that safety change. Keep the coverage register in the same commit.
2. Preserve raw events, normalized rows, worker logs, audit rows, and the deployed parser version.
3. Reproduce the issue from raw events or a reviewed fixture.
4. Fix the parser and add the failing payload as a reviewed regression fixture when policy permits.
5. Replay into staging and compare deterministic outputs before production.
6. Deploy the fixed parser, then run a bounded replay that writes a new finality or parser revision. Never overwrite old evidence in place.

TRACE stores raw events for replay, but a production replay command is not built yet. Until it exists, keep the protocol disabled and treat replay as reviewed engineering work.

## Prices stale

Symptoms:

- `trace_price_latest_age_seconds > 1800`
- market cards become stale or unavailable
- outcomes remain awaiting evaluation or become unpriced

Checks:

```sql
SELECT source, max(observed_at) AS latest, count(*)
FROM price_points
GROUP BY source
ORDER BY source;

SELECT provider, window_start, requests_made, request_budget,
       last_status, last_failure_class, updated_at
FROM provider_quotas
ORDER BY updated_at DESC
LIMIT 50;
```

Confirm `TRACE_PRICE_PROVIDER=defillama`, provider reachability, and quote timestamps. Never substitute zero or relabel stale data as live. Restore polling and allow the normal job to write new observations.

## Database restore

1. Stop API and worker writes.
2. Restore the latest backup into a separate database.
3. Run migrations against the restored copy.
4. Verify row counts, newest raw event, newest price, active sessions, and queue state.
5. Run API readiness and read-only smoke checks against the restored copy.
6. Record the recovery point and estimated data loss before switching traffic.

The exact backup and restore commands depend on the selected managed PostgreSQL host. A restore is not considered tested until this procedure has succeeded in staging.

## Incident closeout

- Record root cause, affected time range, chains, protocols, and user-visible status.
- Link request IDs and audit-event IDs, never secrets.
- Add a regression test, fixture, alert, or runbook correction.
- Confirm coverage and freshness labels tell the truth after recovery.
- Confirm the working tree, deployed commit, and database migration version match the incident record.
