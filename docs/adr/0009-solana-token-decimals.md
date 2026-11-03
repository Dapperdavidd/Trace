# ADR 0009: Solana token decimals

Status: accepted, 2026-11-03

## Context

Finalized Solana transaction responses already contain the raw amount and decimal count for every pre- and post-token balance. TRACE previously discarded the decimal count, leaving real Solana swaps unpriceable even after successful ingestion.

## Decision

After a finalized swap is normalized, ingestion stores a mint's decimal count only when every balance entry for that mint agrees. Wrapped SOL always uses nine decimals. Existing values are never overwritten.

If a finalized transaction reports a value that conflicts with the stored value, ingestion writes a `token_decimals_conflict` audit event containing both values and the transaction signature. The stored value remains unchanged for review.

## Consequences

Solana token amounts can be converted exactly without waiting for a price provider or a separate mint lookup. This does not provide token names or symbols, and transactions without consistent balance metadata leave decimals unavailable.
