FROM rust:1-bookworm AS build
WORKDIR /src
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY crates crates
COPY contracts contracts
COPY services services
COPY tools tools
COPY migrations migrations
RUN cargo build --release --locked -p trace-api -p trace-worker

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home trace
COPY --from=build /src/target/release/trace-api /usr/local/bin/trace-api
COPY --from=build /src/target/release/trace-worker /usr/local/bin/trace-worker
COPY infra/render/start.sh /usr/local/bin/trace-render
USER trace
ENV TRACE_ENV=production RUST_LOG=info,sqlx=warn TRACE_BIND_ADDR=0.0.0.0:10000
EXPOSE 10000
