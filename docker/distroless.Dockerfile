# ==========================================
# Aiome Distroless Production Dockerfile
# Features: Google Distroless, No Shell, Hardened
# ==========================================

# --- Frontend Build Stage ---
FROM node:20-bookworm-slim AS frontend-builder
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

# --- Final Stage ---
# gcr.io/distroless/cc-debian12 includes glibc and libssl3 which are needed for most Rust apps
FROM gcr.io/distroless/cc-debian12:latest

ARG BIN_NAME=api-server
ENV BIN_NAME=${BIN_NAME}

# Labels for security visibility
LABEL org.opencontainers.image.authors="motivationstudio,LLC" \
    security.distroless="true" \
    security.no-shell="true" \
    security.readonly="true"

WORKDIR /app

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
