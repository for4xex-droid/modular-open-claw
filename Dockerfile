# ⚠️ DEPRECATED: This monolith Dockerfile bundles frontend + backend in one image.
# The new architecture uses separate containers:
#   - API Server:          docker/production.Dockerfile
#   - Management Console:  apps/management-console/Dockerfile
# See docker-compose.quickstart.yml for the canonical setup.
# Known issue: `npm ci --ignore-scripts` (L7) breaks esbuild binary download.
ARG RUST_VERSION=1.85

FROM node:20-bookworm-slim AS frontend-builder
WORKDIR /app/apps/management-console
COPY apps/management-console/package*.json ./
RUN npm ci --ignore-scripts
COPY apps/management-console ./
RUN npm run build

FROM rust:${RUST_VERSION}-slim-bookworm AS backend-builder
WORKDIR /app

# Install dependencies needed for compiling
RUN apt-get update -y && \
    apt-get install -y pkg-config libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the entire workspace
COPY Cargo.toml Cargo.lock ./
COPY libs/ libs/
COPY apps/ apps/

# Build the api-server
# We use release for maximum performance
RUN cargo build --release --bin api-server

FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies (OpenSSL, etc)
RUN apt-get update -y && \
    apt-get install -y openssl ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the compiled binary
COPY --from=backend-builder /app/target/release/api-server /usr/local/bin/

# Copy the frontend built assets to the expected static directory
COPY --from=frontend-builder /app/apps/management-console/dist /app/apps/api-server/static

# Extract workspace configs
ENV PORT=3015
ENV RUST_LOG="info,aiome=debug"
ENV WORKSPACE_DIR="/data"
ENV ABYSS_VAULT_PATH="/data/.abyss_vault"

# Create user, structure, and assign permissions BEFORE switching user
RUN adduser --disabled-password --no-create-home aiome && \
    mkdir -p /data/skills /data/forge /data/sandbox /data/.abyss_vault && \
    chown -R aiome:aiome /data /app

# Expose API port
EXPOSE 3015

USER aiome

ENTRYPOINT ["api-server"]
