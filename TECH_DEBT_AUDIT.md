# 🔍 Aiome 技術的負債監査レポート

**監査日**: 2026-07-10（**v11.0** — Public Beta Wave 1/2 完了後の差分監査）  
**前回監査日**: 2026-07-02（v10.0）  
**分析コミット**: `c5b0ca1e`  
**対象コードベース**: Rust / TS 本番ソース（`target/`・`workspace/` 除外）  
**監査ツール**: `cargo audit`, `enforce_unwrap_deny.py`, `deep-scan.sh --ci`, `test_ui_hex_violations.py`, Git hotspot（3ヶ月）, クレート分割サブエージェント監査

---

## 1. Executive Summary

v10.0 以降、Wave 1/2・UI・OP-013・Vault・**OP-075/075-B・App.tsx 分割（2026-07-11）**が進み、Immune Fail-Open と god-shell は解消。残る主戦場は **OP-051（ADR-054）・OP-068 Upstream（OP-032 ✅・deny 21→8）・Human NT-***（OP-059-UI は 2026-07-13 完了）。

### 主要な変化（v10 → v11）

| v10 指摘 | v11 状態 |
|---|---|
| quick-xml RUSTSEC-2026-0194/0195 | `[RESOLVED]`（現行 `quick-xml 0.39.4`。OP-034 は Tauri/plist 経路の ignore 整理として継続） |
| `unwrap_or_else(\|_\| loop {})`（skills） | `[RESOLVED]`（OP-053） |
| `as any` ×4（WorkflowBuilder 等） | `[RESOLVED]`（OP-028）。残は `api_resolver.ts:25` の Tauri 検出 1 件 |
| `biome-popup-entry.tsx` HEX | `[RESOLVED]` 2026-07-10（`var(--bg-base)` + ゲート `extra_files`） |
| `skills/mod.rs` God Module 1134 行 | `[RESOLVED]` 降格（OP-050 → **598 行**。WasmSkillManager コアは残存） |
| `apps/watchtower` deep-scan ドリフト | `[RESOLVED]`（OP-052） |
| MockJQ インライン | `[RESOLVED]`（OP-055 → `testing/mock_jq.rs`）。**GlobalMockJobQueue 二重化が新規負債** |
| html2md GPL | `[RESOLVED]`（OP-067 → htmd）。licenses.json も 2026-07-10 再生成済 |

### 今回の新規・再浮上（v11 監査時点 → 2026-07-11 消化状況）

1. **Immune System 評価失敗時の Fail-Open** — **`[RESOLVED]`** OP-075（chat）+ OP-075-B（napi / goal / nurture MCP / skill_handler）  
2. **commerce webhook の KC 付与 / SSE 握り潰しの一貫性欠如** — 継続監視  
3. **`scc` RUSTSEC-2026-0205**（`serial_test` 経由・allowed warning）— 継続  
4. **Stripe キー名混在** — **`[RESOLVED]`** OP-076（MCP/UI → `$STRIPE_API_KEY`）  
5. **App.tsx 786 行 god-shell** — **`[RESOLVED]`** Wave A2（456行 + shell 分割、/reflexion で型厳密化）

---

## 2. Top 5 Priorities

| # | 負債 | 深刻度 | 影響 | 見積もり | Status |
|---|---|---|---|---|---|
| **P1** | **Immune System Fail-Open**（**OP-075**） | 🔴 | — | — | `[RESOLVED]` 2026-07-10 |
| **P2** | **OP-054** 可視性（契約ギャップ DI は ADR-031 待ちに降格） | 🟡 | — | — | `[RESOLVED]` 可視性 / 契約ギャップは降格 |
| **P3** | **OP-051 Error 3 階層** | 🟡 | ADR-054 Proposed | 実装は承認後 | 継続（ADR 起草済） |
| **P4** | **OP-068 / OP-030・031・033・034**（**OP-032 ✅** 2026-07-22） | 🟡 | Upstream | 継続 | 継続（deny 残 8） |
| **P5** | **App.tsx god-shell**（HEX は OP-029 解消） | 🟡 | 分割完了 | — | **`[RESOLVED]` 2026-07-11**（456行 + navConfig/Sidebar/Header/Routes） |

---

## 3. Quick Wins（1時間以内）

| # | 修正内容 | ファイル | 効果 | Status |
|---|---|---|---|---|
| **QW-19** | `biome-popup-entry.tsx` / `biome-popup.html` の `#030712` をトークン化（**OP-029**） | `biome-popup-entry.tsx` | U-002 残件クローズ | `[RESOLVED]` 2026-07-10 |
| **QW-20** | MCP テンプレ / i18n の `$STRIPE_SECRET_KEY` → `$STRIPE_API_KEY`（**OP-076**） | discovery / i18n / example | 本番設定ミス防止 | `[RESOLVED]` 2026-07-10 |
| **QW-21** | `docs/licenses.json` から `html2md` stale 削除 | `docs/licenses.json` | ライセンス台帳整合 | `[RESOLVED]` 2026-07-10 |
| **QW-22** | webhook SSE の `let _ = sender.send` → `if let Err` ログ化 | stripe dispute/checkout + polar | 可観測性 | `[RESOLVED]` 2026-07-10 |
| **QW-23** | KC allowance 読取失敗時 `warn!`（fail-soft 維持） | `stripe.rs` invoice.paid | 課金経路の透過性 | `[RESOLVED]` 2026-07-10 |
| **QW-15** | `cargo update -p quick-xml` | `Cargo.lock` | — | `[RESOLVED]` |
| **QW-16** | biome-popup HEX（v10） | — | — | 未完了 → **QW-19** |
| **QW-17** | deep-scan watchtower 除外 | `deep-scan.sh` | — | `[RESOLVED]`（OP-052） |
| **QW-18** | `loop {}` 回避策除去 | `skills/mod.rs` | — | `[RESOLVED]`（OP-053） |

---

## 4. Findings Table（12次元・差分更新）

### 4.1 セキュリティ / エラー（Dim 7–8）

| 次元 | 指摘 | 対象 | 深刻度 | 工数 | Status |
|---|---|---|---|---|---|
| **Dim 8** | Immune System 評価失敗でツール実行継続（fail-open） | `apps/api-server/src/tool_call_router.rs:90-96` | 🔴 | 2h | `[NEW]` |
| **Dim 8** | SSE 経路でも Immune fail-open（コメント明記） | `apps/api-server/src/stream.rs:67-70` | 🔴 | 1h | `[NEW]` |
| **Dim 7** | Pro unlock 後 KC 付与: `get_setting_value` Err → `unwrap_or(None)` で静かにスキップ | `apps/api-server/src/routes/commerce_webhook/stripe.rs:341-346` | 🟡 | 0.5h | `[NEW]` |
| **Dim 7** | Stripe/Polar webhook SSE `let _ = sender.send`（invoice.paid はログあり） | `stripe.rs:440+`, `polar.rs:203+` | 🟡 | 1h | `[NEW]` |
| **Dim 8** | OP-024 MCP 課金 Fail-Closed | `tool_call_router.rs:301-316` | — | — | `[RESOLVED]`（健全） |
| **Dim 7** | commune_ws / heartbeat / expression silent | 各ファイル | — | — | `[RESOLVED]`（v10） |

### 4.2 アーキテクチャ / 契約（Dim 1–3）

| 次元 | 指摘 | 対象 | 深刻度 | 工数 | Status |
|---|---|---|---|---|---|
| **Dim 1** | App.tsx god-shell（786→456・分割済） | `App.tsx` + `navConfig` / shell コンポーネント | 🟢 | — | `[RESOLVED]` 2026-07-11 |
| **Dim 1** | `stream.rs` 823 行・テストモジュールなし | `apps/api-server/src/stream.rs` | 🟡 | 4h | `[NEW]` |
| **Dim 1** | `core_services.rs` 954 行（起動組み立て集中） | `apps/api-server/src/bootstrap/core_services.rs` | 🟡 | 4h | `[NEW]` |
| **Dim 1** | `skills/mod.rs` 1134→598（WasmSkillManager 残） | `libs/infrastructure/src/skills/mod.rs` | 🟢 | — | `[RESOLVED]` 降格 |
| **Dim 3** | JobQueue トレイト外 API（Publisher/FederationOps/EvaluationOps/CostOps/embedding） | `libs/infrastructure/src/job_queue/*`, `traits.rs` JobQueue | 🟡 | 3–6h | 継続 **OP-054** |
| **Dim 3** | 本番 `as any` 残 1（Tauri internals） | `apps/management-console/src/lib/api_resolver.ts:25` | 🟢 | 0.5h | 継続（低） |
| **Dim 3** | WorkflowBuilder `as any` ×4 | — | — | — | `[RESOLVED]`（OP-028） |

### 4.3 依存・設定（Dim 5）

| 次元 | 指摘 | 対象 | 深刻度 | 工数 | Status |
|---|---|---|---|---|---|
| **Dim 5** | `scc` RUSTSEC-2026-0205（unsound）via `serial_test`（dev） | `Cargo.lock` / `cargo audit` | 🟢 | 監視 | `[NEW]`（allowed） |
| **Dim 5** | `.cargo/audit.toml` ignore 多数（serenity/tauri 等・OP-068）。Issue C / OP-032 の wasmtime 系は削除済 | `.cargo/audit.toml`（Serenity/Tauri ブロック） | 🟡 | Upstream | 継続 |
| **Dim 5** | Stripe キー名混在（API_KEY vs SECRET_KEY） | `mcp/discovery.rs:72`, `i18n/*/385`, `docker-compose.production.yml:147`（Nurture 別系統） | 🟡 | 1h | `[NEW]` |
| **Dim 5** | quick-xml 脆弱性 | — | — | — | `[RESOLVED]` |

### 4.4 UI / テスト / ドキュメント（Dim 4, 9, 12）

| 次元 | 指摘 | 対象 | 深刻度 | 工数 | Status |
|---|---|---|---|---|---|
| **Dim 12** | biome-popup HEX `#030712` | `biome-popup-entry.tsx` | — | — | `[RESOLVED]` 2026-07-10 |
| **Dim 12** | HEX ゲート本体 | `test_ui_hex_violations.py` | — | — | `[GREEN]`（entry を `extra_files` に追加済） |
| **Dim 4** | `MockJQ` と `GlobalMockJobQueue`（`test_utils.rs`）二重 | `testing/mock_jq.rs`, `test_utils.rs` | 🟡 | 2h | `[NEW]` |
| **Dim 4** | `enforce_unwrap_deny.py` が `test_helpers.rs` の unwrap を 8 件報告 | `apps/api-server/src/test_helpers.rs:20-47` | 🟢 | — | 意図的（`#[cfg(test)]`）→ §5 |
| **Dim 9** | 未ドキュメント pub fn 合計 ~439（deep-scan） | 複数クレート | 🟢 | 継続 | 継続 |
| **Dim 9** | `docs/licenses.json` に html2md stale | `docs/licenses.json` | — | — | `[RESOLVED]` 2026-07-10（`cargo license` 再生成） |
| **Dim 9** | watchtower deep-scan パス | — | — | — | `[RESOLVED]` |

### 4.5 Zero-Panic / Tauri / tokens（Dim 10–12）

| 次元 | 指摘 | Status |
|---|---|---|
| **Dim 10** | `loop {}` パニック回避 | `[RESOLVED]` |
| **Dim 11** | Tauri IPC `as any`（api_resolver） | 低優先で残 |
| **Dim 12** | WebGL 元素 HEX 一括トークン化 | `[RESOLVED]`（OP-029/066） |

---

## 5. Things that look bad but are actually fine

- **`test_helpers.rs` の unwrap（8件）**: `main.rs` の `#[cfg(test)] mod test_helpers` ゲート済み。本番バイナリに含まれない。  
- **`preflight.rs` / bootstrap の `std::env::var(...).ok()`**: 任意シークレットの欠落は仕様。必須は別途 validate。  
- **robots.txt / DNS fail-open（tool_call_router）**: RFC / `AIOME_DEV_MODE` コメント付きの意図的設計。  
- **Stripe v2 thin event の 200 ACK（related_object なし）**: 再送防止の意図的 ACK。  
- **Nurture `STRIPE_SECRET_KEY`（compose L147）**: api-server 正本 `STRIPE_API_KEY` とは別系統（near_term v5.1 で文書化済み）。  
- **`serial_test` → `scc` warning**: **dev-dependency** のみ。本番ランタイム経路ではない。  
- **大ファイルのテスト比率**: `stripe` / workflow 系は従来どおりテスト比重が高い。`stream.rs` は例外（テスト薄い）。

---

## 6. Open Questions

1. ~~Immune System を Fail-Closed にするか？~~ → **解決**: Fail-Closed 採用（OP-075、degraded フラグなし）  
2. OP-054: `Publisher` / `FederationOps` 等を `JobQueue` 合成に含めるか、具象専用のまま封じるか？ → **方針固定**: 可視性のみ完了。合成は ADR-031 延期  
3. ~~biome-popup の `#030712` は `--bg-primary` と意図的に違う色か？~~ → **解決**: `--bg-base`（`#0b0d14`）に統一（2026-07-10）  
4. ~~MCP 例示の `$STRIPE_SECRET_KEY`~~ → **解決**: `$STRIPE_API_KEY` に統一（OP-076。Nurture 別系統は維持）  
5. `GlobalMockJobQueue` を `MockJQ` に統合してよいか（テスト影響範囲）？

---

## 7. メトリクス推移

| 指標 | v10.0 (2026-07-02) | **v11.0 (2026-07-10)** | トレンド |
|---|---|---|---|
| U-002 違反（ゲート） | 1（popup） | ゲート **0** / popup HEX **残** | ⚠️ 取りこぼし |
| `as any` 本番 | 5 | **1**（api_resolver） | ↓ |
| God Module（本番 1k+ 行） | 1（skills） | **0**（skills 598） | ↓ |
| `cargo audit` 新規 | quick-xml 等 | **scc allowed warning 1** | → |
| deep-scan Errors | 0 | **0** | ✅ |
| deep-scan Warnings | — | **6** | — |
| 未ドキュメント pub fn | 388 | **~439** | ↑ |
| `#[test]` / tokio::test 定義 | 1,348 | **~1,466** | ↑ |
| OPEN Wave 1/2 | 未完了 | **完了** | ✅ |
| OPEN Wave 3（OP-051/054） | 未着手 | **設計承認待ち** | → |

---

## 8. Git ホットスポット（過去3ヶ月・再計測）

| # | ファイル | コミット数 | リスク評価 |
|---|---|---|---|
| 1 | `api_integration_tests.rs` | 45 | テスト充実。低 |
| 2 | `bootstrap.rs`（分割済） | 43 | 起動経路。中（core_services 巨大） |
| 3 | `router.rs` | 41 | ルート列挙。低〜中（896 行） |
| 4 | `app_state.rs` | 33 | 適切なサイズ |
| 5 | `i18n/ja.json` / `en.json` | 30/29 | 文言。低 |
| 6 | `App.tsx` | 29 | シェル分割済（456行）。中（変更頻度は残る） |
| 7 | `samsara-hub/main.rs` | 28 | Federation 入口 |
| 8 | `stripe.rs`（commerce） | 26 | テスト厚い。低 |
| 9 | `settings.rs` | 24 | 設定面。中 |
| 10 | `AgentConsole.tsx` | 20 | UI 複雑。中 |

---

## 9. Public Beta との関係（スコープ注意）

直近の市場接触ゲート（`near_term_public_beta_plan.md` v5.1）は **Human 作業（NT-1 Vault 等）が律速**。P1（OP-075）/ OP-075-B / App.tsx 分割 / QW-19..23 / OP-054 可視性は **2026-07-10/11 実装済**。残る主戦場は OP-051 コード実装（ADR-054 承認後）・OP-068 Upstream（**OP-032 ✅**・残 OP-030/031/033/034）・Human NT-*（OP-059-UI ✅ 2026-07-13）。

---

*Generated by `/tech-debt-audit` workflow — 2026-07-10 v11.0（Wave 1/2 完了後差分・ホットスポット再計測・12次元再スキャン）。**消化注記 2026-07-11 /docs-sync**: OP-075-B / App.tsx 分割 / OP-076 を上表に反映。**2026-07-22 /reflexion**: OP-032 完了を P4/Dim5/§9 に反映。*
