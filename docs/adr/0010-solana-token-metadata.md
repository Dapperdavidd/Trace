# ADR 0010: Solana token metadata

Status: accepted

## Context

Finalized swap balances provide mint decimals but not a token name or symbol. Price providers are not an identity authority and must not silently label a mint. TRACE already has a configured Solana RPC for ingestion, so the chain is the preferred metadata source.

## Decision

The metadata worker reads the finalized SPL mint account with `getAccountInfo`. It accepts the classic SPL Token and Token-2022 program owners, requires an initialized mint, and reads decimals from the serialized mint layout.

For name and symbol, it derives the Metaplex Token Metadata PDA from the seeds `metadata`, the Metaplex program id, and the mint. It accepts the account only when the owner is the Metaplex program, the account discriminator is MetadataV1, and the serialized mint matches the requested mint. Name and symbol use their on-chain Borsh strings and strict length and control-character validation. The off-chain URI is not fetched.

Existing decimals, symbols, and names are never overwritten. A different mint decimal value creates a deduplicated `token_decimals_conflict` audit event. Missing or malformed metadata remains unavailable and is retried after the normal metadata interval.

## Consequences

Solana clients can display chain-sourced names and symbols without trusting a market-price provider. RPC URLs and keys stay out of errors. Token-2022 metadata stored inside the mint or in an external metadata-interface account is accepted only after the mint pointer and metadata both identify each other. A valid canonical pointer is never replaced with an unrelated Metaplex record or a guessed symbol.
