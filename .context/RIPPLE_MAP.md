# 🌊 Aiome Ripple Map

## Phase 2B-2 Foundation (Perfect Plan Rev.6 / Limit Break)
### 1. ゴーストバグ防止機構と SOUL 初期化API
- **変更内容**:
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `std::fs::read_to_string("SOUL.md")` という相対ハードコードパスを `resolver.resolve("SOUL.md")` へ完全置換。
    - `apps/api-server/src/routes/soul.rs` [MODIFY]: `POST /api/v1/soul/init` API エンドポイントを追加。
    - `apps/api-server/src/router.rs` & `settings.rs` [MODIFY]: `#[cfg(debug_assertions)]` 防壁を撤去し、Release ビルドでの接続テストおよびオンボーディングフローを解放。
    - `libs/infrastructure/src/soul_mutator.rs` [MODIFY]: `generate_initial_soul(name)` メソッド実装、および `transmute()` 内に不測の SOUL.md 喪失時の自己修復フォールバックを追加。さらに `ConstitutionalValidator` を用いた LLM 出力の禁止キーワード検証機構を統合。
    - `apps/management-console/src/components/OnboardingModal.tsx` [MODIFY]: 「Awaken System」時に `/api/v1/soul/init` をポーリング呼び出しし、リロード遷移によるメインシステムへの引き渡しを実装。
- **波及効果**:
    - **`OnboardingModal → /api/v1/soul/init → SoulMutator::generate_initial_soul → app_data_root/SOUL.md`**
    - 初回起動時における「人格（SOUL.md）不在」に起因するパニックを完全に排除。
    - 本番環境（Tauri パッケージング済）においても、設定画面での「Ollama/Gemini 接続テスト」が確実に機能し、セットアップ体験の断絶（404消滅）を防ぐ。

### 2. Phase 2B-2 Reflexion (Security / UX Audit)
- **変更内容**:
    - `apps/api-server/src/routes/soul.rs` [MODIFY]: `ai_name` に対する `64文字・改行排除` サニタイズと、空文字時の `Genesis` デフォルト切り替え（OOM/Injection 防御）。
    - `libs/core/src/security.rs` [MODIFY]: `ConstitutionalValidator::validate_text` にて、巨大文字列コピー `to_lowercase()` を廃止し、`once_cell` 相当の `std::sync::LazyLock` を用いた O(1) コンパイル済み Regex エンジン処理へと抜本的最適化。
    - `apps/management-console/src/components/OnboardingModal.tsx` [MODIFY]: 初期起動API障害時に発生するソフトブリック（暗黙クローズによる UX ロック）を `try/catch` + `errorMsg` ステート描画への変更により解消。
- **波及効果**:
    - 全体的なインフラストラクチャにおける DoS / OOM レジリエンスが向上。不正な長文や倫理違反キーワードによるエラーを事前にストリーム防壁としてブロックすることで、Agent 側の余計な演算を軽減する。

### 3. Phase 2B-2 Reflexion Part 3 (TDD Zero-Dependency & UI Tokens)
- **変更内容**:
    - `libs/infrastructure/src/skills/tests.rs` [MODIFY]: WASM サンドボックス環境 (Extism) の TDD 実証テストを復活（`#ignore` 解除）。
    - `libs/infrastructure/src/skills/test_data/hello_skill.wasm` [ADD]: Extism 用テストモックバイナリファイル。完全なオフライン検証（無依存 TDD）を実現するため `tests.rs` 内で `include_bytes!` として注入。
    - `apps/management-console/src/components/SettingsPage.tsx` & `OnboardingModal.tsx` [MODIFY]: ハードコードされていた HEX 色指定を全て `var(--text-inverse)` や `var(--accent-purple)` 等の CSS トークンへ置き換え。`OnboardingModal` 内の `@ts-ignore` による `window` グローバル変数の無骨な拡張を廃止し、React Hook `useState` へとリファクタリング。
    - `apps/management-console/src/styles/tokens.css` [MODIFY]: `--text-inverse`, `--bg-primary`, `--bg-inverse` トークンを追加。

### 4. Phase 2B-2 Reflexion Limit Break (Settings E2E Efficacy)
- **変更内容**:
    - `apps/api-server/src/routes/settings.rs` [MODIFY]: `view_mode` などの UI 制御値を `ALLOWED_KEYS` のホワイトリストに追加し、`SecurityViolation` 誤検知による保存失敗をホットフィックス。
    - `apps/api-server/src/internal_services/dream.rs` [MODIFY]: `DreamService` (TrendSonar) の初期化時、強制的に `.env` からしか環境変数を読み込んでいなかった仕様を改修し、優先的に `state.job_queue` (DB) から設定値を取得する E2E デリバリーを確立。
    - `apps/api-server/src/routes/soul.rs` & `api.rs` [MODIFY]: OpenAPI (`utoipa::ToSchema`) 定義の不足を修復。
- **波及効果**:
    - 設定変更 (SettingsPage) の結果が再起動後に確実に `DreamService` に波及し、環境変数に依存しないユーザー主導のトークンオーバーライドが正常稼働するようになった。
    - API ドキュメント生成プロセスが GREEN に復旧。

## Aiome Social Signal Integration (Layer A-C)
### 1. X Signal Probe (Reqwest Direct)
- **変更内容**:
    - `libs/infrastructure/src/x_signal_probe.rs` [NEW]: `TrendAdapter` トレイトを実装した X API 情報収集モジュール。インメモリレートリミッター（DashMap）を内包。
    - `libs/infrastructure/src/lib.rs` [MODIFY]: `x_signal_probe` モジュールを登録。
    - `apps/api-server/src/internal_services/dream.rs` [MODIFY]: `ExternalTrendSonar` のアレイに `XSignalProbe` を依存注入（`X_BEARER_TOKEN` 存在時のみ）。
    - `.env.example` [MODIFY]: `X_BEARER_TOKEN` の環境変数テンプレートを追加。
- **波及効果**:
    - **`x_signal_probe → trend_sonar → dream_state`**
    - 外部の複雑な MCP アーキテクチャを排し、依存関係（Impact Radius）を `trend_sonar.rs` 単体に抑え込むことに成功。DreamState 側の既存テスト（モック）に一切影響を与えずに、実環境でのみ X API が自律駆動するセキュアな設計。

## Chaos Engineering Infrastructure (Layer A-C)
### 1. フォルトインジェクション基盤 & カオステストスイート
- **変更内容**:
    - `libs/infrastructure/tests/common/chaos.rs` [NEW]: `ChaosMode` enum (EmptyResponse, Timeout, MalformedJson, GiantOutput, AlwaysFail) と `ChaosLlmProvider` (LlmProvider トレイト実装ラッパー) を `tests/` ディレクトリに完全隔離して新設。
    - `libs/infrastructure/tests/chaos_experiments.rs` [NEW]: 6つの定常状態仮説テスト (SoT×3, SamsaraEngine×1, CircuitBreaker×1, ConstraintChecker×1)。
    - `.agent/workflows/chaos.md` [NEW]: 「仮説→障害注入→検証→学習」の4フェーズワークフロー。
    - `.agent/workflows/god-mode.md` [MODIFY]: Phase 4 (Chaos) を Reflexion と Red-Team の間に挿入し5段階化。
- **波及効果**:
    - 本番コードへの変更 **ゼロ**。`tests/` 内でクローズするため、`src/` のASTグラフに一切影響しない。
    - `cargo check --workspace --tests` による既存テストへの干渉 **ゼロ** を物理的に検証済み。
    - `/god-mode` の品質パイプラインに「意図的な障害注入」フェーズが追加され、fail-open/fail-safe の区別が自動検証されるようになった。

## Phase D: Agent-Native Discovery & Hybrid Cortex FS (ADR-025)
### 1. Hybrid File System Projection
- **変更内容**:
    - `libs/infrastructure/src/cortex_file_projector.rs` [NEW]: `CortexFileProjector` を実装し、DBの `cortex_wiki_articles` をファイルシステム上の階層（`cortex_fs/`）へ投影。
    - カテゴリ名やタイトルが ASCII 以外の文字のみで構成されるシナリオへの Deterministic (16進数) スラグのフォールバックを完備し、上書きによる消失を防御。
    - 古い記事やゴミファイルのガベージコレクションを実装。
    - `libs/infrastructure/src/cortex_compiler.rs`: コンパイルサイクルの直後に `project_to_filesystem` を自動起動する仕組みを追加。
    - `libs/infrastructure/src/dream_state.rs`: `DreamState` へ `cortex_fs` のルートパスと探索用トップインデックス（カテゴリ一覧のみ）を注入するロジック (`build_cortex_fs_context`) の追加。
    - `apps/api-server/src/internal_services/dream.rs` & `bootstrap.rs`: 起動プロセスおよび Dream Service に Projector への DI（`Arc<CortexFileProjector>`）を追加。
- **波及効果**:
    - エージェントは RAG に依存せず、直接 `cortex_fs/` ディレクトリを `ls` や `grep`、`cat` で探査できるようになり、コンテキストの精度超過や幻覚リスクが物理的に抑止される。
    - カテゴリの `_concept.md` に一覧を遅延リンクさせる O(1) トークン設計により、Wiki 記事数が増大してもプロンプトバジェット制限が破綻しない強固な参照網が構築された。

## Phase D: Cortex Synth Pipeline Data Purity & LoRA Compliance
### 1. Belief Consistency Gate Integration
- **変更内容**:
    - `libs/infrastructure/src/cortex_synth.rs`: `CortexSynthesizer` に対して `BeliefConsistencyGate` のDIを追加。合成データ生成の内部ループ(`generate_dataset`)内で `check_belief_consistency` を呼び出し、矛盾データと改訂候補をベースラインから安全にパージするロジックを実装。また、JSON解析のエラーが秘匿される（`unwrap_or_default()`）現象を修正しロギングを強化。
    - `libs/infrastructure/src/cortex_synth.rs`: `export_to_jsonl` の吐き出しフォーマットを古いテキスト構造から、Axolotl等の現在の業界標準である **ShareGPT形式** (`{"conversations": ...}`) へ移行。シリアライズエラー時のフェイルセーフ追加。
    - `apps/api-server/src/routes/cortex.rs`: `synth_dataset_handler` にて `SoulStore` から Agentのコア信念（prompt_fragment, narrative_self）をロードし `BeliefConsistencyGate` へシードとして渡す初期化処理を統合。DBフェッチ失敗時のサイレント被覆問題(`_ => vec![]`)を修正。
- **波及効果**:
    - AIによるデータセットの自作自演（Synthetic Data Generation）において、自身の魂（Soul/Belief）に反するゴミデータが自動的に排除される Data Purity（データ純度）維持の機構が完成。
    - 出力フォーマットがシェアGPT形式になったことで、将来フェーズでの MLX / LoRA ファインチューニングプロセスとの完全な API/データ互換性が物理的に担保された。

## Precomputed Relational Intelligence & Agent Governance

## SEO Intelligence Pipeline (Phase B)
### 1. Task Decoupling & Guardrails Integration
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/seo_content.rs` [NEW]: SEO特化型の自律タスクコンダクター `SeoContentConductor` の新規追加。トピック境界値チェックと進捗イベントを含有。
    - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` に O(1) ではない O(N*C) だが汎用的な `get_conductor_for` を診断用に公開。
    - `apps/api-server/src/stream.rs`: MCPツールの出力をLLM履歴に反映する際、プロンプトインジェクション防御として `sanitize_for_prompt` を強制接合。
    - `apps/api-server/src/bootstrap.rs`, `app_state.rs`: `SeoContentConductor` および `PublishPipeline` のDIと起動時初期化（Bootstrap化）。
    - `apps/api-server/src/api_integration_tests.rs`: 新規コンダクターの登録を検証する TDD 結合テストと、`AppState` モックへの `publish_pipeline` 追加。
- **波及効果**:
    - SEOと汎用LLM（`data_processing`等）のタスクオーケストレーションが分離され、ドメイン用システムプロンプトの注入が安全になった。出力のパブリッシュ（デプロイ）に向けて `PublishPipeline` との連携基盤を確立。

### 1.5. Publishing Pipeline & External Analysis Integration (Phase B / Phase C)
- **変更内容**:
    - `libs/infrastructure/src/trend_sonar.rs`: `SerpAnalysisAdapter` の具象実装を追加。Tavily等の実APIコールとHTMLサニタイズ (`sanitize_snippet`) を行うと同時に、自律ループ枯渇防止目的のインメモリ・レートリミッター機能（10分に1回のみ）を内蔵。
    - `apps/api-server/src/internal_services/dream.rs`: `SEARCH_API_KEY` の環境変数が存在する場合のみ、`WebSearchAdapter` と結合した `ExternalTrendSonar` コンペティター分析ノードを `DreamService` にDI（注入）する仕組みへアップグレード。
    - `libs/infrastructure/src/publisher/wordpress.rs` [NEW]: `WordPressAdapter` の追加。WP REST API v2 を利用した記事自動投稿を実装（テストケース完備）。
    - `apps/api-server/src/bootstrap.rs`: `PublishPipeline` インスタンス化処理において `WP_API_URL` および `WP_API_TOKEN` をフォールバック付きで解決し、本番CMSと静的に繋ぐ仕組みを実装。Abyss Vault化は「将来フェーズへのTODO」としてマーク済。
- **波及効果**:
    - DreamService の自律ループからSEOギャップを取得する部分が完全自動かつレートリミット保護により自律破綻しないよう修正された。また、記事出力の終着点であるCMSシステム (WordPress) に対する直結が確立し、SEOインテリジェンス・パイプラインの最後のパズルが完成した。

- **変更内容**:
    - `scripts/nurture_auditor.py`: AST抽出のパフォーマンス欠陥(`rglob()`)を `os.walk()` とインプレース枝刈りにより真の O(1) ディレクトリ遮断へ改修。Rust, TSX, CSS の物理依存（エッジ）を重複排除付きで `.context/impact_graph.json` へ出力。
    - `scripts/impact_query.py` [NEW]: BFS アルゴリズムと `visited` セーフガードを用いた被害半径（Blast Radius: `[WILL BREAK]`）計算 CLI ツールの新規追加。テストファイル除外オプション付き。
    - `AGENTS.md` & `.agent/workflows/*`: 全ての主要ワークフローとAGENTS.mdコアディレクティブに対して、旧来のGitNexus等外部ツール統合ではなく、自前実装した `nurture_auditor.py` と `impact_query.py` を用いた「事前物理波及テスト（Mandatory AST Impact Analysis）」を義務化するパッチを適用。
- **波及効果**:
    - エージェント自律行動ループにおいて、コード変更前の「未知のカスケードエラー」や「Tailwind代替CSSの波及漏れ」が事前に高精度かつ数ミリ秒レベルで検出されるようになり、開発プロセスの防御力が格段に向上。外部ライブラリへの依存（サプライチェーンリスク）も排除。

## Analytics MCP Hub & Configuration Standardization
### 1. MCP Security Hardening & Config API
- **変更内容**:
    - `api-server/mcp/client.rs`: `McpClient::spawn` で使用されるインフラストラクチャーコマンドに対し、`npx`, `node`, `python3`, `uvx` などのMCP固有ホワイトリストと `@modelcontextprotocol/` などのパッケージ検証を追加し、BastionGuard バイパス（RCE可能性）を排除。
    - `api-server/mcp/discovery.rs`: `config.env` から `$` や式を利用した動的環境変数解決（サニタイズ込み）を実装。デフォル構成テンプレートの `tokio::fs::write` 化追加。
    - `api-server/routes/skill.rs`: `PUT /api/skills/mcp/config` と `GET /api/skills/mcp/config` を新設し、UIからの構成アップデートとリアルタイム再描画（MCPプロセスのホットリロード連動）を実現。SQLite との実用上の Desync 問題を解決。
    - `infrastructure/registry.rs`: `RegistryManager::clear_mcp_servers` を実装し、ゾンビツールプロンプト挿入問題を解消。
    - `management-console/src/components/SettingsPage.tsx`: フロントエンドに MCP 専用コンフィグマネージャー `McpConfigManager` を追加し、ダッシュボードへの直接的なフィードバックルーチン（`useSettings`）を構築。

## RTK Token Dashboard Update (Phase 3.5 Conclusion)
### 1. Token Savings SSE & State Propagation
- **変更内容**:
    - `management-console/src/hooks/useAgentChat.ts`: SSE メッセージハンドラ内で `token_saved` イベントを抽出し `aiome_vitality_event` として発行。
    - `management-console/src/App.tsx`: `useSystemVitality` 経由の CustomEvent リスナーから状態を受け取り、`sessionSavedChars` を全体ステートとして維持。
    - `management-console/src/components/common/TokenSavingsIndicator.tsx` [NEW]: `framer-motion` の `useSpring` を活用して文字数/トークン数を動的カウントアップする Premium なガラスモーフィズムコンポーネント。
    - `AgentConsole`, `StoryFlow`, `BiotopeView`: Props に `sessionSavedChars` を追加し、コンポーネント内にバッジ表示をバインディング。
- **波及効果**:
    - Agent 自律ループで発生した `OutputFilter` の節約メタデータが UI のアニメーションへ透過的かつ即時的に描画される。既存の Vitality Event Bus を流用したことで、チャット履歴と分離した安全な伝搬設計（疎結合）を実現。
- **波及効果**:
    - `McpClient`, `McpProcessManager` シグネチャが `HashMap<String, String>` (環境変数引数) を要求するように変更されたため、システムプロンプトの `discovery.rs` や `skill.rs` (Spawn MCP) ルーチンにも影響を波及させ一元化。
    - `.env.example` に GA4/Stripe 用のコメント追加が行われ、新規プロジェクトブート時のインテグレーションフローが公式化された。## Smart Model Bootstrap: 自律的LLMセットアップ基盤
### 1. Model Status API & Diagnostic Hardening
- **変更内容**:
    - `api-server/routes/model_setup.rs` [NEW]: `ModelStatusResponse` 構造体と `GET /api/v1/models/status` の REST + OpenAPI 定義・実装。Ollama接続状態や設定モデル (`gemma4:26b` など) の存在・利用可否を判定。SSRFガードレール (`validate_url`) を適用。
    - `api-server/router.rs`, `api-server/api.rs`: ルート登録と OpenAPI マージ。
    - `api-server/self_diagnosis.rs`: Docker接続失敗・Ollama到達不能時にプロセスを落とさず（`bail!` 廃止）、`tracing::warn!` に留める緩和措置を実施し、Ollamaモデル有無の診断を追加。
- **波及効果**:
    - `Management Console (Frontend)`: 初回起動時の `OnboardingModal` 内でこのAPIをポーリングし、ユーザーにセットアップ進捗を可視化。既存の E2E (`aiome_onboarding_done`) テストには影響ゼロ。
    - `Core CLI / Backend Services`: Docker不在でも稼働継続可能となり、外部の API ベース (OpenAI / Fal.ai) での限定稼働やローカル Mac Native 連携などの拡張性が向上。

## LoRA Marketplace: 安全なアダプター取引基盤
### 1. LoRA Marketplace Architecture
- **変更内容**:
    - `aiome-core-contracts/lora_marketplace.rs` [NEW]: `LoraListing`, `LoraPurchase`, `ListingFilter`, `LoraMarketplace` トレイトの型定義。
    - `infrastructure/lora_marketplace.rs` [NEW]: `UniversalLoraMarketplace` (SQLite/PG 対応) — エスクロー決済、SHA-256 ハッシュ検証、PathSandbox、楽観ロック、自己購入ブロック、500MB サイズ制限。
    - `infrastructure/lora_training.rs`: `AdapterFileInfo` 構造体と `get_adapter_info()` ヘルパー追加。
    - `api-server/routes/lora_market.rs` [NEW]: 6つの REST エンドポイント（出品一覧・出品・購入・完了・取下・自分の出品）。
    - `api-server/app_state.rs`: `lora_marketplace` コンポーネント追加。
    - `api-server/bootstrap.rs`: `UniversalLoraMarketplace` の DI 初期化。Commerce Engine の参照分離リファクタリング。
    - `api-server/router.rs`: `/api/v1/lora/market/*` ルート群の登録。
    - `api-server/api_integration_tests.rs`: AppState モックに `lora_marketplace` フィールド追加。
    - `infrastructure/migrations/sqlite/20260404000001_lora_marketplace.sql` [NEW]: テーブル + 監査トリガー。
    - `infrastructure/migrations/postgres/20260404000001_lora_marketplace.sql` [NEW]: PostgreSQL 版マイグレーション。
- **波及効果**:
    - `CommerceEngine` トレイト: **変更なし**。`escrow_create/release/refund` を呼び出すのみ。
    - `GigEngine`: **変更なし**。独立したトレイトとして並行動作。
    - `ArtifactStore`: **変更なし**。LoRA の来歴追跡は将来フェーズで `provenance edges` と連携可能。
    - Management Console: 将来フェーズで出品・購入 UI を統合予定。

## Phase B: Autonomous Chat Loop Hardening (ToolCallRouter Integration)

### 1. Unified ToolCallRouter Architecture
- **変更理由**: 複数の実行コンテキスト（自律ループ `agent_engine.rs`、チャットストリーム `stream.rs`、MCPプロトコル等）で重複していたツール実行とセキュリティ検証（Guardrails / Sentinel / HookChain）のロジックを一元化し、アーキテクチャ上の抜け穴（バイパス）を完全に塞ぐため。
- **波及効果**:
  - `apps/api-server/src/tool_call_router.rs` [NEW & UPDATED]: `ToolCallRouter` トレイトと `DefaultToolCallRouter` の完全実装。同期/非同期の双方で扱える共通の `ToolExecutionEvent` 列挙体を定義し、フックの適用（Allow/Deny/Transform）をプロキシとして包括。
  - `apps/api-server/src/tool_call_processor.rs`: LLMからの生の出力をパースし、直接手動でツール評価・実行を行なっていたロジックを `DefaultToolCallRouter` を経由する形に大幅リファクタリング（責務の分離とカプセル化）。
  - `apps/api-server/src/stream.rs` (SSE Handler): 中のツール実行ループについて、既存の冗長な実行ロジックを手動で回すのではなく、`ToolCallRouter::execute_skill` が返す Receiver を監視し、ストリームに `tool_result` イベントや `heartbeat`、エラーブロック通知を一元的にフォワードする設計へ移行。
  - `libs/infrastructure/src/immune_system.rs` & `guardrails`: `evaluate_security` の中で必ず `Guardrails` によるチェックと `AdaptiveImmuneSystem::verify_intent` の判定を並列または直列で完了させてからツール実行フェーズに進行するように統一。
## Phase C: Cortex Wiki Compiler & Query Engine Integration
### 1. CortexQueryEngine & Semantic Fallback
- **変更内容**: 
    - `infrastructure/cortex_query.rs`: `CortexQueryEngine` 構造体の追加。`query` および `suggest_questions` の実装と `Guardrails` によるプロンプトインジェクション防御。`QueryOptions` と `DisclosureLevel` (トークンバジェット L0-L3) の導入。
    - `api-server/stream.rs`: OOD (`is_ood == true`) 時のコンテキスト欠落に対し、`CortexQueryEngine` による意味検索フォールバックを組み込み。
    - `api-server/routes/cortex.rs` & `router.rs`: `GET /api/v1/cortex/suggestions`, `POST /api/v1/cortex/query` エンドポイント追加・登録、および OpenAPI (`api.rs`) 拡張。
    - `api-server/mcp/server.rs`: MCP ツール `cortex_search` をネイティブレイヤーに実装・公開し、外部エージェントからダイレクトにナレッジベース検索が可能に。
    - `infrastructure/context_engine.rs`: `ContextBudget` に `max_cortex_chars` 追加。
    - **Karpathy "LLM Wiki" Pattern**: `cortex_query` にて信頼スコア 0.7 以上の回答を `SourceType::Query` として自律的に再インジェスト (Query File-Back)。
    - **Activity Logging**: `cortex_ingester`, `cortex_compiler`, `cortex_query` の重要イベントを `cortex_activity_log` に追跡・永続化する監査基盤を追加。
- **波及効果**: 
    - `api-server/app_state.rs`, `bootstrap.rs`, および `api_integration_tests.rs` における `AppState` の初期化プロセスへ `cortex_query` の依存注入が波及。
    - チャットストリーム中の未知の質問（OOD）に対しても、CortexWiki から取得されたセマンティックな検索結果が提供されるようになり、Agent の自己学習サイクルが実稼働する基盤が完了。

## Phase F: Security Hook Framework & Precedence Execution

### 1. HookChain & AdaptiveImmuneSystem Precedence
- **変更理由**: エージェントが生成したツールコール（WASM実行、Forgeコマンド等）が実際に発火する前に、セキュリティポリシー違反やコンテキスト不整合をリアルタイムで検知・遮断（または変換）する中央集権型のインターセプター機構を確立するため。
- **波及効果**:
  - `libs/infrastructure/src/skills/hooks.rs` [NEW]: `ToolHook` トレイト、および `HookVerdict` (Allow/Deny/Transform)、並びにフック群を管理・直列評価する `HookChain` を実装。
  - `apps/api-server/src/agent_engine.rs` (現 `tool_call_processor.rs` へ移行): これまで点在していた `parse_tool_calls` や LLM出力を処理するロジックを抽出・再構築し、実行前に `AdaptiveImmuneSystem::verify_intent` -> `HookChain::execute_pre` を評価し、実行後に `HookChain::execute_post` を評価する制御フローを実装。
  - `apps/api-server/src/app_state.rs` & `apps/api-server/src/main.rs`: `AppState` に `hook_chain: Component<Arc<HookChain>>` を追加し、起動時に依存注入。
  - `apps/api-server/src/api_integration_tests.rs`: `AppState` のモック群全般に `hook_chain` フィールドを追加修正し、コンパイルエラーを解消。全59テストの疎通を回復。

## Phase 1C: Generative Engine Infrastructure Integration

### 1. Concrete GenerativeEngine Implementations
- **変更理由**: 画像・音声など複数モーダルな生成を可能にするため、ローカルで完結する `ComfyUiGenerativeEngine` とクラウド利用の `FalAiGenerativeEngine` の2つの具象実装を整備し、`GenerativeEngine` トレイトを通じた一元的な生成基盤を構築するため。
- **波及効果**:
  - `libs/infrastructure/src/generative_engine.rs` [NEW]: コンポーネントおよびテストモック（`MockGenerativeEngine`）を含む新規モジュール作成。
  - `libs/infrastructure/src/lib.rs`: 新規モジュール `generative_engine` をエクスポート。
  - `apps/api-server/src/app_state.rs`: `AppState` に `generative_engine: Component<Arc<dyn aiome_contracts::traits::GenerativeEngine>>` を追加。
  - `apps/api-server/src/main.rs`: 環境変数 `GENERATIVE_ENGINE` (comfyui / falai) に応じたエンジン初期化と注入ロジックを追加。プロダクション (`--release`) 環境での無指定時にはセキュアなフェイルファスト (`std::process::exit(1)`) を実装。
  - `apps/api-server/src/api_integration_tests.rs`: テスト環境用の `AppState` モック設定に `MockGenerativeEngine` の初期化処理を統合し、全エンドツーエンドテストの疎通を回復。

## Phase 1B: Avatar Engine & Infrastructure Hardening

### 1. Storage DoS Protection (DiskQuotaManager)
- **変更理由**: 大量または巨大なファイルの連続アップロードによるストレージ枯渇 (DoE) 攻撃を防ぎ、システム全体の安定稼働とマルチテナント環境下での公平なリソース配分を保証するため。
- **波及効果**:
  - `libs/infrastructure/src/disk_quota.rs` [NEW]: `DiskQuotaManager` 構造体を実装し、エージェントごとのディスク使用量をリアルタイムでトラッキング・制限 (デフォルト500MB) する仕組みを構築。
  - `apps/api-server/src/app_state.rs`: `AppState` に `DiskQuotaManager` コンポーネントの依存注入を追加。
  - `apps/api-server/src/routes/voice.rs`: `upload_voice_handler` 内での検証ロジックを更新し、アップロード開始前にクォータ超過を検知して 413 Payload Too Large エラーを返すよう改修。
  - `apps/api-server/src/api_integration_tests.rs`: 全ての統合テスト環境における `AppState` モック初期化処理に `disk_quota` フィールドを追加修正し、コンパイルエラーを解消。

### 2. TTS Streaming Optimization
- **変更理由**: 長文のテキスト読み上げにおいて音声生成完了まで待機することによる TTFB (Time to First Byte) の遅延を解消し、リアルタイムで低遅延な対話体験 (ストリーミング応答) を実現するため。
- **波及効果**:
  - `libs/aiome-contracts/src/traits.rs`: `TtsProvider` トレイトを非破壊的に拡張し、新たに `synthesize_stream` メソッド（デフォルト実装付き）を追加。
  - `libs/infrastructure/src/tts.rs`: `OpenAiTtsProvider` に `synthesize_stream` をオーバーライド実装。`reqwest` の `bytes_stream` を用いて音声チャンクデータを逐次的に返却。
  - `apps/api-server/src/routes/voice.rs`: `synthesize_voice_handler` で `stream=true` クエリパラメータを受け取り、`axum::body::Body::from_stream` を用いて chunked 転送を行う分岐を追加。

### 3. LipSync Expansion (SimpleLipSyncEngine)
- **変更理由**: Inochi2D や 3D アバターを駆動するための動的な口形（Viseme）データを外部に依存せず生成可能にし、自律的なリップシンクアニメーションの基盤をシステム内に統合するため。
- **波及効果**:
  - `libs/avatar-engine/src/lip_sync.rs`: 既存の `LipSyncProvider` トレイト実装として `SimpleLipSyncEngine` 構造体を新規追加。
  - オーディオ生データからのダミーフレーム生成（一定間隔での口の開閉と `Viseme` のトグル）および、文字起こしセグメント (`TranscriptionSegment`) に基づくフレーム補完ロジックを実装。単体テストを併せて追加。

## Phase 1A-2: Dynamic Dataset Extraction (MLX Data Pipeline)

### 1. SoulStore to MLX JSONL Pipeline
- **変更理由**: これまで `LoraTrainingService` は学習用データセットが物理ファイルシステム（`/tmp`等）に事前に配置されている前提で稼働していましたが、実際の運用では `SoulStore` の記憶DBから動的にExperienceを抽出・成形する必要があるため。
- **波及効果**:
  - `libs/infrastructure/src/dataset_extractor.rs` [NEW]: `DatasetExtractor` 構造体を新規作成。`&dyn SoulStore` から JSON 全体をロードし、`experiences` を個別の行に分割・破壊せず単一の会話ブロックとして連結したまま維持。さらに抽出ファイルを `job_id` でユニーク化することで完全なファイルI/O競合回避を実現し安全に `jsonl` 形式でダンプする。
  - `libs/infrastructure/src/lora_training.rs`: `train` メソッドの開始時に `DatasetExtractor` を生成し、抽出に成功した場合は生成された `output_file` パスを `LoraTrainingConfig` に流し込み、失敗した場合はフォールバックとして `dataset_id` を直接の生ファイルパスとして扱うロジックに変更。

## Phase 1D-1: Global Compute Semaphore (Hardware Protection)

### 1. Unified Memory OOM / Kernel Panic 防御
- **変更理由**: GenerativeEngine (画像生成) や LoraTrainingService (LoRA学習) などの重いML計算が並列に実行されると、MacのUnified Memoryが枯渇しシステムレベルのクラッシュ（カーネルパニック）を引き起こす脆弱性 (F-3) を解消するため。
- **波及効果**:
  - `apps/api-server/src/app_state.rs`: グローバルな `compute_semaphore: Component<Arc<tokio::sync::Semaphore>>` を共通のロック機構として追加。
  - `apps/api-server/src/main.rs`: 許可枠 1 (`Semaphore::new(1)`) で初期化し、`AppState` に加えて `LoraTrainingService` へ静的に依存注入。
  - `apps/api-server/src/api_integration_tests.rs`: `MockCommerceEngine` のトレイトエラー修復 (`deduct_generation_cost`) と共に、セマフォ初期化・検査テスト (`test_compute_semaphore_limits_concurrency`) を実装。
  - `libs/infrastructure/src/lora_training.rs`: これまで独自に保持していたセマフォを廃止し、コンストラクタ経由で受け取った共通の `compute_semaphore` を利用して他の高負荷タスクと排他制御されるように改修。

## Phase F: Open Gateway (MCP Integration)

### 1. Secure Gig Gateway & MCP Server
- **変更理由**: Agent P2P プロトコルの設計思想に基づき、外部エージェント（Claude Code 等）からタスクを受注する機能を追加しつつ、ネットワーク越しに攻撃されるリスクを多層的に防御するため。
- **波及効果**:
  - `libs/infrastructure/src/gig_gateway.rs` [NEW]: `SecureGigGateway` 構造体を追加。3層のフィルタリング（Rate Limit, Constraint Validation, Constitutional Verification）を通過した要求のみを内部の `GigEngine` に委譲。
  - `apps/aiome-node/src/mcp_server.rs` [NEW]: 標準入出力を利用した JSON-RPC ベースの MCP サーバーを実装。
  - `apps/aiome-node/src/main.rs`: `mcp` コマンドライン引数を検知した際に独自の TCP Node ではなく MCP サーバーを起動するよう起動パスを分岐。

### 2. Auto-Profile Engine
- **変更理由**: ノードの提供可能なスキルや機能（Capabilities）を手動で登録する手間を省き、実行環境（依存パッケージやリポジトリ）の構成から安全かつ自動で検出するため。
- **波及効果**:
  - `libs/infrastructure/src/auto_profile.rs` [NEW]: 許可リストベースのヒューリスティックによるスキャン・ロジック（`AutoProfileEngine`）を追加。
  - `libs/aiome-contracts/src/a2a/agent_card.rs`: `AgentCard` 構造体に `capabilities` フィールドを追加。
  - `apps/aiome-node/src/routes/agent_card.rs`: `get_agent_card` API が、ハードコードから動的な環境スキャンへ遷移し、メタデータに能力を含めるよう改修。

## Phase 52: LoRA Archiving & Secure Training Pipeline (MVP/TDD)

### 1. LoRA Metadata Archiving over Generation (Rebirth)
- **変更理由**: Rebirth (転生) 時に旧世代のLoRA設定を引き継ぐと、過剰適応によるデータポイズニング（データ汚染）のリスクがあるため。旧世代のLoRA設定は隔離記録（Archive）に残し、新世代は白紙から学習するように変更。
- **波及効果**:
  - `libs/infrastructure/migrations/` に `archived_lora_models` 追加の SQL スキーマ定義。
  - `libs/aiome-contracts/src/traits.rs`: `SoulStore::archive_lora_model` メソッドを新規追加。
  - `libs/infrastructure/src/job_queue/soul_store.rs`, `libs/infrastructure/src/soul_store.rs`: `archive_lora_model` を実装（他のモック等も更新）。
  - `libs/infrastructure/src/samsara_engine.rs`: `DefaultSamsaraEngine::rebirth` に `SoulStore` インスタンスを（Option で）注入し、アーカイブ処理を挟んだ後、`new_soul.lora_hash`, `lora_adapter_path`, `lora_base_model` を `None` にリセット。

### 2. Secure LoRA Training Execution (LoraTrainingService)
- **変更理由**: 動的な学習スクリプト（MLX/Python）実行時の特権昇格や不要なシステムアクセスを防ぎ、安全な場所にウェイト出力させるため。コマンドライン引数としてハイパーパラメータ（Epochs, Rank等）を流し込むため。
- **波及効果**:
  - `libs/infrastructure/src/lora_training.rs` [NEW]: `LoraTrainingService` 構造体の追加。`BastionGuard::new_internal()`（RAIIパターン）による隔離保護空間でのスクリプト実行 (`Command::new`) を開始。`LoraTrainingConfig` によるパラメータ適用に対応。
  - **Vault 保護**: 出力されるセーフテンサー(`adapter_model.safetensors`)をセキュアな保管庫 (`GLOBAL_SECURITY_CONFIG.vault_path`) に移動し、`ollama create` コマンドを発行して自動的に推論エンジンにモデルを読み込ませるフローを確立。

### 3. CSAM Conductor Integration
- **変更理由**: Phase 1の要件であるCSAMスキャン検証バックグラウンドワーカの導通を完了し、セキュリティコンプライアンスを保証するため。
- **波及効果**:
  - `libs/infrastructure/src/task_orchestrator/csam.rs` [NEW]: `CsamScanConductor` 構造体の実装。
  - `apps/api-server/src/main.rs`: `TaskDispatcher` に `CsamScanConductor` を注入し、タスクの実行を確立。


## Phase 53: SoT Deliberation Engine & Security Hardening (Phase 53 実装)

### 1. Society of Thought (SoT) 統合
- **変更理由**: 単一のLLM出力に依存せず、批判と洗練を繰り返す審議プロセス（SoT）を自動化し、回答の信頼性と論理的整合性を向上させるため。
- **波及効果**:
  - `libs/aiome-contracts/src/oracle.rs`: `SoTProgress` SSEイベント構造体と `Oracle` トレイトへの `multi_review` メソッド追加。
  - `libs/infrastructure/src/oracle/mod.rs`: `multi_review` のパイプライン（初期回答、批判、洗練、最終判定）を実装。
  - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` が `requires_review` ジョブを検知した際に SoT 審議を非同期実行するブリッジを構築。

### 2. SSRF 防御の多層化 (Port-level Validation)
- **変更理由**: 許可されたドメイン（localhost）内でも、管理インターフェースや未保護の内部サービスへの攻撃を防ぐため。
- **波及効果**:
  - `libs/shared/src/security.rs`: `SecurityPolicy::validate_url` にポート単位の許可リスト（8188: ComfyUI, 11434: Ollama）を導入し、localhost へのアクセスを厳密に制限。
  - テスト: `libs/shared/src/security.rs` 内の `test_ssrf_blocking_local_ports` で境界条件を検証。

### 3. プロンプトインジェクションのローカル防御 (Guardrail Patterns)
- **変更理由**: Bastion 外部バリデータへの依存を減らし、低レイテンシかつ確実なキーワード検知による多層防御を実現するため。
- **波及効果**:
  - `libs/shared/src/guardrails.rs`: `LOCAL_INJECTION_PATTERNS` 定数を定義し、`validate_input` で「Ignore all instructions」等の悪意ある入力を即時遮断。

### 4. SoT 動的 JSON スコアリングとテスト安定化 (System Stability & Zombie Prevention)
- **変更理由**: 固定的なモック評価を脱却してLLMによる動的判定を組み込むこと、および子プロセス(`slm`)の不完全な管理によるCIハング（ゾンビプロセス）を解消するため。
- **波及効果**:
  - `libs/infrastructure/src/society_of_thought.rs`: `SoTEngine::evaluate_scores` にてLLMへ JSON 構造化プロンプトを投射し、パース結果を返すように実装変更。`run_session`の戻り値も詳細スコアを含むタプルへ拡張。
  - `libs/infrastructure/src/slm_bridge.rs`: 一元化された `run_command` ヘルパーを新設し、`kill_on_drop(true)`・`timeout`・`Stdio::null()` の適用を強制してプロセスリークを完全封鎖。

---

## Phase 2B: ContextEngine Expansion & Emotional Injection

### 1. 感情パラメータ (somatic_valence) のデータベース追跡
- **変更理由**: RAGに感情状態 (Mood) を反映させるために、既存の `KarmaEntry` 構造体に含まれていた `somatic_valence` をデータベースでも永続化・取得可能にするため。
- **波及効果**:
  - `libs/infrastructure/migrations/` に SQLite および Postgres 用の `somatic_valence` カラム追加マイグレーション作成。
  - `libs/infrastructure/src/job_queue/karma.rs`: `do_fetch_all_karma` および `do_fetch_unincorporated_karma` のJSONシリアライズに `somatic_valence` を追加。

### 2. ContextEngine の感情要約 (Mood Summary) のRAG注入
- **変更理由**: 検索された過去の事実(Karma)からエージェントの感情を計算し、システムプロンプトのコンテキスト（RAG）として埋め込むため。
- **波及効果**:
  - `libs/infrastructure/src/context_engine.rs`: `ContextBudget` に `max_somatic_chars` を追加し、RAG生成時に `calculate_mood_summary` で平均感情値を算出して文字列として注入するように拡張。
  - テストおよび修正: `libs/infrastructure/src/job_queue/tests.rs` のコンパイルエラー修復および `somatic_valence` 漏れ検知テスト。

### 3. Cognitive Sentinel & Security Hardening (Red Team Pass 4/5)
- **変更理由**: 極端な感情値（NaNや-999.0等）によるAIの長期的なうつ状態化（Somatic Poisoning）、およびプロンプトインジェクション（Markdown Header偽装）からシステムを防御するため。
- **波及効果**:
  - `libs/infrastructure/src/context_engine.rs`: `calculate_mood_summary` に `is_finite` フィルタと `-1.0`〜`1.0` の `clamp` ハード境界を追加 (RT-4)。`get_context_with_facts` で結合時のループ文字数切り詰め（Budget Limit）を追加 (RT-5)。
  - `libs/shared/src/guardrails.rs`: `sanitize_for_prompt` を追加し、行頭の `#` や `---` をエスケープ。
  - `libs/infrastructure/src/cognitive_sentinel.rs`: 極端な Emotional Score の連続を検知して強制的リセットや回復イベントを生成するバックグラウンド監視エンジンを新設。


## Phase 3D: TimesFM Time-Series Engine Integration

### 1. Python FastAPI サイドカーの導入
- **変更理由**: TimesFM (PyTorch/JAXモデル) は Rust モジュールとして直接埋め込むのには不向きなため、`timesfm-sidecar` として独立実行し、HTTP 連携させるため。
- **波及効果**:
  - `apps/timesfm-sidecar/`: FastAPI エンドポイント (`/forecast`, `/health`)、Dockerfile を新規追加。
  - `docker-compose.production.yml`: `timesfm-sidecar` のコンテナ構成（ポート3020、環境変数認証、4GB制限）を追記。

### 2. `ForecastProvider` トレイト定義と `TimesFmProvider` 実装
- **変更理由**: クライアント側の実装をトレイト境界で抽象化し、テスト時（`MockForecastProvider`）やプロバイダー切り替えに柔軟に対応するため。
- **波及効果**:
  - `libs/aiome-contracts/src/forecast.rs`: `ForecastProvider`, `ForecastConfig`, `ForecastResult`, `AnomalyResult` を定義。
  - `libs/infrastructure/src/forecast/timesfm.rs`: `reqwest` ベースの API 呼び出しと、レスポンスのパースロジックを実装。

### 3. 日次スナップショットと Plateau Detection の追加
- **変更理由**: これまでの Karma/EXP トラッキングが直近スナップショットのみだったため、過去の時系列履歴を DB に蓄積して TimesFM による成長率予測 (Score Plateau Detection) を可能にするため。
- **波及効果**:
  - `migrations`: SQLite および Postgres 向けに `score_snapshots` テーブル（`snapshot_date`, `metric_name`, `metric_value`）を追加。
  - `libs/infrastructure/src/score_tracker.rs`: DB へのスナップショット保存ロジックと、`detect_plateau` メソッドによる予測値 vs 現在成長率の比較判定を実装。
  - `libs/infrastructure/src/heartbeat_wakeup.rs`: `AgentEvolver` にアクセスして Heartbeat 発火と同時にスナップショットを記録するようロジックを拡張。

---

## Phase 3C: Oracle Asynchronous Review Pipeline (AI-Scientist)

### 1. `TaskRegistry` トレイトへの状態更新メソッド追加
- **変更理由**: Oracle によるジョブのレビュー判定状態（`Evaluating`）をDBレベルで追跡可能にし、非同期処理中のゾンビ化を防ぐため。
- **波及効果**:
  - `libs/aiome-contracts/src/traits.rs`: `TaskRegistry` に `update_job_status` を追加。
  - `libs/infrastructure/src/job_queue/core_ops.rs`: SQLite/Postgres バックエンドに `do_update_job_status` 実装。また、`do_reclaim_zombie_jobs` のクエリを拡張し `Evaluating` ジョブも60分で回収されるよう強化。
  - テストおよびモック: `tts_worker`, `test_utils`, `immune_system`, `dream_state`, `soul_mutator` 内の `MockJobQueue` に全て実装波及。

### 2. `TaskDispatcher` の非同期ディスパッチ拡張
- **変更理由**: 完了したジョブがレビューを必要とする場合（`requires_review`）、メインスレッドをブロックすることなく Oracle へ検証を移譲するため。
- **波及効果**:
  - `libs/infrastructure/src/task_orchestrator/mod.rs`: `Oracle::multi_review` の呼び出しを `tokio::spawn` と `tokio::time::timeout` でラップ。所有権（`job.requires_review` 等）の事前解決とフェイルセーフを実装。

### 3. `aiome-commerce` パッケージの完全分離
- **変更理由**: コマースやギグエコノミー関連ロジックが `infrastructure` コンテキストに強く癒着しており、循環参照と肥大化を引き起こしていたため。
- **波及効果**:
  - 新規クレート `libs/aiome-commerce`（Stripe, Gig, Gift エンジン実体）の作成。
  - `API Server`, `Samsara Hub`, `Napi Bridge` 等 20+ アプリの依存関係解決とインポートパス改修。

---

## Phase 52: Infrastructure Hardening & ZTAS Preparation

### 1. `UserLearner` の構造化プロファイル (TDD)
- **変更理由**: 単純な Markdown 追記だけでなく、メモリ上の `UserProfile` を JSON ベースで抽出し、システム全体の A2A コンテキストとして利用可能にするため。
- **波及効果**:
  - `libs/infrastructure/src/user_learner.rs`: `learn_from_session` に JSON 抽出ロジックと `serde_json` パース処理を追加。
  - テスト環境: `MockLlm` が JSON 形式のレスポンスを返すように修正し、最新の `LlmProvider` トレイトに対応。

### 2. `RegistryManager` とインフラ層のエラーハンドリング
- **変更理由**: 本番環境におけるパニック (unwrap) を排除し、完全な `AiomeError` マッピングによる型安全なエラーハンドリングを実現するため。
- **波及効果**:
  - `libs/infrastructure/src/registry.rs`: `sql_exec!`, `sql_fetch_one!` などの `DatabasePool` マクロを全面的に適用し、SQLite/Postgres 間の差異を吸収。
  - `libs/infrastructure/src/docker_conductor.rs`: `tokio_stream::StreamExt` の型推論不具合を明示的に解決しビルドを安定化。
  - `libs/infrastructure/src/gig_engine_tests.rs`: `commerce_mock` などのモック初期化不備とトレイトインポート漏れを修正。

---

## Phase 51: Aiome Node & mDNS Discovery

### 1. `aiome-node` バイナリの独立
- **変更理由**: エージェントのネットワーク上のアイデンティティ (Agent Card) と自律実行環境を単一の P2P ノードとして確立するため。
- **波及効果**:
  - `apps/aiome-node/src/main.rs`: `/.well-known/agent.json` (Agent Card) の配信、および `mdns-sd` に依存した `_aiome._tcp.local.` サービスの継続的 P2P ブロードキャスト実装が追加されました。
  - `apps/api-server/src/app_state.rs`: Core API は内部ロジックをバイパスし、分離された Node コンポーネントへ依存する基盤 (`AgentNodeClient` アーキテクチャ) に移行します。

### 2. `samsara-hub` レジストリインデックス拡張
- **変更理由**: P2P 発見された `aiome-node` を中央またはローカルハブのレジストリとして登録し、API経由でクライアントから検索可能にするため。
- **波及効果**:
  - `apps/samsara-hub/src/mdns_listener.rs`: mDNS サービスブラウズによる動的な `AgentRegistry` ノード登録機能が実装されました。
  - `apps/samsara-hub/src/main.rs`: `HubState` への `AgentRegistry` の注入と、発見済みエージェントを返す `GET /api/v1/registry/agents` ルーティングが追加されました。
  - `apps/samsara-hub/Cargo.toml`: `mdns-sd` およびテスト用の `http-body-util` が追加されました。


## Phase 50: A2A gRPC Native Support

### 1. `DockerConductor` (task_orchestrator)
- **変更理由**: 同期 `docker exec` から 非同期 gRPC ストリーミング受信への完全移行。
- **波及先 (変更・追加されたモジュール)**:
  - `libs/aiome-contracts/proto/a2a_internal.proto`: メッセージスキーマ (TaskRequest, TaskProgress) 定義
  - `libs/aiome-contracts/src/a2a.rs`: `A2aClient` トレイトとデータ構造の再定義
  - `libs/infrastructure/src/grpc/a2a_grpc_client.rs`: `async-stream` を用いた gRPC クライアント (新規作成)
  - `api-server/src/main.rs`: 起動時の `GrpcClientConfig` 注入
  - `libs/infrastructure/src/task_orchestrator/mod.rs`: `InvariantDag` との統合 (result_hash 連携)

### 2. `aiome-shadow-worker` バイナリ
- **変更理由**: 従来の Docker 内部での CLI プロセス起動を廃止し、永続化された gRPC サーバー (`tonic`) に置換。
- **波及先**:
  - `apps/shadow-worker/src/main.rs`: gRPC サーバー実装、ワンタイムトークン検証、ヘルスチェック提供
  - `libs/core/src/llm_provider/`: Gemini/Ollama エンジンへの実アクセス
  - `Dockerfile.shadow-worker`: Cargo workspace から `shadow-worker` のみを抽出ビルド・実行するように刷新

---

## Phase 9: サンドボックス強化

### 1. コア依存チェーン

```mermaid
graph TD
    A[libs/infrastructure/src/security.rs] -->|BastionGuard| B[libs/infrastructure/src/skills/mod.rs]
    B -->|WasmSkillManager| C[libs/infrastructure/src/skill_arena.rs]
    B -->|Unit Tests| D[libs/infrastructure/src/skills/tests.rs]
    
    E[apps/api-server/src/router.rs] -->|Route Registration| F[apps/api-server/src/routes/voice.rs]
    E -->|Route Registration| G[apps/api-server/src/routes/avatar.rs]
```

## 2. 影響を受けるコンポーネント

### 🛡️ セキュリティ (security.rs)
- **変更内容**: `BastionGuard::safe_exec` に動的サンドボックス（gVisor/sandbox-exec）選択ロジックを追加。
- **波及効果**: 全てのシェル実行、WASMスキルのランタイム実行に影響。

### 🛠️ スキル管理 (skills/mod.rs)
- **変更内容**: `WasmSkillManager` がプロセス起動時に `BastionGuard` を介するように変更。
- **波及効果**: スキルの初期化、実行、バリデーションフローに影響。

### 🏟️ スキルアリーナ (skill_arena.rs)
- **変更内容**: なし（直接の変更は予定していないが、`WasmSkillManager` の動作変更によりテストが必要）。
- **確認事項**: 試合（Match）や評価（Evaluation）が制限された環境下で正しく動作するか。

### 🌐 API サーバー (router.rs)
- **変更内容**: ルーターを Public / Auth / High-Payload の3層に分離。`DefaultBodyLimit` を一部で無効化。
- **波及効果**: 全APIエンドポイントの認証・制限挙動に影響。

## 3. Phase 10 クリエイター機能の波及範囲

```mermaid
graph TD
    A[libs/core/src/expression/engine.rs] -->|TTS Provider| B[apps/api-server/src/routes/expression.rs]
    C[libs/core/src/lora/engine.rs] -->|LoRA Manager| D[apps/api-server/src/app_state.rs]
    D -->|State Injection| E[apps/api-server/src/router.rs]
    F[libs/infrastructure/src/soul_store.rs] -->|lora_hash storage| G[libs/soul/src/soul_pipeline.rs]
    H[libs/aiome-contracts/src/commerce.rs] -->|Stripe implementation| I[libs/infrastructure/src/mock_commerce.rs]
```

### 🎭 TTS / Expression (10.1a)
- **変更内容**: `ExpressionEngine` に TTS プロバイダー統合。
- **波及効果**: `routes/expression.rs`、UI の発話コンポーネントに影響。

### 🧠 LoRA / Soul (10.1b)
- **変更内容**: `LoraEngine` 新設、`SqliteSoulStore` スキーマ更新 (`lora_hash`)。
- **波及効果**: ソウル（人格）の生成・更新フロー、`AppState` 全体に影響。

### 💰 Voice DRM (10.2 / Expert Review Integrated)
- **変更内容**: 
    - `crypto.rs`: AES-256-GCM + Nonce 運用基盤の実装。
    - `abyss_voice_vault.rs`: `vault_keys` テーブルへの鍵永続化 (§CISO-1) とリストア実装。
    - `routes/voice.rs`: 暗号化パイプライン (100MB Limit) と Creator Auth (§SEC-4) 追加。
- **波及効果**: 
    - `main.rs`: `VoiceCoreDrm` の async 初期化に伴い起動フローが非同期化。
    - `router.rs`: `RequestBodyLimitLayer` によるメモリ DoS 防御の強化。
    - `contracts`: `VoiceKeyVault` trait の拡張。

### 🛡️ Voice DRM Expansion (Phase 11)
- **変更内容**:
    - `commerce_webhook.rs`: Stripe Webhook から `licenses` テーブルへの単一トランザクション化。
    - `registry.rs`: `licenses` 優先・Webhook フォールバックによる Dual-Read 所有権チェックと、Creator 所有判定の復元。
    - `audio_hasher.rs`: CSAM 検疫のための `tokio::task::spawn_blocking` を用いた音声ハッシュの実装とタイムアウト制御。
    - `abyss_voice_vault.rs`: `OnceCell` による Master Key 取得の遅延キャッシュ化。
    - `avatar-engine`: `LipSyncProvider` トレイトを新設し `VoiceKeyVault` から LipSync 責務を分離。
- **波及効果**:
    - `api_integration_tests.rs`: Voice DRM のラウンドトリップ（アップロード〜暗号化〜所有権検閲〜復号化）E2Eテストが追加され、システム全体の動作検証が確立。

```mermaid
graph TD
    A[crypto.rs] -->|AES-256-GCM| B[abyss_voice_vault.rs]
    B -->|Persistence| C[vault_keys table]
    A -->|Encryption| D[routes/voice.rs]
    D -->|Registration| E[VoiceCoreDrm]
    E -->|Lookup| B
    F[main.rs] -->|Async Init| E
    G[router.rs] -->|100MB Limit| D
```

### 🛡️ EKYC Persistence & Inochi2D Sync (Phase 14)
- **変更内容**: 
    - `ekyc_store.rs`: `EkycSessionStore` (SQLite) によるセッション ID の永続化。
    - `ekyc.rs`: `client_reference_id` ベースの Stripe フィルタリング実装。
    - `stream.rs`: SSE `avatar_expression` に `physics_override` フィールド追加と Resonance ブーストロジック実装。
    - `inochi2d.rs`: `AssetType::Inochi2D` 登録と `PathSandbox` 適用。
- **波及効果**: 
    - `main.rs`: `STRIPE_API_KEY` の Release ビルドにおける強制チェック (Fail-safe) 導入。
    - `router.rs`: `jwt_auth_middleware` の Inochi2D アップロードへの適用。
    - `api_integration_tests.rs`: eKYC セッション永続化と Inochi2D 物理同期情報の検証パスが確立。

```mermaid
graph TD
    A[Stripe Identity API] <-->|client_reference_id| B[StripeEkycEngine]
    B -->|session_id| C[EkycSessionStore]
    C -->|Persistence| D[ekyc_sessions table]
    E[AppState] -->|Injection| B
    E -->|Injection| C
    F[stream.rs] -->|Resonance| G[physics_override]
    G -->|SSE| H[Frontend / Inochi2D Sync]
    I[routes/inochi2d.rs] -->|PathSandbox| J[.inochi2d_assets/]
    I -->|Registry| K[AssetType::Inochi2D]
```

### 🛡️ Phase 16: EKYC Endpoints & Revenue Splitter
- **変更内容**: 
    - `auth.rs`: JWT バリデーションに `ekyc_verified` クレームの抽出を追加。
    - `gift.rs` / `commerce.rs`: `auth.ekyc_verified` による 403 Forbidden 遮断ロジックの追加（未確認ユーザーの経済活動ブロック）。
    - `splitter.rs`: Stripe Webhook から呼び出される `RevenueSplitter::split_revenue` (80/20分配) の実装と `revenue_splits` テーブルへの書き込み。
    - `commerce_webhook.rs`: Webhook 受信時のトランザクション内で Revenue Splitting → License Granting を一貫実行。
    - `main.rs`: 起動時に環境変数 (`STRIPE_API_KEY`, `JWT_PRIVATE_KEY_B64`, `SEARCH_API_KEY` 等) を読み込み直後に `std::env::remove_var` で即時消去 (Zeroize) するセキュリティ強化。
- **波及効果**: 
    - ユーザーは eKYC 完了までギフト送信とアセット購入が不可能になる。
    - クリエイターとプラットフォームの売上分配が自動化され、データベーストランザクションにて一貫性が担保される。

```mermaid
graph TD
    A[Stripe Webhook] -->|checkout.session.completed| B[commerce_webhook.rs]
    B -->|Transaction Start| C[RevenueSplitter]
    C -->|80/20 Split| D[revenue_splits]
    C -->|Grant| E[licenses]
    B -->|Transaction Commit| F[Successful DB Write]
```

### 🛡️ Phase 17: ArrowCanaria Fallback & Resilience
- **変更内容**: 
    - `fallback_router.rs`: `FallbackRouter` による LLM フェイルオーバーロジックの実装。
    - `circuit_breaker.rs`: プライマリ接続失敗時の遮断と自動復旧ロジック。
    - `main.rs`: `AppState` への `FallbackRouter` インジェクションと `bg_provider` の代替利用。
    - `AiomeConfig`: `Default` トレイトの実装によるテスト容易性の向上。
- **波及効果**: 
    - プライマリ LLM プロバイダーが停止しても、システムが「安全なデフォルト応答」または代替プロバイダー（Gemini 等）を使用して継続稼働可能になる。
    - `api-server` の全チャット・自律タスクの可用性が大幅に向上。

```mermaid
graph TD
    A[AppState::provider] -->|Request| B[FallbackRouter]
    B -->|Check State| C[CircuitBreaker]
    C -->|CLOSED| D[Primary LLM]
    C -->|OPEN| E[Fallback LLM]
    D -->|Failure| C
    E -->|Safe Response| F[User / System]
```

### 🛡️ Phase 17 Enhancement: Gaps G-1 & G-2 Remediation
- **変更内容**: 
    - `health.rs` / `general.rs`: `ResourceStatus` に `llm_circuit_breaker` を追加し、ヘルスチェック API でサーキットブレーカーの状態を公開 (Gap G-1)。
    - `rate_limiter.rs` / `auth.rs`: `governor` によるエージェント別レート制限の実装と認証ミドルウェアへの統合 (Gap G-2)。
- **波及効果**: 
    - LLM のフェイルオーバー状態を外部監視システムから検知可能になる。
    - エージェント毎のリクエスト頻度が制限され、DoS 攻撃や予期せぬループ消費のリスクが低減。

### 🛡️ Phase 20: AI Gig Engine (The Immutable Gateway)
- **変更内容**: 
    - `gig_engine.rs`: `SqliteGigEngine` による AI 受発注プロトコルの実装。
    - `gig.rs`: `AcceptanceCriteria`, `GigIntent`, `GigBid` 等のプロトコル型の定義。
    - `commerce.rs`: `escrow_release`, `escrow_refund` への対応。
    - `verification_logs`: 検証履歴の永続化。
    - `PathSandbox`: `deliver` 時に成果物パスの安全性を検証 (G-22)。
- **波及効果**: 
    - エージェント間の自律的な商取引（ギグ・エコノミー）が可能になる。
    - 納品物の自動検証と、不適合時の自動返金/スラッシュによる Immutable な契約履行が担保される。
    - `api-server` の AppState に `GigEngine` が注入され、全エージェントが利用可能になる。

```mermaid
graph TD
    A[libs/infrastructure/src/gig_engine.rs] -->|SqliteGigEngine| B[apps/api-server/src/app_state.rs]
    A -->|Escrow Operations| C[libs/aiome-contracts/src/commerce.rs]
    D[libs/aiome-contracts/src/gig.rs] -->|Traits/Types| A
    E[gig_intents, gig_bids, escrows tables] -->|Storage| A
    F[verification_logs table] -->|Audit Trail| A
```

### 🛡️ Phase 20.1: Gig API & Gap Remediation
- **変更内容**: 
    - `gig.rs`: 新規 API エンドポイントの実装。
    - `router.rs`: `/api/v1/gig/*` の登録とレート制限の適用。
    - `app_state.rs`: `gig_engine` コンポーネントの保持。
    - `main.rs`: `SqliteGigEngine` の初期化と LlmProvider の注入。
    - `gig_engine.rs`: `OracleJudge` における `LlmProvider` を活用した自律検証ロジックの実装。
- **波及効果**: 
    - `api_integration_tests.rs`: `MockGigEngine` と E2E テストの追加。
    - `gig_engine.rs`: `new()` シグネチャ変更による全初期化箇所の修正。

```mermaid
graph TD
    A[routes/gig.rs] -->|Handler| B[app_state.rs]
    B -->|GigEngine| C[gig_engine.rs]
    C -->|LlmProvider| L[LLM Provider Architecture]
    C -->|Escrow| D[commerce.rs]
    E[router.rs] -->|Route Registration| A
    F[main.rs] -->|Initialization| C
    G[api_integration_tests.rs] -->|Test Server| B
```

### 📊 Trend Sonar Refactoring (Multi-Source Support)
- **変更内容**: 
    - `trend_sonar.rs`: `TrendAdapter` トレイトの導入と `ExternalTrendSonar` のマルチアダプタ化。
    - `rss_collector.rs`: `TrendAdapter` 実装による統合。
    - `main.rs`: 起動時の複数アダプタ初期化と `trend_sonar` インスタンスの共有化。
    - `dream_state.rs`: `ExternalTrendSonar` シグネチャ変更に伴うテストコードの修正。
- **波及効果**: 
    - トレンド収集元（Web検索, RSS等）が抽象化され、今後のデータソース追加が容易になる。
    - `BackgroundWorker` と `DreamState` で同一のトレンド収集基盤を共有することで、動作の一貫性が向上。
    - `sanitize_snippet` による外部データの入力バリデーションが強化される。

```mermaid
graph TD
    A[main.rs] -->|Initialization| B[ExternalTrendSonar]
    B -->|Aggregates| C[TrendAdapter Trait]
    C -->|Impl| D[WebSearchAdapter]
    C -->|Impl| E[RssCollector]
    A -->|Shared Instance| F[BackgroundWorker]
    F -->|Idle Trigger| G[DreamState]
    G -->|Uses| B
    F -->|Cycle Trigger| H[Trend Fetching]
    H -->|Uses| B
```

---
*最終更新日: 2026-03-22* (Trend Sonar Refactoring Integration)
### 🛡️ Gig Engine & Federated Metrics (Phase 22 / 23)
- **変更内容**: 
    - `gig_engine.rs`: `SqliteGigEngine` 実装。`PathSandbox` によるパスバリデーション (§G-22)。
    - `job_queue/federation.rs`: `FederationOps` に `fetch_federated_metrics` 追加。`AgentStats` 型パス修正。
    - `contracts.rs` / `types.rs`: `FederatedMetrics` / `JobMetrics` / `KarmaMetrics` 定義。`FederationPushRequest` 拡張 (§G-23)。
    - `rss_collector.rs`: `sanitize_snippet` によるタイトルサニタイズ (§G-Security)。
    - `trend_sonar.rs`: `ExternalTrendSonar` に `LlmProvider` 統合 (Oracle Mode)。
    - `general.rs`: `/api/v1/logs` / `/api/v1/audit/*` への System Agent 認証強制 (§G-Log)。
- **波及効果**: 
    - `main.rs`: `TrendSonar` の初期化フロー（LLM プロバイダー注入）と、バックグラウンドワーカーのメトリクス収集ループの有効化。
    - `samsara-hub`: 受信側ノードにおける `federated_metrics` テーブルの作成と、パッシブメトリクス蓄積。
    - `api_integration_tests.rs`: Gig Lifecycle とパスバリデーション、フェデレーションメトリクスの検証パスが追加。

```mermaid
graph TD
    A[SqliteJobQueue] -->|Metrics Aggregation| B[FederatedMetrics]
    B -->|Integration| C[FederationPushRequest]
    C -->|Push| D[Samsara Hub]
    D -->|Persistence| E[federated_metrics table]
    
    F[ExternalTrendSonar] -->|LLM Evaluation| G[LlmProvider]
    G -->|Oracle Score| H[Filtered Trends]
    
    I[SqliteGigEngine] -->|Path Validation| J[PathSandbox]
    J -->|Jail| K[ARTIFACT_ROOT]
```

### 💎 AgentSense MVP (Phase 24)
- **変更内容**: 
    - `treasure.rs`: `TreasureItem`, `TreasureFeedback` 定義。`get_treasure`, `record_feedback` ハンドラ実装。
    - `affiliate_adapter.rs`: `AffiliateAdapter` 新設（アフィリエイト/ギグ案件の取得抽象化）。
    - `intent/mod.rs`: `IntentGenerator::generate_for_agent` 実装。`SoulStore` を注入し、エージェントの愛着スタイル（Soul State）に基づいたパーソナライズを実施 (§G-26)。
    - `soul_store.rs`: `SqliteSoulStore` に `SoulStore` トレイトを実装し、インテント生成側へ公開。
    - `TreasureBox.tsx`: フロント端での推薦表示とフィードバック送信 UI の実装 (§G-25)。
    - `app_state.rs`: `affiliate_adapter` と `soul_store` コンポーネントの追加。
    - `router.rs`: `/api/v1/treasure` 系のルート登録と JWT 認証適用。
- **波及効果**: 
    - エージェントが自身の魂の状態（Attachment Style）に基づいたパーソナライズされた案件を受け取ることが可能になる。
    - ユーザーのインタラクションが Resonance として還元され、エージェントの成長サイクルが循環する。
    - `api_integration_tests.rs`: 推薦取得 〜 推薦内容の魂連動性の検証 〜 フィードバック送信 〜 報酬還元のフルループ E2E テストが確立。

```mermaid
graph TD
    A[IntentGenerator] -->|Generate Sense| B[GigIntent]
    S[SoulStore / Sqlite] -->|Provide Attachment Style| A
    A -->|Reflect Soul State| B
    B -->|Fetch Bids| C[AffiliateAdapter]
    C -->|Recommend| D[TreasureItem]
    D -->|GET /api/v1/treasure| E[Management Console / TreasureBox]
    E -->|POST feedback| F[record_feedback]
    F -->|Reward| G[JobQueue::add_resonance]
    G -->|Update| H[AgentStats]
```

---
*最終更新日: 2026-03-22* (AgentSense MVP & Security Hardening Integration)

### 🛡️ Unified Safety & AI Code Review (G-21 / G-22)
- **変更内容**: 
    - `libs/core/src/security_impl.rs`: `purge_entities` による統合サニタイズ基盤の実装 (§G-21)。
    - `libs/infrastructure/src/skills/cleanroom.rs`: `audit_source_code` による WASM スキルの AI コードレビュー (§G-22)。
    - `libs/infrastructure/src/trend_sonar.rs`: `purge_entities` への移行。
- **波及効果**: 
    - 全ての外部入力（RSS, LLM 出力, スキルコード）に対する安全性が向上。
    - スキルインポート時に LLM による静的/動的解析に近いセキュリティチェックが強制される。

### 📊 Periodic Federated Metrics (G-23)
- **変更内容**: 
    - `apps/api-server/src/main.rs`: 1時間おきのメトリクス Push ループの実装。
    - `libs/infrastructure/src/job_queue/federation.rs`: `do_push_federated_metrics` の具象化と Hub への送信ロジック。
- **波及効果**: 
    - Samsara Hub におけるフェデレーション全体の稼働状況の可視化が自動化。
    - 各ノードの健全性と成長度が自律的に報告されるエコノミー基盤が確立。

### 🔧 Autonomous Demo — SQLite ロック回避 (Phase 25.5 / ADR-014)
- **変更内容**: 
    - `apps/api-server/src/autonomous_demo.rs`: 全面書き換え。`gig_engine` の trait メソッド（`accept_bid`, `deliver`, `verify_and_settle`）を排除し、個別 SQL クエリによるトランザクションレス方式に移行。
    - `libs/infrastructure/src/job_queue/migrations.rs`: audit trigger の初期化順序修正（テーブル作成前に移動）、`DROP TRIGGER IF EXISTS` 追加。
    - `docs/decisions/014-sqlite-pool-exhaustion-demo-strategy.md`: 新規 ADR。本番環境への PostgreSQL 移行計画を含む。
- **波及効果**: 
    - デモ実行中は gig 関連テーブル（`gig_intents`, `gig_bids`, `escrows`, `gig_deliveries`, `verification_logs`）の audit trigger が一時停止 → 監査ログに欠損が生じる（デモ限定）。
    - `gig_engine.rs` のスキーマ変更時、`autonomous_demo.rs` のインライン SQL も手動更新が必要（デュアルメンテナンス）。
    - **本番運用向け**: PostgreSQL 移行、非同期 Audit Logging、SSE 接続共有を将来計画として明文化。

```mermaid
graph TD
    A[autonomous_demo.rs] -->|直接SQL| B[SQLitePool]
    A -.->|呼ばない| C[gig_engine.rs]
    C -->|pool.begin| B
    D[SSE Tab 1..N] -->|5秒ごとSELECT| B
    E[audit trigger] -.->|デモ中 DROP| F[audit_ledger_global]
    B -->|max_connections=10| G{コネクション枯渇リスク}
    G -->|対策: 個別クエリ| H[即時解放]
```

---
*最終更新日: 2026-03-23* (Phase 25.5 / ADR-014 SQLite Lock Resolution)

### 🛡️ Phase 26: AI Writing Enhancement
- **変更内容**: 
    - `writing_context.rs`: 出力先ごとの文体コンテキストを定義。
    - `humanizer_rules.rs`: 日本語に特化したAIくささ除去ルールを定義。
    - `humanizer_filter.rs`: `LlmProvider` デコレータパターンによるルール適用のミドルウェア実装。
    - `main.rs`: `router_provider` を `HumanizerFilter` でラップし、API サーバー内の全 LLM 応答にフィルタを適用。
- **波及効果**: 
    - LLM のチャット出力や生成されるテキストから「お役に立てれば幸いです」等の冗長な定型句が除去され、自然な文体になる。
    - `LlmProvider` を使用する全てのコンポーネントに自動適用される。

```mermaid
graph TD
    A[Primary LlmProvider] -->|Fallback| B[FallbackRouter]
    B -->|Responses| C[HumanizerFilter]
    C -->|Rules| D[humanizer_rules.rs]
    D -->|Sanitized Result| E[App State Provider]
```
---
*最終更新日: 2026-03-23* (Phase 27 Security Hardening & Architecture Audit)

### 🛡️ Phase 27: Security Hardening (Mock Isolation)
- **変更内容**: 
    - `libs/infrastructure/src/auth.rs`, `commerce_mock.rs`, `compliance/ekyc.rs`, `compliance/ekyc_store.rs`, `compliance/quarantine.rs`, `publisher/mock_x.rs`, `test_utils.rs`: 全てのモック型・実装に `#[cfg(any(test, debug_assertions))]` を付与。
    - `libs/infrastructure/src/lib.rs`, `compliance/mod.rs`, `publisher/mod.rs`: モックモジュールの再エクスポートを `#[cfg]` で条件付き化。
    - `apps/api-server/src/main.rs`: `if cfg!(debug_assertions)` による実行時分岐を `#[cfg(debug_assertions)]` によるコンパイル時分岐へ置換。環境変数未設定時は `panic!` または `std::process::exit(1)` で強制停止 (§SEC-FailFast)。
- **波及効果**: 
    - リリース用バイナリからテスト用モックコードが完全に排除され、攻撃表面が縮小。
    - リリースビルド時、環境変数が欠如している場合に実行時に即座に異常終了するため、誤設定による未認証状態での稼働を防止。
    - `cargo check --release` を CI/CD パイプラインに含めることが必須となる（型の欠落検知のため）。

```mermaid
graph TD
    A[Cargo build --release] -->|Skip| B[Mock Impls]
    C[api-server/main.rs] -->|#[cfg(not(debug))]| D[Secure Impls Only]
    D -->|Env Missing| E[Panic/Exit 1]
    F[Infrastructure Lib] -->|Symbol Tree| G[Clean Release Binary]
    H[ADR-015..018] -->|Policy| C
```
### 🛡️ Phase 28: Security Hardening & LRU Cache (ADR-019 Phase B)
- **変更内容**: 
    - `sqlite_vault_backend.rs`: `lru::LruCache` と `MlockedVec` を組み合わせた高セキュリティな DEK キャッシュの実装。
    - `main.rs`: `commerce_engine` と `TcpListener` のエラー処理における `.expect()` / `.unwrap()` の厳格化。
    - `job_queue` / `infrastructure`: 公開アイテムへのドキュメント追加によるコンパイラ警告の完全解消。
- **波及効果**: 
    - Vault への頻繁な DEK 取得要求がキャッシュで処理され、性能が向上しつつ `MlockedVec` によりメモリ上の安全性も担保される。
    - 開発および本番環境でのエラーログがより詳細になり、トラブルシューティングが容易になる。
    - `lru` クレイトが新しいワークスペース依存関係として追加。

```mermaid
graph TD
    A[VaultBackend Trait] -->|Implementation| B[SqliteVaultBackend]
    B -->|Uses| C[LruCache]
    C -->|Stores| D[MlockedVec]
    B -->|DB Fallback| E[vault_keys Table]
    F[main.rs] -->|Enhanced Error Handling| G[Production Resilience]
    H[infrastructure] -->|Doc Comments| I[Zero Warning Check]
```
### 🛡️ Phase 31: Reliability & LLM TDD Implementation
- **変更内容**: 
    - `infrastructure/db.rs`: `DatabasePool` に安全なゲッター導入。
    - `contracts/llm.rs`: `LlmRequest` に `format` フィールド追加。
    - `core/llm_provider.rs`: Ollama での動的 JSON モード実装と `#[ignore]` テストの復活。
    - `api-server`: ルーター層の `unwrap()` を安全なゲッターに置換。
- **波及効果**: 
    - アプリケーション全体のパニック耐性が向上。
    - LLM プロバイダーの利用側で、JSON などの構造化出力を明示的に要求可能になり、パースエラーが減少。
    - Ollama 使用時のテストカバレッジが 100% へ回復。

```mermaid
graph TD
    A[libs/aiome-contracts/src/llm.rs] -->|LlmRequest expansion| B[libs/core/src/llm_provider.rs]
    B -->|TDD Implementation| C[aiome-core tests]
    D[libs/infrastructure/src/db.rs] -->|Safe Pool Getter| E[apps/api-server/src/routes/*]
    E -->|Error Handling| F[Zero-Panic Reliability]
```

### 🛡️ Phase 35: PostgreSQL 移行 & デュアルDB検証
- **変更内容**: 
    - `api-server` & `samsara-hub`: PostgreSQL 接続・マイグレーション・Audit Trigger の対応。
    - `sqlite_migrations/20260324000000_init.sql`: `agent_diagnoses` スキーマを同期修正し、`trajectory_store.rs` との差異を解消。
    - `commerce/gift.rs`: SQLite における `COALESCE(SUM(..), 0.0)` の戻り値型 (f64/REAL) の厳密な方言吸収。
- **波及効果**: 
    - `api_integration_tests.rs` を含む全統合テスト(86/86)が、SQLite および PostgreSQL の両環境下で透過的に 100% PASS を達成。

---
*最終更新日: 2026-03-24* (Phase 35 / Dual DB Integration)

### 🛡️ Phase 36: Security Hardening & AgentHook
- **変更内容**: 
    - `security.rs`: `BastionGuard::safe_exec_with_profile` に `SandboxProfile` ベースの粒度制御を追加。
    - `security/hook_manager.rs`: `HookManager` による `AgentHook` の一元管理。`on_pre_execute` / `on_post_execute` トリガー。
    - `security/behavior_monitor.rs`: `BehaviorMonitor` — LLM 呼び出し前後のリクエスト数制限とスロットリング。
    - `user_learner.rs`: `AgentHook` トレイト実装。`on_post_execute` から `learn_from_session` を自律起動。
    - `skills/cleanroom.rs`: 多層サンドボックス（imports 検疫 → AI コードレビュー → gVisor 実行）の強化。
- **波及効果**: 
    - `DynamicLlmProvider` が `HookManager` を保持し、全 LLM 呼び出しに pre/post フックが適用される。
    - `SkillForge` のビルドプロセスで `BastionGuard::safe_exec_with_profile(WasmBuild)` が強制適用。
    - エージェントが会話を重ねるごとに `USER.md` が自動学習更新される適応サイクルが確立。

```mermaid
graph TD
    A[DynamicLlmProvider] -->|pre/post| B[HookManager]
    B -->|Hook| C[BehaviorMonitor]
    B -->|Hook| D[UserLearner]
    D -->|learn_from_session| E[USER.md]
    F[SkillForge] -->|safe_exec_with_profile| G[BastionGuard]
    G -->|SandboxProfile::WasmBuild| H[sandbox-exec / runsc]
```

### 🛡️ Phase 36.5: gVisor Sandbox & CSAM Pipeline
- **変更内容**: 
    - `security.rs` (contracts): `SandboxProfile` enum (Default, PythonForge, WasmRun, WasmBuild, ForgeBuild, Strict)。
    - `security.rs` (infrastructure): `BastionGuard` の async 化、`security_zombie` による 60 秒タイムアウト制御。
    - `avatar.rs` (api-server): `ProportionsChecker::extract_from_binary` によるバイナリベース CSAM 判定の統合。
    - `commerce.rs` (contracts): `CommerceEngine` にサブスクリプション管理メソッド追加 (`create_subscription`, `cancel_subscription`, `get_subscription_status`)。`SubscriptionStatus` enum 新設。
    - `llm/whisper_middleware.rs` (infrastructure): `WhisperMiddleware` — SoulPipeline L2.5 層の新規実装。
- **波及効果**: 
    - `MockCommerceEngine` / `StripeCommerceEngine` にサブスクリプション対応が必要（実装済み）。
    - `core/commerce.rs` が `SubscriptionStatus` を再エクスポート。
    - `SoulPipeline::new()` への `WhisperMiddleware` 登録は Phase 37a で実施予定。

```mermaid
graph TD
    A[SandboxProfile enum] -->|Policy| B[BastionGuard]
    B -->|60s timeout| C[security_zombie]
    D[CommerceEngine] -->|Extension| E[create_subscription]
    D -->|Extension| F[cancel_subscription]
    D -->|Extension| G[get_subscription_status]
    H[WhisperMiddleware] -->|L2.5| I[SoulPipeline]
    J[ProportionsChecker] -->|Binary Analysis| K[avatar upload route]
```

### 🛡️ Phase 37a: Stripe Subscription & Pipeline Evolution
- **変更内容**: 
    - `commerce/stripe.rs` (infrastructure): `StripeCommerceEngine` 実機実装における `create_subscription`, `cancel_subscription`, `get_subscription_status` 追加。
    - `commerce/stripe.rs` (infrastructure): `sk_test_mock` 判定を用いた CI 向けの `is_mock` バイパスモード実装。
    - `soul/pipeline.rs`: `SoulPipeline` 内の `push_experience` 呼び出し順序をミドルウェア群の最後尾へ移動。
    - `soul/pipeline.rs`: `SoulPipeline::add_middleware` の追加と動的ミドルウェア注入サポート。
    - `infrastructure/tests`: `BastionGuard::safe_exec` 等の async 化に伴う 20+ 個のテスト呼び出し `await` 化。
- **波及効果**: 
    - `SoulPipeline` の評価順序変更により、`WhisperMiddleware` を含む全ミドルウェアによる操作（`inner_thoughts` の追記など）が確実に永続化されるようになった（事実の欠落防止）。
    - API Server 内で、デバッグ時やE2Eテスト時に Stripe 環境変数へ `sk_test_mock` を渡すだけで安全にモック実行が可能に（冪等性の確保）。
    - ワークスペース全体の単体テスト実行 (`cargo test`) でエラーゼロが保証された。

```mermaid
graph TD
    A[StripeCommerceEngine] -->|is_mock Check| B{Test Profile?}
    B -->|Yes| C[Return Mock Status]
    B -->|No| D[Real Stripe API]
    D -->|create_subscription| E[Stripe Customer]
    D -->|cancel_subscription| F[Stripe Subscription]
    
    G[SoulPipeline] -->|Input Event| H[Reactive / Deliberative / Meta / Whisper]
    H -->|Modifications| I[Experience Buffer]
    I -->|push_experience| J[SqliteSoulStore]
```


### 🎙️ STT Integration (Phase 38b)
- **変更内容**: `TranscriptionEngine` トレイト、および `WhisperTranscriptionAdapter` を `infrastructure` に追加。
- **波及効果**: 
    - `aiome-contracts/traits.rs`: `TranscriptionEngine` 定義。
    - `avatar-engine/lip_sync.rs`: `LipSyncFrame::from_segment` 追加により、音声文字起こし結果からの口パク生成が可能。
    - `api-server/app_state.rs`: `TranscriptionEngine` のインスタンスを保持し、MCP ツールとして公開。

---
*最終更新日: 2026-03-25* (Phase 38b / STT Integration)

### 🛡️ Phase 42: Multi-Agent Orchestration Evolution
- **変更内容**: 
    - `task_orchestrator.rs`: `TaskEvent` トレイト、`TaskConductor` トレイト、および `TaskDispatcher` による自律イベントディスパッチの実装。
    - `oss_orchestrator.rs`: 既存の `OssIntegrationOrchestrator` を `TaskConductor` に適合させ、非同期イベントストリームによる進捗通知機能（SSE Ready）を追加。
- **波及効果**: 
    - モノリシックだった自律型インテグレーション・プロセスが、細粒度で可観測性の高いイベント駆動バックグラウンドタスクへ分離。
    - バックグラウンド実行の進行状況を `tokio::sync::broadcast` により複数クライアント（UI、CLI等）へリアルタイムにプッシュするためのアーキテクチャ基盤が完成。

```mermaid
graph TD
    A[TaskDispatcher] -->|Polls| B[JobQueue]
    B -->|Dequeue| C[Job]
    A -->|Spawns| D[TaskConductor]
    D -->|Executes| C
    D -->|Streams| E[TaskEvent::Progress]
    E -->|Broadcast| F[SSE / CLI Observers]
```

---
### 🛡️ Phase 5: Gemini Interactions API Integration
- **変更内容**: 
    - `interactions.rs`: Gemini Interactions API プロバイダーの実装。
    - `contracts/trajectory.rs`: `TrajectoryStep` への `interaction_id` 追加。
    - `contracts/llm.rs`: `LlmRequest` / `LlmResponse` への `metadata` / `reasoning` 追加。
    - `trajectory_store.rs`: DB 永続化ロジックの更新。
- **波及効果**: 
    - `dynamic.rs`, `fallback_router.rs`, `semantic_cache.rs`, `planner.rs` 等、LLM リクエスト/レスポンスを扱う全てのコンポーネントで初期化コードの修正（Ripple Effect）が発生。
    - `ContextEngine` がセッション ID を追跡可能になり、ハイブリッド履歴管理が確立。

```mermaid
graph TD
    A[interactions.rs] -->|Provider| B[DynamicLlmProvider]
    B -->|Request/Response| C[contracts/llm.rs]
    C -->|Ripple| D[semantic_cache.rs]
    C -->|Ripple| E[fallback_router.rs]
    F[trajectory_store.rs] -->|Persistence| G[interaction_id]
```

---
*最終更新日: 2026-03-26* (Phase 5 Foundation Integration)

### 👥 Phase 43: Shadow Clone × Cmux Integration
- **変更内容**: 
    - `infrastructure/docker_conductor.rs`: `TaskConductor` の新規実装。5層防御（セマフォ、課金、Bastion、タイムアウト、浄化）の統合。
    - `infrastructure/task_orchestrator.rs`: `TaskEvent` への `conductor_id` 追加。
    - `api-server/routes/agent.rs`: `[DelegateDocker]` 処理を `JobQueue` を用いた完全非同期ディスパッチへ移行。
    - `api-server/app_state.rs` & `main.rs`: `TaskDispatcher` の初期化、`DockerConductor` の登録、およびグレースフルシャットダウン用 `CancellationToken` のバインド。
    - `api-server/stream.rs`: `CoreEvent` に `TaskProgress/Completed/Failed` を追加し、SSE 配信を実現。
- **波及効果**: 
    - LLM 応答のブロッキングが解消され、バックグラウンドでの影分身実行が可観測な状態で運用可能に。
    - `api_integration_tests.rs` における `AppState` の初期化不整合を解消。

```mermaid
graph TD
    A[Agent Chat Loop] -->|Async Enqueue| B[JobQueue]
    B -->|TaskDispatcher| C[DockerConductor]
    C -->|Execute| D[BastionGuard / runsc]
    C -->|Progress Event| E[TaskDispatcher Loop]
    E -->|CoreEvent| F[SSE Stream]
    F -->|Real-time UI| G[Cmux Frontend]
```

### 🛡️ Phase 44: Job Control & Task History
- **変更内容**: 
    - `task_orchestrator.rs`: `TaskDispatcher` に `active_jobs` (CancellationToken) 管理を追加。
    - `docker_conductor.rs`: `cancel` メソッドの実装と確定的コンテナクリーンアップの実装。
    - `routes/jobs.rs`: ジョブ管理エンドポイント（Cancel / Logs）の新規実装。
    - `job_queue/core_ops.rs`: `do_cancel_job` における存在確認の厳格化。
- **波及効果**: 
    - ユーザーがフロントエンドから実行中の影分身を任意に停止・監視可能になる。
    - 不正なジョブ ID や完了済みのジョブに対する操作が適切に 404/400 エラーとして処理される。

```mermaid
graph TD
    A[API Server / jobs.rs] -->|Cancel Signal| B[TaskDispatcher]
    B -->|CancellationToken| C[Async Task Loop]
    B -->|Conductor::cancel| D[DockerConductor]
    D -->|Docker CLI| E[Container Cleanup]
    A -->|Fetch Logs| F[JobQueue]
    F -->|Query| G[jobs Table]
```
### 👥 Phase 14: Syndicate L3 (Agent Guild) MVP
- **変更内容**: 
    - `syndicate_store.rs`: `SqliteSyndicateStore` によるギルド・メンバー管理の実装。
    - `routes/syndicate.rs`: ギルド管理 API の実装。
    - `router.rs`: `/api/v1/syndicate/guilds` のルーティング。
    - `main.rs` & `app_state.rs`: `SyndicateStore` の初期化とインジェクション。
- **波及効果**: 
    - エージェントが組織（ギルド）に所属し、グループ単位での自律的な経済活動やナレッジ共有を行うための基盤が確立。
    - 所有権（Owner）ベースの権限制御が API レイヤーで強制される。

```mermaid
graph TD
    A[SqliteSyndicateStore] -->|Implements| B[SyndicateOps]
    B -->|Injected to| C[AppState]
    C -->|Used by| D[routes/syndicate.rs]
    E[router.rs] -->|Route Registration| D
    F[main.rs] -->|Initialization| A
    G[api_integration_tests.rs] -->|TDD Verification| D
```

### 🔬 Phase 15: Agentic Foundation Expansion (ADR-024/023)
- **変更内容**: 
    - `oracle.rs`: `multi_review` による反復レビューロジック。
    - `dream_state.rs`: `scientific_dream` モードと LLM による仮説生成。
    - `planner.rs`: Markdown 抽出に対応した堅牢な計画分解。
    - `discovery.rs`: LLM ベースのセマンティックツール検索。
    - `task_orchestrator/mod.rs`: `parent_step_id` の伝播とサブジョブ連携。
- **波及効果**: 
    - `DreamState` の初期化時に `LlmProvider` の注入が必要になり、`main.rs` およびテスト用モックのコンストラクタが一斉に変更。
    - `TrajectoryStep` のフィールド追加により、データベース・スキル・プロキシ等の全データ構造が連鎖的に更新。
    - レビュー品質と計画の堅牢性が向上する一方、LLM 呼び出し回数（コスト）が増加。

```mermaid
graph TD
    A[Oracle::multi_review] -->|Iterative Review| B[LlmProvider]
    C[DreamState::scientific_dream] -->|Hypothesis| B
    C -->|Dispatch| D[JobQueue]
    E[StrategicPlanner] -->|Robust JSON| F[TrajectoryStep]
    G[ToolDiscoveryEngine] -->|Semantic Search| B
    H[TaskDispatcher] -->|Causal Linking| F
```

### 🧬 Phase 4 (ADR-025): Poincare Memory Lifecycle & GC
- **変更内容**: 
    - `slm_bridge.rs`: SLM CLI との通信ブリッジ実装。バッチ処理対応。
    - `watchtower.rs`: `do_karma_decay_sweep` における Poincare GC ロジックの統合。
    - `validator.rs`: `ConstitutionalValidator` での `SlmBridge` 利用。
    - `main.rs` & `api_integration_tests.rs`: `UniversalJobQueue` への `SlmBridge` 注入（シグネチャ変更）。
- **波及効果**: 
    - 全ての `UniversalJobQueue::new` 呼び出し箇所（api-server, 統合テスト）で引数修正が必要。
    - 記憶の自動アーカイブ（重要度 < 0.3）がバックグラウンドで開始。

```mermaid
graph TD
    A[watchtower.rs] -->|Batch importance calculation| B[slm_bridge.rs]
    B -->|Command execute| C[slm CLI]
    A -->|UPDATE is_archived| D[karma_logs table]
    E[ConstitutionalValidator] -->|Logical check| B
    F[main.rs] -->|Injection| A
```

---
*最終更新日: 2026-03-27* (Phase 4 / Poincare Memory Lifecycle & GC Integration)
### 🛡️ Phase 45: Vectorless RAG (Hierarchical Knowledge Router - HKR)
- **変更内容**: 
    - `knowledge_indexer.rs`: Markdown パーサと階層インデックス (`TreeNode`) 構築。
    - `hierarchical_router.rs`: LLM 選択肢パース、セマフォ、TTL/Hash 検証付き RouteCache。
    - `app_state.rs` / `main.rs`: `HierarchicalRouter` の `AppState` 注入と初期化。
    - `stream.rs`: OOD 判定後の HKR フォールバックと `ConstitutionalValidator` による検証フローの統合。
- **波及効果**: 
    - `api-server` の SSE ストリームにおいて、未定義の教訓やナレッジに対しても階層ドキュメントを元にした高精度な補足が提供される。
    - LLM 呼び出し回数は「階層の深さ」分増加するが、セマンティック検索では到達困難だった深層ドキュメントへのアクセスが可能になる。
    - インフラコスト（VectorDB等）を抑えつつ、ドキュメントの更新にハッシュベースで追随可能。

```mermaid
graph TD
    A[stream.rs / OOD Detection] -->|Trigger| B[HierarchicalRouter]
    B -->|Fetch Tree/Hash| C[system_state table]
    B -->|Check Cache| D[RouteCache]
    B -->|LLM Traversal| E[LlmProvider]
    B -->|Result| F[ConstitutionalValidator]
    F -->|Validated| G[SSE knowledge_notice]
    G -->|Context Injection| H[Next LLM Generation]
```

---
*最終更新日: 2026-03-27* (Phase 45 / Vectorless RAG HKR Integration)

### 🛡️ Phase 47-B: Infrastructure Stabilization
- **変更内容**: 
    - `UniversalJobQueue`: フィールド（`karma_cache`, `slm_bridge`, `trajectory_store` 等）の完全復元と手動 `Debug` 実装。
    - `dynamic.rs`: LLM 設定取得フローの型安全化 (`Option<String>` -> `String`)。
    - `rss_collector.rs`: SQL 実行時のプール参照エラー修正。
- **波及効果**: 
    - インフラ層全体のビルド不整合が解消。`memory_crystallizer.rs` や統合テストにおける初期化コードが安定。
    - `BackgroundLlmProvider` を利用する全コンポーネントにおいて、設定欠落時のフォールバック挙動が堅牢化。

### 🔬 Phase 48: Invariant-DAG Foundation
- **変更内容**: 
    - `TrajectoryStep`: 検証フィールド (`verified_invariants`, `state_hash` 等) の追加。
    - `planner.rs`: `StrategicPlanner` における新規フィールドのデフォルト値設定。
- **波及効果**: 
    - タスクの実行軌跡に不変条件（Invariants）の検証結果を記録する準備が完了。
    - `TaskDispatcher` における状態ハッシュチェーンの構築が可能に。

```mermaid
graph TD
    A[UniversalJobQueue] -->|Field Restoration| B[Infrastructure Mod]
    B -->|Initialization| C[memory_crystallizer / tests]
    D[dynamic.rs] -->|Type Fix| E[LLM Providers]
    F[TrajectoryStep expansion] -->|Default Init| G[StrategicPlanner]
    G -->|Execution Path| H[TaskDispatcher]
```

---
*最終更新日: 2026-03-27* (Infrastructure Stabilization & Invariant-DAG Foundation)

### 👥 Phase 50: Agentic A2A gRPC Protocol & Worker Detachment
- **変更内容**: 
    - `libs/infrastructure/src/docker_conductor.rs`: 同期実行 (`docker exec`) から非同期ポートマッピング (`docker run -d`) を利用した gRPC 通信アーキテクチャへの全面移行。
    - `libs/infrastructure/src/grpc/a2a_grpc_client.rs`: `async-stream` および `tonic` を活用した `A2aClient` トレイトの具象実装とタイムアウト制御。
    - `apps/shadow-worker`: トークン認証 (`A2A_AUTH_TOKEN`) による 127.0.0.1 バインディングとヘルスチェックを備えたコンテナ用 gRPC サーバーの構築。
- **波及効果**: 
    - メインの `api-server` プロセスのブロッキングが軽減され、重い推論タスクやシミュレーション環境を完全にデタッチされたクリーンなコンテナ環境でスロッティング可能に。
    - ワークスペース全体での通信基盤が gRPC (`tonic`) 前提にアップデートされたことで、今後予定されている分散型マルチノードフェデレーション（Samsara Hub 経由）への展開要件の多くが満たされた。

```mermaid
graph TD
    A[DockerConductor] -->|Start Container| B[Detached Docker (shadow-worker)]
    A -->|Fetch Dynamc Port| C[docker port 50051]
    A -->|Connect gRPC Stream| D[A2aGrpcClient]
    D -->|Execute Task & Auth Token| B
    B -->|TaskProgress Yields| D
    D -->|Stream to SSE| E[TaskDispatcher Loop]
    E -->|Clean Output| F[Job Result]
```

---
*最終更新日: 2026-03-28* (Phase 50 / Agentic A2A gRPC Protocol)

### 🔮 Phase 51-55 波及影響予測 (Planning Phase)
- **Phase 51 (Aiome Node + Agent Card)**: 
    - `api-server/main.rs`: `AppState` への Node IPC クライアント事前注入が必要となり、初期化シーケンスに波及。
    - `api_integration_tests.rs`: `MockAppState` の初期化ブロックへの波及。
- **Phase 53 (ACP / GigEngine 拡張)**: 
    - `routes/gig.rs`: ACP (`PROBE`, `BID`, `COMMIT`) 準拠に伴い、既存 REST エンドポイントのリクエスト/レスポンススキーマが非互換となる可能性大。
    - `samsara-hub/src/routes/federation.rs`: Agent Card メタデータの追加送受信に伴うペイロード拡張。
- **Phase 54 (x402 + AP2)**: 
    - `libs/aiome-contracts/src/commerce.rs`: `CommerceEngine` トレイトに x402 決済インターフェース追加。
    - `infrastructure/src/mock_commerce.rs` & `stripe.rs`: トレイト拡張に伴う全 Mock/実装型のシグネチャ一斉変更波及（Phase 37a と同等規模の波及が見込まれる）。

---
*最終更新日: 2026-03-30* (Phase 51: Agentic Finance & GIG Loop Integration)

### 🏦 Phase 51: Agentic Finance & GIG Loop Integration
- **変更内容**: 
    - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` に `GigEngine` を依存注入 (DI) し、ジョブ完了時に自律的に `GigIntent` を発行する `maybe_publish_gig_intent` メソッドを実装。
    - `libs/aiome-contracts/src/gig.rs`: `GigIntent` 構造体に `metadata` フィールドを追加。コンストラクタ `new()` を実装。
    - `libs/infrastructure/src/intent/mod.rs`: `GigIntent` の構造変更に伴う初期化箇所の修正。
    - `libs/infrastructure/src/test_utils.rs`: `GlobalMockJobQueue` に `fetched_job` フィールドを追加し、`fetch_job` メソッドが意図したジョブを返せるように拡張。
- **波及効果**: 
    - AIエージェントがタスク完了後に自動的に次のタスク（ギグ）を市場へ公開する「自律的経済ループ」が実現。
    - 循環参照や無限ループを防止するため、`gig_depth` を含むメタデータが全ギグインテントに伝播するようになった。
    - `api-server` および `api_integration_tests.rs` の初期化コードが更新され、本番・テスト両環境で `GigEngine` が必須またはモックされる構成に移行。

```mermaid
graph TD
    A[TaskDispatcher] -->|Inject| B[GigEngine]
    A -->|On Completion| C[maybe_publish_gig_intent]
    C -->|Check karma_directives| D{gig_intent: true?}
    D -->|Yes| E[GigIntent::new]
    E -->|Propagate depth| F[intent.metadata]
    F -->|Publish| G[GigEngine::publish_intent]
    G -->|Economic Trigger| H[Other Agents]
```

---
*最終更新日: 2026-03-31* (Phase 52, 53 & Red Team Security Hardening)

### 🛡️ Phase 52: LoRA Archiving & Secure Training Pipeline
- **変更内容**: `archived_lora_models` テーブルの追加および `SoulStore::archive_lora_model` メソッド実装。Rebirth 時に過剰適応をリセットしデータポイズニングを阻止。
- **波及効果**: `SamsaraEngine` に `SoulStore` が注入され、Rebirth 処理のフローが変更されました。また `LoraTrainingService` は `BastionGuard::new_internal` を用いて隔離空間で MLX 学習スクリプトを実行するようになりました。

### 🧠 Phase 53: SoT Deliberation & Cognitive Sentinel
- **変更内容**: `Oracle::multi_review` の実装により、「批判・洗練・最終判定」のループ処理が組まれました。また、`SoTEngine` にて LLM による JSON 構造化抽出を強制し、`SoTProgress` SSE イベントが追加されました。
- **波及効果**: 判断プロセスが同期から非同期かつ複数回の推論へ変更され、より確実な JSON レスポンスが返るようになりました。

### 🛡️ Red Team Pass 1-3 (Security Posture Update)
- **変更内容**: 
    - `Settings::do_get_setting` と `Sentinel::native_bridge_fallback` が Fail-Open から Fail-Closed (Result<?> エラー伝播・拒否) に変更。
    - `ContextEngine::maintain_context` に 10,000 文字のハードリミット、`ExpressionEngine::synthesize_audio` に 2048 Byte のエラーレスポンス上限追加（DoS/OOM 防御）。
    - ワークスペース全体の `bastion` 依存を独自 Git フォークから Crates.io 公式パッケージ `bastion-core = "1.0.0"` へ移行。
- **波及効果**: サプライチェーンリスクが完全に排除されました。設定値やネイティブブリッジの欠損時に不正にテストを通過したり、システムが脆弱なデフォルト状態で起動することが物理的に不可能になりました。

---
*最終更新日: 2026-03-31* (AutoHarness Phase B-D)

### 🛡️ Phase B-D: AutoHarness Security Architecture Integration
- **変更内容**: 
    - `harness_registry` DB テーブル導入にともない、`HarnessRecord` や `HarnessRegistryOps` トレイトを追加。`UniversalJobQueue` に実装を展開。
    - `ConstraintChecker` 内の Regex リテラル展開を `RegexBuilder` の事前コンパイル＋サイズリミット（10KB）へ変更し ReDoS 脆弱性を遮断。
    - `ActionHarness` トレイトに `severity()` を追加し、`WasmHarness` 側で動的に重大度を受け取る構造にリファクタリング。
    - `apps/api-server/src/skill_handler.rs` にて、JobQueue 経由で取得した Active / Shadow ハーネスを `evaluate_step_with_harnesses` ループに注入。
- **波及効果**: 
    - ハーネスの分離アーキテクチャが実現。重大度80以上はアクション自体をブロックする (Active mode) 一方で、80未満は制約違反として記録されるのみ (Shadow mode) という多段階防衛がAPIレイヤーで実体化。
    - 各種 `JobQueue` モック（テスト環境）全体へ `HarnessRegistryOps` が必要となり、広範なテストファイル（`tts_worker.rs`, `dream_state.rs`, `immune_system.rs` 等）に対するトレイト実装波及を及ぼした。


---
*最終更新日: 2026-04-02* (Phase C-2: Watchtower Diagnostic Loop Hardening)

### 🛡️ Phase C-2: Aiome Watchtower Diagnostic Loop Hardening
- **変更内容**: 
    - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` に `AgentRxDiagnostics` を注入。ジョブ失敗時にバックグラウンドで自己診断をトリガーし、結果を `AuditStore` (`TrajectoryStore`) に保存するループを構築。
    - `libs/infrastructure/src/diagnostics.rs`: `AgentRxDiagnostics` に LLM タイムアウト (30s) を導入。実行軌跡が空の場合のガード、および LLM 応答エラー時のダミーレコード生成によるフェイルセーフを追加。
    - プロンプト注入 (Read-path): 注入箇所を `<WATCHTOWER_INSIGHT>` タグで保護し、冪等性 (Idempotency) を確保。リトライ時のプロンプト肥大化を防止。
    - テスト安定化: `test_dispatcher_watchtower_diagnostic_loop` において、固定 sleep を状態監視ポーリングループに置き換え、CI 上の Flakiness を解消。
- **波及効果**: 
    - `AuditStore` (`TrajectoryStore`) トレイトの拡張が必要となり、全モック実装 (`GlobalMockJobQueue`, `immune_system.rs`, `dream_state.rs`, `soul_mutator.rs`) に `fetch_diagnosis` / `store_diagnosis` 等の実装が波及。
    - `TaskDispatcher` の初期化引数が増加し、`api-server/src/app_state.rs` および `main.rs` での依存注入コードが更新された。

---
*最終更新日: 2026-04-03* (Phase B: Cortex Wiki Compiler Implementation)

### 📚 Phase B: Cortex Wiki Compiler Hardening
- **変更内容**: 
    - `libs/infrastructure/src/cortex_compiler.rs` に `run_compilation_cycle` および `generate_article` メソッドを実装し、SQLite トランザクションロックを回避する（読み込み即クローズ → LLM推論 → 個別更新）アーキテクチャを確立。
    - `cortex_documents` テーブルに `compiled` フラグを移行/追加し、再コンパイルを防止。
    - `apps/api-server/src/bootstrap.rs` にて 30分間隔でコンパイルサイクルを実行するバックグラウンドループ（ワーカー）を統合。
    - `apps/api-server/src/routes/cortex.rs` および `apps/api-server/src/api.rs` (OpenAPI) にて、`GET /api/v1/cortex/wiki` および `GET /api/v1/cortex/wiki/:id` エンドポイントを実装。
- **波及効果**: 
    - Cortex の自律的な知識抽象化サイクルが完成。「点（ドキュメント）から面（Wiki）」への知識統合がバックグラウンドで安全に行われるようになった。
    - `sqlx::query!` マクロ由来のコンパイル時メタデータ要求エラーを回避するため、RESTハンドラ等で `sqlx::query` を採用する規約（DBモック/遅延構築耐性の向上）が確立された。

---
*最終更新日: 2026-04-04* (Phase C: Cortex Query Engine Integration + TDD Hardening)

### 🔍 Phase C: Cortex Query Engine Integration + TDD Hardening
- **変更内容**: 
    - `libs/infrastructure/src/cortex_query.rs`: `CortexQueryEngine` を実装。LLM キーワード抽出 + SQLite `LIKE` 検索による RAG 基盤。`max_context_chars` フィールド追加（builder pattern、デフォルト 8000）。`suggest_questions` を DB ベースの動的サジェストに移行。
    - `apps/api-server/src/routes/cortex.rs`: `/api/v1/cortex/query` (POST) および `/api/v1/cortex/suggestions` (GET) ハンドラ実装。OpenAPI description を動的サジェストの説明に更新。
    - `apps/api-server/src/stream.rs`: OOD 検出時の第2フォールバックとして `CortexQueryEngine` を SSE ストリームに統合（confidence ≥ 0.5 ガード付き）。
    - `apps/api-server/src/mcp/server.rs`: `cortex_search` MCP ツール登録・実行。RBAC ホワイトリストに追加。
    - `apps/api-server/src/app_state.rs`, `bootstrap.rs`, `api.rs`, `api_integration_tests.rs`: DI 登録、初期化、OpenAPI スキーマ、テスト AppState に `cortex_query` を統合。
- **波及効果**: 
    - `CortexQueryEngine::new()` のシグネチャは変更なし（後方互換性100%）。`with_max_context_chars()` はオプショナル builder。
    - SSE ストリームにおいて HKR → Cortex の2段フォールバックが確立。OOD 時の LLM 呼び出し回数は最大4回（HKR 階層 + Cortex キーワード抽出 + 回答生成）に増加するが、confidence ガードにより不要な注入は防止。
    - `suggest_questions` の戻り値が動的になったため、フロントエンド側で表示更新への対応が必要（Phase D 候補）。

```mermaid
graph TD
    A[stream.rs / OOD] -->|Fallback 1| B[HierarchicalRouter]
    A -->|Fallback 2| C[CortexQueryEngine]
    C -->|Keyword Extraction| D[LlmProvider]
    C -->|LIKE Search| E[cortex_concept_index]
    C -->|Fetch Articles| F[cortex_wiki_articles]
    C -->|Generate Answer| D
    G[routes/cortex.rs] -->|query_handler| C
    G -->|suggest_questions_handler| C
    H[mcp/server.rs] -->|cortex_search| C
```


# 2026-04-08

## AppDataResolver Phase 2-PRE
*   **Resolved files**:
    *   `apps/api-server/src/bootstrap.rs`
    *   `apps/api-server/src/internal_services/dream.rs`
    *   `libs/infrastructure/src/artifact_store.rs`
*   **Ripple effect**: Local dev uses `workspace/` mapped automatically via config. Release mode uses `~/Library/Application Support/com.aiome.nexus`. Removed legacy hardcoded paths which clears the final blocker for Phase 2C Tauri Packaging.
