# --- Build Stage ---
FROM rust:1.93-slim-bookworm AS builder

# libssl-dev, pkg-config are needed for many rust crates
# python3-dev is needed for PyO3
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    python3 \
    python3-dev \
    protobuf-compiler \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build nurture-api within the workspace context
RUN cargo build --release --bin nurture-api

# --- Runtime Stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    python3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/nurture-api /app/nurture-api
# Copy migrations just in case for manual CLI usage, although embedded in binary
COPY --from=builder /app/commercial/migrations /app/migrations

# Default config
ENV RUST_LOG=info
ENV PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
EXPOSE 3020

# 🔒 Q-C: Non-root user for container hardening (matches Aiome UID 1001)
RUN groupadd -g 1001 nurture && useradd -u 1001 -g nurture -s /bin/false nurture \
    && mkdir -p /app/data && chown -R nurture:nurture /app
USER nurture

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3020/health || exit 1

ENTRYPOINT ["/app/nurture-api"]
