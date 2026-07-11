# ==========================================
# Aiome Distroless Production Dockerfile
# Features: Google Distroless, No Shell, Hardened
# ==========================================

# --- Frontend Build Stage ---
FROM node:20-bookworm-slim AS frontend-builder
# file:../../libs/biome-engine/pkg from apps/management-console
COPY libs/biome-engine/pkg /app/libs/biome-engine/pkg
WORKDIR /app/apps/management-console
COPY apps/management-console/package*.json ./
RUN npm ci --ignore-scripts
COPY apps/management-console ./
RUN npm run build


# --- Build Stage ---
FROM rust:1.93-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    protobuf-compiler \
    curl \
    cmake \
    g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY . .

# Build target
ARG BIN_NAME=api-server
ARG FEATURES=""
RUN if [ -n "$FEATURES" ]; then \
    cargo build --release --bin ${BIN_NAME} --features ${FEATURES}; \
    else \
    cargo build --release --bin ${BIN_NAME}; \
    fi

# --- Runtime OpenSSL (cc-distroless does NOT ship libssl3) ---
FROM debian:bookworm-slim AS runtime-libs
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    && mkdir -p /out/data /out/workspace \
    && chown -R 65532:65532 /out \
    && rm -rf /var/lib/apt/lists/*

# --- Final Stage ---
# gcr.io/distroless/cc-debian12: glibc/libgcc only — OpenSSL must be copied in.
FROM gcr.io/distroless/cc-debian12:latest

ARG BIN_NAME=api-server
ENV BIN_NAME=${BIN_NAME}

# Labels for security visibility
LABEL org.opencontainers.image.authors="motivationstudio,LLC" \
    security.distroless="true" \
    security.no-shell="true" \
    security.readonly="true"

WORKDIR /app

COPY --from=runtime-libs /usr/lib/x86_64-linux-gnu/libssl.so.3 /usr/lib/x86_64-linux-gnu/
COPY --from=runtime-libs /usr/lib/x86_64-linux-gnu/libcrypto.so.3 /usr/lib/x86_64-linux-gnu/
COPY --from=runtime-libs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
# Prefer compose AIOME_DATA_DIR=/app/data; workspace is DEV_MODE fallback.
COPY --from=runtime-libs /out/data /app/data
COPY --from=runtime-libs /out/workspace /app/workspace

# Copy binary from builder
COPY --from=builder /app/target/release/${BIN_NAME} /app/aiome-app

# Copy default assets/config if needed
# COPY --from=builder /app/resources /app/resources
COPY --from=frontend-builder /app/apps/management-console/dist /app/apps/api-server/static

# Standard ports
EXPOSE 3015

USER nonroot:nonroot

# Execution
# Distroless has no shell, so MUST use exec form []
ENTRYPOINT ["/app/aiome-app"]
