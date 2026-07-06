#!/usr/bin/env bash
# OP-012 / R3-1: Production-like PostgreSQL migration + BAN integration verification.
# Verification Protocol: Positive → Negative injection → Revert (AGENTS.md).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

COMPOSE_FILE="docker-compose.production-verify.yml"
PG_BASE="postgres://aiome:aiome_verify_password@localhost:5434"
export PRODUCTION_VERIFY_PG_BASE="$PG_BASE"

log() { echo "[verify-production-postgres] $*"; }
fail() { echo "[verify-production-postgres] ERROR: $*" >&2; exit 1; }

cleanup() {
  log "Stopping production-verify postgres..."
  docker compose -f "$COMPOSE_FILE" down -v 2>/dev/null || true
}
trap cleanup EXIT

log "Starting production-verify postgres (postgres:16-alpine + init.sql)..."
docker compose -f "$COMPOSE_FILE" up -d

log "Waiting for PostgreSQL health (init.sql + restart cycle)..."
for i in $(seq 1 60); do
  if docker compose -f "$COMPOSE_FILE" exec -T postgres pg_isready -U aiome -d aiome >/dev/null 2>&1; then
    if docker compose -f "$COMPOSE_FILE" exec -T postgres \
      psql -U aiome -d aiome -c "SELECT 1" >/dev/null 2>&1; then
      break
    fi
  fi
  if [ "$i" -eq 60 ]; then
    fail "PostgreSQL did not become ready within 60s"
  fi
  sleep 1
done

log "Checking init.sql databases (nurture, samsara_hub)..."
for db in aiome nurture samsara_hub; do
  docker compose -f "$COMPOSE_FILE" exec -T postgres \
    psql -U aiome -d "$db" -c "SELECT 1" >/dev/null \
    || fail "Database '$db' is not reachable"
done

log "=== Step 1: Positive Test — migrations + BAN roundtrip ==="
cargo test -p infrastructure --test postgres_production_verify -- --nocapture

log "=== Step 2: Negative Test — connection to non-existent database ==="
cargo test -p infrastructure --test postgres_production_verify test_negative_invalid_database -- --nocapture

log "=== Step 3: Revert — BAN unban verified inside Rust test (test_ban_store_postgres_roundtrip) ==="
log "Positive + Negative + Revert all passed."

log "✅ OP-012 production PostgreSQL verification complete."
