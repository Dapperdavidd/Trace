# Build:  docker build --target api    -t trace-api .
#         docker build --target worker -t trace-worker .
# Not verified in CI yet: build it once before relying on it.

FROM rust:1-bookworm AS build
WORKDIR /src
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY crates crates
COPY services services
COPY tools tools
COPY migrations migrations
RUN cargo build --release --locked -p trace-api -p trace-worker

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home trace
USER trace
ENV TRACE_ENV=production RUST_LOG=info,sqlx=warn

FROM runtime AS api
COPY --from=build /src/target/release/trace-api /usr/local/bin/trace-api
ENV TRACE_BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=3s --start-period=20s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/v1/health/ready || exit 1
CMD ["trace-api"]

FROM runtime AS worker
COPY --from=build /src/target/release/trace-worker /usr/local/bin/trace-worker
CMD ["trace-worker"]
