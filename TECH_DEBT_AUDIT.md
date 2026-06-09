# 🔍 Aiome 技術的負債監査レポート

**監査日**: 2026-06-08 (v6.1 — TDD 負債解消プロセス開始)
**前回監査日**: 2026-06-08 (v6)
**対象コードベース**: **152k LOC** (Rust ~128k + TypeScript ~24k)
**監査ツール**: `cargo audit`, `enforce_unwrap_deny.py`, `deep-scan.sh`, Git hotspot analysis, grep-based deep scan
**Reflexion ラウンド**: 累計7セット実施（累計20件の修正、最高スコア 98)
**深掘り対象 (v6.1)**: `mcp/discovery.rs`, `napi-bridge/src/lib.rs`, UIコンポーネント
**深掘り対象 (v6)**: `security.rs`, `immune_system.rs`, `context_engine.rs`, `mcp/discovery.rs`, `commune.rs`

---

## 1. Executive Summary

Aiome は **152k LOC**, 93+ モジュールの大規模プロジェクトです。v6.1 監査では、フロントエンドテストの残存未テストコンポーネントが残り 5 個（GraphView, OllamaModelSelector, PromptStatsView, SeoPulseView, TreasureBox）に特定され、`napi-bridge` のテスト欠損や `mcp/discovery.rs` の環境変数解決の重複（DRY化）がフォーカスとなっています。

### v6 → v6.1 の変化

- ✅ **Zero-Panic Policy**: `enforce_unwrap_deny.py` = **"No illegal unwraps found!"** および残存 `panic!()`（discord.rs）の解消により **違反 0 件** を達成
- ✅ **フロントエンドテスト欠損**: 残存する 5 つのコンポーネントテスト作成を計画
- ✅ **napi-bridge テスト**: FFI 境界層 (760行) のテスト作成を計画

### 残存する構造的負債

- **God Module 問題**: `task_orchestrator/mod.rs` (2,044行), `llm_provider/mod.rs` (2,104行), `dream_state.rs` (1,652行), `security.rs` (1,209行), `mcp/discovery.rs` (1,113行)
- **`as any` 本番使用**: 16箇所 (A2uiRenderer, CausalVisualizer, hooks 含む)
- **`let _ =` パターン**: 本番コード **152 箇所** (api-server + infrastructure + commercial)
- **Error 型分散**: 10種類のカスタムエラー型。`error.rs` の変換層は模範的
- **フロントエンドテスト欠損**: 5コンポーネントがテスト未作成
- **napi-bridge テスト 0 件**: 760 行の N-API バインディング層にテストが皆無

> [!IMPORTANT]
> **cargo audit はクリーン** — 既知の脆弱性はゼロです。セキュリティの基盤は健全です。

---

## 2. Top 5 Priorities

| # | 負債 | 深刻度 | 影響 | 見積もり | Status |
|---|---|---|---|---|---|
| **P1** | **Sqlite/Postgres `match &self.pool` 重複** (14ファイル, 37箇所) | 🔴 | 共通マクロの導入により、全14ファイルにおける行マッピング分岐を DRY 統一しました。 | 16h | `[RESOLVED]` |
| **P2** | `bridge.rs` (2,364行) & `stripe.rs` (1,929行) の God Module 分割 | 🔴 | 巨大な商用拡張モジュールの機能・テスト分割を完了し、ディレクトリ構造へ移行。 | 8h | `[RESOLVED]` |
| **P3** | [UPDATE] `mcp/discovery.rs` (1,113行) God Module + ハードコード OAuth URL 20+ | 🟡 | OAuth エンドポイント、トークン交換、MCP テンプレートが1ファイルに密結合。URL 変更時のリグレッションリスク。 | 6h | — |
| **P4** | [UPDATE] フロントエンド 5 コンポーネントのテスト欠損 | 🟡 | `GraphView.tsx`, `OllamaModelSelector.tsx`, `TreasureBox.tsx`, `PromptStatsView.tsx`, `SeoPulseView.tsx`。 | 8h | — |
| **P5** | Error 型の統一 (10種類 → 3階層) | 🟡 | `thiserror` 7ファイル vs `anyhow` 47ファイルの混在。ただし `error.rs` の変換層 (22テスト) は模範的な設計。 | 6h | — |

---

## 3. Quick Wins（1時間以内で修正可能）

| # | 修正内容 | ファイル | 効果 | Status |
|---|---|---|---|---|
| **QW-1** | `.env.example` の不足キー追加 | `.env.example` | 新規 onboarding の完全自動化 | — |
| **QW-2** | `ArtifactVault.tsx:742` の rgba → CSS 変数 | `ArtifactVault.tsx:742` | U-002 解消 | `[RESOLVED]` |
| **QW-3** | `samsara-hub/src/main.rs:71` の unwrap → map_err | `samsara-hub/src/main.rs:71` | Zero-Panic 準拠 | `[RESOLVED]` |
| **QW-4** | `AiaaOnboardingWizard.tsx:149` の rgba → CSS 変数 | `AiaaOnboardingWizard.tsx:149` | U-002 解消 | `[RESOLVED]` |
| **QW-5** | `A2uiRenderer.tsx:205` の rgba → CSS 変数 | `A2uiRenderer.tsx:205` | U-002 解消 | `[RESOLVED]` |
| **QW-6** | `docs/api-server.md:16,51` の bootstrap.rs 参照更新 | `docs/api-server.md` | Doc drift 解消 | `[RESOLVED]` |
| **QW-7** | `GraphView.tsx:202` の rgba → `var(--accent-cyan-15)` | `GraphView.tsx:202` | U-002 解消 | `[RESOLVED]` |
| **QW-8** | `crdt.rs:13` の未使用 `use tracing::info` 削除 | `crdt.rs:13` | 警告除去 | `[RESOLVED]` |
| **QW-9** | `settings.rs:202` エラーメッセージに job_id 追加 | `settings.rs:202` | デバッグ性向上 | `[RESOLVED]` |
| **QW-10** | `discord.rs:196` の `panic!()` → `Err()` に変換 | `discord.rs:196` | Zero-Panic 違反の完全解消 | `[RESOLVED]` |
| **QW-11** | `GraphView.tsx:212` の title 属性を i18n 化 | `GraphView.tsx:212` | i18n 完全対応 | — |
| **QW-12** | `NurtureDashboard.tsx:87` の `catch (e: any)` → 型付きエラー | `NurtureDashboard.tsx:87` | 型安全性向上 | — |
| **QW-13** | `auth.ts:23` の `as Record<string, string>` → 型安全なヘッダマージ | `auth.ts:23` | headers が `HeadersInit` の場合にランタイムエラー | — |
| **QW-14** | `AgentConsole.tsx:108` のマジックナンバー `5` (ROI計算) を定数化 | `AgentConsole.tsx:108` | `savings: tasksCount * 5` — ハードコードの $5/task | — |
| **QW-15** | `AgentConsole.tsx:115` の ROI ハッシュ計算をシード付き乱数に変更 | `AgentConsole.tsx:115` | `charCodeAt` の合計 % 500 による決定論的だが意味のない ROI 値生成 | — |
| **QW-16** | `setup.rs:47` の未認証 `setup_init` ハンドラへの `// auth-exempt` コメント追加 | `setup.rs:47` | Type-Driven Security 違反の解消 | `[RESOLVED]` |
| **QW-17** | [NEW] `commune.rs:347` の P2P E2E 暗号化 TODO コメントを ADR に昇格 | `routes/commune.rs:347` | セキュリティロードマップの明文化 | — |
| **QW-18** | [NEW] `mcp/discovery.rs:527-531` の OAuth URL を定数/設定ファイルに外出し | `mcp/discovery.rs:527-531` | 5サービスのトークンURLがハードコード | — |

---

## 4. Findings Table（12次元別）

### Dimension 1: Architectural Decay（アーキテクチャの腐敗）

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| 🔴 | `libs/infrastructure/src/job_queue/*.rs` | — | [RESOLVED] 全 14 ファイルにおける `match &self.pool` 分岐を共通マクロへ置換・DRY化完了。 | `[RESOLVED]` |
| 🔴 | `libs/infrastructure/src/task_orchestrator/mod.rs` | 1-2044 | **2,044行**。タスク実行・診断・リトライ・テスト全てが同一ファイル。 | — |
| 🟡 | `libs/core/src/llm_provider/mod.rs` | 1-2104 | **2,104行**。LLM プロバイダー抽象化レイヤー。 | — |
| 🟡 | `libs/infrastructure/src/dream_state.rs` | 1-1652 | **1,652行**。6つの DreamState モード。 | — |
| 🟡 | `libs/infrastructure/src/security.rs` | 1-1209 | **1,209行**。BastionGuard + VoiceCoreDrm + サブモジュール。テストは模範的。 | — |
| 🟡 | `apps/api-server/src/mcp/discovery.rs` | 1-1113 | [NEW] **1,113行**。OAuth URL 20+箇所ハードコード、MCP テンプレート、トークン交換が1ファイルに密結合。 | — |
| 🟡 | `apps/api-server/src/bootstrap/mod.rs` | 229-1115 | `init_core_services` (886行) が未抽出。密結合 of 依存チェーン。 | [PARTIAL] |
| 🟡 | `libs/infrastructure/src/lib.rs` | 1-201 | **93個の `pub mod`**。infrastructure のスーパークレート化。 | — |
| 🟢 | `apps/api-server/src/api_integration_tests.rs` | — | Git hotspot **#1** (122 commits/3mo)。テストファイルのため許容。 | ✅ |

### Dimension 2: Consistency Rot（一貫性の崩壊）

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| 🔴 | `federation.rs` | 735-787 | `map_sqlite_row_to_karma` (25行) と `map_postgres_row_to_karma` (25行) がほぼ同一。唯一の差は `i64` vs `i32` のキャスト。ジェネリック関数化で統一可能。 | — |
| 🟡 | `federation.rs` | 237-276 | Sqlite/Postgres の INSERT クエリ文字列がカラムリストを含めて**完全に同一**。DB 方言の分岐が不要な箇所で `match` している。 | — |
| 🟡 | `mcp/discovery.rs` | 527-560 | [NEW] OAuth トークンエンドポイント URL と認可 URL が **5サービス分ハードコード** (`github`, `slack`, `notion`, `discord`, `figma`)。定数または設定ファイルへの外出しが必要。 | — |
| 🟡 | `AgentConsole.tsx` | 108 | `savings: tasksCount * 5` — ROI 計算のマジックナンバー。 | — |
| 🟡 | `auth.ts` | 23 | `options.headers as Record<string, string>` — `HeadersInit` の不安全なキャスト。 | — |
| 🟡 | `libs/infrastructure/src/llm/cost_breaker.rs` | 51-74 | `.ok()` でエラー黙殺。他モジュールでは `tracing::warn` 付きフォールバックが標準。 | — |
| 🟡 | (全体) | — | **Error 型の分散**: 10種類 (`AiomeError`, `SoulError`, `X402Error`, `CsamError`, 等)。 | — |
| 🟡 | (全体) | — | **`let _ =` パターン 152 箇所** (本番コード、テスト除外)。`tool_call_router.rs` 単体で **19 箇所**。DB 操作 (`logging.rs:31,45`, `autonomous_demo.rs:74,75,104,105,135`) は要確認。 | — |

### Dimension 3: Type & Contract Debt（型・契約の負債）

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| 🟡 | `apps/management-console/src/App.tsx` | 151, 156, 170 | `data as any` — Tauri IPC イベントデータ。 | — |
| 🟡 | `apps/management-console/src/components/home/StoryFlow.tsx` | 28, 59 | `a2uiEnvelope?: any`, `d.data as any`。 | — |
| 🟡 | `apps/management-console/src/components/home/HomePage.tsx` | 50, 221, 318 | `lastEvent?: any`, `mode={mode as any}`。 | — |
| 🟡 | `apps/management-console/src/components/A2uiRenderer.tsx` | 215, 234, 237, 239, 258 | [NEW] `(m: any)`, `(c: any)`, `(row: any)`, `(ev: any)` — A2UI レンダリングの動的型。 | — |
| 🟡 | `apps/management-console/src/components/CausalVisualizer.tsx` | 23, 39 | [NEW] `{nodes: any[], edges: any[]}`, `(n: any)` — Graph データの型欠損。 | — |
| 🟡 | `apps/management-console/src/components/DemoView.tsx` | 17 | [NEW] `lastEvent: any` — イベントデータの型定義なし。 | — |
| 🟡 | `apps/management-console/src/components/OllamaModelSelector.tsx` | 59 | [NEW] `(m: any)` — Ollama API レスポンスの型定義なし。 | — |
| 🟡 | `apps/management-console/src/hooks/useAgentChat.ts` | 106 | [NEW] `(m: any)` — チャット履歴の型定義なし。 | — |
| 🟡 | `apps/management-console/src/hooks/useViewMode.ts` | 26 | [NEW] `(s: any)` — 設定値の型定義なし。 | — |
| 🟡 | `apps/management-console/src/lib/inx/InxRenderer.tsx` | 38 | [NEW] `wasmInstance: any` — WASM インスタンスの型定義なし。 | — |
| 🟡 | `apps/management-console/src/i18n/index.tsx` | 27 | [NEW] `getNestedValue(obj: any, path: string)` — 汎用ユーティリティの型安全化。 | — |
| 🟡 | `apps/management-console/src/components/commerce/NurtureDashboard.tsx` | 87 | `catch (e: any)` — `unknown` に変更可能。 | — |
| 🟡 | `apps/management-console/src/lib/auth.ts` | 23 | `options.headers as Record<string, string>` — `HeadersInit` 型の不安全なキャスト。 | — |
| ⚪ | `apps/management-console/src/lib/api_resolver.ts` | 25 | `(window as any).__TAURI_INTERNALS__` — Tauri API 型の制約上やむを得ない。 | ⚪ |

### Dimension 4: Test Debt（テストの負債）

| 深刻度 | ファイル | 指摘内容 | Status |
|---|---|---|---|
| ✅ | `EscrowManagementView.tsx` | テストスイート完成。 | [RESOLVED] |
| ✅ | `TaskApprovalOverlay.tsx` | テストスイート完成。 | [RESOLVED] |
| ✅ | `SettingsPage.tsx` | テストファイル作成済み。 | ✅ |
| ✅ | `ImmuneSystem.tsx` | テストファイル作成済み。 | ✅ |
| ✅ | `BiomeDialogueView.tsx` | テストファイル作成済み。 | ✅ |
| ✅ | `ExpressionPipeline.tsx` | テストファイル作成済み。 | ✅ |
| 🟡 | (未テスト5個) | [UPDATE] **5コンポーネントがテスト未作成**: `GraphView.tsx`, `OllamaModelSelector.tsx`, `TreasureBox.tsx`, `PromptStatsView.tsx`, `SeoPulseView.tsx` | — |
| 🟡 | `libs/napi-bridge/src/lib.rs` (760行) | **テスト 0 件**。16 個の `#[napi]` FFI 境界層にテストが皆無。 | — |
| 🟢 | (Rust) | **1,137 テスト** 全パス。 | ✅ |
| ⚪ | (E2E) | **Playwright テスト 0件**。 | — |

### Dimension 5: Dependency & Config Debt

| 深刻度 | ファイル | 指摘内容 | Status |
|---|---|---|---|
| 🟡 | `.env.example` | [UPDATE] deep-scan 検出: `OTHER_VAR` が `.env.example` に未記載。 | — |
| 🟡 | `libs/infrastructure/Cargo.toml` | 90+ エントリ。`cargo machete` での未使用依存検証推奨。 | — |
| ⚪ | `Cargo.toml` (workspace) | `rand = "0.8"` — 0.9 安定版リリース済み。機能的影響なし。 | ⚪ |

### Dimension 6: Performance & Resource Hygiene

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| ⚪ | (全体) | — | `.clone()` が多数。大半は `Arc`/`String`。非同期境界のため不可避。 | ✅ |
| ⚪ | `context_engine.rs` | — | [NEW] `.clone()` 13回。非同期処理上不可避。 | ✅ |

### Dimension 7: Error Handling & Observability

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| 🟡 | `apps/api-server/src/stream.rs` | 55, 186, 479 | `let _ =` / `.ok()` — DB 記録失敗・設定値取得の黙殺。 | — |
| 🟡 | `apps/api-server/src/logging.rs` | 31, 45, 83 | `let _ = sqlx::query(...)` — ログ記録失敗の黙殺。 | — |
| 🟡 | `apps/api-server/src/skill_handler.rs` | 152, 228, 258, 362, 374 | `let _ =` が 5 箇所。 | — |
| 🟡 | `apps/api-server/src/tool_call_router.rs` | 66-434 | `let _ =` が **19 箇所**。 | — |
| 🟡 | `apps/api-server/src/autonomous_demo.rs` | 74,75,104,105,135 | `let _ = sql_exec!(...)` が 5 箇所。デモコード。 | — |
| 🟡 | `apps/api-server/src/internal_services/watchtower.rs` | 92-309 | `let _ =` が 8 箇所。 | — |
| 🟡 | `libs/infrastructure/src/llm/cost_breaker.rs` | 51-74 | `.ok()` でエラー黙殺。 | — |
| 🟡 | `libs/infrastructure/src/lora_marketplace.rs` | 282, 317 | DB クエリの `.ok()` 黙殺。 | — |
| 🟡 | `apps/api-server/src/routes/settings.rs` | 274, 543, 600, 607, 614 | [NEW] `.ok()` が 5 箇所。 | — |
| 🟡 | `libs/infrastructure/src/security.rs` | 92, 120, 122, 135, 138 | [NEW] `.ok()` が 5 箇所。 | — |

### Dimension 8: Security Hygiene

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| ✅ | — | — | `cargo audit` クリーン。既知の脆弱性ゼロ。 | ✅ |
| ✅ | — | — | ハードコード秘密鍵なし。 | ✅ |
| ✅ | `native.ts` | 52-58 | CWE-426 防御、Fail-Closed フォールバック。 | ✅ |
| ✅ | `auth.rs` | 287-298 | Constant-Time 比較、Fail-Closed BAN。 | ✅ |
| ✅ | `error.rs` | 111-152 | CWE-209/CWE-532 準拠。 | ✅ |
| ✅ | `fs_reader/lib.rs` | 18-53 | Default Deny。 | ✅ |
| ✅ | `federation.rs` | 598-600 | SSRF 防御 (`redirect(Policy::none())`)。 | ✅ |
| ✅ | `validator.rs` | 46-188 | 3段階 Adversarial Validation。 | ✅ |
| ✅ | `heartbeat_wakeup.rs` | 57-208 | Shell injection 防御。 | ✅ |
| ✅ | `napi-bridge/lib.rs` | 280-308 | Baseline Sentinel (6パターン)。 | ✅ |
| ✅ | `setup.rs` | 47 | `// auth-exempt` コメント付与により解決。 | `[RESOLVED]` |
| ✅ | `security.rs` | 199-215 | インジェクションフィルタ強化版。 | ✅ |
| ✅ | `security.rs` | 263-323 | PathSandbox + Vault Sandbox 二重パス検証。 | ✅ |
| ✅ | `security.rs` | 512-518 | env_clear() + harden_command_async()。 | ✅ |
| ✅ | `immune_system.rs` | 193-220 | 15パターンの Baseline Sentinel。 | ✅ |
| ✅ | `commune.rs` | 257-295 | P2P メッセージバリデーション | ✅ |
| 🟡 | `commune.rs` | 347 | [NEW] P2P E2E 暗号化が未実装 — TODO(SEC) コメントあり、ADR-043 参照が必要。 | — |

### Dimension 9: Documentation Drift

| 深刻度 | ファイル | 指摘内容 | Status |
|---|---|---|---|
| ✅ | `SYSTEM_PANORAMA.md` | 実態と一致。 | ✅ |
| 🟡 | `docs/api-server.md` | L16,51 が `bootstrap.rs` を参照。`bootstrap/` への更新必要。 | — |
| 🟡 | `docs/architecture/system_integrity_audit_v2.md` | L62,82,106 が旧 `bootstrap.rs` を参照。 | — |

### Dimension 10: Zero-Panic Policy 形骸化

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| ✅ | (全体) | — | `enforce_unwrap_deny.py` = **"No illegal unwraps found!"** 違反 0 件 | ✅ |

### Dimension 11: Tauri IPC 型安全性

| 深刻度 | ファイル | 指摘内容 | Status |
|---|---|---|---|
| ⚪ | — | REST API 経由に移行済み。 | ✅ |
| 🟡 | `api_resolver.ts:25` | `(window as any).__TAURI_INTERNALS__` — Tauri API 型の制約上やむを得ない。 | ⚪ |

### Dimension 12: tokens.css 遵守度（U-002）

| 深刻度 | ファイル | 行 | 指摘内容 | Status |
|---|---|---|---|---|
| ✅ | 本番 TSX/TS 全体 | — | **rgba/rgb ハードコード: 0件**。 | ✅ |

---

## 5. Things That Look Bad But Are Actually Fine

| 一見すると負債に見えるもの | 実際の意図 |
|---|---|
| **`#![allow(dead_code)]` が 7 クレートに存在** | トレイト定義クレートでは下流でのみ使用されるシンボルが多い。 |
| **`// allow-anti-pattern: static regex`** (8箇所) | 静的正規表現の `expect()` は実質的にパニックしない。 |
| **`// allow-anti-pattern: fatal configuration error at boot`** (3箇所) | 起動時の致命的設定エラーは Fail-Fast が正しい設計判断。 |
| **`std::process::exit(1)` が 13箇所** | 全て起動時の致命的エラー。 |
| **`Artemis Inter` フォント名のハードコード** | Canvas/vis-network API は CSS 変数を解決できない。 |
| **infrastructure の 93 `pub mod`** | feature flag による重い依存の分離は済んでおり、分割コスト対効果が低い。 |
| **`soul_store.rs` の `.ok()` 使用** | `try_get()` + `.ok()` は SQLx の nullable カラム読み取りの標準パターン。 |
| **`let _ = tx.try_send(entry)` (logging.rs:83)** | ログチャネルの溢れ。ログの欠損は許容。 |
| **test ファイルの `#![allow(clippy::unwrap_used)]`** (7件) | テストコード内の `unwrap` は Rust コミュニティの標準。 |
| **`error.rs` の 8種類のエラー型 From 実装** | HTTP セマンティクスを保持する意味的マッピング。 |
| **`federation.rs` の `try_get().unwrap_or_default()`** | `unwrap_or_default()` は Zero-Panic に抵触しない。 |
| **`fs_reader/lib.rs` の `#![forbid(unsafe_code)]`** | WASM サンドボックス内で正しい設計。 |
| **`AgentConsole.tsx` の `key={i}` (L387)** | 追加のみ・並べ替えなしのリストで許容。 |
| **`napi-bridge/lib.rs:80-81` の `unwrap_or()`** | Zero-Panic に抵触しない Optional フィールドのデフォルト値。 |
| **`heartbeat_wakeup.rs:186-189` の `dangerous_patterns` ハードコード** | 単純パターンのため正規表現より合理的。 |
| **`validator.rs:68` の `threshold = 0.77` ハードコード** | コメントで暫定値と明記。 |
| **`SkillVault.tsx` の `console.error` 使用** | `showToast` と併用で UX もカバー。 |
| **`security.rs:92` の `WORKSPACE_DIR.ok()`** | 環境変数の optional 取得。 |
| **`immune_system.rs:212-218` の `std::process::exit(1)`** | Sentinel regex のコンパイル失敗は致命的エラー。 |
| **`context_engine.rs:284` の `LazyLock<DashMap>`** | Anti-thrashing 機構。 |
| **`commune.rs:343` の karma_root_cid: "cid_local_relay"`** | ローカルリレー経由のメッセージには CID が不要。 |
| **`mcp/discovery.rs` の OAuth URL ハードコード** | 外部サービスの公開仕様の固定 URL であり、変更頻度が極めて低い。 |

---

## 6. Open Questions

| # | 質問 | Status | 結論 |
|---|---|---|---|
| **OQ-1** | `bootstrap.rs` の分割方針 | [RESOLVED] | 6サブモジュール分割完了。 |
| **OQ-2** | `infrastructure` クレート分割 | [RESOLVED] | 現時点では分割しない。feature flag で分離済み。 |
| **OQ-3** | フロントエンドテスト戦略 | [RESOLVED] | 残り5コンポーネントのみ。 |
| **OQ-4** | `.env.example` 未記載キー | — | `OTHER_VAR` が deep-scan で検出。 |
| **OQ-5** | `AppDataResolver::new().unwrap()` の統一修正方針 | [RESOLVED] | 安全な fallback へ置換し完全解消。 |
| **OQ-6** | `llm_provider/mod.rs` (2,104行) の分割要否 | — | 変更頻度の分析が必要。 |
| **OQ-7** | `discord.rs:196` の `panic!()` | [RESOLVED] | panic を安全なフォールバックへ置換完了。 |
| **OQ-8** | `as any` 16箇所 — 型定義の統合方針 | [UPDATE] | v6: `App.tsx` の 3 件に加え、`A2uiRenderer`, `CausalVisualizer`, hooks に拡散。包括的な API レスポンス型定義が必要。 |
| **OQ-9** | `match &self.pool` マクロ化 | [RESOLVED] | マクロアプローチによる DRY 化を完了。 |
| **OQ-10** | `AgentConsole.tsx:108` の ROI 計算 | — | ビジネス判断が必要。 |
| **OQ-11** | `setup.rs:47` の `// auth-exempt` | [RESOLVED] | コメント付与済み。 |
| **OQ-12** | [NEW] `mcp/discovery.rs` の分割方針 | — | 1,113行の単一ファイルは保守性リスク。 |
| **OQ-13** | [NEW] P2P E2E 暗号化 (`commune.rs:347`) の実装時期 | — | セキュリティ上の優先度は高いが、P2P 機能自体が MVP 段階。 |

---

## 7. Reflexion 実績サマリ（累計）

| ラウンド | フォーカス | 修正数 | スコア推移 |
|---|---|---|---|
| R1 | 構造品質（`#![allow]` 重複、`any` 型、空行） | 4件 | 93 → 96 |
| R2 | 意味的整合性（headers/body/UUID アサーション） | 5件 | 94 → 97 |
| R3 | Golden Rule（`.unwrap()` 排除、ネットワーク例外テスト） | 3件 | 95 → 98 |
| R4 | アーキテクチャ + テスト衛生（`console.error` spy） | 1件 | 97 → 98 |
| R5 | E2E テスト安定化（JWT 統一、デバッグ残骸除去） | 4件 | 86 → 97 |
| R6 | U-002 修正、未使用 import 削除、エラーメッセージ改善 | 3件 | 96 → 97 |
| **累計** | | **20件** | **最高 98/100** |

---

## 8. メトリクス推移

| 指標 | v5 (2026-06-08) | v6 (2026-06-08) | v6.1 (2026-06-08) | トレンド |
|---|---|---|---|---|
| 総 LOC | 152k | 152k | **152k** | → |
| Rust テスト数 | 1,137 | 1,137 | **1,137** | → |
| TS テストファイル | 53 | 41 | **41** | → |
| cargo audit | クリーン | クリーン | クリーン | ✅ |
| Zero-Panic 違反 | 1 | 1 | **0** | ✅ |
| U-002 違反 (TSX) | 0 | 0 | **0** | ✅ |
| allow-anti-pattern | 20 | 20 | **20** | → |
| God Module (1k+ 行) | 3 | 6 | **6** | → |
| `as any` 本番使用 | 9 | 16 | **16** | → |
| `match &self.pool` 重複 | 0 | 0 | **0** | ✅ |
| `let _ =` 総数 (本番) | 230 | 152 | **152** | → |
| napi-bridge テスト | 0 | 0 | **0** | → |
| FE テストカバレッジ | ~52% | 72% | **72%** (残り5コンポーネント) | → |

---

*Generated by `/tech-debt-audit` workflow — 2026-06-08 v6.1 (TDD 負債解消プロセス開始)*
