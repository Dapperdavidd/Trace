# ADR 0011: NEAR is the agent control plane

## Status

Accepted.

## Context

TRACE observes wallet activity on multiple chains. Treating NEAR as one more
parser target would not make it essential to the product and would duplicate
swap routing, authorization and evidence logic for every observed chain.

Agents that may eventually act with real funds need user-owned authority that
is narrow, inspectable and revocable. A server-held private key or an informal
database flag is not sufficient authority.

## Decision

NEAR is TRACE's control plane:

- a NEAR contract stores agent mandates and tamper-evident execution receipts;
- a mandate names one owner and one agent, with explicit chain, asset, trade,
  daily, slippage and expiry limits;
- approval-required execution must be confirmed by the owner;
- delegated execution is accepted only from the named NEAR agent account;
- owners can pause a mandate and revocation is permanent;
- NEAR Intents supplies cross-chain asset discovery and quote previews;
- TRACE's API never submits funds or signed intents on behalf of a user;
- Ethereum and Solana remain observation and destination networks, not the
  authorization source.

Large evidence stays in TRACE's append-only database. The NEAR receipt stores a
SHA-256 commitment so the evidence can be verified without placing private or
large payloads on-chain.

## Consequences

TRACE can demonstrate paper and approval-required flows without custody. Live
delegation requires contract deployment, a funded NEAR owner account, a named
agent account and a separate security review. A database mandate is only a
draft until its commitment is anchored to the configured NEAR contract.

