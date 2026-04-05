#!/bin/bash
# Aiome - The Autonomous AI Operating System
# Copyright (C) 2026 motivationstudio, LLC
#
# Licensed under the Apache License, Version 2.0.
#
# install_hooks.sh — Git Hooks インストーラー
#
# 使用法: bash scripts/install_hooks.sh
#
# セキュリティ: hooks は .git/hooks/ に書き込みのみ。
# 実行されるチェックは全て read-only（ファイル変更なし）。

set -euo pipefail

HOOKS_DIR=".git/hooks"

if [[ ! -d "$HOOKS_DIR" ]]; then
  echo "❌ .git/hooks が見つかりません。リポジトリのルートで実行してください。"
  exit 1
fi

# ──────────────────────────────────────
# Pre-commit Hook
# ──────────────────────────────────────
PRE_COMMIT_HOOK="$HOOKS_DIR/pre-commit"

echo "📋 Installing pre-commit hook..."

cat << 'PRECOMMIT_EOF' > "$PRE_COMMIT_HOOK"
#!/bin/bash
# Aiome Pre-commit Hook
# Runs fast local checks before allowing a commit.
# All checks are read-only and non-destructive.

set -uo pipefail

echo ""
echo "🔍 Aiome Pre-commit Checks"
echo "──────────────────────────"

ERRORS=0

# ── Check 1: Rust formatting check ──
if command -v cargo &> /dev/null; then
  echo -n "  fmt check...  "
  if cargo fmt --all -- --check 2>/dev/null; then
    echo "✅"
  else
    echo "❌ (run: cargo fmt --all)"
    ERRORS=$((ERRORS + 1))
  fi
fi

# ── Check 2: Anti-Pattern Enforcer (fast grep) ──
if [[ -f "scripts/pattern-enforcer.sh" ]]; then
  echo -n "  patterns...   "
  # Only check staged .rs files for speed
  STAGED_RS=$(git diff --cached --name-only --diff-filter=ACMR | grep '\.rs$' || true)
  if [[ -z "$STAGED_RS" ]]; then
    echo "Skipped (no .rs files staged)"
  else
    # Run the full enforcer but only care about errors (not warnings)
    if bash scripts/pattern-enforcer.sh --ci 2>/dev/null; then
      echo "✅"
    else
      echo "❌ (Errors found! Run: bash scripts/pattern-enforcer.sh to see details)"
      ERRORS=$((ERRORS + 1))
    fi
  fi
fi

# ── Check 3: Critical file change warning ──
CRITICAL_FILES=(
  "libs/core/src/traits.rs"
  "libs/infrastructure/src/job_queue/mod.rs"
  "libs/infrastructure/src/job_queue/swarm.rs"
  "libs/infrastructure/src/job_queue/migrations.rs"
  ".cargo/config.toml"
  "Cargo.toml"
)

STAGED_FILES=$(git diff --cached --name-only || true)
CRITICAL_CHANGED=""
for cf in "${CRITICAL_FILES[@]}"; do
  if echo "$STAGED_FILES" | grep -q "^${cf}$"; then
    CRITICAL_CHANGED="$CRITICAL_CHANGED\n    ⚡ $cf"
  fi
done

if [[ -n "$CRITICAL_CHANGED" ]]; then
  echo ""
  echo "  ⚠️  Critical files modified (CODEOWNERS review required):"
  echo -e "$CRITICAL_CHANGED"
  echo ""
  echo "  Ensure ADR rules (R-001~R-008) are respected."
  echo "  See: .agent/skills/architecture-rules.md"
fi

# ── Check 4: Prevent committing secrets ──
echo -n "  secrets... "
SECRET_PATTERNS='(GEMINI_API_KEY|OPENAI_API_KEY|ANTHROPIC_API_KEY|DISCORD_TOKEN|TELEGRAM_TOKEN|API_SERVER_SECRET|VAULT_SECRET|FEDERATION_SECRET)=[^$]'
STAGED_CONTENT=$(git diff --cached --diff-filter=ACMR -U0 | grep '^+' | grep -v '^+++' || true)
if echo "$STAGED_CONTENT" | grep -qE "$SECRET_PATTERNS"; then
  echo "❌ POTENTIAL SECRET DETECTED!"
  echo "  🔴 Staged diff contains what appears to be a secret value."
  echo "  Review staged changes carefully before committing."
  ERRORS=$((ERRORS + 1))
else
  echo "✅"
fi

# ── Check 5: Prevent committing strategic documents ──
echo -n "  strategy docs... "
STRATEGY_PATTERNS='(strategy|valuation|buyout|business_plan).*\.md$'
STAGED_FILES=$(git diff --cached --name-only || true)
if echo "$STAGED_FILES" | grep -iqE "$STRATEGY_PATTERNS"; then
  echo "❌ STRATEGIC DOCUMENT DETECTED!"
  echo "  🔴 You are trying to commit a sensitive business/strategy document."
  echo "  Rename it or bypass this check only if you are absolutely sure."
  ERRORS=$((ERRORS + 1))
else
  echo "✅"
fi

# ── Result ──
echo ""
if [[ $ERRORS -gt 0 ]]; then
  echo "❌ Pre-commit: $ERRORS check(s) failed."
  echo "   Fix the issues above, or bypass with: git commit --no-verify"
  exit 1
else
  echo "✅ Pre-commit: All checks passed."
fi
PRECOMMIT_EOF

chmod +x "$PRE_COMMIT_HOOK"
echo "  ✅ Pre-commit hook installed."

# ──────────────────────────────────────
# Pre-push Hook (existing: ARCHITECTURE.md auto-update)
# ──────────────────────────────────────
PRE_PUSH_HOOK="$HOOKS_DIR/pre-push"

echo "📋 Installing pre-push hook..."

cat << 'PREPUSH_EOF' > "$PRE_PUSH_HOOK"
#!/bin/bash
# Aiome Pre-push Hook
# Runs heavier checks before pushing to remote.

set -uo pipefail

echo ""
echo "🚀 Aiome Pre-push Checks"
echo "──────────────────────────"

if ! make preflight; then
  echo "❌ Preflight failed! Please fix the errors before pushing."
  exit 1
fi

echo ""
echo "✅ Pre-push: Checks complete."
exit 0
PREPUSH_EOF

chmod +x "$PRE_PUSH_HOOK"
echo "  ✅ Pre-push hook installed."

# ──────────────────────────────────────
# Commit-msg Hook (Conventional Commits)
# ──────────────────────────────────────
COMMIT_MSG_HOOK="$HOOKS_DIR/commit-msg"

echo "📋 Installing commit-msg hook..."

cat << 'COMMITMSG_EOF' > "$COMMIT_MSG_HOOK"
#!/bin/bash
# Aiome Commit-msg Hook
# Validates Conventional Commits format.

COMMIT_MSG_FILE="$1"
if [[ -f "scripts/check-commit-msg.sh" ]]; then
  bash scripts/check-commit-msg.sh --from-file "$COMMIT_MSG_FILE"
  exit $?
fi
COMMITMSG_EOF

chmod +x "$COMMIT_MSG_HOOK"
echo "  ✅ Commit-msg hook installed."

# ──────────────────────────────────────
# Summary
# ──────────────────────────────────────
echo ""
echo "══════════════════════════════════════"
echo "🎉 Git hooks installed successfully!"
echo ""
echo "  pre-commit: fmt + patterns + secrets + critical file warnings"
echo "  commit-msg: Conventional Commits validation"
echo "  pre-push:   architecture auto-update + docs-sync"
echo ""
echo "  Bypass: git commit --no-verify / git push --no-verify"
echo "══════════════════════════════════════"
