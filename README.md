# TRACE

A discovery-first crypto research wallet. Rust (Actix Web, Tokio, SQLx, PostgreSQL) backend,
Next.js web app and Flutter mobile app that share one API.

TRACE answers: what are capable on-chain traders buying, why might that matter, and would a
follower still have a viable trade at their own price?

Scope today is watch-only research and paper trading. There is no custody, no signing and no
live trade execution. See `docs/` for the capability register and limitations.

## Run the backend locally

```bash
docker compose -f infra/docker-compose.yml up -d
cp .env.example .env
cargo run -p trace-api          # runs migrations, then serves on 127.0.0.1:8080
curl localhost:8080/v1/health/ready
curl localhost:8080/v1/chains
```

## Checks

```bash
export DATABASE_URL=postgres://trace:trace@localhost:5432/trace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace      # DB tests create throwaway databases (needs CREATEDB)
```

## Layout

- `crates/` shared libraries (domain, database, chain adapters, and more per milestone)
- `services/api` Actix Web API, `services/worker` Tokio jobs
- `migrations/` forward-only SQLx migrations
- `apps/web`, `apps/mobile` clients
- `docs/` ADRs, coverage register, provider matrix, milestone reports

Source of truth for behaviour: `TRACE v3 Master AI Build Instructions`. Where it conflicts with
older text, v3 wins.
