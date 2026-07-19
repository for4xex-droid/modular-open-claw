#!/usr/bin/env bash
# OP-087 P1: sync management-console dist/ → api-server static (Path B).
# Does NOT replace sync_production_sources.sh (source allowlist).
#
# Usage:
#   ./scripts/sync_mc_static.sh
#   SKIP_BUILD=1 ./scripts/sync_mc_static.sh
#   DEST=/path/to/static SKIP_BUILD=1 DIST_DIR=/path/to/dist ./scripts/sync_mc_static.sh
#   DEST=user@host:/app/aiome/apps/api-server/static ./scripts/sync_mc_static.sh
#
# Env:
#   SKIP_BUILD  default 0 — run npm ci && npm run build in MC dir
#   DEST        default apps/api-server/static (repo-relative or absolute; remote user@host:path OK)
#   DIST_DIR    default apps/management-console/dist
#   MC_DIR      default apps/management-console
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MC_DIR="${MC_DIR:-apps/management-console}"
DIST_DIR="${DIST_DIR:-${MC_DIR}/dist}"
DEST="${DEST:-apps/api-server/static}"
SKIP_BUILD="${SKIP_BUILD:-0}"

log() { echo "==> $*"; }
err() { echo "ERROR: $*" >&2; }

is_remote_dest() {
  # user@host:/path — not Windows drive letters in this project
  [[ "$DEST" == *:* && "$DEST" != /* ]]
}

resolve_local() {
  local p="$1"
  if [[ "$p" = /* ]]; then
    echo "$p"
  else
    echo "${ROOT}/${p}"
  fi
}

if [[ "$SKIP_BUILD" != "1" ]]; then
  log "build ${MC_DIR} (npm ci && npm run build)"
  (
    cd "$(resolve_local "$MC_DIR")"
    npm ci
    npm run build
  )
else
  log "SKIP_BUILD=1 — using existing dist at ${DIST_DIR}"
fi

DIST_ABS="$(resolve_local "$DIST_DIR")"
if [[ ! -d "$DIST_ABS" ]]; then
  err "dist missing: ${DIST_ABS} (build first or set DIST_DIR)"
  exit 1
fi

# Portable rsync flags (same family as sync_production_sources.sh)
RSYNC=(rsync -a --delete --human-readable)

BAK_PATH=""
if ! is_remote_dest; then
  DEST_ABS="$(resolve_local "$DEST")"
  if [[ -e "$DEST_ABS" ]]; then
    TS="$(date +%Y%m%d-%H%M%S)"
    BAK_PATH="${DEST_ABS}.bak-${TS}"
    log "backup ${DEST_ABS} → ${BAK_PATH}"
    mkdir -p "$BAK_PATH"
    # Copy existing tree into bak (do not follow into previous baks)
    rsync -a --human-readable "${DEST_ABS}/" "${BAK_PATH}/"
  else
    mkdir -p "$DEST_ABS"
  fi
  log "rsync ${DIST_ABS}/ → ${DEST_ABS}/"
  "${RSYNC[@]}" "${DIST_ABS}/" "${DEST_ABS}/"
  SMOKE_ROOT="$DEST_ABS"
else
  log "remote DEST=${DEST} — skip local bak; verify smoke on host after sync"
  log "rsync ${DIST_ABS}/ → ${DEST}/"
  "${RSYNC[@]}" "${DIST_ABS}/" "${DEST}/"
  SMOKE_ROOT=""
fi

smoke_local() {
  local root="$1"
  local failed=0
  if [[ ! -f "${root}/index.html" ]]; then
    err "smoke: missing index.html"
    failed=1
  elif ! grep -qE 'type="module"|/assets/' "${root}/index.html"; then
    err "smoke: index.html is not a Vite SPA shell (need type=module or /assets/)"
    failed=1
  fi
  if [[ ! -d "${root}/checkout" ]]; then
    err "smoke: missing checkout/"
    failed=1
  fi
  if [[ ! -f "${root}/biome-popup.html" ]]; then
    err "smoke: missing biome-popup.html"
    failed=1
  fi
  if [[ ! -d "${root}/avatar" && ! -d "${root}/vrm" ]]; then
    err "smoke: missing avatar/ or vrm/"
    failed=1
  fi
  if [[ "$failed" -ne 0 ]]; then
    if [[ -n "$BAK_PATH" ]]; then
      err "smoke failed — restore with: rsync -a ${BAK_PATH}/ ${DEST_ABS}/"
    fi
    exit 1
  fi
  log "smoke PASS"
}

if [[ -n "$SMOKE_ROOT" ]]; then
  smoke_local "$SMOKE_ROOT"
fi

log "✅ sync_mc_static done → ${DEST}"
[[ -n "$BAK_PATH" ]] && log "backup kept at ${BAK_PATH}"
