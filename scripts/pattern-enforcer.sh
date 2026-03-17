#!/usr/bin/env bash
# Aiome - The Autonomous AI Operating System
# Copyright (C) 2026 motivationstudio, LLC
#
# Licensed under the Apache License, Version 2.0.
#
# pattern-enforcer.sh — アンチパターン検出エンジン
#
# 目的: .aiome/anti-patterns.yml に定義されたルールに基づき、
#       コードベースのアンチパターンを検出する
#
# セキュリティ設計:
#   - 外部依存なし（bash + grep + awk のみ）
#   - eval / source を一切使用しない
#   - ファイル書き込みなし（read-only 検査）
#   - YAML パーサーの代わりに awk でフィールド抽出（安全）
#
# 使用法:
#   bash scripts/pattern-enforcer.sh              # ローカル実行
#   bash scripts/pattern-enforcer.sh --ci         # CI 上で実行
#   bash scripts/pattern-enforcer.sh --fix-hints  # 修正ヒント付き

set -euo pipefail

# ──────────────────────────────────────
# Configuration
# ──────────────────────────────────────
RULES_FILE=".aiome/anti-patterns.yml"
SCAN_ROOT="."
ERROR_COUNT=0
WARNING_COUNT=0
INFO_COUNT=0
CI_MODE=false
FIX_HINTS=false

for arg in "$@"; do
  case "$arg" in
    --ci) CI_MODE=true ;;
    --fix-hints) FIX_HINTS=true ;;
  esac
done

# ──────────────────────────────────────
# Output helpers
# ──────────────────────────────────────
red()    { echo -e "\033[31m$1\033[0m"; }
yellow() { echo -e "\033[33m$1\033[0m"; }
cyan()   { echo -e "\033[36m$1\033[0m"; }
green()  { echo -e "\033[32m$1\033[0m"; }

# ──────────────────────────────────────
# YAML Parser (minimal, safe, no eval)
# Extracts rule blocks from the YAML file
# ──────────────────────────────────────
parse_and_enforce() {
  if [[ ! -f "$RULES_FILE" ]]; then
    red "❌ Rules file not found: $RULES_FILE"
    exit 1
  fi

  local current_id="" current_name="" current_pattern="" current_severity=""
  local current_include="" current_exclude="" current_adr=""

  while IFS= read -r line || [[ -n "$line" ]]; do
    # Skip comments and empty lines
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// /}" ]] && continue

    # New rule block
    if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*id:[[:space:]]*(.*) ]]; then
      # Process previous rule if exists
      if [[ -n "$current_id" && -n "$current_pattern" ]]; then
        enforce_rule "$current_id" "$current_name" "$current_pattern" \
                     "$current_severity" "$current_include" "$current_exclude" "$current_adr"
      fi
      current_id="${BASH_REMATCH[1]}"
      current_name="" current_pattern="" current_severity="warning"
      current_include="" current_exclude="" current_adr=""
      continue
    fi

    # Extract fields (safe string parsing, no eval)
    if [[ "$line" =~ ^[[:space:]]*name:[[:space:]]*\"(.*)\" ]]; then
      current_name="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^[[:space:]]*name:[[:space:]]*(.*) ]]; then
      current_name="${BASH_REMATCH[1]}"
      current_name="${current_name//\"/}"
    fi

    if [[ "$line" =~ ^[[:space:]]*pattern:[[:space:]]*\'(.*)\' ]]; then
      current_pattern="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^[[:space:]]*pattern:[[:space:]]*\"(.*)\" ]]; then
      current_pattern="${BASH_REMATCH[1]}"
    fi

    if [[ "$line" =~ ^[[:space:]]*severity:[[:space:]]*(.*) ]]; then
      current_severity="${BASH_REMATCH[1]}"
    fi

    if [[ "$line" =~ ^[[:space:]]*include:[[:space:]]*\"(.*)\" ]]; then
      current_include="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^[[:space:]]*include:[[:space:]]*(.*) ]]; then
      current_include="${BASH_REMATCH[1]}"
      current_include="${current_include//\"/}"
    fi

    if [[ "$line" =~ ^[[:space:]]*exclude:[[:space:]]*\"(.*)\" ]]; then
      current_exclude="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^[[:space:]]*exclude:[[:space:]]*(.*) ]]; then
      current_exclude="${BASH_REMATCH[1]}"
      current_exclude="${current_exclude//\"/}"
    fi

    if [[ "$line" =~ ^[[:space:]]*adr:[[:space:]]*(.*) ]]; then
      current_adr="${BASH_REMATCH[1]}"
    fi

  done < "$RULES_FILE"

  # Process last rule
  if [[ -n "$current_id" && -n "$current_pattern" ]]; then
    enforce_rule "$current_id" "$current_name" "$current_pattern" \
                 "$current_severity" "$current_include" "$current_exclude" "$current_adr"
  fi
}

# ──────────────────────────────────────
# Enforce a single rule
# ──────────────────────────────────────
enforce_rule() {
  local id="$1" name="$2" pattern="$3" severity="$4"
  local include="$5" exclude="$6" adr="$7"

  # Build grep include patterns
  local grep_opts=("-rnE" "--color=never")
  local find_pattern=""

  # Build find command for file filtering
  local files=""
  if [[ -n "$include" ]]; then
    # Convert pipe-separated include to find -name patterns
    IFS='|' read -ra INCLUDES <<< "$include"
    for inc in "${INCLUDES[@]}"; do
      if [[ -n "$files" ]]; then
        files="$files"$'\n'
      fi
      files="$files$(find "$SCAN_ROOT/libs" "$SCAN_ROOT/apps" -name "$inc" -type f 2>/dev/null || true)"
    done
  else
    files=$(find "$SCAN_ROOT/libs" "$SCAN_ROOT/apps" -name "*.rs" -type f 2>/dev/null || true)
  fi

  [[ -z "$files" ]] && return

  # Apply exclude filter
  if [[ -n "$exclude" ]]; then
    IFS='|' read -ra EXCLUDES <<< "$exclude"
    for exc in "${EXCLUDES[@]}"; do
      files=$(echo "$files" | grep -v "$exc" || true)
    done
  fi

  [[ -z "$files" ]] && return

  # Run grep on filtered files
  local matches=""
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    [[ ! -f "$file" ]] && continue
    local result
    result=$(grep -nE "$pattern" "$file" 2>/dev/null || true)
    if [[ -n "$result" ]]; then
      while IFS= read -r match_line; do
        matches="${matches}${file}:${match_line}"$'\n'
      done <<< "$result"
    fi
  done <<< "$files"

  if [[ -z "$matches" || "$matches" == $'\n' ]]; then
    return
  fi

  # Report findings
  local icon=""
  case "$severity" in
    error)   icon="🔴"; ERROR_COUNT=$((ERROR_COUNT + 1)) ;;
    warning) icon="🟡"; WARNING_COUNT=$((WARNING_COUNT + 1)) ;;
    info)    icon="🔵"; INFO_COUNT=$((INFO_COUNT + 1)) ;;
  esac

  echo ""
  echo "${icon} [$id] $name (${severity})"
  if [[ -n "$adr" ]]; then
    echo "   📜 ADR: $adr (see .agent/skills/architecture-rules.md)"
  fi
  echo "   ──────────────────────────────"

  local match_count=0
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    echo "   $line"
    match_count=$((match_count + 1))
    # Limit output to 10 matches per rule
    if [[ $match_count -ge 10 ]]; then
      echo "   ... (showing first 10 matches)"
      break
    fi
  done <<< "$matches"
}

# ──────────────────────────────────────
# Main
# ──────────────────────────────────────
main() {
  echo ""
  echo "╔══════════════════════════════════════════╗"
  echo "║   🛡️  Aiome Anti-Pattern Enforcer        ║"
  echo "╚══════════════════════════════════════════╝"
  echo ""
  echo "Rules: $RULES_FILE"
  echo "Scan:  $SCAN_ROOT/{libs,apps}"

  parse_and_enforce

  echo ""
  echo "══════════════════════════════════════════"
  echo "📊 結果サマリー"
  echo "   🔴 Errors:   $ERROR_COUNT"
  echo "   🟡 Warnings: $WARNING_COUNT"
  echo "   🔵 Info:     $INFO_COUNT"
  echo "══════════════════════════════════════════"

  if [[ $ERROR_COUNT -gt 0 ]]; then
    echo ""
    red "❌ $ERROR_COUNT 件のエラーが検出されました。修正してください。"
    if [[ "$CI_MODE" == true ]]; then
      exit 1
    fi
  elif [[ $WARNING_COUNT -gt 0 ]]; then
    echo ""
    yellow "⚠️  $WARNING_COUNT 件の警告があります。確認してください。"
    if [[ "$CI_MODE" == true ]]; then
      exit 0  # 警告はブロックしない
    fi
  else
    echo ""
    green "✅ アンチパターンは検出されませんでした！"
  fi
}

main "$@"
