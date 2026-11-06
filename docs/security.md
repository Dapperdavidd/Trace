# Security notes

## Dependency audit

CI scans the Rust lockfile with the RustSec advisory database. The web job also runs `npm audit` through the existing project script and lockfile checks.

The API disables Actix Web default features because TRACE terminates TLS and HTTP/2 at its production ingress. This removes an unused vulnerable HTTP/2 implementation from the server build.

`RUSTSEC-2023-0071` is ignored explicitly in CI because the affected `rsa` crate belongs to `sqlx-mysql`, which is only recorded through SQLx macro metadata. TRACE disables SQLx default features, enables PostgreSQL only, and does not compile `sqlx-mysql`. The ignore must be removed if MySQL support is ever enabled or if the dependency becomes reachable.

To verify the exception:

```sh
cargo tree --target all -i sqlx-mysql
cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2026-0387
```

The first command must report that nothing depends on `sqlx-mysql`; the audit
command is completed after checking the second exception below.

`RUSTSEC-2026-0387` is also ignored for the locked `crypto-bigint 0.5` copy.
It is reachable only through NEAR SDK's native unit-test runtime. It is absent
from the mandate contract's normal dependency graph and from the
`wasm32-unknown-unknown` build. Remove the exception when NEAR SDK updates its
test dependency.

Verify that boundary with:

```sh
cargo tree -p trace-mandates-contract --edges normal -i crypto-bigint@0.5.5
cargo tree --target wasm32-unknown-unknown -p trace-mandates-contract -i crypto-bigint@0.5.5
cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2026-0387
```

Both tree commands must report that nothing depends on the affected version.

## Sessions

TRACE issues opaque 30-day session tokens and stores only SHA-256 hashes. Users can list active sessions, revoke one session, sign out the current session, or revoke every session on the account. Session lists expose only a coarse device label such as `Mac` or `iPhone`; the API does not store full user-agent strings or client IP addresses.

## Passkeys

Passkeys are the primary account entry path on web and mobile. TRACE stores the
public credential and authenticator counter, never the private key. Registration
and authentication ceremony state stays on the server, expires after five
minutes and can be consumed only once.

The relying-party ID and origin are explicit deployment configuration. A
production API rejects a localhost relying-party ID or a non-HTTPS origin.
Native apps also require the platform association files for the same web
domain before passkeys can be verified on physical devices.

A TRACE passkey authenticates an account. It does not authorize an on-chain
trade. Agent authority is a separate NEAR mandate signed by the user's NEAR
wallet, with limits and revocation enforced by the NEAR contract.

## External agents

External agents receive separate `trca_` credentials, never a user's app
session. Tokens are shown once, stored as SHA-256 hashes, expire within 90 days
and can be revoked independently. Scope is limited to `observe` and `propose`.
There is intentionally no `execute` scope.

The agent context returns watched public addresses, status-labelled alerts and
active NEAR mandate policy. It does not return passkeys, app sessions, wallet
keys or signing material. A proposal is checked against the anchored mandate
and stored as a dry NEAR Intents preview. It is not a transaction.

## HTTP boundary

The API restricts CORS to configured origins, limits JSON request bodies to 64 KiB, assigns request IDs, applies per-peer and per-session rate limits, and returns browser hardening headers for content sniffing, framing, referrers, and unused device capabilities.

TLS terminates at production ingress. HSTS must be configured there because the API process receives proxied HTTP and cannot safely infer that the original client connection used HTTPS. The ingress must also apply a shared client-IP rate limit across replicas.
