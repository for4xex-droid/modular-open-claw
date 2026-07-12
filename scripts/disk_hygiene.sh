#!/usr/bin/env bash
# Local workspace disk hygiene for aiome (regenerable artifacts only).
# Usage:
#   ./scripts/disk_hygiene.sh          # dry-run (default)
#   ./scripts/disk_hygiene.sh --apply  # delete listed paths
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APPLY=false
if [[ "${1:-}" == "--apply" ]]; then
  APPLY=true
fi

paths=(
  "$ROOT/target"
  "$ROOT/.codeql-db"
  "$ROOT/workspace"
  "$ROOT/venv"
  "$ROOT/venv_new"
  "$ROOT/.venv"
  "$ROOT/tools/ruri-embed-server/.venv"
  "$ROOT/docs/landing/.venv"
  "$ROOT/docs/landing/node_modules"
  "$ROOT/apps/management-console/coverage"
  "$ROOT/apps/management-console/playwright-report"
)

echo "=== aiome disk hygiene (apply=$APPLY) ==="
df -h /System/Volumes/Data 2>/dev/null | tail -1 || df -h . | tail -1
echo "Workspace: $(du -sh "$ROOT" 2>/dev/null | cut -f1)"
echo

total=0
for p in "${paths[@]}"; do
  if [[ -e "$p" ]]; then
    size=$(du -sk "$p" 2>/dev/null | cut -f1)
    total=$((total + size))
    human=$(du -sh "$p" 2>/dev/null | cut -f1)
    echo "  $human  $p"
    if $APPLY; then
      if [[ "$p" == "$ROOT/target" ]]; then
        (cd "$ROOT" && cargo clean) || rm -rf "$p"
      else
        rm -rf "$p"
      fi
    fi
  fi
done

echo
if $APPLY; then
  echo "Removed ~$((total / 1024))MB of regenerable artifacts."
  echo "After: $(du -sh "$ROOT" 2>/dev/null | cut -f1)"
else
  echo "Dry-run only (~$((total / 1024))MB reclaimable). Re-run with --apply to delete."
fi
