# ==========================================
# Aiome Production Dockerfile Template
# Features: Rootless, Read-only compatible, Hardened
# ==========================================

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

# --- Runtime Stage ---
FROM debian:bookworm-slim

ARG BIN_NAME=api-server
ENV BIN_NAME=${BIN_NAME}

# Labels for security visibility
LABEL org.opencontainers.image.authors="motivationstudio,LLC" \
    security.rootless="true" \
    security.readonly="true" \
    security.no-docker-socket="true"

# 1. Create a non-privileged service user
RUN groupadd -g 1001 aiome && \
    useradd -u 1001 -g aiome -m -s /bin/false aiome

# 2. Hardening: Install only necessary CA certs, runtime libraries, and gVisor (runsc)
# fff-mcp: High-performance file search MCP server (MIT, github.com/dmtrKovalenko/fff.nvim)
# SUPPLY CHAIN NOTE: The fff-mcp install step pipes a remote script. The script downloads
# a checksummed binary from GitHub Releases. For hardened builds, pin to a specific release
# tarball and verify SHA256 manually instead of using the convenience installer.
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    gnupg \
    libcap2-bin \
    nodejs \
    npm \
    python3 \
    python3-venv \
    && curl -fsSL https://gvisor.dev/archive.key | gpg --dearmor -o /usr/share/keyrings/gvisor-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/gvisor-archive-keyring.gpg] https://storage.googleapis.com/gvisor/releases release main" > /etc/apt/sources.list.d/gvisor.list \
    && apt-get update && apt-get install -y runsc \
    && setcap cap_sys_ptrace+ep /usr/bin/runsc \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 3. Copy binary from builder
COPY --from=builder /app/target/release/${BIN_NAME} /app/aiome-app
RUN chown root:root /app/aiome-app && chmod 555 /app/aiome-app

# 4. Prepare a writable data directory
RUN mkdir -p /app/data && chown aiome:aiome /app/data && chmod 700 /app/data

# 5. Security: Set the unprivileged user
USER aiome

# 6. Environment Sanity
ENV RUST_LOG=info \
    AIOME_DATA_DIR=/app/data \
    TMPDIR=/tmp

# Standard ports (Command Center uses 3015, Key Proxy uses 9999 - mapped via compose)
EXPOSE 3015 9999

# Execution
ENTRYPOINT ["/app/aiome-app"]
