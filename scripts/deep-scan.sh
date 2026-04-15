#!/usr/bin/env bash
# Aiome - The Autonomous AI Operating System
# Copyright (C) 2026 motivationstudio, LLC
#
# Licensed under the Apache License, Version 2.0.
#
# deep-scan.sh — セグメント型ディープスキャン
#
# プロジェクトをクレート単位で分割スキャンし、
# 最後にクロスカッティング整合性チェックを実行する。
#
# セキュリティ: 外部依存なし、ファイル書き込みなし、eval なし
#
# 使用法:
#   bash scripts/deep-scan.sh              # 全クレートスキャン
#   bash scripts/deep-scan.sh --ci         # CI モード
#   bash scripts/deep-scan.sh core         # 単一クレートのみ

set -uo pipefail

# ──────────────────────────────────────
# Configuration
# ──────────────────────────────────────
WARN_COUNT=0
ERROR_COUNT=0
CI_MODE=false
TARGET_CRATE=""

for arg in "$@"; do
  case "$arg" in
    --ci) CI_MODE=true ;;
    -*) ;;
    *) TARGET_CRATE="$arg" ;;
  esac
done

# Crate registry — dependency order (leaves first)
CRATES=(
  "libs/aiome-contracts"
  "libs/shared"
  "libs/core"
  "libs/infrastructure"
  "libs/wasm-skills/fs_reader"
  "libs/wasm-skills/terminal_exec"
  "libs/wasm-skills/fs_writer"
  "libs/napi-bridge"
  "apps/api-server"
  "apps/watchtower"
  "apps/samsara-hub"
  "apps/key-proxy"
)

# ──────────────────────────────────────
# Output helpers
# ──────────────────────────────────────
warn()  { WARN_COUNT=$((WARN_COUNT + 1));  echo "  ⚠️  $1"; }
err()   { ERROR_COUNT=$((ERROR_COUNT + 1)); echo "  🔴 $1"; }
pass()  { echo "  ✅ $1"; }
info()  { echo "  ℹ️  $1"; }

# ──────────────────────────────────────
# Phase 1: クレート個別スキャン
# ──────────────────────────────────────
scan_crate() {
  local crate_path="$1"
  local crate_name
  crate_name=$(basename "$crate_path")

  echo ""
  echo "┌─────────────────────────────────────────"
  echo "│ 📦 Scanning: $crate_path"
  echo "└─────────────────────────────────────────"

  if [[ ! -d "$crate_path" ]]; then
    info "ディレクトリが存在しません — スキップ"
    return
  fi

  local rs_files
  rs_files=$(find "$crate_path/src" -name "*.rs" -type f 2>/dev/null || true)
  [[ -z "$rs_files" ]] && { info "Rust ファイルなし"; return; }

  local file_count
  file_count=$(echo "$rs_files" | wc -l | tr -d ' ')
  info "ファイル数: $file_count"

  # Check 1: unwrap/expect in non-test files
  local unwrap_hits
  unwrap_hits=$(echo "$rs_files" | grep -v "tests\.\|test_\|_test\." | xargs grep -l '\.unwrap()\|\.expect(' 2>/dev/null || true)
  if [[ -n "$unwrap_hits" ]]; then
    local unwrap_count
    unwrap_count=$(echo "$unwrap_hits" | wc -l | tr -d ' ')
    warn ".unwrap()/.expect() が $unwrap_count ファイルに存在"
  else
    pass "unwrap/expect なし"
  fi

  # Check 2: silent error suppression
  local silent_err
  silent_err=$(echo "$rs_files" | grep -v "tests\.\|test_" | xargs grep -c '\.ok();' 2>/dev/null | grep -v ':0$' || true)
  if [[ -n "$silent_err" ]]; then
    warn "silent .ok() エラー抑制を検出"
  else
    pass "silent error なし"
  fi

  # Check 3: TODO/FIXME count
  local todo_count
  todo_count=$(echo "$rs_files" | xargs grep -h 'TODO\|FIXME\|HACK\|XXX' 2>/dev/null | wc -l | tr -d ' ')
  if [[ "$todo_count" -gt 0 ]]; then
    info "TODO/FIXME: $todo_count 件"
  fi

  # Check 4: Hardcoded URLs (non-test)
  local url_hits
  url_hits=$(echo "$rs_files" | grep -v "tests\.\|test_" | xargs grep -l 'http://127\.0\.0\.1\|http://localhost' 2>/dev/null || true)
  if [[ -n "$url_hits" ]]; then
    local url_count
    url_count=$(echo "$url_hits" | wc -l | tr -d ' ')
    warn "ハードコード URL: $url_count ファイル"
  else
    pass "ハードコード URL なし"
  fi

  # Check 5: format!-based JSON (potential injection)
  local json_fmt
  json_fmt=$(echo "$rs_files" | grep -v "tests\.\|test_" | xargs grep -l 'format!.*{.*".*:' 2>/dev/null || true)
  if [[ -n "$json_fmt" ]]; then
    warn "format! による JSON 構築の可能性"
  fi

  # Check 6: pub fn without doc comment
  local undocumented
  undocumented=$(echo "$rs_files" | grep -v "tests\.\|test_\|mod\.rs" | while read -r f; do
    # Count pub fn lines NOT preceded by a doc comment
    awk '/^[[:space:]]*pub (async )?fn / { if (prev !~ /\/\/\//) print FILENAME":"NR":"$0 } { prev=$0 }' "$f" 2>/dev/null
  done | head -5 || true)
  if [[ -n "$undocumented" ]]; then
    local undoc_count
    undoc_count=$(echo "$rs_files" | grep -v "tests\.\|test_\|mod\.rs" | while read -r f; do
      awk '/^[[:space:]]*pub (async )?fn / { if (prev !~ /\/\/\//) count++ } { prev=$0 } END { print count+0 }' "$f" 2>/dev/null
    done | awk '{s+=$1} END {print s+0}')
    info "未ドキュメント pub fn: $undoc_count 件"
  fi
}

# ──────────────────────────────────────
# Phase 2: クロスカッティング整合性チェック
# ──────────────────────────────────────
cross_cutting_checks() {
  echo ""
  echo "╔══════════════════════════════════════════╗"
  echo "║   🔀 Phase 2: クロスカッティング整合性   ║"
  echo "╚══════════════════════════════════════════╝"

  # CC-1: trait定義 ↔ impl 整合性（async_trait メソッド数チェック）
  echo ""
  echo "── CC-1: JobQueue trait vs impl メソッド数 ──"
  local trait_methods impl_methods
  trait_methods=$(grep -c 'async fn ' libs/core/src/traits.rs 2>/dev/null | tr -d '\n ' || echo "0")
  impl_methods=$(grep -c 'async fn ' libs/infrastructure/src/job_queue/mod.rs 2>/dev/null | tr -d '\n ' || echo "0")
  if [[ "$trait_methods" != "$impl_methods" ]]; then
    warn "JobQueue trait ($trait_methods メソッド) と impl ($impl_methods メソッド) の数が不一致"
  else
    pass "JobQueue trait/impl メソッド数一致 ($trait_methods)"
  fi

  # CC-2: Box::pin 適用率
  echo ""
  echo "── CC-2: Box::pin 適用率 ──"
  local boxpin_count
  boxpin_count=$(grep -c 'Box::pin' libs/infrastructure/src/job_queue/mod.rs 2>/dev/null | tr -d '\n ' || echo "0")
  local direct_do_count
  direct_do_count=$(grep -cE '^\s+self\.do_[a-z_]+\(' libs/infrastructure/src/job_queue/mod.rs 2>/dev/null | tr -d '\n ' || echo "0")
  if [[ "$direct_do_count" -gt 0 ]]; then
    err "impl JobQueue に Box::pin なしの直接 do_* 呼び出しが $direct_do_count 箇所"
  else
    pass "全 do_* 呼び出しが Box::pin 済み ($boxpin_count 箇所)"
  fi

  # CC-3: Error 型の統一性
  echo ""
  echo "── CC-3: Error 型の統一性 ──"
  local error_types
  error_types=$(find libs/ apps/ -name "*.rs" -not -path "*/target/*" \
    -exec grep -ohE 'Result<[^,]+,[[:space:]]*[A-Za-z]+Error>' {} \; 2>/dev/null \
    | sort -u | head -10 || true)
  local error_type_count
  error_type_count=$(echo "$error_types" | grep -c 'Error>' 2>/dev/null || echo "0")
  if [[ "$error_type_count" -gt 3 ]]; then
    warn "Error 型が $error_type_count 種類存在 — 統一を検討"
  else
    pass "Error 型は $error_type_count 種類（許容範囲）"
  fi

  # CC-4: 環境変数の一貫性（.env.example vs コード内参照）
  echo ""
  echo "── CC-4: 環境変数の一貫性 ──"
  local env_in_code
  env_in_code=$(find libs/ apps/ -name "*.rs" -not -path "*/target/*" \
    -exec grep -ohE 'env::var\("([A-Z_]+)"\)' {} \; 2>/dev/null \
    | sort -u | sed 's/env::var("//;s/")//' || true)
  local missing_env=""
  while IFS= read -r var; do
    [[ -z "$var" ]] && continue
    if ! grep -q "$var" .env.example 2>/dev/null; then
      missing_env="$missing_env $var"
    fi
  done <<< "$env_in_code"
  if [[ -n "${missing_env// /}" ]]; then
    warn ".env.example に未記載の環境変数:$missing_env"
  else
    pass "全環境変数が .env.example に記載済み"
  fi

  # CC-5: MockJobQueue の完全性
  echo ""
  echo "── CC-5: MockJobQueue 完全性 ──"
  local mock_fn_count
  mock_fn_count=$(find libs/ -name "*.rs" -not -path "*/target/*" \
    -exec grep -l 'impl JobQueue for Mock' {} \; 2>/dev/null | head -1)
  if [[ -n "$mock_fn_count" ]]; then
    local mock_methods
    mock_methods=$(grep -c 'async fn ' "$mock_fn_count" 2>/dev/null | tr -d '\n ' || echo "0")
    if [[ "$mock_methods" -lt "$trait_methods" ]]; then
      err "MockJobQueue ($mock_methods メソッド) が JobQueue trait ($trait_methods メソッド) より少ない — 未実装メソッドあり"
    else
      pass "MockJobQueue メソッド数一致 ($mock_methods/$trait_methods)"
    fi
  fi

  # CC-6: Type-Driven Security (Auth Extractor Enforcement)
  echo ""
  echo "── CC-6: Type-Driven Security (Auth Extractor Enforcement) ──"
  local total_handlers auth_handlers missing_auth_count
  total_handlers=$(find apps/api-server/src/routes apps/api-server/src/stream.rs -name "*.rs" -exec awk '
    /auth-exempt/ { exempt_seen=1 }
    /pub async fn/ {
      if (!exempt_seen) c++
      exempt_seen=0
    }
    END { print c+0 }
  ' {} + | awk '{s+=$1} END {print s+0}' 2>/dev/null || echo "0")
  auth_handlers=$(find apps/api-server/src/routes apps/api-server/src/stream.rs -name "*.rs" -exec awk '/(auth|_auth): .*Authenticated|Extension.*AuthenticatedUser/{c++} END{print c+0}' {} + | awk '{s+=$1} END {print s+0}' 2>/dev/null || echo "0")
  
  if [[ "$total_handlers" -gt 0 ]]; then
    missing_auth_count=$((total_handlers - auth_handlers))
    if [[ "$missing_auth_count" -gt 0 ]]; then
      err "Type-Driven Security 違反: $missing_auth_count 個のAPIハンドラで (auth|_auth): Authenticated が未定義 ($auth_handlers/$total_handlers 保護済み)"
    else
      pass "Type-Driven Security: $auth_handlers/$total_handlers のハンドラで認証型制約を確認完了"
    fi
  else
    pass "Type-Driven Security: 検査対象なし"
  fi
}

# ──────────────────────────────────────
# Main
# ──────────────────────────────────────
main() {
  echo ""
  echo "╔══════════════════════════════════════════╗"
  echo "║   🔎 Aiome Segmented Deep Scanner       ║"
  echo "╚══════════════════════════════════════════╝"

  echo ""
  echo "Phase 1: セグメント個別スキャン"
  echo "═══════════════════════════════"

  if [[ -n "$TARGET_CRATE" ]]; then
    # 単一クレートスキャン
    local found=false
    for crate in "${CRATES[@]}"; do
      if [[ "$crate" == *"$TARGET_CRATE"* ]]; then
        scan_crate "$crate"
        found=true
      fi
    done
    if [[ "$found" == false ]]; then
      echo "❌ Crate '$TARGET_CRATE' not found"
      exit 1
    fi
  else
    # 全クレートスキャン
    for crate in "${CRATES[@]}"; do
      scan_crate "$crate"
    done
  fi

  cross_cutting_checks

  echo ""
  echo "══════════════════════════════════════════"
  echo "📊 ディープスキャン完了"
  echo "   🔴 Errors:   $ERROR_COUNT"
  echo "   ⚠️  Warnings: $WARN_COUNT"
  echo "══════════════════════════════════════════"

  if [[ $ERROR_COUNT -gt 0 && "$CI_MODE" == true ]]; then
    exit 1
  fi
}

main "$@"
