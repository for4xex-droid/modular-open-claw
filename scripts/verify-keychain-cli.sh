#!/usr/bin/env bash
# OP-014 / R3-2: abyss-vault CLI set/get roundtrip verification (env fallback path).
# macOS Keychain bootstrap is attempted when `security` is available (non-interactive).
# Verification Protocol: Positive → Negative → Revert.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VAULT_DIR="$(mktemp -d /tmp/aiome-vault-verify-XXXXXX)"
export ABYSS_VAULT_PATH="${VAULT_DIR}/abyss_vault.db"
export VAULT_MASTER_PASSWORD="verify-master-password-$(date +%s)"
TEST_KEY="SEARCH_API_KEY"
TEST_VALUE="op014-roundtrip-$(uuidgen 2>/dev/null || echo "$RANDOM")"

log() { echo "[verify-keychain-cli] $*"; }
fail() { echo "[verify-keychain-cli] ERROR: $*" >&2; exit 1; }

cleanup() {
  rm -rf "$VAULT_DIR"
  if [ "${KEYCHAIN_BOOTSTRAPPED:-0}" = "1" ] && [ "$(uname -s)" = "Darwin" ]; then
    security delete-generic-password -a "${USER:-aiome-user}" -s "com.aiome.vault-test-roundtrip" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

run_vault() {
  cargo run -q --bin abyss-vault -- "$@"
}

log "=== Step 1: Positive — set/get roundtrip via env-backed AbyssVault ==="
run_vault set "$TEST_KEY" "$TEST_VALUE"
GOT="$(run_vault get "$TEST_KEY")"
[ "$GOT" = "$TEST_VALUE" ] || fail "get returned '$GOT', expected '$TEST_VALUE'"

log "=== Step 2: Negative — disallowed key must be rejected ==="
if run_vault set "NOT_ON_WHITELIST" "x" 2>/dev/null; then
  fail "disallowed key was accepted"
fi
log "Negative test passed (disallowed key rejected)"

log "=== Step 3: Revert — delete test secret ==="
run_vault delete "$TEST_KEY" --yes
if run_vault get "$TEST_KEY" 2>/dev/null; then
  fail "secret still readable after delete"
fi
log "Revert complete (secret removed)"

if [ "$(uname -s)" = "Darwin" ] && command -v security >/dev/null 2>&1; then
  log "=== macOS Keychain smoke (bootstrap write + read) ==="
  KC_VALUE="kc-smoke-$(date +%s)"
  shared_service="com.aiome.vault-test-roundtrip"
  security add-generic-password -a "${USER:-aiome-user}" -s "$shared_service" -w "$KC_VALUE" -U 2>/dev/null \
    || fail "security add-generic-password failed"
  KEYCHAIN_BOOTSTRAPPED=1
  READ_BACK="$(security find-generic-password -a "${USER:-aiome-user}" -s "$shared_service" -w 2>/dev/null)" \
    || fail "security find-generic-password failed"
  [ "$READ_BACK" = "$KC_VALUE" ] || fail "Keychain roundtrip mismatch"
  log "macOS Keychain smoke passed"
else
  log "Skipping macOS Keychain smoke (not Darwin or security CLI unavailable)"
fi

log "✅ OP-014 abyss-vault CLI verification complete."
