# ADR 0006: Watchlist alert generation

Status: accepted, 2026-11-02

## Context

The alerts API and user preferences existed, but every alert came from demo seeding. TRACE needs to turn real watched-wallet activity into in-app alerts without replaying the same event or presenting tentative activity as final.

## Decision

The worker scans newly ingested normalized swaps for every watchlist once per minute.

- Only final, non-demo, directly attributed swaps produce alerts.
- A stable-to-non-stable swap is a wallet buy. A non-stable-to-stable swap is a wallet sell. Other pairs are skipped because the direction is not safely classified.
- Disabled preferences suppress creation for that user.
- Paused watchlists do not create alerts, and their scan cursor still advances so resuming does not replay old activity.
- Each user, transaction event, and alert type has a unique deduplication key.
- The alert links the source transaction and uses the swap observation time as the event time.
- Scanning uses ingestion time for its cursor so a late-ingested on-chain event is not lost.

## Consequences

Real in-app wallet alerts are now available without push or email infrastructure. Risk and signal alerts remain separate future jobs. Aggregator and ambiguous activity stays excluded until attribution is supported by reviewed fixtures.
