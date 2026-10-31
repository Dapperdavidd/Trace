# Solana ingestion

```
getSignaturesForAddress(program, until=cursor)   newest first, finalized
        │  (paged; at most max_backlog)
        ▼
 skip failed on chain ─► getTransaction(jsonParsed, finalized)
        │                       │ null for a finalized signature: error, retry, never skipped
        ▼                       ▼
 oldest first ─► solana_delta parser ─► raw event + swap (final) ─► cursor = last handled
```

## Rules

* Only protocols the registry marks Enabled or Partial run. `raydium_amm_v4` is Disabled until three reviewed real fixtures exist (`docs/fixtures.md`).
* Everything is read at `finalized`, so swaps are `final`. There is no reorg path.
* The first run starts at the newest signature. There is no backfill.
* More than `max_backlog` (1000) new signatures in one step is a gap. The oldest are not read, and an `ingest_gap` audit event names the signatures on each side. Raise the limit, shorten the poll interval, or run a dedicated indexer before relying on complete coverage of a busy program.
* `max_per_step` (100) bounds work per job; the rest is picked up next step in order.
* One worker per protocol (Postgres advisory lock).

## Limits to know

* The parser attributes a swap to the transaction fee payer. Swaps routed through a relayer or aggregator whose fee payer is not the trader are attributed to the payer or skipped as not a simple swap.
* The Raydium program id is in `services/worker/src/solana_job.rs` and must be checked on an explorer when enabling.
* A public RPC will rate limit a busy program. Use a provider key in `TRACE_RPC_SOLANA`.

## Turning it on

1. Capture and review 3+ real transactions: `capture-fixture solana --protocol raydium_amm_v4 --program <id> --tx <signature>`.
2. Set the protocol to Enabled in `registry.rs` with the fixtures in the same commit.
3. Set `TRACE_RPC_SOLANA` and start the worker.
