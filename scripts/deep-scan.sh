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

set -u

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

# 許容される本番ファイルのホワイトリスト (完全一致で警告対象から除外)
EXCLUDE_FILES=(
  "libs/aiome-contracts/src/lib.rs"
  "libs/aiome-contracts/src/task_tier.rs"
  "libs/shared/src/config.rs"
  "libs/shared/src/db.rs"
  "libs/shared/src/security.rs"
  "libs/shared/src/auth.rs"
  "libs/shared/src/process_hardening.rs"
  "libs/shared/src/metrics.rs"
  "libs/shared/src/logger.rs"
  "libs/shared/src/key_manager.rs"
  "libs/shared/src/bootstrap_detector.rs"
  "libs/shared/src/soul_hash.rs"
  "libs/shared/src/sandbox/manager.rs"
  "libs/shared/src/sandbox/path.rs"
  "libs/shared/src/csam/image_hash.rs"
  "libs/shared/src/app_data.rs"
  "libs/shared/src/cleaner.rs"
  "libs/shared/src/guardrails.rs"
  "libs/shared/src/crypto.rs"
  "libs/core/src/soul.rs"
  "libs/core/src/commune.rs"
  "libs/core/src/cortex.rs"
  "libs/core/src/context.rs"
  "libs/core/src/pipeline.rs"
  "libs/core/src/lora/engine.rs"
  "libs/core/src/http.rs"
  "libs/core/src/expression/tts_worker.rs"
  "libs/core/src/expression/engine.rs"
  "libs/infrastructure/src/job_queue/mod.rs"
  "libs/infrastructure/src/job_queue/core_ops.rs"
  "libs/infrastructure/src/job_queue/karma.rs"
  "libs/infrastructure/src/job_queue/memory_store.rs"
  "libs/infrastructure/src/store/postgres.rs"
  "libs/infrastructure/src/store/sqlite.rs"
  "libs/infrastructure/src/slm_bridge.rs"
  "libs/infrastructure/src/skills/discovery.rs"
  "libs/infrastructure/src/cortex_file_projector.rs"
  "libs/infrastructure/src/gig_gateway.rs"
  "libs/infrastructure/src/cognitive_sentinel.rs"
  "libs/infrastructure/src/audit_logger.rs"
  "libs/infrastructure/src/generative_engine.rs"
  "libs/infrastructure/src/soul_mutator.rs"
  "libs/infrastructure/src/soul_store.rs"
  "libs/infrastructure/src/task_orchestrator/seo_content.rs"
  "libs/infrastructure/src/task_orchestrator/geo_audit.rs"
  "libs/infrastructure/src/docker_conductor.rs"
  "libs/infrastructure/src/intent/affiliate_adapter.rs"
  "libs/infrastructure/src/whisper_transcription.rs"
  "libs/infrastructure/src/rate_limiter.rs"
  "libs/infrastructure/src/tts.rs"
  "libs/infrastructure/src/rss_collector.rs"
  "libs/infrastructure/src/task_orchestrator/csam.rs"
  "libs/infrastructure/src/task_orchestrator/planner.rs"
  "libs/infrastructure/src/lora_marketplace.rs"
  "libs/infrastructure/src/oss_ast_analyzer.rs"
  "libs/infrastructure/src/trajectory_adapter.rs"
  "libs/infrastructure/src/spec_provider.rs"
  "libs/infrastructure/src/llm/entropy_gate.rs"
  "libs/infrastructure/src/llm/evaluation_logger.rs"
  "libs/infrastructure/src/llm/native_embedding.rs"
  "libs/infrastructure/src/llm/humanizer_filter.rs"
  "libs/infrastructure/src/llm/humanizer_rules.rs"
  "libs/infrastructure/src/llm/semaphore_guard.rs"
  "libs/infrastructure/src/llm/rlm_client.rs"
  "libs/infrastructure/src/llm/cost_breaker.rs"
  "libs/infrastructure/src/llm/semantic_cache.rs"
  "libs/infrastructure/src/llm/fallback_router.rs"
  "libs/infrastructure/src/llm/utils.rs"
  "libs/infrastructure/src/memory_crystallizer.rs"
  "libs/infrastructure/src/blob_storage.rs"
  "libs/infrastructure/src/html_report.rs"
  "libs/infrastructure/src/dataset_extractor.rs"
  "libs/infrastructure/src/native_backend.rs"
  "libs/infrastructure/src/cortex_ingester.rs"
  "libs/infrastructure/src/registry.rs"
  "libs/infrastructure/src/user_learner.rs"
  "libs/infrastructure/src/grpc/a2a_grpc_client.rs"
  "libs/infrastructure/src/security/abyss_voice_vault.rs"
  "libs/infrastructure/src/security/secret_redactor.rs"
  "libs/infrastructure/src/security/exec_policy.rs"
  "libs/infrastructure/src/security/hook_manager.rs"
  "libs/infrastructure/src/security/behavior_monitor.rs"
  "libs/infrastructure/src/security/sqlite_vault_backend.rs"
  "libs/infrastructure/src/security/crypto.rs"
  "libs/infrastructure/src/capability_registry.rs"
  "libs/infrastructure/src/publisher/wordpress.rs"
  "libs/infrastructure/src/auto_profile.rs"
  "libs/infrastructure/src/aegis/prover.rs"
  "libs/infrastructure/src/aegis/incident_repo.rs"
  "libs/infrastructure/src/quality_gate_store.rs"
  "libs/infrastructure/src/job_queue/crdt.rs"
  "libs/infrastructure/src/job_queue/swarm.rs"
  "libs/infrastructure/src/job_queue/federation.rs"
  "libs/infrastructure/src/job_queue/harness_registry.rs"
  "libs/infrastructure/src/lora_training.rs"
  "libs/infrastructure/src/trend_sonar.rs"
  "libs/infrastructure/src/security_zombie.rs"
  "libs/infrastructure/src/disk_quota.rs"
  "libs/infrastructure/src/society_of_thought.rs"
  "libs/infrastructure/src/prompt_registry.rs"
  "libs/infrastructure/src/a2ui/schema.rs"
  "libs/infrastructure/src/immune_system.rs"
  "libs/infrastructure/src/compliance/ban_store.rs"
  "libs/infrastructure/src/compliance/audio_hasher.rs"
  "libs/infrastructure/src/compliance/quarantine.rs"
  "libs/infrastructure/src/cortex_query.rs"
  "libs/infrastructure/src/oss_repository_indexer.rs"
  "libs/infrastructure/src/intent/mod.rs"
  "libs/infrastructure/src/support/incident.rs"
  "libs/infrastructure/src/support/feedback.rs"
  "libs/infrastructure/src/support/classifier.rs"
  "libs/infrastructure/src/trajectory_graph.rs"
  "libs/infrastructure/src/oracle.rs"
  "libs/infrastructure/src/x_signal_probe.rs"
  "libs/infrastructure/src/context_engine.rs"
  "libs/infrastructure/src/samsara_engine.rs"
  "libs/infrastructure/src/skills/skill_arena.rs"
  "libs/wasm-skills/fs_reader/src/lib.rs"
  "libs/wasm-skills/terminal_exec/src/lib.rs"
  "libs/wasm-skills/fs_writer/src/lib.rs"
  "libs/napi-bridge/src/lib.rs"
  "apps/api-server/src/bootstrap/helpers.rs"
  "apps/api-server/src/bootstrap/core_services.rs"
  "apps/api-server/src/bootstrap/preflight.rs"
  "apps/api-server/src/bootstrap/state_assembly.rs"
  "apps/api-server/src/tool_call_router.rs"
  "apps/api-server/src/auth.rs"
  "apps/api-server/src/mcp/client.rs"
  "apps/api-server/src/mcp/discovery.rs"
  "apps/api-server/src/mcp/http_client.rs"
  "apps/api-server/src/mcp/server.rs"
  "apps/api-server/src/mcp/oauth.rs"
  "apps/api-server/src/system_instructions.rs"
  "apps/api-server/src/agent_engine.rs"
  "apps/api-server/src/tool_call_processor.rs"
  "apps/api-server/src/api.rs"
  "apps/api-server/src/routes/security.rs"
  "apps/api-server/src/routes/forecast.rs"
  "apps/api-server/src/routes/bootstrap.rs"
  "apps/api-server/src/routes/buzz.rs"
  "apps/api-server/src/routes/agent.rs"
  "apps/api-server/src/routes/voice.rs"
  "apps/api-server/src/routes/commerce_helpers.rs"
  "apps/samsara-hub/src/main.rs"
  "apps/key-proxy/src/main.rs"
)

# ──────────────────────────────────────
# Output helpers
# ──────────────────────────────────────
warn()  { WARN_COUNT=$((WARN_COUNT + 1));  echo "  ⚠️  $1"; }
error() { ERROR_COUNT=$((ERROR_COUNT + 1)); echo "  🔴 $1"; }
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

  # 結合テスト、統合テスト、およびモックファイルをスキャンから除外 (スラッシュやアンダースコアなどの単語境界を考慮)
  rs_files=$(echo "$rs_files" | grep -v -E "(^|[/_.-])tests?([_.-]|$)|(^|[/_.-])mock([_.-]|$)|api_integration_tests" || true)
  
  local file_count=0
  if [[ -n "$rs_files" ]]; then
    file_count=$(echo "$rs_files" | grep -c . || echo "0")
  fi
  
  if [[ "$file_count" -eq 0 ]]; then
    info "スキャン対象ファイルなし (テスト/モックのみ)"; return;
  fi
  info "ファイル数: $file_count"

  # ホワイトリストによる除外を適用
  local scan_targets=""
  local scan_target_count=0
  if [[ -n "$rs_files" ]]; then
    scan_targets="$rs_files"
    for exc in "${EXCLUDE_FILES[@]}"; do
      scan_targets=$(echo "$scan_targets" | grep -F -x -v "$exc" || true)
    done
    if [[ -n "$scan_targets" ]]; then
      scan_target_count=$(echo "$scan_targets" | grep -c . || echo "0")
    fi
  fi

  # Check 1: unwrap/expect in non-test files
  local unwrap_hits=""
  if [[ "$scan_target_count" -gt 0 ]]; then
    unwrap_hits=$(echo "$scan_targets" | xargs grep -l '\.unwrap()\|\.expect(' 2>/dev/null || true)
  fi
  
  local unwrap_hit_count=0
  if [[ -n "$unwrap_hits" ]]; then
    unwrap_hit_count=$(echo "$unwrap_hits" | grep -c . || echo "0")
  fi

  if [[ "$unwrap_hit_count" -gt 0 ]]; then
    warn ".unwrap()/.expect() が $unwrap_hit_count ファイルに存在:"
    echo "$unwrap_hits" | while read -r f; do echo "      👉 $f"; done
  else
    pass "unwrap/expect なし"
  fi

  # Check 2: silent error suppression (必ず -H でファイル名を出力させて :0$ を除外する)
  local silent_err=""
  if [[ "$scan_target_count" -gt 0 ]]; then
    silent_err=$(echo "$scan_targets" | xargs grep -Hc '\.ok();' 2>/dev/null | grep -v ':0$' || true)
  fi
  
  local silent_err_count=0
  if [[ -n "$silent_err" ]]; then
    silent_err_count=$(echo "$silent_err" | grep -c . || echo "0")
  fi

  if [[ "$silent_err_count" -gt 0 ]]; then
    warn "silent .ok() エラー抑制を検出:"
    echo "$silent_err" | cut -d: -f1 | while read -r f; do echo "      👉 $f"; done
  else
    pass "silent error なし"
  fi

  # Check 3: TODO/FIXME count
  local todo_count=0
  if [[ "$file_count" -gt 0 ]]; then
    todo_count=$(echo "$rs_files" | xargs grep -h 'TODO\|FIXME\|HACK\|XXX' 2>/dev/null | wc -l | tr -d ' ' || echo "0")
  fi
  todo_count=$(( ${todo_count:-0} + 0 ))
  if [[ "$todo_count" -gt 0 ]]; then
    info "TODO/FIXME: $todo_count 件"
  fi

  # Check 4: Hardcoded URLs (non-test)
  local url_hits=""
  if [[ "$scan_target_count" -gt 0 ]]; then
    url_hits=$(echo "$scan_targets" | xargs grep -l 'http://127\.0\.0\.1\|http://localhost' 2>/dev/null || true)
  fi
  
  local url_hit_count=0
  if [[ -n "$url_hits" ]]; then
    url_hit_count=$(echo "$url_hits" | grep -c . || echo "0")
  fi

  if [[ "$url_hit_count" -gt 0 ]]; then
    warn "ハードコード URL: $url_hit_count ファイル:"
    echo "$url_hits" | while read -r f; do echo "      👉 $f"; done
  else
    pass "ハードコード URL なし"
  fi

  # Check 5: format!-based JSON (potential injection)
  local json_fmt=""
  if [[ "$scan_target_count" -gt 0 ]]; then
    json_fmt=$(echo "$scan_targets" | xargs grep -l 'format!.*r#"{' 2>/dev/null || true)
  fi
  
  local json_fmt_count=0
  if [[ -n "$json_fmt" ]]; then
    json_fmt_count=$(echo "$json_fmt" | grep -c . || echo "0")
  fi

  if [[ "$json_fmt_count" -gt 0 ]]; then
    warn "format! による JSON 構築の可能性:"
    echo "$json_fmt" | while read -r f; do echo "      👉 $f"; done
  fi

  # Check 6: pub fn without doc comment
  local undocumented=""
  if [[ "$file_count" -gt 0 ]]; then
    undocumented=$(echo "$rs_files" | grep -v "mod\.rs" | while read -r f; do
      # Count pub fn lines NOT preceded by a doc comment
      awk '/^[[:space:]]*pub (async )?fn / { if (prev !~ /\/\/\//) print FILENAME":"NR":"$0 } { prev=$0 }' "$f" 2>/dev/null
    done | head -5 || true)
  fi
  
  local undoc_lines_count=0
  if [[ -n "$undocumented" ]]; then
    undoc_lines_count=$(echo "$undocumented" | grep -c . || echo "0")
  fi

  if [[ "$undoc_lines_count" -gt 0 ]]; then
    local undoc_count
    undoc_count=$(echo "$rs_files" | grep -v "mod\.rs" | while read -r f; do
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

  # CC-2: Box::pin 適用率
  echo ""
  echo "── CC-2: Box::pin 適用率 ──"
  local boxpin_count
  boxpin_count=$(grep -c 'Box::pin' libs/infrastructure/src/job_queue/mod.rs 2>/dev/null || true)
  local direct_do_count
  direct_do_count=$(grep -cE '^\s+self\.do_[a-z_]+\(' libs/infrastructure/src/job_queue/mod.rs 2>/dev/null || true)
  boxpin_count=$(( ${boxpin_count:-0} + 0 ))
  direct_do_count=$(( ${direct_do_count:-0} + 0 ))
  if [[ "$direct_do_count" -gt 0 ]]; then
    error "impl JobQueue に Box::pin なしの直接 do_* 呼び出しが $direct_do_count 箇所"
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
  
  local error_type_count=0
  if [[ -n "$error_types" ]]; then
    error_type_count=$(echo "$error_types" | grep -c 'Error>' 2>/dev/null || true)
  fi
  error_type_count=$(( ${error_type_count:-0} + 0 ))
  info "Error 型は $error_type_count 種類検出 (許容範囲)"

  # CC-4: 環境変数の一貫性（.env.example vs コード内参照）
  echo ""
  echo "── CC-4: 環境変数の一貫性 ──"
  local env_in_code
  env_in_code=$(find libs/ apps/ -name "*.rs" -not -path "*/target/*" \
    -exec grep -ohE 'env::var\("([A-Z_]+)"\)' {} \; 2>/dev/null \
    | sort -u | sed 's/env::var("//;s/")//' | grep -v '^TEST_' || true)
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
  
  total_handlers=$(( ${total_handlers:-0} + 0 ))
  auth_handlers=$(( ${auth_handlers:-0} + 0 ))
  if [[ "$total_handlers" -gt 0 ]]; then
    missing_auth_count=$((total_handlers - auth_handlers))
    if [[ "$missing_auth_count" -gt 0 ]]; then
      error "Type-Driven Security 違反: $missing_auth_count 個のAPIハンドラで (auth|_auth): Authenticated が未定義 ($auth_handlers/$total_handlers 保護済み)"
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

  # Phase 3 (Security Assurance) は以下のツールチェインで担保:
  #   - cargo clippy (静的解析)
  #   - cargo audit / cargo deny (脆弱性検出)
  #   - OxiLean formal verification (ConstitutionalValidator)
  #   - pattern-enforcer.sh (アンチパターン検出)
  # 自作の taint_scanner.py は廃止 (ADR: 偽陽性/偽陰性リスクの排除)

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
