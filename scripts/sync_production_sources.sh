#!/usr/bin/env bash
# Allowlist rsync of build sources to a production host.
# Does NOT overwrite docker-compose.production.yml unless SYNC_COMPOSE=1.
#
# Usage:
#   DEST=root@HOST:/app/aiome ./scripts/sync_production_sources.sh
#   SYNC_COMPOSE=1 DEST=... ./scripts/sync_production_sources.sh   # rare; prefer host sqlite overlay
set -euo pipefail

if [[ -z "${DEST:-}" ]]; then
  echo "ERROR: set DEST=user@host:/path/to/aiome" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# macOS system rsync lacks --info=stats2; keep portable flags only.
RSYNC=(rsync -a --delete --human-readable)
ALLOW=(
  --include='/'
  --include='/Cargo.toml'
  --include='/Cargo.lock'
  --include='/libs/***'
  --include='/apps/api-server/***'
  --include='/apps/management-console/***'
  --include='/apps/key-proxy/***'
  --include='/apps/shadow-worker/***'
  --include='/docker/***'
  --include='/commercial/***'
  --exclude='*'
)

echo "==> sync allowlist → ${DEST}"
"${RSYNC[@]}" "${ALLOW[@]}" ./ "${DEST}/"

if [[ "${SYNC_COMPOSE:-0}" == "1" ]]; then
  echo "==> SYNC_COMPOSE=1: copying compose files (review host sqlite overlay after)"
  rsync -a \
    docker-compose.production.yml \
    docker-compose.production.sqlite.yml \
    "${DEST}/"
else
  echo "==> skip compose (default). Host should keep sqlite overlay:"
  echo "    docker compose -f docker-compose.production.yml -f docker-compose.production.sqlite.yml ..."
fi

echo "✅ sync done"
