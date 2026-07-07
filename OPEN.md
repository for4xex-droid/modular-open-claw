# 📋 OPEN.md — 未解決タスク台帳（Single Source of Truth）

**最終更新: 2026-07-07**

## 運用ルール

- 未解決タスクは**このファイルのみ**で管理する（`memory/` の Open は当日分の追記メモであり、翌日以降はここへ反映する）。
- 各行は `- [ ] **ID**: 内容（初出日）` 形式。解決時はチェックを付け「✅ 解決」セクションへ移し、解決日と根拠（コミット/CHANGELOG）を1行添える。四半期ごとに解決済みを削除する。
- 凍結タスクは「⏸️ 凍結」セクションで管理し、解除条件を明記する。

## 🔴 P0 / ブロッカー

- [x] **OP-071**: `GET /api/v1/ekyc/status` 常時 500 → **2026-07-07 解決**（ユーザー承認のうえ U0-B1: `jwt_auth_middleware` を route layer 適用、U0-B3: status ハンドラから Stripe セッション作成除去、U0-B2: トーストデデュープ + パネル内再試行。CHANGELOG [Unreleased] 参照）
- [x] **OP-072**: UI 情報設計の確定改修（Phase U6）— **2026-07-07 完了**（U0-B + U6-1〜8 実装、Jest 394 PASS、hex ゲート GREEN、`sync:tokens` template ベース idempotent 同期対応）
- [x] **OP-001**: v8.3 リリースタスク（17個）の実装と検証 → **2026-07-05 クローズ**（照合結果: 17項目の正本リストは memory/2026-06-11.md に件数のみで列挙が現存せず。CHANGELOG L840–854 + コミット dbb2b92c/12196ad1 で実装をコード実体確認、2026-06-12 の memory Open ゼロ。残課題は OP-002 / biome_lenia_overhaul_plan / OP-066 に分散済みのため再定義不要）
- [ ] **OP-002**: BiomeBackground + alpha:false 修正の目視検証（ブラウザ確認）（2026-06-30）

## 🟠 P1 / 次期リリース

- [ ] **OP-070**: **リリース・本番化マスタープラン**（`docs/roadmaps/release_master_plan.md` v1）の実行。**R0/R1/R2 コード完了**（2026-07-05〜06）。**R3-1 OP-012 ✅** / **R3-2 OP-014 ✅** / **R3-4 チェックリスト ✅**（2026-07-06）。残: R2-1 本番 env（Human）/ R3-4 実走（Human）/ R4 ローンチ資材（Human）/ R5 preflight + 公開。
- [x] **OP-010**: Stripe Customer Portal 統合 — クレート追加、ポータル URL 生成エンドポイント新設（2026-05-28、HANDOVER.md P1-1）→ **2026-07-06 クローズ**（R2-5 照合: 実 Stripe Billing Portal API 実装済み。ADR-051）
- [ ] **OP-011**: `execute_autonomous_purchase` の封印解除 — Nurture /internal/purchase へのプロキシ実装（2026-05-28、HANDOVER.md P1-4）→ **R3-3 リリース判定（2026-07-06）: Public Beta では封印維持。自律購買（実通貨 A2C 購入）はポストリリーススコープ。コード変更なし**
- [x] **OP-012**: PostgreSQL 本番環境での統合デプロイ検証（BAN 統合含む）→ **2026-07-06 完了**（R3-1: `docker-compose.production-verify.yml` + `scripts/verify-production-postgres.sh` + `postgres_production_verify.rs` — 3 DB マイグレーション + BAN ラウンドトリップ）
- [ ] **OP-013**: Stripe E2E テストの実行と結合挙動の継続確認（P2-2）（2026-05-29）
- [x] **OP-014**: CLI ツールを用いたローカル Keychain 移行動作検証 → **2026-07-06 完了**（R3-2: `scripts/verify-keychain-cli.sh` — abyss-vault set/get/delete + 非 whitelist 拒否 + macOS Keychain smoke）

## 🟡 P2 / 継続課題（技術的負債は REMAINING_TASKS.md 2026-07-02 版から吸収）

- [ ] **OP-020**: Phase 2b（Tauri シェル）、Phase 4（経済接続）、Phase 5（Federation）未着手（2026-06-10、ロードマップ参照）
- [ ] **OP-021**: BAN 管理ダッシュボード UI の設計検討（2026-05-22）
- [ ] **OP-022**: CausalVisualizer（Trajectory Graph の UI 可視化）未着手（MEMORY.md Blind Spots より）
- [ ] **OP-023**: `infrastructure` コアに残存する一時的 `unwrap()` / ドキュメント警告のリファクタ（R-005 違反）（MEMORY.md Blind Spots より）
- [ ] **OP-024**: `tool_call_router.rs` 課金チェックの Fail-Closed 化（DB エラーを握り潰さず明示拒否）（MEMORY.md Phase 48 より）
- [ ] **OP-025**: `key-proxy` への Telemetry（`caller_id`）追加と Cross-Node Auth Reliability モニタリング（MEMORY.md Phase 4 より）
- [ ] **OP-026**: X Signal Probe 設定画面 UI（SettingsPage.tsx, settings.rs）（2026-04-07）
- [ ] **OP-027**: Stripe API 実装追加時の一元化モック拡充（2026-06-01）
- [x] **OP-028**: フロントエンド `as any` 型キャスト4箇所の解消（WorkflowBuilder.tsx ×3, workflowConverter.ts ×1）→ **2026-07-05 完了**（release_master_plan R1-14）
- [x] **OP-029**: `biome-popup-entry.tsx` HEX カラー直書きの解消（U-002 違反）→ **2026-07-05 完了**（OP-066 U3-1 として `lib/biome/*` 含む 13 ファイル・58 違反を一括トークン化。`test_ui_hex_violations.py` 0 violations）
- [x] **OP-050**: `skills/mod.rs`（1,134行）God Module の責務分解 → 2026-07-03 完了（599行に縮小、code_mode.rs / host_fns.rs / types.rs へ分離。refactor/skills-god-module ブランチ）
- [ ] **OP-051**: Error 型定義の統一（thiserror/anyhow 混在 10種類 → 3階層）（2026-07-02）
- [x] **OP-052**: `deep-scan.sh` CRATES 設定修正（廃止済み `apps/watchtower` の除外）→ **2026-07-05 完了**（release_master_plan R1-15）
- [x] **OP-053**: `skills/mod.rs` L163 `unwrap_or_else(|_| loop {})` の安全なエラー伝搬への修正（Dim 10 違反） → 2026-07-03 完了（DUMMY_REGEX 削除、`LazyLock<Option<Regex>>` 化）
- [ ] **OP-054**: JobQueue トレイトの API 乖離解消（補助メソッドのトレイト引き上げ or private 化）（2026-07-02）
- [x] **OP-055**: `immune_system.rs` 内 MockJQ（約700行）の共有化 → 2026-07-03 完了（新クレートではなく `infrastructure::testing::mock_jq` クレート内モジュールとして抽出）
- [x] **OP-056**: フロント `useWorkflowApi` の `POST /api/v1/workflows/validate` とバックエンド `/api/v1/workflows/:id/validate` のパス不整合 → **2026-07-05 完了**（release_master_plan R1-13）
- [x] **OP-057**: LP Stripe 決済基盤（Payment Link URL・Price ID・ローカル `.env`）→ 2026-07-05 完了（`plink_1TpXHCBcUTwo5TwLnO1BJneY` / `price_1TpXFpBcUTwo5TwLmK9SQbKL`）。**残（本番 env・決済→Pro 自動有効化）は ⏸️ 凍結 OP-057-R へ移管**（Biome ブラッシュアップと同一バッチ）
- [x] **OP-058**: `ProUpgradeModal`（402→アップグレード導線）→ 2026-07-04 解消（`App.tsx` ルートマウント + `STRIPE_PRICE_ID`）
- [ ] **OP-059**: ハイブリッド価格のバックエンド実装（2026-07-03 部分完了）。✅ 月次 KC 含み枠 + ✅ W-1 OXP relay 修正 + ✅ **R2-3 月間支出上限**（ADR-050、DB マイグレーション、インターセプタ、Settings UI `economy.monthly_spend_limit`、2026-07-06）
- [ ] **OP-060**: coin-charge DLQ（`outbox_dead_letters`）の自動再送機構 — 現状は手動再送のみ（2026-07-04）
- [ ] **OP-061**: OXP ヘッダ生成の stripe/auth/relay 3箇所を `generate_header()` へ統一リファクタ（Safety-Critical、人間レビュー後）（2026-07-04）
- [ ] **OP-062**: Tauri `NurtureMode::InProcess` variant — sidecar 起動と in-process の排他（ADR-012 残タスク）（2026-07-04）
- [ ] **OP-067**: `html2md`（GPL-3.0+、infrastructure 経由）を MIT 系代替（htmd 等）へ置換。暫定で deny.toml に例外を登録済み（2026-07-04。旧 OP-060 重複採番を 2026-07-05 改番）
- [ ] **OP-068**: deny.toml `[advisories].ignore` に登録した 21 件（wasmtime 41.x / rustls-webpki 旧版 / idna 0.4 / quick-xml 0.39 / rand 0.8）の解消。実体は OP-030〜OP-034 の Upstream 待ちと同根。上流更新後に ignore を削除すること（2026-07-04。旧 OP-061 重複採番を 2026-07-05 改番）
- [ ] **OP-069**: implementation_plan.md Phase 3.5 残2件 — (1) `test_helpers.rs`（`create_test_app_state()`）未作成（HIGH・開発基盤でありリリース非ブロッカー） (2) Federation 5メソッドのアンスタブ実装に対する ADR 記録が未作成（実装は CHANGELOG「Federation Unstubbing」で完了済み、方針変更の追認 ADR のみ残）（2026-07-05 照合で発見）
- [ ] **OP-063**: LP 用実プロダクト証拠ビジュアル撮影（MESSAGING §8 ショットリスト7点 + Quick Start GIF）。実データ・ダークテーマ・1920×1080 以上。バイラル32原則 #10 対応（ユーザー実施、2026-07-05）
- [ ] **OP-064**: ベータユーザー 5〜10 人の獲得と実名テスティモニアル収集。launch（本格トラフィック獲得）の前提条件。バイラル32原則 #14/#29 対応（ユーザー実施、2026-07-05）
- [x] **OP-065**: Pro 価格改定 $9.99 → **$19.99/月**（2026-07-05 ユーザー決定）。MESSAGING.md / LP i18n / README / ProUpgradeModal 表示 / stripe-setup.md / .env.example を同期。Stripe Payment Link・Price ID の実体差し替えは OP-057 に統合。
- [x] **OP-066**: UI 全体改善計画 — **2026-07-05 R1 完了**（U0–U5-B + U4 A2UI。Jest 392 PASS / hex 0 / deep-scan 0）。残: U2-4 の `variant` props 統合（任意・Context 化で履歴分断は解消済み）、U1-3 ギフト/ギルド（FE 未実装のため対象外）、OP-002 目視（Human）。
- [x] **OP-073**: **W2 ワークフロー実行エンジン本実装** — W2-0〜W2-7 完了（2026-07-08）。Conductor DI、依存ゲート、全 wf_* ノード、SubWorkflow 解消、execution tracker、FE prompt/approval/polling。残: W2-8 総合検証（Human 実走 + E2E 拡張）。
- [ ] **OP-074**: **WorkflowExecutionTracker 再起動復旧** — api-server 再起動後、Running 状態の `workflow_executions` を orphan 検知し Failed 確定または job 状態から再同期する仕組みが未実装。

## 🔵 Upstream 待ち（scripts/watch_upstream_blockers.py で監視中）

- [ ] **OP-030**: serenity 0.13+ リリース待ち → `discord.rs` 改修で RUSTSEC-2026-0098 等を解除（Issue A）
- [ ] **OP-031**: bastion-core TLS/DNS 近代化（Issue A 完了後、idna 等 CVE 解除）（Issue B）
- [ ] **OP-032**: extism v1.22+ / wasmtime v43+ で Wasmtime CVE 解除（Issue C）
- [ ] **OP-033**: tauri v3.0.0+ で GTK4/unic CVE 解除（Issue D）
- [ ] **OP-034**: Tauri の `plist` 依存更新後、`.cargo/audit.toml` の quick-xml 無視設定（RUSTSEC-2026-0194/0195）を削除し `cargo update -p quick-xml`（2026-07-02）

## 🌱 Project-Nurture 側（経済・コンプライアンス）

- Nurture 側の残存タスク（TLA+ 形式仕様、VRAM 競合調停、On-memory DRM、Saga 補償設計、資金決済法対応、特商法表記、自律購買ポリシー、CP 報酬変換、コールドスタート対策等）は `REMAINING_TASKS.md` セクション3を参照（次回 Nurture 側スプリント時に本台帳または Nurture 側台帳へ正式移入する）。

## ⏸️ 凍結（解除条件つき）

- [ ] **OP-040**: OGP 画像（og:image）・プロモーション動画の埋め込み — **完全凍結**。解除条件: ユーザーから完成版ロゴ・音声素材の提供。仮画像・プレースホルダーでの代用は厳禁（HANDOVER.md より）
- [ ] **OP-057-R**: OP-057 残タスク。**(1)** 本番ホストへの env 反映 — **Human 待ち** **(2)** 決済→Pro 自動有効化 — **2026-07-05 コード完了**（subscription checkout Webhook + customer upsert + MCP unlock + integration test）。本番デプロイ前に人間レビュー推奨。

## ✅ 解決（直近のみ保持）

- [x] **OP-071**: Nurture 品質最大化計画 v4（`docs/roadmaps/nurture_quality_max_plan.md`）— Phase A–E-0 + /reflexion + D-1 → **2026-07-07 完了**（CHANGELOG [Unreleased]、`cargo test --workspace` PASS、D-1 `CoinBalanceProvider` 集約）
- [x] **OP-900**: 異常ファイル名 `memory/2026-04-07.md\`` の整理 — 2026-07-03 解決（Lessons を正規版へマージし memory/archive/ へ移動）
