# Multi-stage Dockerfile for apmw
# Builder: rust:1.75-slim with full toolchain
# Runtime: debian:bookworm-slim, non-root user, healthcheck

FROM rust:1.75-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false apmw
WORKDIR /app
COPY --from=builder /app/target/release/apmw /usr/local/bin/apmw
USER apmw
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD apmw --health-check || exit 1
ENTRYPOINT ["apmw"]
