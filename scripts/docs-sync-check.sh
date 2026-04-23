#!/usr/bin/env bash
# Aiome - The Autonomous AI Operating System
# Copyright (C) 2026 motivationstudio, LLC
#
# Licensed under the Apache License, Version 2.0.
#
# docs-sync-check.sh — ドキュメント同期チェッカー
#
# 目的: コード変更に対してドキュメントの同期漏れを検出する
# セキュリティ: 外部依存なし（bash + git + grep のみ）、
#              シークレット参照なし、ファイル書き込みなし
#
# 使用法:
#   bash scripts/docs-sync-check.sh          # ローカル実行
#   bash scripts/docs-sync-check.sh --ci     # CI 上で実行（exit code を返す）

set -euo pipefail

# ──────────────────────────────────────
# Configuration
# ──────────────────────────────────────
WARN_COUNT=0
CI_MODE=false

if [[ "${1:-}" == "--ci" ]]; then
  CI_MODE=true
fi

warn() {
  WARN_COUNT=$((WARN_COUNT + 1))
  echo "⚠️  [DOCS-SYNC] $1"
}

info() {
  echo "ℹ️  [DOCS-SYNC] $1"
}

pass() {
  echo "✅ [DOCS-SYNC] $1"
}

# ──────────────────────────────────────
# Check 1: CHANGELOG.md 同期
# .rs ファイルが変更されている場合、CHANGELOG.md の
# [Unreleased] セクションにも変更があるべき
# ──────────────────────────────────────
check_changelog_sync() {
  info "Check 1: CHANGELOG.md 同期チェック"

  # 直近コミットで変更された .rs ファイルを検出
  local rs_changed
  rs_changed=$(git diff HEAD~1 --name-only --diff-filter=ACMR 2>/dev/null | grep '\.rs$' || true)

  if [[ -z "$rs_changed" ]]; then
    pass "Rustファイルの変更なし — CHANGELOG チェックをスキップ"
    return
  fi

  # CHANGELOG.md が変更されているか
  local changelog_changed
  changelog_changed=$(git diff HEAD~1 --name-only 2>/dev/null | grep '^CHANGELOG.md$' || true)

  if [[ -z "$changelog_changed" ]]; then
    warn "Rustファイルが変更されていますが CHANGELOG.md が更新されていません"
    echo "    変更ファイル: $(echo "$rs_changed" | head -5 | tr '\n' ' ')"
  else
    pass "CHANGELOG.md が更新されています"
  fi
}

# ──────────────────────────────────────
# Check 2: README_en.md 同期
# README.md が変更された場合、README_en.md も
# 変更されるべき
# ──────────────────────────────────────
check_readme_sync() {
  info "Check 2: README_en.md 同期チェック"

  local readme_changed
  readme_changed=$(git diff HEAD~1 --name-only 2>/dev/null | grep '^README\.md$' || true)

  if [[ -z "$readme_changed" ]]; then
    pass "README.md の変更なし — スキップ"
    return
  fi

  local readme_en_changed
  readme_en_changed=$(git diff HEAD~1 --name-only 2>/dev/null | grep '^README_en\.md$' || true)

  if [[ -z "$readme_en_changed" ]]; then
    warn "README.md が変更されていますが README_en.md が更新されていません"
  else
    pass "README_en.md が更新されています"
  fi
}

# ──────────────────────────────────────
# Check 3: .env.example 同期
# .cargo/config.toml や Cargo.toml に環境変数設定が
# 追加された場合、.env.example にも反映されるべき
# ──────────────────────────────────────
check_env_example_sync() {
  info "Check 3: .env.example 同期チェック"

  # .cargo/config.toml や .rs ファイルが変更されているか
  local relevant_files
  relevant_files=$(git diff HEAD~1 --name-only --diff-filter=ACMR 2>/dev/null | grep -E '(\.cargo/config\.toml|\.rs)$' || true)

  if [[ -z "$relevant_files" ]]; then
    pass "環境変数に関連するファイルの変更なし — スキップ"
    return
  fi

  # コード内に新しい env::var("VAR") が追加されたかチェック
  local new_envs
  new_envs=$(echo "$relevant_files" | tr '\n' '\0' | xargs -0 git diff HEAD~1 -U0 --diff-filter=ACMR -- 2>/dev/null | grep '^\+' | grep -oE 'env::var\("([A-Z_]+)"\)' | sed 's/env::var("//;s/")//' | sort -u || true)

  local env_example_changed
  env_example_changed=$(git diff HEAD~1 --name-only 2>/dev/null | grep '^\.env\.example$' || true)

  if [[ -n "$new_envs" && -z "$env_example_changed" ]]; then
    # 追加された環境変数が .env.example にすでにあるか確認
    local missing_in_example=""
    while IFS= read -r var; do
      [[ -z "$var" ]] && continue
      if ! grep -q "$var" .env.example 2>/dev/null; then
        missing_in_example="$missing_in_example $var"
      fi
    done <<< "$new_envs"

    if [[ -n "${missing_in_example// /}" ]]; then
      warn "コード内に新しい環境変数が追加されましたが .env.example が更新されていません: $missing_in_example"
    else
      pass "新しい環境変数はすでに .env.example に記載されています"
    fi
  else
    pass ".env.example の同期に問題ありません"
  fi
}

# ──────────────────────────────────────
# Check 4: ドキュメントタイムスタンプ鮮度
# 主要ドキュメントの「最終更新日」が30日以上
# 古くないかチェック
# ──────────────────────────────────────
check_doc_freshness() {
  info "Check 4: ドキュメントタイムスタンプ鮮度チェック"

  local today
  today=$(date +%Y-%m-%d)
  local threshold_epoch
  # 30日前の UNIX timestamp
  threshold_epoch=$(date -v-30d +%s 2>/dev/null || date -d "30 days ago" +%s 2>/dev/null || echo "0")

  if [[ "$threshold_epoch" == "0" ]]; then
    info "日付計算がサポートされていない環境です — スキップ"
    return
  fi

  local docs_to_check=(
    "ARCHITECTURE.md"
    "docs/architecture/SECURITY_DESIGN.md"
    "docs/architecture/INFRASTRUCTURE_MODULES.md"
  )

  for doc in "${docs_to_check[@]}"; do
    if [[ ! -f "$doc" ]]; then
      continue
    fi

    # ファイル内のタイムスタンプを抽出（YYYY-MM-DD パターン）
    local doc_date
    doc_date=$(grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' "$doc" | tail -1 || true)

    if [[ -z "$doc_date" ]]; then
      warn "$doc にタイムスタンプが見つかりません"
      continue
    fi

    local doc_epoch
    doc_epoch=$(date -j -f "%Y-%m-%d" "$doc_date" +%s 2>/dev/null || date -d "$doc_date" +%s 2>/dev/null || echo "0")

    if [[ "$doc_epoch" == "0" ]]; then
      continue
    fi

    if [[ "$doc_epoch" -lt "$threshold_epoch" ]]; then
      warn "$doc のタイムスタンプ ($doc_date) が30日以上古い可能性があります"
    else
      pass "$doc は最新です ($doc_date)"
    fi
  done
}

# ──────────────────────────────────────
# Check 5: セキュリティドキュメント整合性
# security 関連の .rs ファイルが変更された場合、
# SECURITY_DESIGN.md か SECURITY_WHITEPAPER.md も
# 変更されるべき
# ──────────────────────────────────────
check_security_doc_sync() {
  info "Check 5: セキュリティドキュメント同期チェック"

  local security_rs_changed
  security_rs_changed=$(git diff HEAD~1 --name-only --diff-filter=ACMR 2>/dev/null \
    | grep -E '(security|guardrails|immune|bastion|swarm).*\.rs$' || true)

  if [[ -z "$security_rs_changed" ]]; then
    pass "セキュリティ関連Rustファイルの変更なし — スキップ"
    return
  fi

  local sec_doc_changed
  sec_doc_changed=$(git diff HEAD~1 --name-only 2>/dev/null \
    | grep -E '(SECURITY_DESIGN|SECURITY_WHITEPAPER|SECURITY)\.md$' || true)

  if [[ -z "$sec_doc_changed" ]]; then
    warn "セキュリティ関連コードが変更されていますが、セキュリティドキュメントが更新されていません"
    echo "    変更ファイル: $(echo "$security_rs_changed" | head -5 | tr '\n' ' ')"
  else
    pass "セキュリティドキュメントが更新されています"
  fi
}

# ──────────────────────────────────────
# Check 6: 新モジュールの INFRASTRUCTURE_MODULES.md 反映
# libs/infrastructure/src/ 配下に新ファイルが追加された場合、
# INFRASTRUCTURE_MODULES.md にも記載されるべき
# ──────────────────────────────────────
check_infra_module_doc() {
  info "Check 6: Infrastructure モジュールドキュメント同期チェック"

  local new_infra_files
  new_infra_files=$(git diff HEAD~1 --name-only --diff-filter=A 2>/dev/null \
    | grep '^libs/infrastructure/src/.*\.rs$' \
    | grep -v 'tests\.\|mod\.\|lib\.' || true)

  if [[ -z "$new_infra_files" ]]; then
    pass "新規 Infrastructure モジュールの追加なし — スキップ"
    return
  fi

  local infra_doc_changed
  infra_doc_changed=$(git diff HEAD~1 --name-only 2>/dev/null \
    | grep 'INFRASTRUCTURE_MODULES\.md$' || true)

  if [[ -z "$infra_doc_changed" ]]; then
    warn "新規 Infrastructure モジュールが追加されていますが INFRASTRUCTURE_MODULES.md が更新されていません"
    echo "    新規ファイル: $(echo "$new_infra_files" | head -5 | tr '\n' ' ')"
  else
    pass "INFRASTRUCTURE_MODULES.md が更新されています"
  fi
}

# ──────────────────────────────────────
# Check 7: DESIGN.md 同期
# tokens.css や animations.css が変更された場合、
# DESIGN.md も変更されるべき (AGENTS.md Rule 10)
# ──────────────────────────────────────
check_design_sync() {
  info "Check 7: DESIGN.md 同期チェック"

  local css_changed
  css_changed=$(git diff HEAD~1 --name-only --diff-filter=ACMR 2>/dev/null \
    | grep -E '(tokens|animations)\.css$' || true)

  if [[ -z "$css_changed" ]]; then
    pass "デザイントークン(CSS)の変更なし — スキップ"
    return
  fi

  local design_doc_changed
  design_doc_changed=$(git diff HEAD~1 --name-only 2>/dev/null \
    | grep 'DESIGN\.md$' || true)

  if [[ -z "$design_doc_changed" ]]; then
    warn "tokens.css または animations.css が変更されていますが DESIGN.md が更新されていません"
    echo "    変更ファイル: $(echo "$css_changed" | tr '\n' ' ')"
  else
    pass "DESIGN.md が更新されています"
  fi
}


# ──────────────────────────────────────
# Main
# ──────────────────────────────────────
main() {
  echo ""
  echo "╔══════════════════════════════════════╗"
  echo "║   📚 Aiome Docs-Sync Checker        ║"
  echo "╚══════════════════════════════════════╝"
  echo ""

  # git リポジトリかチェック
  if ! git rev-parse --is-inside-work-tree &>/dev/null; then
    echo "❌ Error: Not inside a git repository"
    exit 1
  fi

  # 最低1コミットが存在するか
  if ! git rev-parse HEAD~1 &>/dev/null; then
    info "初回コミット — 比較対象なし。スキップ。"
    exit 0
  fi

  check_changelog_sync
  echo ""
  check_readme_sync
  echo ""
  check_env_example_sync
  echo ""
  check_doc_freshness
  echo ""
  check_security_doc_sync
  echo ""
  check_infra_module_doc
  echo ""
  check_design_sync

  echo ""
  echo "──────────────────────────────────────"
  if [[ $WARN_COUNT -gt 0 ]]; then
    echo "📋 結果: $WARN_COUNT 件の同期漏れの可能性"
    echo ""
    echo "修正手順: /docs-sync ワークフローを実行してください"
    if [[ "$CI_MODE" == true ]]; then
      # CI では厳格に同期漏れをブロックする
      exit 1
    fi
  else
    echo "🎉 全チェック通過！ドキュメントは同期されています。"
  fi
}

main "$@"
