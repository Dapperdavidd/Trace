# ADR 0007: EVM token metadata

Status: accepted, 2026-11-02

## Context

Ingestion learns token addresses but not their decimals, symbols, or names. The price adapter can supply decimals and a symbol, but provider metadata must not silently override reviewed chain data. Missing decimals also prevent exact conversion from base units.

## Decision

When an EVM RPC is configured, the worker reads `decimals()`, `symbol()`, and `name()` directly from token contracts every five minutes for incomplete records.

- Decimals are required and must be between 0 and 38.
- Standard ABI strings and legacy `bytes32` strings are supported.
- Empty, invalid, control-character, and overlong text is rejected.
- Existing reviewed symbols and names are preserved.
- A successful read is marked complete. A revert or non-token contract is marked unavailable and retried after seven days.
- Transport failures retry through the job queue and never expose the RPC URL.
- At most 200 tokens are checked in one run.

## Consequences

Ethereum token amounts can use on-chain decimals and user surfaces can show contract-provided names without trusting the price provider for identity. Base and BNB use the same reader when their RPC URLs and ingested tokens exist. Solana mint decimals and metadata remain a separate implementation task.
