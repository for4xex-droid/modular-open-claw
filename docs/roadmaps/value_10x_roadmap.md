# Aiome 10x Value Roadmap — 価値を桁で変える機能ロードマップ

**作成日**: 2026-07-03（同日 /perfect-plan 検証済み・実在確認反映）
**位置づけ**: `implementation_plan.md`（v1.0.0 リリースロードマップ = 品質・安定化）の**次**を定義する成長ロードマップ。v1.0 の完成を置き換えるものではなく、その上に積む。
**前提となる既存計画**: Phase 3.5（Infra Remediation）→ Phase 4（Biome Reputation）→ Phase 5（Cognitive Observability）→ Release Preflight は本ロードマップの共通前提。OP-xxx 番号は `OPEN.md` を正本とする。
**検証状態**: 全10機能の「既存資産」記載は 2026-07-03 に rg による実在確認済み。各機能の「実在確認済み資産」「新規（既存なし）」の区別は本文に反映済み。

---

## 1. 価値分析: なぜ今の Aiome は「10分の1」なのか

現状の Aiome は技術資産（WASM スキルサンドボックス、Karma ハッシュチェーン、E2E フェデレーション、Mock/Stripe 経済エンジン、26画面の管理コンソール）に対して、**価値の刈り取り構造が3点欠けている**。

| 欠落 | 現状 | 10x の姿 |
|---|---|---|
| **ネットワーク効果ゼロ** | スキル・LoRA・ワークフローは各インスタンス内に閉じる | 作った資産が他ユーザーに流通し、作者に収益が還る |
| **価値の不可視** | エージェントの働きは Karma ログでしか見えない | 「今月◯時間・◯円分の仕事をした」が数字で見え、続ける理由になる |
| **導入の崖** | Docker/ソースビルド＋空のエージェントから手作業で育成 | 5分で「仕事をする状態」に到達するテンプレートと配布形態 |

戦略は「**既存資産の開放**」。新規の大型サブシステムをゼロから作る項目は意図的に排除し、すべての機能を既存クレート・既存画面の延長線に置いた。

---

## 2. ロードマップ全体像（3ホライズン・10機能）

```
Horizon 1: 価値の可視化と導入の崖の解消（v1.1〜v1.2）
  F-1 Agent Playbooks（業務テンプレート）
  F-2 Outcome Ledger（ROI ダッシュボード）
  F-3 Skill Marketplace α（署名付きスキル配布）
Horizon 2: ネットワーク効果の起動（v1.3〜v1.5）
  F-0 Secure Remote Access（F-4/F-7 の共通前提: リモート到達の公式手順）
  F-4 Aiome as MCP Provider（外部エージェントへの能力開放）
  F-5 Soul Sync（クロスデバイス人格同期）
  F-6 Proof of Agent Work（検証可能な作業証明）
  F-7 リモート承認（モバイル/PWA コンパニオン）
Horizon 3: プラットフォーム化（v2.0〜）
  F-8 Multi-Tenant Agency Mode（セル分離の商用化）
  F-9 開放経済圏（Gig/LoRA マーケットのノード間接続）
  F-10 Voice Interface（音声対話 → Voice Commerce 助走）
```

依存関係: F-3 → F-9、F-2 → F-6、F-4 → F-9、F-5 は ADR-053 transport ✅ + 製品 DoD（正本: [`wave_ui_p2p_tauri_plan.md`](wave_ui_p2p_tauri_plan.md) §3 / OP-020-F5）。F-10 は独立。

> [!IMPORTANT]
> **共通前提（/perfect-plan Gate 4 で摘出）**: api-server は現在 localhost ファースト設計である。**F-4（外部 MCP クライアント）と F-7（外出先からの承認）は「セルフホストサーバーへのセキュアなリモート到達」を暗黙の前提としており、これ自体が未実装**。H2 着手前に TLS 終端＋トンネル/リバースプロキシの公式手順（推奨構成のドキュメント化と E2E テスト）を先行タスク **F-0: Secure Remote Access** として切り出すこと。これを飛ばすと F-4/F-7 は「LAN 内でしか使えない機能」になり価値が半減する。

---

## 3. Horizon 1: 価値の可視化と導入の崖の解消

### F-1 Agent Playbooks — 業務テンプレートのワンクリック導入

> **実装済み（2026-07-03）**: `docs/roadmaps/f1_agent_playbooks_implementation_plan.md` に基づき実装完了。

- **何か**: 「SEO 運用」「SNS 運用」「競合リサーチ」「サポート一次対応」等の業務一式（ワークフロー定義＋必要スキル＋スケジュール＋承認ポリシー）を1つの Playbook としてパッケージ化し、SetupWizard 直後に選択導入できるようにする。
- **10x への寄与**: 「空のエージェント」問題の解消。初日から成果が出るため継続率と口コミが変わる。
- **既存資産（実在確認済み）**: `WorkflowBuilder.tsx` + `workflowConverter.ts`（GUI と変換器）、`routes/workflow.rs`（CRUD・`execute`・**`fork`・`validate`** 実装済み）、**`workflows` テーブルに `is_template` / `visibility`（community/marketplace）列が既に存在**（`libs/infrastructure/src/workflow/store.rs`）、`mcp/discovery.rs` の初回シード機構（`mcp_servers.json` 自動生成 — Playbook シードの参照パターン）、`routes/buzz`（SNS ドラフト）、`geo-optimizer`（SEO 監査）。
- **新規（既存なし）**: ワークフロー import/export エンドポイント、Playbook マニフェスト形式、SetupWizard の Playbook 選択ステップ（現状の SetupWizard は認証・SOUL 初期化のみでコンテンツ投入なし）。
- **主な作業**: Playbook マニフェスト形式（JSON、スキル依存・スケジュール・必要 MCP を宣言）の策定 / **`is_template` + `fork` を核にした**インポート・エクスポート API / SetupWizard への選択 UI / 公式 Playbook 4本の同梱（シードは `mcp/discovery.rs` パターンを踏襲）。
- **受け入れ基準**:
  1. クリーンインストール後、SetupWizard から Playbook「SEO 運用」を選択し、**5分以内**（モデル DL 除く）に最初のジョブが JobQueue に投入される。
  2. Playbook のインポートは依存スキル・MCP が欠けている場合に**具体的な欠落一覧を返して失敗**する（サイレント部分適用禁止）。
  3. `POST /api/playbooks/import` に不正マニフェスト（スキーマ違反・パストラバーサルを含む名前）を与えると 400 で拒否される Negative Test がテストスイートに存在する。
  4. 公式 Playbook 4本すべてが Mock Commerce モード（API キー無し）で完走する。
- **リスク**: テンプレートの品質が低いと逆効果。→ 各 Playbook に Quality Gate スコアの下限を設定して出荷判定。

### F-2 Outcome Ledger — エージェント ROI ダッシュボード

- **何か**: エージェントが完了した仕事を「節約時間・生成収益・処理件数」に換算し、ホーム画面に「今月の成果」として常設表示する。タスク種別ごとの換算係数はユーザーが設定可能。
- **10x への寄与**: 価値の証明。個人には継続動機、法人（Agency モード）には請求根拠を与える。全マネタイズ機能の説得力の土台。
- **既存資産（実在確認済み）**: `job_queue/trajectory_store.rs`（`trajectory_steps` に job_id/tool_name/reward_signal カラム）、`skill_arena.rs` の **`record_outcome()`（スキル成功/失敗・latency・karma_delta 記録済み）**、`routes/audit.rs` の `prompt-stats` / `diagnostics/summary`（集計 API の参照実装）、`score_tracker`（日次スナップショット）。
- **新規（既存なし）**: 成果台帳（Outcome Ledger）専用の集約 API と換算レイヤ。集計エンジンは新造せず上記3ソースの集約ビューとして実装。
- **主な作業**: trajectory + skill_arena outcome → 成果イベントの集計ビュー（SQL）/ 換算係数の Settings 項目 / HomePage ウィジェット / 月次サマリの export（CSV）。
- **受け入れ基準**:
  1. ジョブ完了時に成果イベントが記録され、`GET /api/outcomes/summary?period=month` が種別別の件数・換算値を返す（統合テストで検証）。
  2. HomePage に「今月: タスク N 件 / 節約 X 時間 / 相当額 ¥Y」ウィジェットが表示され、換算係数を Settings で変更すると**リロードなしで**再計算される。
  3. 換算係数が未設定の種別は金額表示せず件数のみ表示する（虚偽の金額を出さない）。
  4. CSV export に PII が含まれないことを確認するテスト（Phase 5 の Data Masking 方針に準拠）。
- **依存**: Phase 5（Cognitive Observability）のトレース基盤と重複させず、同じ trajectory ビューを共有する。

### F-3 Skill Marketplace α — 署名付きスキル配布と収益分配

- **何か**: ユーザーが作成した WASM スキル（＋メタデータ＋dry-run 用テストペイロード）を署名付きパッケージとして export/import できるようにし、Nurture マーケットプレイス基盤に「スキル」商品タイプを追加する。α版は公式キュレーション制（誰でも出品ではなく審査付き）。
- **10x への寄与**: ネットワーク効果の起点。スキルが増えるほど OS の価値が増え、作者に収益が還る循環を作る。
- **既存資産（実在確認済み）**: import 側パイプラインは完備 — `POST /api/skills/import`（URL import、SSRF 検証→`Cleanroom::process_import`→Forge）、`importer.rs`（.md/.yaml/OpenAPI の3層マニフェスト）、`actions_importer.rs`（GitHub Actions 変換）、`dry_run_skill`（Layer 3 隔離）、TypeState、`skill_maturity` テーブル（migration 20260423000000）。**商品タイプ enum `CommodityKind` に `WasmSkill` が既に定義済み**（`commercial/libs/commerce-protocol/src/commodity.rs`）。Ed25519 署名は samsara-hub / JWT / OxiLean で3実装が存在（流用元は `libs/shared/src/auth.rs` 系を推奨）。
- **新規（既存なし）**: export/download/share エンドポイント（import の対称 API として `SkillImporter` の逆方向シリアライズで実装）、パッケージ署名形式、SkillVault の「マーケット」タブ。
- **主な作業**: スキルパッケージ形式（.wasm + meta.json + Ed25519 署名）/ 署名検証つき import（検証失敗は即拒否）/ **`CommodityKind::WasmSkill` の販売フロー結線**（enum 追加は不要）/ SkillVault 画面に「マーケット」タブ。
- **受け入れ基準**:
  1. `aiome skill package` 相当の API でパッケージを生成し、別インスタンスで import すると**必ず Quarantined 状態で登録**され、dry-run 通過まで実行不可（TypeState が維持されている）。
  2. 署名が改ざんされたパッケージの import は検証エラーで拒否され、Aegis 監査ログに記録される（Negative Test 必須）。
  3. 購入フローは既存エスクロー経由で完結し、Mock Commerce モードでも全経路がテスト可能。
  4. `is_sensitive_path` 等のホスト防御をバイパスするスキルを意図的に作成して import し、実行時にブロックされる敵対テストが CI に存在する。
- **リスク**: 悪意あるスキルの流通 = 最大のブランドリスク。→ α版はキュレーション制、cleanroom AI 監査 + dry-run + 署名の3層を出荷条件とし、1層でも欠けたら公開しない。
- **依存**: OP-027（Stripe モック一元化）を先に消化。Safety-Critical Zone（commerce.rs）に触れる部分は人間レビュー必須。**法務前提**: 有償販売の開始には特商法表記ページと資金決済法対応（`REMAINING_TASKS.md` §3 の Nurture 側タスク）が先行完了していること。未完了の間は無償配布のみ。

---

## 4. Horizon 2: ネットワーク効果の起動

### F-4 Aiome as MCP Provider — 外部エージェントへの能力開放

- **何か**: 既存の MCP サーバー実装を「外部公開可能」な品質に引き上げ、Cursor/Claude 等の外部 AI クライアントから Aiome のスキル実行・Cortex 検索・ワークフロー起動を安全に利用できるようにする。認証はスコープ付き API トークン。
- **10x への寄与**: Aiome が「他のエージェントの道具箱」になる。単体アプリからインフラへの転換点であり、導入経路が npm/デスクトップ以外に広がる。
- **既存資産（実在確認済み）**: `mcp/server.rs`（`transcribe`・`cortex_search` + ホワイトリスト WASM スキル公開済み、認証は JWT Bearer）、`libs/shared/src/auth.rs` の `JwtAuthManager` + `Permission` enum（**RBAC/ABAC の `has_permission()` 実装済み**）、`routes/auth.rs` の OAuth 2.1 PKCE フロー（**`AuthorizeRequest.scope` フィールドは受理するが JWT claims に未反映** — ここが結線ポイント）。
- **新規（既存なし）**: PAT（Personal Access Token）の発行・失効・永続化レイヤ、scope → claims 反映、トークン管理 UI。認証基盤そのものは新造しない。
- **主な作業**: PAT 発行/失効 API + Settings UI / OAuth scope の claims 反映 / ツールごとの許可リスト / レート制限の外部クライアント適用 / 公開用ドキュメント。
- **受け入れ基準**:
  1. 外部 MCP クライアントから read スコープのトークンで Cortex 検索が成功し、**同じトークンでスキル実行を試みると 403** になる（スコープ分離の統合テスト）。
  2. トークン失効後のリクエストが即時拒否される。
  3. 高リスクツール（filesystem write・commerce 系）は外部トークンではデフォルト非公開で、有効化には管理画面での明示操作が必要。
  4. MCP ツール定義が既存の description 品質監査（V-C）をすべて PASS。
- **依存**: OP-024（tool_call_router Fail-Closed 化）を先に消化。

### F-5 Soul Sync — クロスデバイス人格同期

- **何か**: samsara-hub の E2E 暗号化チャネルで、Soul（人格・記憶・Karma）を複数端末（自宅デスクトップ⇄ノート）間で同期する。「どこでも同じ相棒」を実現する。
- **10x への寄与**: 記憶と人格こそ Aiome の乗り換え不能な moat。端末を跨げるようになると、この moat が日常の利便性として体感される。
- **既存資産**: `samsara-hub`（CRDT 同期・X25519+ChaCha20-Poly1305）、`soul_store`（L1-L3 永続化）、Cell Isolation Guard。
- **主な作業**: Soul スナップショットの差分同期プロトコル / 競合解決ポリシー（CRDT の適用範囲決定）/ ペアリング UI（QR/コード）。
- **受け入れ基準**:
  1. 端末 A で発生した経験（Experience）が 60 秒以内に端末 B の `soul_store` に反映される（2ノード統合テスト）。
  2. 中継ノード（samsara-hub）上でペイロードが常に暗号化されており、平文の Soul データが hub のログ・DB に存在しないことを検査するテストがある。
  3. 同一 Experience の二重適用が冪等に処理される（重複同期 Negative Test）。
  4. ペアリング解除後、相手端末からの同期要求が拒否される。
- **依存**: **ADR-053（Federation transport ✅）+ OP-020-F5 製品 DoD ✅ 2026-07-22**（S-1〜S-4。正本: [`h2_f0_f4_f7_implementation_plan.md`](h2_f0_f4_f7_implementation_plan.md) PART 4 / [`wave_ui_p2p_tauri_plan.md`](wave_ui_p2p_tauri_plan.md) §3）。OP-083 非ブロッカー。

### F-6 Proof of Agent Work — 検証可能な作業証明

- **何か**: Karma ハッシュチェーン＋OxiLean 形式検証を使い、「このエージェントがこの作業をこの品質で行った」ことを第三者が検証できる証明書（JSON + 署名）として export する。F-2 の Outcome Ledger と連動。
- **10x への寄与**: B2B の扉。AI の作業を外部に請求・納品する際の信頼インフラは競合に存在しない差別化。
- **既存資産（実在確認済み）**: Karma チェーン（改ざん困難ログ）、`OxiLeanProofCertificate`（`libs/aiome-core-contracts/src/oxilean.rs`、Nurture 連携で署名インフラ実装済み）、`shadow-worker`、`POST /api/skills/verify-proof` エンドポイント（検証系の参照実装）。
- **主な作業**: 証明書スキーマ策定 / export API / 独立した検証 CLI（`aiome-verify`、チェーン整合と署名を検証）。
- **受け入れ基準**:
  1. 任意の完了ジョブについて証明書を export し、`aiome-verify` が別マシン（DB アクセスなし）で PASS を返す。
  2. 証明書の1バイトを改ざんすると `aiome-verify` が FAIL を返す（Negative Test）。
  3. 証明書に含まれるのは作業メタデータのみで、プロンプト本文・PII が含まれないことを検査するテストがある。
- **依存**: F-2（成果イベントの定義を共有）。

### F-7 リモート承認 — モバイル/PWA コンパニオン

- **何か**: `TaskApprovalOverlay`（高リスク操作の人間承認）を PWA + Web Push でスマホから行えるようにする。外出中も自律運転が止まらない。
- **10x への寄与**: 現状、承認待ちでエージェントが止まる = 自律性の実質上限。承認のレイテンシが分単位になると、任せられる仕事の範囲が質的に変わる。
- **既存資産（実在確認済み）**: `TaskApprovalOverlay.tsx`、`routes/jobs.rs` の承認 API（`GET /api/v1/jobs/awaiting-input`、`POST /api/v1/jobs/:id/review` — `status=approved` で再キュー）。
- **新規（既存なし）**: **PWA / Service Worker / Web Push は現状ゼロ**（manifest.webmanifest・sw.js・VAPID の実装なし）。本機能は10機能中で唯一フロント基盤を新設する項目であり、見積りは他より厚めに取ること。
- **主な作業**: PWA 化（manifest + Service Worker）/ 承認専用の軽量ビュー / Web Push 通知（VAPID）/ 承認トークンの短命化・単回性。
- **受け入れ基準**:
  1. 高リスクジョブ発生から 10 秒以内にプッシュ通知が届き、スマホから Approve すると元のジョブが再開する（E2E テスト）。
  2. 承認リンクは**単回・有効期限 15 分**で、再利用・期限切れは 401（Negative Test）。
  3. 承認 UI はネットワーク断で明示エラーを出し、二重タップでも二重承認されない。
  4. Push 購読情報はセル内 DB のみに保存され、外部サービスへ承認内容の平文が送信されない（通知は「承認要求あり」のみ）。

---

## 5. Horizon 3: プラットフォーム化

### F-8 Multi-Tenant Agency Mode — セル分離の商用化

- **何か**: Cell-Based Architecture（1プロセス1セル）を活かし、1台のサーバーで複数クライアント分のエージェントを分離運用する管理機能（セルの作成/停止/課金メータリング/横断ダッシュボード）を提供する。`AiaaOnboardingWizard` の実体化。
- **10x への寄与**: 「代理店が顧客ごとにエージェントを運用する」B2B2C モデルの解禁。1ユーザー→1事業者×N顧客に単価構造が変わる。
- **既存資産（実在確認済み）**: Cell Isolation Guard、`AiaaOnboardingWizard.tsx`（UI 骨格）、`routes/admin`、Nurture サブスク基盤。メータリングの素材は部分実装あり — LLM は `llm/evaluation_logger.rs`（`prompt_evaluation_log`）+ `GET /api/v1/audit/prompt-stats`、ジョブ数は `job_queue/core_ops.rs` の `get_job_count_since` / `get_pending_job_count`。
- **新規（既存なし）**: セルのライフサイクル管理 API、**セル単位に統合されたメータリング API**（素材の集約レイヤ）、セル横断ダッシュボード。
- **主な作業**: セルのライフサイクル管理 API / セル横断の読み取り専用ダッシュボード / 既存カウンタを集約したセル単位使用量メータリング。
- **受け入れ基準**:
  1. セル A の管理者トークンでセル B のあらゆる API にアクセスすると 403（クロスセル遮断の網羅テスト）。
  2. セル作成→Playbook 導入→初回ジョブ完了が管理 API のみで完結する（ヘッドレス統合テスト）。
  3. 使用量メータが実際の JobQueue 実行数・LLM 呼び出し数と一致する（±0 の照合テスト）。
  4. 1セルのクラッシュ・OOM が他セルのジョブ実行に影響しないことを障害注入テストで確認。
- **依存**: OP-012（PostgreSQL 本番デプロイ）、F-1（Playbook）、F-2（メータリングの成果定義）。

### F-9 開放経済圏 — Gig/LoRA マーケットのノード間接続

- **何か**: 現在インスタンス内に閉じている Gig マーケット（AI 同士のタスク発注）と LoRA マーケットを、フェデレーションノード間で相互接続する。エージェントが他所のエージェントに仕事を発注し、エスクローで決済する。
- **10x への寄与**: MASTER_BLUEPRINT L3（Syndicate/Escrow Court）の実体化。エージェント経済圏そのものがプロダクトになる。
- **既存資産（実在確認済み）**: **Gig の状態機械はほぼ完成** — `libs/aiome-core-contracts/src/gig.rs`（`GigIntent`/`GigOrderStatus`/`AcceptanceCriteria`/`GigEngine` trait: 入札→エスクロー→納品→検証→決済）、`aiome-node` の `UniversalGigEngine` + `SecureGigGateway` + MCP ツール（`gig/publish`・`gig/capabilities`・`gig/status`）、`routes/commerce.rs` のエスクロー API、Cross-Service OXP Trust。
- **新規（既存なし）**: **samsara-hub 側には Gig・決済メッセージ型が存在しない**（Karma 連合と Arena マッチングのみ）。よって作業の本体は「新規プロトコル発明」ではなく「既存 GigEngine ⇄ samsara-hub federation ハンドラの橋渡し」。
- **主な作業**: samsara-hub への Gig メッセージ中継ハンドラ追加 / 相手ノードの信頼スコア連動（Phase 4 の KarmaForge を利用）/ 紛争時の自動返金ポリシー。
- **受け入れ基準**:
  1. 2ノード構成の統合テストで、ノード A のエージェントが発注した Gig をノード B が納品し、検収後にエスクローが解放される全経路が Mock Commerce で完走する。
  2. 納品期限超過時に自動返金される（タイムアウト Negative Test）。
  3. 信頼スコアが閾値未満のノードからの受託申請は自動拒否される。
  4. 全経済ミューテーションが OXP 署名検証を通過し、無署名リクエストは 401（Negative Test）。
- **依存**: Phase 4（Biome Reputation）、OP-011（自律購買封印解除）、F-3、F-4。**Safety-Critical Zone（commerce.rs・Webhook）に触れるため、実装フェーズは全項目人間レビュー必須**。

### F-10 Voice Interface — 音声対話と Voice Commerce の助走

- **何か**: 既存 TTS（SSE ストリーミング・Viseme 対応）に音声入力（ローカル STT サイドカー）を加え、デスクトップで「話しかけて頼む→アバターが答える」体験を完成させる。Blueprint L2 の Voice Commerce への助走。
- **10x への寄与**: アバター（VRM/Inochi2D）と TTS という既存投資が「見た目」から「インターフェース」に昇格する。デモ映え＝獲得コスト低下の効果も大きい。
- **既存資産（実在確認済み）**: **STT エンジンは既に実装済み** — `libs/infrastructure/src/whisper_transcription.rs`（`WhisperTranscriptionAdapter`、insanely-fast-whisper CLI を `RuntimeJail` 経由で実行、`TranscriptionEngine` trait + bootstrap DI 済み）、MCP ツール `transcribe`。TTS は `tts.rs` + `routes/voice.rs`（`?stream=true` で SSE）、Viseme は trait レベルで定義済み（OpenAI TTS は非対応、Mock は対応）。`ExpressionPipeline`・avatar-engine リップシンク。
- **新規（既存なし）**: STT の REST 公開層と管理コンソールのプッシュトゥトーク UI のみ。**Whisper エンジンの再構築・新サイドカー追加は不要**（当初想定より大幅に小さい）。
- **主な作業**: `TranscriptionEngine` の REST ラッパー / プッシュトゥトーク UI / 音声→AgentConsole パイプライン接続 / ローカル TTS プロバイダの Viseme 対応確認。
- **受け入れ基準**:
  1. プッシュトゥトークで日本語の指示を与え、テキスト起こし→エージェント応答→TTS 再生＋リップシンクまでが**エンドツーエンドでローカル完結**する（ネットワーク遮断状態のテストで確認）。
  2. 応答開始まで 3 秒以内（M シリーズ Mac 基準、ベンチマークを CI 外の手動チェックリストに記録）。
  3. 音声データがディスクに平文で残留しない（一時ファイルの削除検査）。
  4. STT サイドカーは `desktop_sidecar_manager.py` の物理検証（サイズ・マジックバイト）を PASS し、`tauri.conf.json` の externalBin 変更は T-003 チェックを実施。
- **依存**: なし（独立着手可能）。Tauri シェル変更部分は Safety-Critical Zone のため人間レビュー必須。

---

## 6. 優先順位の根拠（効果 × リスク）

| 機能 | 効果 | 実装リスク | 既存資産カバー率 | 推奨着手順 |
|---|---|---|---|---|
| F-1 Playbooks | 高（継続率） | 低 | 高 | 1 |
| F-2 Outcome Ledger | 高（全機能の土台） | 低 | 高 | 2 |
| F-3 Skill Market α | 最高（ネットワーク効果） | 中〜高（セキュリティ） | 中 | 3 |
| F-0 Secure Remote Access | —（F-4/F-7 の前提） | 中 | 低 | 4 |
| F-4 MCP Provider | 高（導入経路） | 中 | 高 | 5 |
| F-7 リモート承認 | 中〜高（自律性上限の解除） | 中（PWA 基盤新設） | 低 | 6 |
| F-6 Proof of Work | 中（B2B 差別化） | 低 | 高 | 7 |
| F-10 Voice | 中（体験・獲得） | 低〜中（STT エンジン実装済み） | 高 | 8（前倒し可） |
| F-5 Soul Sync | 高（moat） | 低（S-1〜S-4 ✅ 2026-07-22） | 中 | 9（製品 DoD 完了・MC UI 任意） |
| F-8 Multi-Tenant | 高（単価構造） | 高 | 中 | 10 |
| F-9 開放経済圏 | 最高（長期） | 最高（ただし GigEngine 状態機械は実装済み） | 中 | 11 |

---

## 7. やらないこと（本ロードマップのスコープ外）

1. **既存 v1.0 計画（Phase 3.5〜5、Release Preflight）の置き換え・先食い** — 品質基盤なしに機能を積まない。
2. **メタバース Zone（Blueprint L4）** — 前提となる L2/L3 が未成熟。
3. **独自ブロックチェーン/トークン発行** — Karma チェーンと OXP 署名で十分。規制リスクを負わない。
4. **クラウド SaaS 版の先行提供** — セルフホストの信頼性が価値の核。F-8 の成熟後に再検討。
5. **凍結中の OP-040（プロモ素材）への着手** — 凍結条件（完成版ロゴ・音声素材）は変わらない。

## 8. 計測（10x の定義）

各ホライズン完了時に以下を計測し、ロードマップの継続/軌道修正を判断する:

- **H1 完了時**: 新規セットアップ→初ジョブ完了の中央値時間（目標: 30分→5分）/ 7日後継続率
- **H2 完了時**: 外部 MCP クライアント経由の週間ツール呼び出し数 / 承認レイテンシ中央値（時間→分）
- **H3 完了時**: マーケットプレイス流通額（Mock 含むテストネット値でも可）/ 1事業者あたり運用セル数
