# TRACE on NEAR

TRACE is a NEAR-native agentic trading intelligence system. Ethereum, Solana
and future chains provide markets to observe. NEAR owns the authorization,
cross-chain execution policy and verifiable evidence.

## Complete flow

1. TRACE ingests finalized swaps and ranks wallets using explicit evidence.
2. A user creates a paper Shadow strategy.
3. TRACE prepares a NEAR agent mandate with exact limits and an expiry.
4. The user's wallet creates the mandate on the TRACE NEAR contract.
5. TRACE requests a dry NEAR Intents quote for an eligible action.
6. In `approval_required` mode, the user reviews and signs the live action.
7. In `delegated` mode, the named agent may act only within the on-chain policy.
8. TRACE retains the complete evidence and anchors its SHA-256 commitment in an
   execution receipt on NEAR.
9. Shadow and live outcomes update the agent's evidence-backed track record.

## Authority boundary

The API can read supported assets and request dry quotes. It cannot transfer a
deposit, sign an intent or access a wallet key. A quote is not an execution.

The contract enforces:

- owner and agent NEAR account IDs;
- allowed origin and destination assets;
- allowed chains;
- maximum USD value per trade and per UTC day, stored as integer microdollars;
- maximum slippage in basis points;
- expiry, pause and permanent revocation;
- unique execution receipt IDs and SHA-256 evidence commitments.

## Storage

Mandates, daily spend totals and receipt commitments use NEAR contract state.
Normalized chain evidence remains in Postgres because it is large and may
contain data that should not be copied permanently to a public chain. Anyone
with the retained evidence can recompute the commitment recorded on NEAR.

## Deployment state

The contract builds to WebAssembly and is covered by native unit tests. It is
not deployed until an owner chooses a NEAR account, network and funded signing
wallet. Until deployment, the UI and API must label all mandates as drafts or
unavailable, never live.

