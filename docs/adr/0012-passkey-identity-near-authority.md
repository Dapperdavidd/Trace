# ADR 0012: Separate passkey identity from NEAR agent authority

## Status

Accepted.

## Context

TRACE needs a low-friction sign-in path on web and mobile. It also needs a
clear security boundary for agents that may observe markets, prepare actions or
eventually execute within user-defined limits.

Using a custodial wallet key as an account password would join two different
trust decisions. A stolen web session could then become permission to move
funds. Making Google the only identity provider would also add an unnecessary
external dependency to the primary product path.

## Decision

- passkeys are the primary TRACE account authentication method;
- Google remains an optional alternative sign-in method;
- passkey ceremony state is held by the API, expires after five minutes and is
  single-use;
- the API stores only public passkey material and opaque session-token hashes;
- a TRACE session may observe data and prepare a proposed agent action;
- a TRACE session is never sufficient authority to submit an on-chain trade;
- NEAR mandates remain the source of agent authority, limits, pause state and
  revocation;
- approval-required actions must still be signed by the user's NEAR wallet;
- live delegated execution remains disabled until the NEAR deployment and the
  execution agent pass a separate security review.

## Consequences

Users can enter TRACE without a password or mandatory social login. Device
recovery depends on synced passkeys or an explicitly linked alternative method,
so the product must not claim that Google can recover a passkey-only account.

Production deployment requires one HTTPS relying-party domain and matching
Apple and Android association files. Changing that domain requires an explicit
credential migration plan.
