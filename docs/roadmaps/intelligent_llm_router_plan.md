# Intelligent LLM Router 実装計画（v1.3 — 第3次実コード検証）

> **作成**: 2026-08-01  
> **改訂**: 2026-08-01 v1.3（stream 課金 name 問題 / OpenAPI 手順明記）  
> **根拠**: v1.0 plan × 実コード突合（3 パス）  
> **ステータス**: **Phase 0–4 実装済**（既定 `LLM_ROUTE_MODE=legacy`・`/reflexion` rules モードは別途 Human 検証）  
> **2026-08-04**: code-review 修正でキャッシュ配置・キー・pin_local 等を上書き → 正本は [`op099_review_fix_plan.md`](op099_review_fix_plan.md) / ADR-058  
> **前身 ADR**: [010-resilient-llm-routing.md](../decisions/010-resilient-llm-routing.md)（可用性）  
> **非目標**: RouteLLM ML、Soul ポリシー、LB、HierarchicalRouter 転用、Safety-Critical 変更

---

## 0. 30 秒サマリ

| 項目 | v1.2 判定 |
|------|-----------|
| 方向性 | **妥当** — ADR-010 の「Dynamic Re-weighting」を実装する正しい場所 |
| 最大の再開発リスク | **`LlmRouteTier` 新設** → 既存 `TaskTier` を使う |
| 最大の抜け | **`FallbackRouter` に `complete_with_cache` 無し**（format ルールが死ぬ） |
| bootstrap 修正 | `standard_chain` は **KeyProxy 判定後**に `core_services` で組立。`ProviderResult` に載せない |
| ロールアウト | `LLM_ROUTE_MODE=legacy` 既定（行動変化ゼロ） |

---

## 1. 実コード正本

### 1.1 チャット経路（唯一の Phase 1 変更対象）

[`core_services.rs`](../../apps/api-server/src/bootstrap/core_services.rs) L325–385:

```text
primary       = ProxyLlm（KeyProxy ping OK）| DynamicLlmProvider（provider クローン）
standard_base = FallbackRouter(primary, bg_provider)
entropy       = EntropyGate(standard_base)
router_provider = HumanizerFilter(entropy)  ← AppState.provider / stream.rs
```

- [`stream.rs`](../../apps/api-server/src/stream.rs) L228: **`stream_complete`** が本線（complete ではない）
- [`state_assembly.rs`](../../apps/api-server/src/bootstrap/state_assembly.rs) L54: `provider = router_provider`

### 1.2 Fast 経路（Phase 1 非変更・参照のみ）

[`llm_providers.rs`](../../apps/api-server/src/bootstrap/llm_providers.rs):

```text
local_ollama → FallbackRouter(local, bg) [local_fallback_policy]
            → SemaphoreGuardedProvider (= fast_provider)
```

12+ コンポーネント（Oracle, ContextEngine 等）が `fast_provider` を使用 — **触らない**。

### 1.3 既存資産マップ（再利用 / 流用禁止）

| 資産 | パス | v1.2 方針 |
|------|------|-----------|
| FallbackRouter | `llm/fallback_router.rs` | 内側チェーン。**`complete_with_cache` 追加**（穴埋め） |
| CostCircuitBreaker | `llm/cost_breaker.rs` | `check_state()?.is_tripped` 既存 |
| model_pricing | `llm/cost.rs` | tier 判定の**唯一の価格正本** |
| **TaskTier** | `aiome-contracts/task_tier.rs` | **`Fast`/`Smart` をルート tier として再利用**（未配線） |
| EvaluationLogger | `llm/evaluation_logger.rs` | route_* 列追加 |
| SemanticCache | `llm/semantic_cache.rs` | 本番 DI 0。Phase 4 で薄ラップ |
| EntropyGate | `llm/entropy_gate.rs` | `complete_with_cache` 実装済。IR の**外側** |
| ADR-021 | `docs/decisions/021-*.md` | provider-side cache。SemanticCache と役割分離 |

| 流用禁止 | 理由 |
|----------|------|
| HierarchicalRouter | 知識ツリー探索 |
| tool_call_router.rs | セキュリティ / 課金 Fail-Closed |
| SoT `select_protocol` | 熟議プロトコル選定。チャット tier とは別 |

---

## 2. v1.1 → v1.2 追加修正（第2次検証）

### 2.1 新規抜け

| ID | 抜け | 根拠 | 修正 |
|----|------|------|------|
| G-7 | **`TaskTier` 二重定義** | `libs/aiome-contracts/src/task_tier.rs` に `Fast`/`Smart` 既存。CHANGELOG で fast_provider 意図と一致 | **`LlmRouteTier` 新設禁止**。`LlmRouteDecision { tier: TaskTier, ... }` を使用 |
| G-8 | **`FallbackRouter` が format を落とす** | FR に `complete_with_cache` 無し → trait デフォルトが `complete()` へ降格 | Phase 1: FR に **`complete_with_cache` 明示実装**（primary→fallback 委譲） |
| G-9 | **`standard_chain` を ProviderResult に載せられない** | `primary` は `core_services` の KeyProxy ping **後**に確定 | `ProviderResult` は **`local_provider` のみ追加**。cheap/standard 組立は core_services |
| G-10 | **local Ollama 競合** | cheap チェーンと `fast_provider` が同一 local を叩く。セマフォは fast のみ | Phase 1 は**許容**し ADR-058 に記載。将来 `SharedLocalLlmSemaphore` を検討（Phase 5） |
| G-11 | **Humanizer が `complete_with_cache` 未実装** | EG→IR 経路で HG は complete_with_cache を通さないが、将来整合のため | Phase 1 範囲外。EG が IR に complete_with_cache するため HG は透過 delegate 追加を **任意** |
| G-12 | **stream 課金がデコレータ名を参照** | `stream.rs` L441: `provider.name()` → 外側は `"HumanizerFilter"`。local fallback 時も ollama 判定不能 | Phase 1 **非変更**（commerce 触らない）。IR は応答 `metadata` に `resolved_provider` / `resolved_model` を注入し Phase 2 eval と将来課金修正の正本にする |
| G-13 | **ProxyLlm は真の stream 非対応** | `proxy.rs` に `stream_complete` 無し → trait デフォルト（complete 1 チャンク） | Smart chain stream は現状どおり動作。計画の「stream→Smart 固定」と整合。KeyProxy 本番では degraded stream 許容 |

### 2.2 再開発排除（確定）

| やってはいけないこと | 代わりに |
|---------------------|----------|
| `LlmRouteTier { Cheap, Standard }` 新設 | 既存 `TaskTier::Fast` → cheap chain、`TaskTier::Smart` → standard chain |
| `model_catalog.rs` | `cost.rs` に `task_tier_for_model()` |
| `CostCircuitBreaker::is_tripped()` | `check_state().await?.is_tripped` |
| core_services で BackgroundLlm 再 new | `llm_providers::build_local_provider()` 1 関数 |
| SemanticCache ロジック複製 | `CachingLlmProvider` 薄ラップのみ |
| prompt hash 3 箇所 | `llm/utils.rs::compute_prompt_hash()` 共通化 |

---

## 3. 目標アーキテクチャ

```text
HumanizerFilter
  → [CachingLlmProvider if rules]   ← 2026-08-04: EG 外側・channel_id スコープ必須
  → EntropyGate
  → IntelligentRouter
       ├─ TaskTier::Fast  → local_provider または FallbackRouter(local, bg)  ※LocalOnly 尊重
       └─ TaskTier::Smart → FallbackRouter(primary, bg)   ← KeyProxy 後 primary
```

> 旧図 `HF→EG→Caching→IR` は obsolete（[`op099_review_fix_plan.md`](op099_review_fix_plan.md) FIX-6）。

**固定ルール**

- `stream_complete` → **常に Smart chain**（BackgroundLlm は stream 未実装）
- `legacy` モード → 常に Smart chain（v1.0 挙動とビット一致）
- tier 選定 → **同期 I/O なし**（ルールのみ Phase 1–3）
- EntropyGate リトライ → **初回 tier を sticky**（metadata 固定）

---

## 4. フェーズ計画

### Phase 0 — ADR + 契約 + 設定

- ADR-058: IR と FR の責務、TaskTier マッピング、非目標
- `aiome-contracts/llm.rs`: `LlmRouteDecision { tier: TaskTier, reason_code, reason_detail }`
- metadata 定数: `route_tier`, `route_reason`, `route_mode`（`task_tier.rs` ではなく llm 側にキー定数）
- `AiomeConfig` + `.env.example`:
  - `LLM_ROUTE_MODE=legacy|rules`（既定 **`legacy`**）
  - `LLM_ROUTE_BUDGET_DEGRADE=true|false`（rules 時、既定 `true`）
  - `LLM_ROUTE_SHORT_PROMPT_CHARS=512`

**DoD**: `cargo check -p aiome-contracts -p shared`

---

### Phase 1 — ルール + 配線 + FR 穴埋め

**新規**

- `llm/route_rules.rs` — `decide_route(prompt, format, metadata, config) -> LlmRouteDecision`
- `llm/intelligent_router.rs` — sticky tier / stream→Smart / legacy→Smart

**既存拡張**

- `llm/fallback_router.rs` — **`complete_with_cache` 追加**（G-8）
- `llm/cost.rs` — `task_tier_for_model(model: &str) -> TaskTier`
- `llm_providers.rs` — `build_local_provider()` + `ProviderResult.local_provider`
- `core_services.rs` — KeyProxy 後に cheap/standard FR を組み、IR 挿入

**ルール優先順位**

1. `metadata["route_tier"]` → `TaskTier` parse
2. `format == "json"` → Smart
3. キーワード（security / audit / reasoning / 要約）→ Smart
4. len ≤ `LLM_ROUTE_SHORT_PROMPT_CHARS` → Fast
5. default → Smart

**DoD**

- route_rules / IR 単体 P/N
- FR `complete_with_cache` が format を primary へ伝播
- `LLM_ROUTE_MODE=legacy` で既存 Fallback / chat 回帰 PASS
- stream が Fast chain に落ちない Negative テスト

---

### Phase 2 — 観測

- migration: `prompt_evaluation_log` + `route_tier`, `route_reason`, `route_mode`
- IR が delegate 前に metadata 注入（`route_*` + **`resolved_provider` / `resolved_model`**）→ `log_evaluation` 拡張
- `ProviderEvalStat` + `cheap_ratio`
- OpenAPI 再生成: `cargo test -p api-server test_openapi_schema_generation` → `(cd apps/management-console && npm run generate-types)`
- `PromptStatsView.tsx` 最小拡張

---

### Phase 3 — 予算降格

- `degrade_recommended()` = tripped && `LLM_ROUTE_BUDGET_DEGRADE`
- `LocalCostBypassProvider` — local path の `enforce` スキップ（~30 行）
- IR: degrade 時 Fast 強制 + `reason_code=budget_degrade`

---

### Phase 4 — SemanticCache DI

- `llm/utils.rs::compute_prompt_hash()` — eval / semantic_cache 共通化
- `llm/caching_provider.rs` — SemanticCache 薄ラップ
- 配置: ~~EG 内側、IR 外側~~ → **2026-08-04**: EG **外側**・rules 限定（[`op099_review_fix_plan.md`](op099_review_fix_plan.md) FIX-6）
- `stream_complete`: 非キャッシュ

---

### Phase 5 — 将来（本計画外）

Preference Data、embed tier、Soul/Playbook、LB、`SharedLocalLlmSemaphore`、RLM 統合

---

## 5. bootstrap 責務（再確認）

| 処理 | 担当 | 理由 |
|------|------|------|
| `local_provider` 構築 | `llm_providers.rs` | fast と共有 |
| KeyProxy → `primary` 確定 | `core_services.rs` | ping 結果依存 |
| `standard_chain = FR(primary, bg)` | `core_services.rs` | primary 確定後 |
| `cheap_chain = FR(local, bg)` | `core_services.rs` | local は ProviderResult から |
| IR + EG + Humanizer ラップ | `core_services.rs` | 現行と同位置 |

---

## 6. 変更ファイル（v1.2）

| Phase | ファイル |
|-------|----------|
| 0 | ADR-058, `llm.rs`, `config.rs`, `.env.example` |
| 1 | `route_rules.rs`, `intelligent_router.rs`, **`fallback_router.rs`**, `cost.rs`, `llm_providers.rs`, `core_services.rs`, `mod.rs` |
| 2 | migrations, `evaluation_logger.rs`, `api.rs`, `PromptStatsView.tsx` |
| 3 | `cost_breaker.rs`, `local_cost_bypass.rs`, `intelligent_router.rs` |
| 4 | `caching_provider.rs`, `utils.rs`, `semantic_cache.rs`, bootstrap |
| 文書 | CHANGELOG, RIPPLE_MAP, OPEN OP-099, `LLM_PROVIDER_ARCHITECTURE.md` §3.15 |

---

## 7. /reflexion 検証

1. **Positive** (`rules`): 短文→Fast ログ、JSON→Smart ログ
2. **Negative** (`legacy`): route_* NULL、stream≠Fast
3. **Revert**: `legacy` 回帰 PASS

---

## 8. 成功定義

- [ ] `TaskTier` 再利用（新 tier enum なし）
- [ ] FR `complete_with_cache` で format ルールが機能
- [ ] `legacy` = 現行と同一
- [ ] rules で complete コスト削減が stats 可視
- [ ] 価格表 / hash / local provider の単一正本

---

*v1.3 — 第3次実コード検証 2026-08-01*
