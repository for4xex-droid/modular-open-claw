#!/usr/bin/env bash
# TDD harness for scripts/sync_mc_static.sh (OP-087 P1).
# Verification Protocol: Positive → Negative → Revert.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="${ROOT}/scripts/sync_mc_static.sh"
WORK="$(mktemp -d /tmp/aiome-sync-mc-static-test-XXXXXX)"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

fail() { echo "[test_sync_mc_static] FAIL: $*" >&2; exit 1; }
pass() { echo "[test_sync_mc_static] OK: $*"; }

[[ -x "$SCRIPT" || -f "$SCRIPT" ]] || fail "missing $SCRIPT (implement sync_mc_static.sh first)"

make_good_dist() {
  local d="$1"
  mkdir -p "$d/assets" "$d/checkout" "$d/avatar"
  cat >"$d/index.html" <<'EOF'
<!doctype html>
<html><head>
<script type="module" crossorigin src="/assets/main-TESTHASH.js"></script>
</head><body><div id="root"></div></body></html>
EOF
  echo "console.log('mc')" >"$d/assets/main-TESTHASH.js"
  echo "ok" >"$d/checkout/success.html"
  echo "vrm" >"$d/avatar/placeholder.txt"
  echo "popup" >"$d/biome-popup.html"
}

echo "==> Positive: SKIP_BUILD=1 sync good dist → DEST"
DIST="$WORK/dist-good"
DEST="$WORK/static-dest"
OLD="$WORK/static-dest/old-file.txt"
mkdir -p "$DEST"
echo "stale" >"$OLD"
make_good_dist "$DIST"

SKIP_BUILD=1 DIST_DIR="$DIST" DEST="$DEST" "$SCRIPT" || fail "positive sync exited non-zero"

[[ -f "$DEST/index.html" ]] || fail "index.html missing after sync"
grep -q 'type="module"' "$DEST/index.html" || grep -q '/assets/' "$DEST/index.html" || fail "index smoke content missing"
[[ -d "$DEST/checkout" ]] || fail "checkout/ missing"
[[ -f "$DEST/biome-popup.html" ]] || fail "biome-popup.html missing"
[[ -d "$DEST/avatar" || -d "$DEST/vrm" ]] || fail "avatar/ or vrm/ missing"
[[ ! -f "$OLD" ]] || fail "rsync --delete did not remove stale file"

BAK="$(ls -d "$WORK"/static-dest.bak-* 2>/dev/null | head -1 || true)"
# bak is sibling of DEST: DEST.bak-TS next to DEST's parent... plan says DEST.bak-TS
# Script should create "${DEST}.bak-TIMESTAMP" or sibling — accept either pattern under WORK
BAK="$(find "$WORK" -maxdepth 1 -type d -name 'static-dest.bak-*' 2>/dev/null | head -1 || true)"
[[ -n "$BAK" ]] || fail "local DEST backup directory not created"
[[ -f "$BAK/old-file.txt" ]] || fail "backup did not preserve pre-sync content"
pass "Positive sync + backup"

echo "==> Negative: empty/fake dist must fail smoke"
BAD="$WORK/dist-bad"
BADDEST="$WORK/static-bad"
mkdir -p "$BAD" "$BADDEST"
echo "<html>empty</html>" >"$BAD/index.html"

if SKIP_BUILD=1 DIST_DIR="$BAD" DEST="$BADDEST" "$SCRIPT" 2>"$WORK/neg.err"; then
  fail "negative: expected non-zero exit for incomplete dist"
fi
grep -qiE 'smoke|missing|fail|ERROR' "$WORK/neg.err" || fail "negative: stderr should explain failure"
pass "Negative incomplete dist rejected"

echo "==> Revert: restore from bak"
# Re-run positive to get fresh bak, then wipe DEST and restore
DEST2="$WORK/static-revert"
mkdir -p "$DEST2"
echo "before" >"$DEST2/keep-me.txt"
make_good_dist "$DIST"
SKIP_BUILD=1 DIST_DIR="$DIST" DEST="$DEST2" "$SCRIPT" || fail "setup for revert failed"
BAK2="$(find "$WORK" -maxdepth 1 -type d -name 'static-revert.bak-*' | head -1)"
[[ -n "$BAK2" ]] || fail "bak2 missing"
rm -rf "$DEST2"
mkdir -p "$DEST2"
rsync -a "$BAK2/" "$DEST2/"
[[ -f "$DEST2/keep-me.txt" ]] || fail "revert from bak lost pre-sync file"
pass "Revert from bak"

echo "==> Q6 stub: tracked static.stub must not be product Dashboard / Vite shell"
# OP-087 Q6: apps/api-server/static/ is fully gitignored. Contract lives in static.stub/.
STUB_PATH="${ROOT}/apps/api-server/static.stub/index.html"
[[ -f "$STUB_PATH" ]] || fail "Q6 stub missing: $STUB_PATH"
grep -qi 'not the product ui\|not product ui' "$STUB_PATH" || fail "stub must declare Not the product UI"
grep -q 'sync_mc_static' "$STUB_PATH" || fail "stub must point at sync_mc_static.sh"
grep -qvi 'cdn.jsdelivr.net\|unpkg.com' "$STUB_PATH" || fail "stub must not load CDN dashboard scripts"
if grep -qE 'src="/assets/main-' "$STUB_PATH"; then
  fail "stub must not be a Vite hashed shell"
fi
# Negative: static/index.html must not be a tracked git path
if git -C "$ROOT" ls-files --error-unmatch apps/api-server/static/index.html >/dev/null 2>&1; then
  fail "apps/api-server/static/index.html must be untracked (Q6)"
fi
pass "Q6 stub + untrack contract"

echo "✅ test_sync_mc_static.sh: Positive / Negative / Revert / Stub PASS"
