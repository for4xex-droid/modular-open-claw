# 📋 OPEN.md — 未解決タスク台帳（Single Source of Truth）

**最終更新: 2026-07-20（OP-087 P1–P3 ✅）**

> **実装手順の正本**:
> - **MC 配布・ソース正本**: [`docs/roadmaps/mc_static_deploy_plan.md`](docs/roadmaps/mc_static_deploy_plan.md)（**v1.0 FINAL**・実装待ち。Path B 必須 / Human 許可→Agent 実行 / bind-mount 今期維持 / index スタブ→後日 untrack）
> - **Agentic 本番硬化（Human 後回し）**: [`docs/roadmaps/agentic_production_hardening_plan.md`](docs/roadmaps/agentic_production_hardening_plan.md)（**v1.3**・Wave A+B+D ✅・Wave C: **OP-051 ✅** / **OP-083 ✅**）
> - **OP-083-C/D x402**: [`docs/roadmaps/op083_cd_x402_plan.md`](docs/roadmaps/op083_cd_x402_plan.md)（**v1.0**・**C/D ✅ 2026-07-20**）
> - **OP-051 Error 3 階層**: [`docs/roadmaps/op051_error_hierarchy_plan.md`](docs/roadmaps/op051_error_hierarchy_plan.md)（**v1.0**・ADR-054 Accepted・**P1–P4 ✅ 2026-07-20**）
> - **課金クローズアウト**: [`docs/roadmaps/billing_closeout_plan.md`](docs/roadmaps/billing_closeout_plan.md)（**v1.5**・2026-07-18・R1–R4 / H4 / L5-3 **完了**）
> - **Human Wave 実行計画（残 NT の状態・順・DoD 一冊）**: [`docs/roadmaps/human_wave_execution_plan.md`](docs/roadmaps/human_wave_execution_plan.md)（**v1.2**・2026-07-14）
> - **Human 実行ランブック（NT-1〜7・コピペ超詳細）**: [`docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md`](docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md)（**v1.6**）  
> - **Agent アシスト（推奨・1ステップ進行）**: [`.agent/workflows/nt-assist.md`](.agent/workflows/nt-assist.md)（`/nt-assist`）+ [`scripts/nt_gate.py`](scripts/nt_gate.py)
> - **残存ワーク統合（Human + Agent・foolproof）**: [`docs/roadmaps/remaining_work_foolproof_plan.md`](docs/roadmaps/remaining_work_foolproof_plan.md)（**v1.2**・2026-07-10・Human NT-* 詳細化 + Agent `/perfect-plan` PASS）
> - **直近 Public Beta（Human ゲート中心）**: [`docs/roadmaps/near_term_public_beta_plan.md`](docs/roadmaps/near_term_public_beta_plan.md)（**v5.1**・2026-07-10・`/perfect-plan`+`/reflexion` 検証済み。秘密は AbyssVault、compose への API キー追加は禁止）
> - **技術負債 Wave 3 以降**: [`docs/roadmaps/remaining_tasks_implementation_plan.md`](docs/roadmaps/remaining_tasks_implementation_plan.md)（v6。Wave 1/2 完了済み）
> - **TECH_DEBT Top 5（OP-075/054/051/068/029+QW）**: [`docs/roadmaps/tech_debt_top5_plan.md`](docs/roadmaps/tech_debt_top5_plan.md)（**v1.3**・2026-07-10・実装完了。OP-054=可視性のみ）
> - **リリース全体**: [`docs/roadmaps/release_master_plan.md`](docs/roadmaps/release_master_plan.md)
>
> 本台帳は「何が未解決か」のみを管理する。手順の複製はしない。

## 運用ルール

- 未解決タスクは**このファイルのみ**で管理する（`memory/` の Open は当日分の追記メモであり、翌日以降はここへ反映する）。
- 各行は `- [ ] **ID**: 内容（初出日）` 形式。解決時はチェックを付け「✅ 解決」セクションへ移し、解決日と根拠（コミット/CHANGELOG）を1行添える。四半期ごとに解決済みを削除する。
- 凍結タスクは「⏸️ 凍結」セクションで管理し、解除条件を明記する。
- 実装の進め方・順序は直近は [`docs/roadmaps/near_term_public_beta_plan.md`](docs/roadmaps/near_term_public_beta_plan.md)、統合手順は [`docs/roadmaps/remaining_work_foolproof_plan.md`](docs/roadmaps/remaining_work_foolproof_plan.md)、技術負債 Wave 詳細は [`docs/roadmaps/remaining_tasks_implementation_plan.md`](docs/roadmaps/remaining_tasks_implementation_plan.md) に従う（本ファイルに手順を複製しない）。

## 🔴 P0 / ブロッカー

- [x] **OP-071**: `GET /api/v1/ekyc/status` 常時 500 → **2026-07-07 解決**（ユーザー承認のうえ U0-B1: `jwt_auth_middleware` を route layer 適用、U0-B3: status ハンドラから Stripe セッション作成除去、U0-B2: トーストデデュープ + パネル内再試行。CHANGELOG [Unreleased] 参照）
- [x] **OP-072**: UI 情報設計の確定改修（Phase U6）— **2026-07-07 完了**（U0-B + U6-1〜8 実装、Jest 394 PASS、hex ゲート GREEN、`sync:tokens` template ベース idempotent 同期対応）
- [x] **OP-001**: v8.3 リリースタスク（17個）の実装と検証 → **2026-07-05 クローズ**（照合結果: 17項目の正本リストは memory/2026-06-11.md に件数のみで列挙が現存せず。CHANGELOG L840–854 + コミット dbb2b92c/12196ad1 で実装をコード実体確認、2026-06-12 の memory Open ゼロ。残課題は OP-002 / biome_lenia_overhaul_plan / OP-066 に分散済みのため再定義不要）
- [x] **OP-002**: BiomeBackground + alpha:false 修正の目視検証（ブラウザ確認）→ **2026-07-13 完了**（NT-3 / LL-C / H-3: Human Positive PASS + Negative canvas 非表示確認。コックピット → そだてる → ワールド）

## 🟠 P1 / 次期リリース

- [x] **OP-070**: **リリース・本番化マスタープラン**（`docs/roadmaps/release_master_plan.md` v1）の実行 → **2026-07-14 Public Beta 公開完了**（R5-1〜R5-3 ✅ / **R5-5**: タグ `v1.2.0` + [GitHub Release](https://github.com/motivationstudio-llc/aiome/releases/tag/v1.2.0)。リポジトリは public。**方針 B（Live）は OP-084 で 2026-07-18 クローズ** / **closeout R4 完了**。残フォローは OP-064 / ポストリリース）。
- [x] **OP-078**: NT-2 /reflexion 残リスク §8 **R-A+R-B** → **2026-07-13 完了**（R-A: `--no-build` Generative FATAL → `--build` healthy；R-B 代理: クリーン volume Setup/login/Neg403/chat+MC proxy SSE → `down`。`nt_gate` browser=PASS）
- [x] **OP-077**: api-server release 向け unused import 掃除（`router.rs` OpenApi / `karma.rs` tracing を `cfg(debug_assertions)` 化）→ **2026-07-13 完了**（NT-2 §8 R-C）
- [x] **OP-079**: compose entrypoint の `/app/.intent_tmp` 二重 mkdir 撤去 → **2026-07-13 完了**（`/data/.intent_tmp` のみ・再作成後 healthy。NT-2 §8 R-D）
- [x] **OP-080**: Local LLM **Pattern B 実機検証**（`/reflexion` LL-A）→ **2026-07-13 完了**（Positive: `host.docker.internal` + `gemma4:26b` + health。Negative: `aiome-ollama` 併走で 11434 dual-bind リスク記録、stop 後 B 復帰。`pattern-a-up` で A 復帰 + `ollama:11434` / `gemma4:e4b`）
- [x] **OP-081**: Local LLM A/B + ViewMode + disk hygiene **git 分割コミット**（`/reflexion` LL-B）— `.env` 除外（2026-07-13 本コミットで反映）
- [x] **OP-083**: **Commerce レイヤー技術負債 v3.4** — **B→A→C→D ✅ 2026-07-20**。実行正本: [`op083_cd_x402_plan.md`](docs/roadmaps/op083_cd_x402_plan.md)。ゲート再定義（Q2+SC、ADR-053 非ブロッカー）。**日次0=無制限化禁止** 維持。broadcast / AiomeCoin 置換なし
  - [x] **OP-083-B**: Phase 2 spend_guard — **2026-07-13 完了**
  - [x] **OP-083-A**: Phase 1 supertrait — **2026-07-13 完了**
  - [x] **OP-083-C**: AgentWallet + Vault `X402_SIGNER_KEY` + 実署名 + Factory + DI — **2026-07-20 完了**
  - [x] **OP-083-D**: `OnChainAmount` / `currency.rs` — **2026-07-20 完了**
- [x] **OP-010**: Stripe Customer Portal 統合 — クレート追加、ポータル URL 生成エンドポイント新設（2026-05-28、HANDOVER.md P1-1）→ **2026-07-06 クローズ**（R2-5 照合: 実 Stripe Billing Portal API 実装済み。ADR-051）
- [ ] **OP-011**: `execute_autonomous_purchase` の封印解除 — Nurture /internal/purchase へのプロキシ実装（2026-05-28、HANDOVER.md P1-4）→ **R3-3 リリース判定（2026-07-06）: Public Beta では封印維持。自律購買（実通貨 A2C 購入）はポストリリーススコープ。コード変更なし**。**OP-086 後回し**
- [x] **OP-012**: PostgreSQL 本番環境での統合デプロイ検証（BAN 統合含む）→ **2026-07-06 完了**（R3-1: `docker-compose.production-verify.yml` + `scripts/verify-production-postgres.sh` + `postgres_production_verify.rs` — 3 DB マイグレーション + BAN ラウンドトリップ）
- [x] **OP-014**: CLI ツールを用いたローカル Keychain 移行動作検証 → **2026-07-06 完了**（R3-2: `scripts/verify-keychain-cli.sh` — abyss-vault set/get/delete + 非 whitelist 拒否 + macOS Keychain smoke）

## 🟡 P2 / 継続課題（技術的負債は REMAINING_TASKS.md 2026-07-02 版から吸収）

- [ ] **OP-087**: MC static 配布規律 — P1–P3 ✅（`sync_mc_static.sh` + `test_sync_mc_static.sh` P/N/R、`MC_STATIC_DEPLOY.md`、index スタブ）。**残**: P4 本番 Path B（都度 Human 許可）、§8 Q5/Q6。正本: [`mc_static_deploy_plan.md`](docs/roadmaps/mc_static_deploy_plan.md)
- [ ] **OP-020**: Phase 2b（Tauri シェル）✅ / Phase 4（経済接続）✅（CHANGELOG 根拠）。**Phase 5 製品 P2P 残**は要再定義（Federation **transport**=ADR-053 ✅。implementation_plan Phase 5=Cognitive Observability と番号衝突注意）。OP-083-C のブロッカーではない
- [ ] **OP-021**: BAN 管理ダッシュボード UI の設計検討（2026-05-22）
- [ ] **OP-022**: CausalVisualizer（Trajectory Graph の UI 可視化）未着手（MEMORY.md Blind Spots より）
- [x] **OP-024**: `tool_call_router.rs` 課金チェックの Fail-Closed 化（DB エラーを握り潰さず明示拒否）（MEMORY.md Phase 48 より）→ **2026-07-09 完了**（`get_setting_value` Err 時に MCP ツール拒否 + Negative テスト）
- [ ] **OP-026**: X Signal Probe 設定画面 UI（SettingsPage.tsx, settings.rs）（2026-04-07）
- [ ] **OP-027**: Stripe API 実装追加時の一元化モック拡充（2026-06-01）
- [x] **OP-028**: フロントエンド `as any` 型キャスト4箇所の解消（WorkflowBuilder.tsx ×3, workflowConverter.ts ×1）→ **2026-07-05 完了**（release_master_plan R1-14）
- [x] **OP-029**: `biome-popup-entry.tsx` HEX → `var(--bg-base)` + ゲート `extra_files` 追加 + html transparent → **2026-07-10 完了**（`test_ui_hex_violations.py` GREEN）
- [x] **OP-050**: `skills/mod.rs`（1,134行）God Module の責務分解 → 2026-07-03 完了（599行に縮小、code_mode.rs / host_fns.rs / types.rs へ分離。refactor/skills-god-module ブランチ）
- [x] **OP-051**: Error 3 階層 — **ADR-054 Accepted + P1–P4 ✅ 2026-07-20**（[`054-error-hierarchy.md`](docs/decisions/054-error-hierarchy.md) / [`op051_error_hierarchy_plan.md`](docs/roadmaps/op051_error_hierarchy_plan.md)）。契約トレイト `AiomeError` 化 / FactoryReset+subsystem `From` / 境界 anyhow 選択 map（`AppError::internal` 不透明・`QuarantineStore`）。一括置換なし。NurtureError core `From` は延期
- [x] **OP-052**: `deep-scan.sh` CRATES 設定修正（廃止済み `apps/watchtower` の除外）→ **2026-07-05 完了**（release_master_plan R1-15）
- [x] **OP-053**: `skills/mod.rs` L163 `unwrap_or_else(|_| loop {})` の安全なエラー伝搬への修正（Dim 10 違反） → 2026-07-03 完了（DUMMY_REGEX 削除、`LazyLock<Option<Regex>>` 化）
- [x] **OP-054**: JobQueue 補助 API 可視性（`with_llm` / `get_embedding_provider` → `pub(crate)`）→ **2026-07-10 完了**（計画 v1.3。契約ギャップ DI は除外）
- [x] **OP-075**: Immune Fail-Closed — `evaluate_security` + stream 初期 / agent_engine 集約。N1 router + N3 agent_engine（`immune_db_error_fail_closed`）+ Positive sentinel。N2 は共有経路で担保。→ **2026-07-10 完了**（運用注意: DB 障害時チャット拒否）
- [x] **OP-076**: MCP/i18n/example を `$STRIPE_API_KEY` に統一（Nurture `STRIPE_SECRET_KEY` は非変更）→ **2026-07-10 完了**
- [x] **OP-055**: `immune_system.rs` 内 MockJQ（約700行）の共有化 → 2026-07-03 完了（新クレートではなく `infrastructure::testing::mock_jq` クレート内モジュールとして抽出）
- [x] **OP-056**: フロント `useWorkflowApi` の `POST /api/v1/workflows/validate` とバックエンド `/api/v1/workflows/:id/validate` のパス不整合 → **2026-07-05 完了**（release_master_plan R1-13）
- [x] **OP-057**: LP Stripe 決済基盤（Payment Link URL・Price ID・ローカル `.env`）→ 2026-07-05 完了（`plink_1TpXHCBcUTwo5TwLnO1BJneY` / `price_1TpXFpBcUTwo5TwLmK9SQbKL`）。**本番反映は OP-057-R ✅ 2026-07-14**（(2) 決済→Pro コードは 2026-07-05 完了済み）
- [x] **OP-058**: `ProUpgradeModal`（402→アップグレード導線）→ 2026-07-04 解消（`App.tsx` ルートマウント + `STRIPE_PRICE_ID`）
- [x] **OP-059**: ハイブリッド価格のバックエンド実装（2026-07-03 部分完了）。✅ 月次 KC 含み枠 + ✅ W-1 OXP relay 修正 + ✅ **R2-3 月間支出上限**（ADR-050、DB マイグレーション、インターセプタ、Settings UI `economy.monthly_spend_limit`、2026-07-06） → **2026-07-10 docs クローズ**（コードは先行完了）
- [x] **OP-059-UI**: Settings への `pro_monthly_kc_allowance` 入力 UI 実装 → **2026-07-13 完了**（cockpit Commerce セクション、`SettingsPage.test` 16 PASS）
- [ ] **OP-062**: Tauri `NurtureMode::InProcess` variant — sidecar 起動と in-process の排他（ADR-012 残タスク）（2026-07-04）
- [ ] **OP-068**: deny.toml `[advisories].ignore` に登録した 21 件（wasmtime 41.x / rustls-webpki 旧版 / idna 0.4 / quick-xml 0.39 / rand 0.8）の解消。実体は OP-030〜OP-034 の Upstream 待ちと同根。上流更新後に ignore を削除すること（2026-07-04。旧 OP-061 重複採番を 2026-07-05 改番）
- [ ] **OP-064**: ベータユーザー 5〜10 人の獲得と実名テスティモニアル収集。launch（本格トラフィック獲得）の前提条件。バイラル32原則 #14/#29 対応（**Human**・OP-086 では後回し、2026-07-05）
- [x] **OP-065**: Pro 価格改定 $9.99 → **$19.99/月**（2026-07-05 ユーザー決定）。MESSAGING.md / LP i18n / README / ProUpgradeModal 表示 / stripe-setup.md / .env.example を同期。Stripe Payment Link・Price ID の実体差し替えは OP-057 に統合。
- [x] **OP-066**: UI 全体改善計画 — **2026-07-05 R1 完了**（U0–U5-B + U4 A2UI。Jest 392 PASS / hex 0 / deep-scan 0）。残: U2-4 の `variant` props 統合（任意・Context 化で履歴分断は解消済み）、U1-3 ギフト/ギルド（FE 未実装のため対象外）。OP-002 目視 ✅ 2026-07-13。
- [x] **OP-073**: **W2 ワークフロー実行エンジン本実装** — W2-0〜W2-8 完了（2026-07-08）。E2E 3本 PASS、`cargo test --workspace` PASS。Human SSRF Walkthrough A/B/C PASSED（2026-07-09）。
- [x] **OP-074**: **WorkflowExecutionTracker 再起動復旧** — **2026-07-09 完了**（`recover_orphan_executions` + store クエリ + 起動時呼び出し。CHANGELOG [Unreleased] 参照）

## 🔵 Upstream 待ち（scripts/watch_upstream_blockers.py で監視中）

- [ ] **OP-030**: serenity 0.13+ リリース待ち → `discord.rs` 改修で RUSTSEC-2026-0098 等を解除（Issue A）
- [ ] **OP-031**: bastion-core TLS/DNS 近代化（Issue A 完了後、idna 等 CVE 解除）（Issue B）
- [ ] **OP-032**: extism v1.22+ / wasmtime v43+ で Wasmtime CVE 解除（Issue C）
- [ ] **OP-033**: tauri v3.0.0+ で GTK4/unic CVE 解除（Issue D）
- [ ] **OP-034**: Tauri の `plist` 依存更新後、`.cargo/audit.toml` の quick-xml 無視設定（RUSTSEC-2026-0194/0195）を削除し `cargo update -p quick-xml`（2026-07-02）

## 🌱 Project-Nurture 側（経済・コンプライアンス）

- Nurture 側の残存タスク（TLA+ 形式仕様、VRAM 競合調停、On-memory DRM、Saga 補償設計、資金決済法対応、特商法表記、自律購買ポリシー、CP 報酬変換、コールドスタート対策等）は `REMAINING_TASKS.md` セクション3を参照（次回 Nurture 側スプリント時に本台帳または Nurture 側台帳へ正式移入する）。

## ⏸️ 凍結（解除条件つき）

- [ ] **OP-040**: OGP 画像（og:image）・プロモーション動画の埋め込み — **OGP 画像は 2026-07-09 解除・配置完了**（`docs/assets/logo/` + `docs/landing/public/ogp.png` / `aiome-hero-white.png`）。**プロモーション動画は凍結継続**（音声素材提供待ち）。

## ✅ 解決（直近のみ保持）

- [x] **OP-086**: **Agentic 本番硬化** → **2026-07-19 Wave D 本番クローズ**（正本 v1.3）。A1–A4 / B1–B3 コード ✅ + **Wave D**: key-proxy 本番再ビルド・B1 telemetry 反映・A1 Unauthenticated 0・Vault 整合。compose key-proxy に `CELL_ID` 追加。Wave C=ゲート待ち。CHANGELOG [Unreleased]
- [x] **OP-025**: key-proxy `caller_id` metrics/span + 401 構造化（秘密非出力）→ **2026-07-18**（OP-086 B1、`cargo test -p key-proxy` 34 PASS）
- [x] **OP-023**: ホットパス unwrap 棚卸し → **2026-07-18**（OP-086 B3、`enforce_unwrap_deny.py` 0）
- [x] **OP-082**: Pattern B Linux `extra_hosts` → **2026-07-18**（OP-086 B2）
- [x] **OP-084**: **実課金オープン（Stripe 方針 B / Live 切替）** → **2026-07-18 H4 PASS + L5-3 クローズ + R4 完了**（正本: [`live_billing_open_plan.md`](docs/roadmaps/live_billing_open_plan.md) v1.2 + [`billing_closeout_plan.md`](docs/roadmaps/billing_closeout_plan.md) **v1.5**）。L3-2 Vault `sk_live`/`whsec`・L3-3 `STRIPE_TEST_MODE=false`+live Price・L3-4 正本 Webhook 7 イベント＋legacy disabled・L4-1 Live Checkout→unlock・L4-2 偽署名 400・L4-3 `subscription.deleted`→suspend・L4-4 返金/クレジット+cancel→Free。closeout **R4 ✅**（空文字ガード本番同梱 + P/N/R）
- [x] **OP-085**: **法務ドキュメント整備** → **2026-07-15 エージェント完了 + 2026-07-16 公開検証完了**（正本: [`COMPLIANCE_CHECKLIST.md`](docs/legal/COMPLIANCE_CHECKLIST.md) §7）。Deploy Landing Page success・公開 bundle に特商法住所/電話・トライアル非提供・解約可文言。Payment Link に「14 days free」なし（`pk_test` = Test）。**Live プロフィール/領収書は OP-084** に移管。
- [x] **OP-063**: LP 用証拠ビジュアル（MESSAGING §8 / NT-5）→ **2026-07-14 撮影完了** + **R4-2 組込完了**（`docs/assets/evidence/2026-07-14/` 7/7、LP Showcase 実機画像、README/README_en 同期）
- [x] **OP-057-R**: OP-057 残タスク → **2026-07-14 完了**（**(1)** NT-1 / R2-1: `app.aiome.dev` 方針 A（Test）・Step0 distroless PASS・Vault（key-proxy）・Webhook7 + whsec・MC Checkout→PlanBadge Pro・Negative（`STRIPE_API_KEY` 削除→拒否→復元）。**(2)** 決済→Pro コードは 2026-07-05 完了済み）
- [x] **OP-002**: Biome `alpha:false` 目視（NT-3 / LL-C）→ **2026-07-13 完了**（Human PASS + Negative）
- [x] **OP-075-B**: Immune Fail-Open 残（napi・goal_processor・nurture-api MCP・skill_handler）の Fail-Closed 修正 → **2026-07-10/11 完了**（B1–B5 + N-B5 `test_execute_wasm_skill_immune_db_error_fail_closed` PASS。/reflexion 96）
- [x] **OP-013**: Stripe E2E テストの実行と結合挙動の継続確認（P2-2）→ **2026-07-10 完了**（NT-4: commerce 28 PASS / commerce_e2e 2 PASS / aiome-commerce 65 PASS。Positive: subscription checkout unlock・signature green。Negative: missing signature・production rejects test secrets）
- [x] **OP-061**: OXP `generate_header` 統一 + forget Bearer/`NURTURE_INTERNAL_SECRET` + stripe `require_oxp_header` fail-closed — **2026-07-10 完了**（ユーザー明示「修正」承認。CHANGELOG Fixed 2026-07-10、delete_account 5テスト + aiome-commerce generate_oxp_header 3テスト PASS。forget URL は `state.nurture_url`/`NURTURE_API_URL` 正本）
- [x] **OP-060**: coin-charge DLQ 自動再送 — **2026-07-09 完了**（`coin_charge_dlq_worker` + poison 隔離）
- [x] **OP-067**: html2md → htmd — **2026-07-09 完了**
- [x] **OP-069**: `create_test_app_state` + ADR-053 — **2026-07-09 完了**
- [x] **OP-071**: Nurture 品質最大化計画 v4（`docs/roadmaps/nurture_quality_max_plan.md`）— Phase A–E-0 + /reflexion + D-1 → **2026-07-07 完了**（CHANGELOG [Unreleased]、`cargo test --workspace` PASS、D-1 `CoinBalanceProvider` 集約）
- [x] **OP-900**: 異常ファイル名 `memory/2026-04-07.md\`` の整理 — 2026-07-03 解決（Lessons を正規版へマージし memory/archive/ へ移動）
