# リリース・本番化マスタープラン（Release Master Plan v1）

- **ステータス**: 実装進行中（R0/R1/R2 コード完了 — R2-1 Human 待ち / R3-1・R3-2・R3-4 チェックリスト ✅ 2026-07-06 / R3-4 実走 Human 待ち）。**Wave 1/2（OP-024/060/061/067/069）完了**。**TECH_DEBT Top5 のうち OP-075/054/029/076 完了**（2026-07-10）。OP-051 は ADR-054 Proposed。
- **作成日**: 2026-07-05
- **最終更新**: 2026-07-11
- **目的**: 残存タスク全量を単一の実行計画に統合し、Public Beta リリースと本番化を「計画通りに実装すれば確実に完了する」状態にする。
- **正本関係**: タスク台帳の正本は `OPEN.md`。本計画は「リリースまでの実行順序・依存・完了基準」を定義する層であり、二重管理はしない（本計画の項目は必ず OP 番号で OPEN.md と対応させる）。
- **根拠**: OPEN.md 全量精査 + ロードマップ14本の未完了項目調査（サブエージェント）+ CHANGELOG 突合。調査報告のうち以下4件は**陳腐化**を確認済み: W-1 OXP relay（✅ CHANGELOG L79）/ ProUpgradeModal マウント（✅ OP-058）/ LP main push（✅ 2026-07-05）/ Biome Phase 5 P5-1〜P5-5（✅ CHANGELOG「面白さの核」+ ADR-049）。

---

## 0. ゴールとリリース定義

**リリース = Public Beta**（GitHub 公開リポジトリ + aiome.dev LP + Stripe 課金可能 + セルフホスト手順完備）。

| 判定基準 | 検証方法 |
|---|---|
| G1. 新規ユーザーが README だけで5分セットアップ→ログイン→チャットまで到達 | クリーン環境で Quick Start 実走 |
| G2. Free→Pro の課金導線が Day 1 で機能（LP決済 or アプリ内 Checkout → Pro 有効化） | R2 の E2E 手順 |
| G3. `/release-preflight` 全ステップ PASS（0 / 0.5 / 1–8 / 5.5 / 7.5） | ワークフロー完走ログ |
| G4. 既知のリリースブロッカー（P0/P1 台帳）ゼロ | OPEN.md 照合 |

**スコープ外（ポストリリース、本計画で追わない）**:
- `value_10x_roadmap.md` F-2〜F-10（Outcome Ledger / Marketplace α / Remote Access / Federation / Agency / Voice 等）
- 技術的負債系: OP-020〜023, OP-025〜027, OP-051（ADR-054 Accepted 後）, OP-062, OP-068, OP-059-UI
  - ✅ 完了済み（本計画スコープ外だったが消化）: OP-024 / OP-060 / OP-061 / OP-067 / OP-069（Wave 1/2）; **OP-075 / OP-054 / OP-029 / OP-076**（Top5・2026-07-10）; **OP-075-B / App.tsx シェル分割**（Wave A2/A3・2026-07-11）
- Upstream 待ち: OP-030〜034（`watch_upstream_blockers.py` 監視継続のみ）
- OP-040（OGP 動画）: 完全凍結のまま（解除条件はユーザー素材提供）
- 有償スキル販売・F-3 マーケット α: 法務（特商法・資金決済法）完了までスコープ外。**リリースは Mock 経済 + Pro サブスクのみで成立させる**

---

## 1. フェーズ構成（R0〜R5、依存順）

```
R0 台帳照合 ──→ R1 コード完成（UI残 + 小粒P1） ──→ R2 課金閉ループ（Safety-Critical）
                                     │                     │
                                     └──→ R3 本番インフラ検証 ←──┘
                                                │
                     R4 ローンチ資材（並行・ユーザー主導） ──→ R5 リリースゲート → 公開
```

- R1 と R4 は並行可。R2 は人間レビューが律速のため R1 中に計画書だけ先行提出。
- 各タスクの「担当」列: **Main** = メインエージェント（設計・Safety 境界）/ **Sub** = 低トークンサブエージェント（パターン確立後の横展開・機械的作業）/ **Human** = ユーザー実施（エージェント代行不可）。

---

## 2. Phase R0 — 台帳照合と計画の地ならし（0.5日）

「抜け漏れ・重複ゼロ」の土台。**コード変更なし**。

| ID | タスク | 対応 OP | 担当 | DoD |
|---|---|---|---|---|
| R0-1 | ✅ OPEN.md 重複採番の解消（OP-060/061 ×2組 → OP-067/068 に改番） | — | Main | 済（2026-07-05） |
| R0-2 | ✅ OP-001「v8.3」実体照合 → **クローズ**（17項目の正本リスト消失。CHANGELOG L840–854 + dbb2b92c/12196ad1 で実装確認済み。残課題は OP-002 / OP-066 に分散済み） | OP-001 | Sub | 済（2026-07-05） |
| R0-3 | ✅ Phase 3.5 照合 → **リリースブロッカーなし**（CRITICAL 3件すべて解消済み。残2件 = test_helpers.rs 未作成 + Federation アンスタブの追認 ADR → **OP-069 起票**、ポストリリース扱い） | OP-069 | Sub | 済（2026-07-05） |
| R0-4 | ✅ OP-057-R の凍結解除注記を OPEN.md に反映（Biome Phase 5 完了により条件充足。着手はユーザーの「実装しろ」承認後） | OP-057-R | Main | 済（2026-07-05） |
| R0-5 | 陳腐化ロードマップへのバナー追記（synergy_maximization / pr_quality の完了項目に「OPEN.md 正本」注記） | — | Sub | 各文書1行 |

**R0 結果サマリー（2026-07-05）**: R0-1〜R0-4 完了。照合の結論 — **本計画策定時点で未知のリリースブロッカーは存在しない**。OP-001 由来の隠れタスクなし、Phase 3.5 の CRITICAL（auth scrub_env / forecast SSRF / main.rs panic）は全て解消済み。R0-5 のみ残（R1 開始時に Sub へ委譲）。

---

## 3. Phase R1 — コード完成: UI 残タスク + リリース必須の小粒修正（3〜4日）

### R1-A: OP-066 UI Overhaul 残（`ui_overhaul_plan.md` v5 準拠、詳細仕様はそちらを正とする）

| ID | タスク | 担当 | DoD（計画書 §DoD 準拠） |
|---|---|---|---|
| R1-1 | **U2-6**: `ModelSetupStep.tsx` + テスト削除（参照ゼロ確認済み: 本体とテストのみ） | Sub | `rg ModelSetupStep src/` 0件・test PASS |
| R1-2 | **U2-5**: nav/page i18n 値の日常語化（キー名維持・値のみ変更、MESSAGING.md 整合） | Sub（対訳表は Main 作成） | Jest 修正ゼロで PASS |
| R1-3 | **U2-4**: `useAgentChat` Context 化 → StoryFlow/AgentConsole 統一（`variant` props）。**本計画最大の工数** | Main | タブ切替で履歴分断なし・両テスト PASS |
| R1-4 | **U3-2**: DESIGN.md ⇔ tokens.css 乖離解消（rose 値の文書内矛盾含む） | Sub | 差分ゼロ |
| R1-5 | **U3-3**: インライン style のクラス化（NurtureDashboard 起点） | Sub | lint PASS |
| R1-6 | **U4-0〜U4-6**: A2UI 実戦投入（catalog 登録・flag トグル・ナビ action・updateComponents 実装・ウィジェット） | Main（U4-4/5/6 の横展開は Sub） | A2UI E2E spec（新規 `e2e/a2ui.spec.ts`）+ Negative |
| R1-7 | **U5-6**: 共通 Modal プリミティブ + NavItem button 化 + aria-label 一括 | Main 設計 → Sub 横展開 | axe-core スモーク重大違反0 |
| R1-8 | **U5-7**: レスポンシブ最小対応（`--bp-*` トークン・ドロワー化） | Main | 375px 幅で横スクロールなし |
| R1-9 | **U5-8**: reduced-motion（`MotionConfig` ルート1箇所 + フック昇格） | Sub | OS 設定 ON で停止 |
| R1-10 | **U5-9**: DioramaView lazy 化 + chunk サイズ記録 | Sub | メイン chunk 削減を数値記録 |
| R1-11 | **U5-10**: animations.css 死活整理 | Sub | 未使用定義 0 |
| R1-12 | U1-3 残: ギフト/ギルドの FE が実装された時点で LockedOverlay 適用（**FE 自体が未実装のため現状対象なし** — 追加時の規約として ui_overhaul_plan Appendix A に記載済み） | — | 規約化のみ |

### R1-B: リリース必須の小粒修正（OPEN.md P0/P2 から選抜）

| ID | タスク | 対応 OP | 担当 | DoD |
|---|---|---|---|---|
| R1-13 | workflows validate パス不整合の修正（FE `POST /workflows/validate` → `/:id/validate`） | OP-056 | Sub | E2E で 404 消滅・Jest PASS |
| R1-14 | FE `as any` 4箇所解消（WorkflowBuilder ×3 / workflowConverter ×1） | OP-028 | Sub | `rg "as any"` 対象0・lint PASS |
| R1-15 | `deep-scan.sh` CRATES 修正（廃止 `apps/watchtower` 除外） | OP-052 | Sub | deep-scan 正常終了 |
| R1-16 | BiomeBackground + alpha:false の目視検証 | OP-002 | **Human** | ブラウザ確認の一言報告 |
| R1-17 | `.env.example` 追記（deep-scan CC-4 警告: `AIOME_API_HOST` / `MOCK_SUBSCRIPTION_STATUS`） | — | Sub | deep-scan Warnings 0 |

**R1 全体 DoD**: `npm run lint && npm test`（390+ PASS）・`cargo check --workspace --tests && cargo test --workspace`・`python3 scripts/test_ui_hex_violations.py` 0 違反・deep-scan Errors/Warnings 0。

---

## 4. Phase R2 — 課金閉ループ（Safety-Critical・人間レビュー必須、2〜3日 + レビュー待ち）

> 🔐 本フェーズ全体が Safety-Critical Zone（`commerce.rs` / Webhook / auth）。**実装前に本計画の R2 詳細設計書を提出し、ユーザーの明示的な「実装しろ」を得ること**。エージェントの自律変更は禁止。

| ID | タスク | 対応 OP | 担当 | DoD |
|---|---|---|---|---|
| R2-1 | 本番ホストへの秘密・非秘密反映（Vault: `STRIPE_API_KEY` / `STRIPE_WEBHOOK_SECRET`、非秘密: `STRIPE_TEST_MODE` / `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` / `VITE_STRIPE_PRICE_ID`） | OP-057-R (1) | **Human**（手順書 ✅ `stripe-production-setup.md` Vault 正本化 2026-07-10） | 本番 API が実 Price ID を返す + テスト決済 Pro unlock |
| R2-2 | 決済→セルフホスト Pro ライセンス自動有効化（Stripe Webhook → `register_license`） | OP-057-R (2) | Main + **人間レビュー** | Positive: テスト決済→Pro 反映 / Negative: 不正署名 Webhook 拒否 / Revert 確認 |
| R2-3 | 月間支出上限（W-7d）: DB マイグレーション（**要 ADR**）+ Settings UI 入力欄 | OP-059 残 | Main + **人間レビュー** | 上限超過購入がインターセプタで拒否される Negative Test |
| R2-4 | ✅ Stripe E2E の実行・結合確認 → **2026-07-10 PASS**（NT-4） | OP-013 | Main | 済（commerce 28 / e2e 2 / aiome-commerce 65） |
| R2-5 | ✅ Stripe Customer Portal 統合の要否判定 — **OP-010 CLOSED**（実 Stripe Billing Portal API。Mock ではない） | OP-010 | Main | 済（2026-07-06、ADR-051） |

**検証プロトコル**: 各項目とも AGENTS.md Verification Protocol の3段階（Positive / Negative 注入 / Revert）を必須とし、結果を CHANGELOG に記録する。

---

## 5. Phase R3 — 本番インフラ検証（2日、R1 と一部並行可）

| ID | タスク | 対応 OP | 担当 | DoD |
|---|---|---|---|---|
| R3-1 | ✅ PostgreSQL 本番構成での統合デプロイ検証（BAN 統合含む） | OP-012 | Main | 済（2026-07-06、`docker-compose.production-verify.yml` + `scripts/verify-production-postgres.sh` + `postgres_production_verify.rs`） |
| R3-2 | ✅ ローカル Keychain 移行の CLI 動作検証 | OP-014 | Main | 済（2026-07-06、`scripts/verify-keychain-cli.sh` — env ラウンドトリップ + macOS Keychain smoke） |
| R3-3 | ✅ `execute_autonomous_purchase` 封印解除の**リリース時判定**: Public Beta では封印のまま（自律購買はポストリリース。OPEN.md OP-011 に記録） | OP-011 | Main | 済（2026-07-06、コード不変） |
| R3-4 | クリーン環境 Quick Start 実走（G1 検証。Docker 1コマンド→ログイン→チャット） | — | **Human**（✅ チェックリスト `docs/guides/QUICK_START_VERIFICATION.md` 2026-07-06） | 5分以内に完走 |

---

## 6. Phase R4 — ローンチ資材（ユーザー主導・R1〜R3 と並行、5日）

| ID | タスク | 対応 OP | 担当 | DoD |
|---|---|---|---|---|
| R4-1 | 証拠ビジュアル7点 + Quick Start GIF 撮影（MESSAGING.md §8 ショットリスト。実データ・ダーク・1920×1080+） | OP-063 | **Human**（撮影手順書・画面準備は Main） | 7点 + GIF 1本 |
| R4-2 | LP / README への撮影素材組込 | OP-063 | Sub | LP ビルド PASS・デプロイ |
| R4-3 | ベータユーザー 5〜10人獲得 + 実名テスティモニアル | OP-064 | **Human** | 5名以上 |
| R4-4 | GitHub About / Topics / Social Preview 設定（MESSAGING.md §7） | PR-7 | **Human** | 設定完了 |
| R4-5 | Show HN / Product Hunt 投稿文ドラフト（MESSAGING.md SSOT 準拠、禁止表現リスト遵守） | — | Main | ユーザー承認済みドラフト |

---

## 7. Phase R5 — リリースゲート（0.5日、全フェーズ完了後）

| ID | タスク | 担当 | DoD |
|---|---|---|---|
| R5-1 | `/release-preflight` 全ステップ実行（0 / 0.5 / 1–8 / 5.5 / 7.5 — DAG・gitleaks・衛生・ローカルパス・URL・ビルド・ゲートテスト・サイズ・About・CHANGELOG・LICENSE）。実行順の正本は [`near_term_public_beta_plan.md`](near_term_public_beta_plan.md) NT-6 | Main | 全ステップ OK |
| R5-2 | CHANGELOG [Unreleased] のバージョン切り出し（現在200行超過 → ステップ7.5 必須） | Sub | [Unreleased] 200行以下 |
| R5-3 | ロールバック計画の明文化（Feature Flag 無効化・`git revert`・DB ダウングレード手順） | Main | リリースノートに記載 |
| R5-4 | ドキュメント最終同期（README/README_en・RIPPLE_MAP・OPEN.md クローズ処理） | Sub | docs-sync チェック PASS |
| R5-5 | タグ付与 + リリースノート + 公開 | **Human** 承認 → Main | GitHub Release 発行 |

---

## 8. 工数サマリーと実行順

| フェーズ | 見積 | 依存 | 並行性 |
|---|---|---|---|
| R0 | 0.5日 | なし | 即時（照合サブエージェント実行中） |
| R1 | 3〜4日 | R0 | R1-A と R1-B は並行。Sub 委譲比率 ~60% |
| R2 | 2〜3日 + レビュー | R1-B（R2 詳細設計は R1 中に先行提出） | 人間レビューが律速 |
| R3 | 2日 | R2-1（Vault+非秘密） | R3-1/2 は R1 と並行可 |
| R4 | 5日 | なし | 全期間並行（Human 主導） |
| R5 | 0.5日 | R1〜R4 全完了 | — |
| **合計** | **実働 8〜10日**（並行込みカレンダー ~2週間） | | |

## 9. トークン運用方針（サブエージェント委譲規約）

1. **Sub 委譲対象**: パターン確立済みの横展開（R1-2/4/5/9/10/11/13/14/15/17、R4-2、R5-2/4）と読み取り専用の照合（R0-2/3、R2-5 照合）。モデルは低トークン系を指定。
2. **Main 専任**: 設計判断（U2-4 Context 化・U4 A2UI・U5-6 Modal）、Safety-Critical 全て（R2）、ユーザー向け報告。
3. **委譲時の必須項目**: 対象ファイル絶対パス・DoD コマンド・「tokens.css へ新規 HEX 追加禁止」等の制約を prompt に明記（U3-1 委譲で実証済みのパターン）。
4. **並列上限**: 同時2体まで（テスト実行の競合防止。同一 package.json を触るタスクは直列化）。

## 10. リスクと対策

| リスク | 対策 |
|---|---|
| R2 人間レビューの遅延が全体を止める | R2 詳細設計書を R1 初日に提出し、レビューを R1 実装と並行させる |
| U2-4（チャット統一）のスコープ膨張 | `ui_overhaul_plan.md` §リスク4 の通り「Context 化 + variant 統合」のみ。ストリーミング仕様変更はしない |
| 法務（特商法・資金決済法）未完了 | リリースを Mock 経済 + Pro サブスクに限定（§0 スコープ外宣言）。有償 KC マーケットは非公開のまま |
| ~~OP-001 の実体が大量に残存していた場合~~ | **解消**（R0-2 照合完了: 隠れタスクなし、クローズ済み） |
| ベータユーザー獲得（R4-3）の不確実性 | リリースゲートの必須条件から外す（テスティモニアルなしでも公開可、後追い掲載） |
