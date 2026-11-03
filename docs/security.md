# Security notes

## Dependency audit

CI scans the Rust lockfile with the RustSec advisory database. The web job also runs `npm audit` through the existing project script and lockfile checks.

The API disables Actix Web default features because TRACE terminates TLS and HTTP/2 at its production ingress. This removes an unused vulnerable HTTP/2 implementation from the server build.

`RUSTSEC-2023-0071` is ignored explicitly in CI because the affected `rsa` crate belongs to `sqlx-mysql`, which is only recorded through SQLx macro metadata. TRACE disables SQLx default features, enables PostgreSQL only, and does not compile `sqlx-mysql`. The ignore must be removed if MySQL support is ever enabled or if the dependency becomes reachable.

To verify the exception:

```sh
cargo tree --target all -i sqlx-mysql
cargo audit --ignore RUSTSEC-2023-0071
```

The first command must report that nothing depends on `sqlx-mysql`; the second must pass.

## Sessions

TRACE issues opaque 30-day session tokens and stores only SHA-256 hashes. Users can list active sessions, revoke one session, sign out the current session, or revoke every session on the account. Session lists expose only a coarse device label such as `Mac` or `iPhone`; the API does not store full user-agent strings or client IP addresses.

## HTTP boundary

The API restricts CORS to configured origins, limits JSON request bodies to 64 KiB, assigns request IDs, applies per-peer and per-session rate limits, and returns browser hardening headers for content sniffing, framing, referrers, and unused device capabilities.

TLS terminates at production ingress. HSTS must be configured there because the API process receives proxied HTTP and cannot safely infer that the original client connection used HTTPS. The ingress must also apply a shared client-IP rate limit across replicas.
