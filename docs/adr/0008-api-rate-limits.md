# ADR 0008: API rate limits

Status: accepted, 2026-11-03

## Context

The API accepted unlimited requests, including Google sign-in attempts. This left database-backed routes and authentication exposed to accidental floods and simple abuse.

## Decision

Each API process applies fixed one-minute request limits in memory:

- 120 requests per direct peer for unsigned traffic.
- 600 requests per direct peer for traffic with a bearer token.
- 300 requests per bearer token hash.
- 10 Google sign-in attempts per direct peer.

Bearer tokens are hashed before they are used as limiter keys. A rejected request returns HTTP 429, a `Retry-After: 60` header, and the standard API error envelope with code `rate_limited` and a request ID.

The limiter deliberately uses the socket peer address and does not trust forwarding headers. Production ingress must enforce a shared client-IP limit before proxying to TRACE, especially when more than one API process is running.

## Consequences

A single API process now has a bounded first line of defence, and sign-in has a stricter bucket. Horizontal deployments still need a shared edge limiter because in-memory counters are not coordinated across processes. The edge proxy is also the correct place to interpret forwarded client addresses.
