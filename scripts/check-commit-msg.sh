#!/usr/bin/env bash
# Aiome - The Autonomous AI Operating System
# Copyright (C) 2026 motivationstudio, LLC
#
# Licensed under the Apache License, Version 2.0.
#
# check-commit-msg.sh — Conventional Commits バリデーター
#
# コミットメッセージが Conventional Commits 仕様に準拠しているか検証する。
# https://www.conventionalcommits.org/
#
# セキュリティ: 外部依存なし、eval なし、read-only
#
# 使用法:
#   bash scripts/check-commit-msg.sh "feat: add new feature"
#   bash scripts/check-commit-msg.sh --from-file .git/COMMIT_EDITMSG
#   bash scripts/check-commit-msg.sh --ci  # CI: 直近コミットを検証

set -uo pipefail

# ──────────────────────────────────────
# Configuration
# ──────────────────────────────────────
# Allowed types (Conventional Commits + Angular convention)
VALID_TYPES="feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert|security|audit"

# Max subject line length
MAX_SUBJECT_LENGTH=100

# ──────────────────────────────────────
# Get the commit message
# ──────────────────────────────────────
COMMIT_MSG=""

case "${1:-}" in
  --ci)
    COMMIT_MSG=$(git log -1 --pretty=%B 2>/dev/null || echo "")
    ;;
  --from-file)
    if [[ -n "${2:-}" && -f "$2" ]]; then
      COMMIT_MSG=$(head -1 "$2")
    else
      echo "❌ File not found: ${2:-}"
      exit 1
    fi
    ;;
  *)
    COMMIT_MSG="${1:-}"
    ;;
esac

if [[ -z "$COMMIT_MSG" ]]; then
  echo "❌ No commit message provided."
  echo "Usage: bash scripts/check-commit-msg.sh \"feat: description\""
  exit 1
fi

# Extract first line only
SUBJECT=$(echo "$COMMIT_MSG" | head -1)

# ──────────────────────────────────────
# Validations
# ──────────────────────────────────────
ERRORS=0

# Check 1: Conventional Commits format
# type(scope)?: description
# type(scope)?!: description (breaking change)
PATTERN="^(${VALID_TYPES})(\(.+\))?\!?: .+"
if ! echo "$SUBJECT" | grep -qE "$PATTERN"; then
  echo "❌ Commit message does not follow Conventional Commits format."
  echo ""
  echo "   Expected: <type>(<scope>): <description>"
  echo "   Got:      $SUBJECT"
  echo ""
  echo "   Valid types: ${VALID_TYPES//|/, }"
  echo ""
  echo "   Examples:"
  echo "     feat: add new karma endpoint"
  echo "     fix(deadlock): resolve SQLite transaction nesting"
  echo "     docs: update CHANGELOG with security fixes"
  echo "     ci: add pattern-enforcer job"
  ERRORS=$((ERRORS + 1))
else
  echo "✅ Format: Conventional Commits"
fi

# Check 2: Subject line length
SUBJECT_LEN=${#SUBJECT}
if [[ $SUBJECT_LEN -gt $MAX_SUBJECT_LENGTH ]]; then
  echo "⚠️  Subject line too long: $SUBJECT_LEN chars (max: $MAX_SUBJECT_LENGTH)"
  # Warning only, don't fail
else
  echo "✅ Length: $SUBJECT_LEN/$MAX_SUBJECT_LENGTH"
fi

# Check 3: No period at end
if [[ "$SUBJECT" =~ \.$ ]]; then
  echo "⚠️  Subject line should not end with a period"
fi

# Check 4: First letter of description should be lowercase
DESC=$(echo "$SUBJECT" | sed -E "s/^(${VALID_TYPES})(\(.+\))?\!?: //")
if [[ -n "$DESC" ]] && echo "$DESC" | grep -qE '^[A-Z]'; then
  echo "ℹ️  Description starts with uppercase (convention: lowercase)"
fi

# ──────────────────────────────────────
# Result
# ──────────────────────────────────────
if [[ $ERRORS -gt 0 ]]; then
  exit 1
else
  echo "✅ Commit message is valid."
  exit 0
fi
