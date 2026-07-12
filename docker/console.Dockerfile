# Management Console image for Quick Start / GHCR.
# Build context MUST be the repository root (file:../../libs/biome-engine/pkg).
# Pattern mirrors .github/workflows/ci.yml wasm-pack → npm build.

# --- WASM (biome-engine) ---
FROM rust:1.93-slim-bookworm AS wasm-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh \
    && rustup target add wasm32-unknown-unknown

WORKDIR /src
COPY libs/biome-engine ./libs/biome-engine
WORKDIR /src/libs/biome-engine
RUN wasm-pack build --target web --out-dir pkg

# --- SPA ---
FROM node:20-alpine AS builder

WORKDIR /app
# Preserve repo-relative path so package.json file:../../libs/biome-engine/pkg resolves.
COPY --from=wasm-builder /src/libs/biome-engine/pkg /app/libs/biome-engine/pkg

WORKDIR /app/apps/management-console
COPY apps/management-console/package.json \
     apps/management-console/package-lock.json \
     apps/management-console/.npmrc \
     ./
RUN npm ci
RUN npm rebuild esbuild

COPY apps/management-console/ ./
RUN npm run build

# --- Runtime ---
FROM nginx:alpine

COPY --from=builder /app/apps/management-console/dist /usr/share/nginx/html
COPY apps/management-console/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -qO- http://localhost:80/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
