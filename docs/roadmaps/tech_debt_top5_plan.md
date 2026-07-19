# TECH_DEBT Top 5 実装計画（v1.3）

> **作成**: 2026-07-10（v1.0）  
> **改訂**: v1.1 → v1.2 → **v1.3**（`/perfect-plan` 再検証 — 実装時コンパイル落とし穴・OPEN ドリフト・ADR-031 重複を固定）  
> **根拠**: [`TECH_DEBT_AUDIT.md`](../../TECH_DEBT_AUDIT.md) v11.0  
> **タスク正本**: [`OPEN.md`](../../OPEN.md)  

> **関連**: [`remaining_tasks_implementation_plan.md`](remaining_tasks_implementation_plan.md) v6、[`near_term_public_beta_plan.md`](near_term_public_beta_plan.md) v5.1、[`031-jobqueue-isp-deferral.md`](../decisions/031-jobqueue-isp-deferral.md)  
> **ステータス**: **実装完了**（2026-07-10）・計画判定 ✅ PASS

---

## 0a. v1.2 → v1.3 で潰した抜け・曖昧さ

| # | v1.2 の問題 | 実コード事実 | v1.3 の固定 |
|---|-------------|--------------|------------|
| 1 | stream の `provider` を「実装時に確認」 | `stream.rs:251` で `provider.stream_complete`、`:464` で課金判定。Immune 除去後も **必須** | `let provider = (*state.provider).clone();` は **削除禁止** |
| 2 | agent_engine 置換コードに import なし | 現状 `tool_call_router` 未 use（`agent_engine.rs` 先頭） | `use crate::tool_call_router::{DefaultToolCallRouter, ToolCallRouter};` を必須記載 |
| 3 | OPEN OP-075 が stream/router のみ・degraded 併記 | 計画は agent_engine 必須・degraded 禁止 | 実装 DoD で OPEN 文言同期（agent_engine 追加、degraded 削除） |
| 4 | AppState 具象を「将来」とだけ記載 | **ADR-031** が JobQueue ISP 完遂を Phase 4 まで延期済み | OP-054 で ISP/AppState 抽象化に手を出さない根拠を ADR-031 に固定 |
| 5 | Begging / 出力 Guardrails に触れず | `stream.rs:372` は `event("error")` + SECURITY BLOCK 文言（出力側） | **OP-075 対象外**と明記（入力側 Fail-Closed と混同禁止） |
| 6 | agent_engine Sentinel 文言差分 | 直呼びは短い文面、`evaluate_security` は `Pattern:` 付き | 集約後は **router 文言に統一**（意図的・テストは部分一致） |

## 0. v1.1 → v1.2 で潰した抜け・重複・再発明

| # | v1.1 の問題 | 実コード事実 | v1.2 の固定 |
|---|-------------|--------------|------------|
| 1 | OP-054-B で `Arc<dyn FederationOps>` キャストを「正攻法」 | `main.rs:78-124` は既に `Arc<UniversalJobQueue>` + `use FederationOps` で **trait メソッド呼び出し済み**。キャスト追加は **ゼロ価値の車輪** | **OP-054-B を本計画から削除**（監査ギャップは「AppState が具象固定」であり、bg ループは既に解決済み） |
| 2 | buzz を `Arc<dyn EvaluationOps>` DI と記載 | `buzz.rs:231` は `state.job_queue.get_inner()`。続けて `TaskRegistry::update_job_status`（`:376`）も呼ぶ。Evaluation だけ切ると **二重依存** | **OP-054-B から除外**。必要なら将来「AppState 抽象化」別計画 |
| 3 | stream 初期を tool-loop（L390）と「同型」と記載 | tool-loop は `event("error")`（`:392`）。初期 Guardrails/Sentinel は `event("security_block")`（`:45,:64`） | 初期置換後も **`security_block` を維持**（クライアント契約）。tool-loop の `error` は触らない |
| 4 | agent_engine は Immune Err 分岐のみ修正 | Guardrails + Immune 直呼びが `evaluate_security` と**三重** | **`evaluate_security` 呼び出しに置換**（Guardrails 直呼び削除）。Sentinel 時は `Ok(block_msg)`（`Err` にしない） |
| 5 | `sql_exec!` を `infrastructure::` のみ記載 | マクロは `libs/shared/src/db.rs:399` と `libs/infrastructure/src/db.rs:17`。OP-024 テストは infrastructure 経由 | **OP-024 テストと同パス**を正とする |
| 6 | N2 を「security_block のみ」と曖昧 | tool-loop 失敗は `error` イベント | N2 は **初期経路**のみ。イベント名 `security_block` を明示 |
| 7 | OP-054 完了条件が A+B | remaining_tasks §7 は可視性のみ | **OP-054 = A のみでクローズ可**。監査 P2 の「契約ギャップ」は TECH_DEBT に「AppState 具象・将来」と降格記載 |

### 禁止リスト（v1.3）

| 禁止 | 代替 |
|------|------|
| `Arc<dyn FederationOps>` キャストの新規追加 | 現状維持（既に trait 経由） |
| buzz の EvaluationOps 単独 DI | 現状維持（具象 + 複数トレイト） |
| FederationRegistry へ push/sync / JobQueue ISP 完遂 | **ADR-031** 延期方針に従う |
| AppState → `Arc<dyn JobQueue>` | 別計画（ADR-031 Phase 4 以降） |
| stream の `provider` クローン削除 | LLM/課金で使用中 |
| stream 初期を `event("error")` に合わせる / Begging 経路の改修 | 初期は `security_block`。`:372` は触らない |
| agent_engine で `Err(AppError)` | `Ok(block_msg)` |
| OP-024 再実装 / react-router / Nurture キーリネーム / Upstream bump | — |
| OP-075-B（napi/goal/skill/commercial）同 PR | 別追跡 |
| KC fail-closed / webhook ビジネス変更 | QW-23 は warn のみ |
| `#030712` 用新トークン | `--bg-base` |
| degraded mode フラグ | Fail-Closed 固定（別 ADR） |

---

## 1. 目的と成功基準

| 成功基準 | 検証 |
|----------|------|
| P1 Fail-Closed（ADR-033） | P: SENTINEL。N: `DROP TABLE immune_rules` + benign → 拒否 |
| P2 OP-054 = 可視性のみ完了 | `with_llm` / `get_embedding_provider` が `pub(crate)`。外部呼び出しゼロ確認済み |
| P3 ADR 起草のみ | `docs/decisions/054-error-hierarchy.md`（番号空き） |
| P4 bump なし | OP-030–034 非実装 |
| P5 HEX | entry ゲート GREEN。html 手動 |

---

## 2. スコープ地図

### 2.1 含める

| ID | 内容 |
|----|------|
| **OP-075** | router Fail-Closed + stream 初期を `evaluate_security` + agent_engine を同 API に集約 |
| **OP-054**（旧 A） | `with_llm` / `get_embedding_provider` → `pub(crate)` |
| **OP-051** | ADR-054 起草のみ |
| **OP-068** | 監視のみ |
| **OP-029** | HEX + ゲート |
| **OP-076** | MCP/UI/example キー名 |
| **QW-21..23** | licenses / SSE 残ログ / KC warn |

### 2.2 明示除外（v1.2 で追加）

| 除外 | 理由 |
|------|------|
| **旧 OP-054-B**（Federation/Evaluation DI） | 本番は既に trait 経由 or 具象が正当。キャストは車輪 |
| AppState → `Arc<dyn JobQueue>` | 8h+・別計画 |
| Public Beta NT-* / App.tsx 分割 / OP-075-B / Upstream / Nurture リネーム | 従来どおり |

### 2.3 対応図

```
P1 ──► Wave A: OP-075
P2 ──► Wave B: OP-054（可視性のみ）← 旧 B 削除
P3 ──► Wave C: ADR-054
P4 ──► Wave D: 監視
P5+QW ► Wave E: OP-029 / OP-076 / QW-21..23
```

---

## 3. Wave A — OP-075 Immune Fail-Closed（P1）

### 3.1 アンカー

| 箇所 | 現状 | 方針 |
|------|------|------|
| `tool_call_router.rs:90-96` | fail-open | `return Err(SECURITY BLOCK…)` |
| `stream.rs:43-72` | Guardrails+Immune 直呼び、Immune fail-open | **丸ごと** `evaluate_security` に置換。失敗時 **`event("security_block")`** |
| `stream.rs:390-393` | tool-loop は既に router。イベントは `error` | **変更しない**（初期とイベント名が違うのは意図的差分として文書化） |
| `agent_engine.rs:71-101` | Guardrails+Immune 直呼び、Immune fail-open | **`evaluate_security` に置換**。ブロック時 `Ok(block_msg)` |
| processor / mcp / stream:390 | 経由済み | 自動解消 |
| `immune_system.rs:226-247` | baseline → DB | 変更なし |

### 3.2 再利用

1. OP-024 `match Err`（`tool_call_router.rs:301-316`）
2. N 注入: OP-024 テストと同型 `infrastructure::sql_exec!(&state.job_queue.pool, "DROP TABLE immune_rules")`（`tool_call_router.rs:745+`）
3. `DefaultToolCallRouter` / `setup_mock_state`（同ファイル）

### 3.3 本番差分（確定形）

**evaluate_security Err:**

```rust
Err(e) => {
    tracing::error!(
        error = %e,
        "[Security] immune verify_intent failed; denying request (fail-closed)"
    );
    return Err(
        "🚨 [SECURITY BLOCK] Unable to verify immune status. Request denied.".into(),
    );
}
```

**stream 初期（L43–72 を置換）:**

```rust
// L40 の `let provider = (*state.provider).clone();` は残す（L251 stream_complete / L464 課金）
let router = crate::tool_call_router::DefaultToolCallRouter;
if let Err(block_msg) = router.evaluate_security(&payload.prompt, &state).await {
    yield Ok::<Event, Infallible>(
        Event::default().event("security_block").data(block_msg),
    );
    return;
}
```

- L43–47 Guardrails 直呼びは **削除**（`evaluate_security` 内に含む）
- **触らない**: BeggingSupervisor（`:367-372`、出力側・`event("error")`）

**agent_engine（L71–101 を置換）:**

```rust
use crate::tool_call_router::{DefaultToolCallRouter, ToolCallRouter};

let router = DefaultToolCallRouter;
if let Err(block_msg) = router.evaluate_security(prompt, state).await {
    return Ok(block_msg); // GUARDRAIL / SENTINEL / SECURITY いずれも Ok(文字列)
}
```

- `shared::guardrails` の入力直呼びは削除可（router 内で実行）
- ImmuneAlert は `evaluate_security` 内（`:71-84`）→ 再実装禁止
- Sentinel 文言は router 形式（`Pattern:` 付き）に揃う

### 3.4 テスト

| ID | 内容 |
|----|------|
| P1 | `test_security_regression_sentinel_block` / `test_immune_system_precedes_hook` PASS |
| N1 | OP-024 と同型: `infrastructure::sql_exec!(&state.job_queue.pool, "DROP TABLE immune_rules")` + `"hello status check"` → Err / `Unable to verify` |
| N2 | SSE **初期**のみ: `security_block`、LLM 非開始（tool-loop の `error` は対象外） |
| N3 | `AgentEngine::chat`（`routes/agent.rs` / watchtower）: `Ok` + SECURITY BLOCK 部分一致 |

### 3.5 DoD

- [x] router Fail-Closed
- [x] stream 初期 = evaluate_security + `security_block`（`provider` クローン維持）
- [x] agent_engine = evaluate_security + `Ok(block_msg)` + trait import
- [x] N1（router）+ N3（agent_engine）+ Positive（sentinel / precedes-hook）。N2 は同一 `evaluate_security` 共有で担保（専用 SSE ハーネスは省略）
- [x] OPEN OP-075 ✅ / CHANGELOG / TECH_DEBT P1 RESOLVED
- [x] degraded フラグは作らない

### 3.6 Phase B（同 PR 禁止）

`goal_processor.rs:146`、`napi-bridge`、`skill_handler.rs:294`、commercial mcp

---

## 4. Wave B — OP-054 可視性のみ（P2）

### 4.1 変更（検証済み）

| メソッド | 行 | 呼び出し | 方針 |
|----------|-----|----------|------|
| `with_llm` | `mod.rs:253-256` | **ゼロ** | `pub(crate)` |
| `get_embedding_provider` | `mod.rs:248-251` | 同クレートのみ（`karma.rs` / tests） | `pub(crate)` |
| `set_embedding_provider` | `mod.rs:243-246` | `llm_providers.rs:60` | `pub` 維持 |
| `from_pool` | `mod.rs:181+` | commercial / tests 多数 | `pub` 維持 |

### 4.2 やらないこと（旧 B + ADR-031）

- Federation キャスト追加 / buzz EvaluationOps DI / CostOps / Publisher / GlobalMock
- `FederationRegistry` 拡張
- **JobQueue ISP 完遂・AppState の `Arc<dyn JobQueue>` 化** — [`031-jobqueue-isp-deferral.md`](../decisions/031-jobqueue-isp-deferral.md) により Phase 4 まで延期。本 OP で再発明しない

監査 v11 P2 の残りは TECH_DEBT に「AppState 具象（`app_state.rs:72`）— ADR-031 待ち」と降格。OPEN OP-054 は可視性完了で ✅ 可。

### 4.3 テスト / DoD

- `cargo test -p infrastructure -p api-server`
- OPEN OP-054 ✅、CHANGELOG、remaining_tasks §7 と整合

---

## 5. Wave C — OP-051（P3）

ADR-054 **Accepted 2026-07-20**。実装正本: [`op051_error_hierarchy_plan.md`](op051_error_hierarchy_plan.md) v1.0（**P1–P4 ✅ / OP-051 完了**・一括置換禁止）。

---

## 6. Wave D — OP-068（P4）

監視のみ。scc は dev。cargo bump なし。

---

## 7. Wave E — P5 + Quick Wins

### 7.1 OP-029

| ファイル | 変更 |
|----------|------|
| `biome-popup-entry.tsx:36` | `var(--bg-base)`（`App.css`→tokens 読込済み） |
| `biome-popup.html:20` | `transparent`（ルートに委譲）または tokens link + `var(--bg-base)` |
| `test_ui_hex_violations.py` | `extra_files` に **entry.tsx のみ**（html はスキャン対象外） |

### 7.2 App.tsx

着手しない。

### 7.3 OP-076

| 変更 | 触らない |
|------|----------|
| `mcp/discovery.rs:72` | Nurture compose / commercial `.env` |
| `mcp_servers.json.example:29` | |
| `i18n/en.json:385` / `ja.json:385` | |
| `McpConfigManager.tsx:85` defaultValue | |
| `.env.example:164` → `STRIPE_API_KEY` + Nurture 別系統注記 | |

### 7.4 QW-21..23

| ID | 内容 |
|----|------|
| QW-21 | `cargo license --json > docs/licenses.json` |
| QW-22 | `stripe.rs:440` dispute、`:470` checkout、`polar.rs:203/218/246` のみ。payment_failed は完了済み |
| QW-23 | `stripe.rs:341-346` Err 時 `warn!`。fail-closed は別承認 |

---

## 8. 実行順序

```
Wave E（並列）──► OP-029, OP-076, QW-21..23
Wave A ─────────► OP-075
Wave B ─────────► OP-054（可視性）
Wave C ─────────► ADR-054
Wave D ─────────► 監視
```

E ∥ A 可。B は A と独立。旧 B 承認ゲートは **不要**。

---

## 9. NURTURE / 影響波及

| § | 扱い |
|---|------|
| §3 MCP / §4 セキュリティ | OP-075 / OP-076 |
| §2 経済 | QW 観測性のみ |
| §8 P2P | **本計画でコード変更なし**（旧 B 削除） |

| 核 | 波及 |
|----|------|
| `evaluate_security` | processor / mcp / stream tool-loop（既存）+ **stream 初期 + agent_engine** |
| JobQueue 可視性 | 外部影響なし（呼び出しゼロ / 同クレート） |

---

## 10. 悪魔の弁護人

1. **最悪**: DB 障害で全チャット拒否 → 運用注意。baseline は残る。
2. **誤前提（v1.2 残存）**: 「provider を消してよい」「OPEN の degraded を実装」「ISP を今やる」「Begging も SECURITY BLOCK だから直す」→ **いずれも誤り（v1.3 で否定）**。
3. **やらないメリット**: 旧 B / ADR-031 スコープ / Begging / App.tsx / Upstream を触らない → 差分がレビュー可能。

---

## 11. 着手テンプレ

```
現在は TECH_DEBT Top5 計画（tech_debt_top5_plan.md v1.3）の実装フェーズです。
Wave E（OP-029/076）または Wave A（OP-075）から着手します。
NT-* / OP-054-B（削除済み）/ ADR-031 ISP 完遂 / OP-051 コード置換 / App.tsx /
Nurture STRIPE_SECRET_KEY / Upstream bump / OP-075-B / BeggingSupervisor には触れません。
```

---

## 12. `/perfect-plan` 検証結果（v1.3）

## 検証対象
`docs/roadmaps/tech_debt_top5_plan.md` v1.3

## Gate 1: 構造スキャン
- ✅ stream `provider` L251/464 必須、agent_engine import 欠落を計画に固定、ADR-031 実在
- ✅ 再利用: `evaluate_security` / OP-024 N テスト / `--bg-base`
- ✅ 車輪排除: Federation キャスト、ISP 完遂、Begging 改修を禁止

## Gate 2: 要件カバレッジ
- ✅ §3/§4。§8 非変更

## Gate 3: 依存関係
- ✅ `AgentEngine::chat` 呼び出し元は `Ok(文字列)` 契約維持
- ✅ OPEN OP-075 ドリフトを DoD で同期
- ✅ mcp/processor は自動解消

## Gate 4: 悪魔の弁護人
- ✅ 可用性 / v1.2 誤前提の否定 / やらない選択

## Gate 5: 実行順序
- ✅ E ∥ A、B 独立。追加依存なし

## 判定
- [x] ✅ **PASS** — 実コードと整合。承認後に実装可能。
- 任意確認（非ブロッカー）: biome `#0b0d14` 寄せの視覚受け入れ

---

## 付録: 版履歴

| 版 | 要点 |
|----|------|
| v1.0 | Top5 初版（PATCH） |
| v1.1 | Ok 契約 / HEX / QW-22 / OP-076 列挙 |
| v1.2 | OP-054-B 削除、SSE イベント名、agent_engine 集約 |
| v1.3 | provider 必須、import 明記、OPEN/ADR-031/Begging スコープ固定 |

---

*Generated by `/perfect-plan` v1.3 re-verification — 2026-07-10*
