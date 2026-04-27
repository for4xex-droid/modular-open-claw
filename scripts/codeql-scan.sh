#!/usr/bin/env bash
# =============================================================================
# codeql-scan.sh — CodeQL Inter-Procedural Taint Analysis Runner
# =============================================================================
# Usage:
#   bash scripts/codeql-scan.sh [TARGET_DIR]
#
# TARGET_DIR defaults to current directory (.)
# Environment:
#   CODEQL_DB_DIR  — directory for the CodeQL database (default: .codeql-db)
#   CODEQL_OUTPUT  — SARIF output path (default: docs/architecture/codeql_taint_report.sarif)
#   CODEQL_SUMMARY — Human-readable summary path (default: docs/architecture/codeql_taint_summary.md)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_DIR="${1:-${REPO_ROOT}}"
DB_DIR="${CODEQL_DB_DIR:-${REPO_ROOT}/.codeql-db}"
SARIF_OUTPUT="${CODEQL_OUTPUT:-${REPO_ROOT}/docs/architecture/codeql_taint_report.sarif}"
SUMMARY_OUTPUT="${CODEQL_SUMMARY:-${REPO_ROOT}/docs/architecture/codeql_taint_summary.md}"
QUERY_PACK="${REPO_ROOT}/codeql-custom"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log() { echo -e "${CYAN}[CodeQL]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
err() { echo -e "${RED}[ERROR]${NC} $*" >&2; }
ok() { echo -e "${GREEN}[OK]${NC} $*"; }

# --- Pre-flight checks -------------------------------------------------------
log "Pre-flight checks..."

if ! command -v codeql &>/dev/null; then
    err "CodeQL CLI is not installed. Install via: brew install codeql"
    exit 1
fi

CODEQL_VERSION=$(codeql --version 2>&1 | head -1)
log "CodeQL version: ${CODEQL_VERSION}"

if [ ! -f "${QUERY_PACK}/qlpack.yml" ]; then
    err "Custom query pack not found at ${QUERY_PACK}/qlpack.yml"
    exit 1
fi

if [ ! -f "${QUERY_PACK}/AiomeTaintTracking.ql" ]; then
    err "Main query not found at ${QUERY_PACK}/AiomeTaintTracking.ql"
    exit 1
fi

ok "Pre-flight checks passed"

# --- Step 1: Install query pack dependencies ----------------------------------
log "Installing query pack dependencies..."
codeql pack install "${QUERY_PACK}" 2>&1 | tail -3
ok "Dependencies installed"

# --- Step 2: Create / update CodeQL database ----------------------------------
log "Creating CodeQL database at ${DB_DIR}..."
log "Source root: ${TARGET_DIR}"

# Use overwrite to allow re-runs
codeql database create "${DB_DIR}" \
    --language=rust \
    --source-root="${TARGET_DIR}" \
    --overwrite \
    2>&1 | grep -E "(Finaliz|Creat|Error|WARN)" || true

if [ ! -d "${DB_DIR}" ]; then
    err "Database creation failed"
    exit 1
fi
ok "Database created"

# --- Step 3: Run taint analysis -----------------------------------------------
log "Running taint analysis..."
mkdir -p "$(dirname "${SARIF_OUTPUT}")"

codeql database analyze "${DB_DIR}" \
    "${QUERY_PACK}/AiomeTaintTracking.ql" \
    --format=sarif-latest \
    --output="${SARIF_OUTPUT}" \
    --additional-packs="${QUERY_PACK}" \
    --rerun \
    2>&1 | grep -v "^$"

if [ ! -f "${SARIF_OUTPUT}" ]; then
    err "SARIF output not generated"
    exit 1
fi
ok "Analysis complete. SARIF output: ${SARIF_OUTPUT}"

# --- Step 4: Generate human-readable summary ----------------------------------
log "Generating summary report..."

python3 - "${SARIF_OUTPUT}" "${SUMMARY_OUTPUT}" << 'PYEOF'
import json, sys, os
from datetime import datetime

sarif_path = sys.argv[1]
summary_path = sys.argv[2]

with open(sarif_path) as f:
    data = json.load(f)

findings = []
for run in data.get("runs", []):
    for result in run.get("results", []):
        msg = result.get("message", {}).get("text", "")
        locs = result.get("locations", [])
        flows = result.get("codeFlows", [])
        for loc in locs:
            phys = loc.get("physicalLocation", {})
            uri = phys.get("artifactLocation", {}).get("uri", "?")
            line = phys.get("region", {}).get("startLine", "?")
            steps = 0
            if flows:
                for flow in flows:
                    for tf in flow.get("threadFlows", []):
                        steps = max(steps, len(tf.get("locations", [])))
            findings.append({
                "file": uri,
                "line": line,
                "message": msg,
                "steps": steps
            })

total = len(findings)
timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

lines = [
    f"# 🔍 CodeQL Taint Analysis Report",
    f"",
    f"> Generated: {timestamp}",
    f"> Tool: CodeQL Inter-Procedural Taint Tracking",
    f"> Query: `AiomeTaintTracking.ql`",
    f"",
    f"## Summary",
    f"",
    f"| Metric | Value |",
    f"|--------|-------|",
    f"| Total Findings | **{total}** |",
    f"| Severity | Critical (9.0) |",
    f"| SARIF Output | `{os.path.basename(sarif_path)}` |",
    f"",
]

if total == 0:
    lines.append("✅ **No taint tracking violations found.** All data flows are properly sanitized.")
else:
    lines.append(f"⚠️ **{total} taint tracking violation(s) detected.**")
    lines.append("")
    lines.append("## Findings")
    lines.append("")
    lines.append("| # | File | Line | Flow Steps | Description |")
    lines.append("|---|------|------|-----------|-------------|")
    for i, f_item in enumerate(findings, 1):
        desc = f_item["message"][:100].replace("|", "\\|")
        lines.append(f"| {i} | `{f_item['file']}` | L{f_item['line']} | {f_item['steps']} | {desc} |")
    lines.append("")
    lines.append("## Remediation")
    lines.append("")
    lines.append("For each finding, ensure user input is sanitized before reaching sensitive operations:")
    lines.append("- **Path injection**: Validate paths, reject `..` traversals, use allowlists")
    lines.append("- **Command injection**: Never pass user input to `Command::new()`, use allowlists")

lines.append("")
lines.append("---")
lines.append(f"*Report generated by `scripts/codeql-scan.sh`*")

os.makedirs(os.path.dirname(summary_path), exist_ok=True)
with open(summary_path, "w") as out:
    out.write("\n".join(lines))

print(f"Summary: {total} finding(s) → {summary_path}")
PYEOF

ok "Summary written to ${SUMMARY_OUTPUT}"

# --- Step 5: Exit code decision -----------------------------------------------
FINDING_COUNT=$(python3 -c "
import json, sys
with open('${SARIF_OUTPUT}') as f:
    data = json.load(f)
count = sum(len(r.get('results',[])) for r in data.get('runs',[]))
print(count)
")

log "Total findings: ${FINDING_COUNT}"

if [ "${FINDING_COUNT}" -gt 0 ]; then
    warn "⚠️  ${FINDING_COUNT} taint tracking violation(s) found!"
    warn "Review ${SUMMARY_OUTPUT} for details."
    # In CI mode, exit 1 to block the build
    if [ "${CI:-false}" = "true" ]; then
        err "CI mode: blocking build due to taint violations"
        exit 1
    fi
    exit 0
else
    ok "✅ No taint tracking violations. Code is clean."
    exit 0
fi
