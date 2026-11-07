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

* Only protocols the registry marks Enabled or Partial run. `raydium_amm_v4` is Enabled from three reviewed real fixtures (`docs/fixtures.md`).
* Everything is read at `finalized`, so swaps are `final`. There is no reorg path.
* The first live run starts at the newest signature. Explicit historical ranges use `trace-solana-backfill` and never move the live cursor.
* More than `max_backlog` (1000) new signatures in one step is a gap. The oldest are not read, and an `ingest_gap` audit event names the signatures on each side. Raise the limit, shorten the poll interval, or run a dedicated indexer before relying on complete coverage of a busy program.
* `max_per_step` (100) bounds work per job; the rest is picked up next step in order.
* One worker per protocol (Postgres advisory lock).
* Token decimals come from finalized transaction balances and are cross-checked against the SPL mint account. Name and symbol come from the mint's Metaplex metadata PDA when it exists.

## Limits to know

* Direct Raydium instructions are attributed to the transaction fee payer. Raydium calls found only in inner instructions are marked routed and excluded from direct trader scoring.
* The Raydium program id in `services/worker/src/solana_job.rs` was verified as the Raydium AMM Program on Solana Explorer on 2026-11-07.
* A public RPC will rate limit a busy program. Use a provider key in `TRACE_RPC_SOLANA`.
* Token-2022 names and symbols are read from embedded or external metadata-interface state only when the mint's metadata pointer targets that state and the metadata points back to the same mint. A valid canonical pointer is never replaced with an unrelated Metaplex record.

## Turning it on

1. Set `TRACE_RPC_SOLANA` to a production-capable Solana RPC URL.
2. Start `trace-worker`; the live cursor begins at the newest finalized Raydium signature.
3. Use `trace-solana-backfill` for bounded historical ranges.
