## [Unreleased] - 2026-04-09
  - **Atomic Security Gating**: プラン（Goal分解）に含まれるすべてのステップを、キュー投入前に一括検証する「Plan-First Verification」を実装。一部のステップが実行されてから違反を検知する「部分実行リスク」を完全に排除。
  - **Semantic UX Refinement**: ユーザー介入通知に専用の `TaskEvent::AwaitingInput` を新設。UI上で「エラー」と「承認待ち」を明確に区別可能にし、インタラクティブなセキュリティ体験を提供。
  - **Execution Robustness**: `Goal` カテゴリのジョブデキューバグを修正。ConstitutionalValidator と AdaptiveImmuneSystem の多層防御を TaskDispatcher に完全統合し、全インフラテストを 100% GREEN 化。

## [Unreleased] - 2026-04-09 (Previous)

### Added
- **Phase 3-D: Onboarding & Global Alignment [完了]:**
  - **Onboarding Modal Hardening**: `OnboardingModal.tsx` and `ModelSetupStep.tsx` を完全にトークン化し、Golden Rule U-002 (HEX/RGBA排除) をシステムの起点から強制。
  - **Onboarding i18n Expansion**: 「Manifestation (権身)」「Abyss Vault Security」「Experience Level」といった高度な設定項目を `en.json` / `ja.json` へ追加し、初期導線の多言語化を完遂。
  - **Infrastructure Stabilization**: `shadow-worker` における HTTP クライアントの所有権エラーを修復し、`KarmaTaxonomy` のフォールバック値を大文字ホワイトリスト（General, Uncategorized）へ正規化。
  - **Premium UI Polish**: `VoiceStore` のカスタムトーストシステム導入および `SettingsPage` の未使用インポート削除によるコード衛生の徹底。

## [Unreleased] - 2026-04-08

### Added
- **Phase 3-C: Management Console Mastery [完了]:**
  - **Full UI Hardening (U-002)**: Eliminated 100% of hardcoded HEX/RGBA/RGB color values and animation timings across all major Management Console components (`Timeline`, `DiagnosticsHistory`, `SoTProgressBar`, `VoiceStore`, `LoraTrainingView`, `BiomeDialogueView`, `ImmuneSystem`, `ExpressionPipeline`). 全ファイルにおいて `scripts/test_ui_hex_violations.py` にて 0 違反を達成。
  - **Dynamic localization**: `Timeline`, `DiagnosticsHistory`, `SoTProgressBar` 等の動的ステータス文字列を i18n 化し、日英の完全同期を完遂。
  - **Premium Feedback System**: `VoiceStore` における `alert()` を排し、Framer Motion を活用したトースト通知システムを導入。
  - **Responsive Layouts**: 複雑な設定項目を持つ `Lora` および `Biome` ビューにメディアクエリを導入し、レスポンシブ対応を強化。
- **Phase 3-B: UI Tokenization & Security Hardening (Current):**
  - **CSS Token Validation Strictness**: `test_ui_hex_violations.py` スクリプトを拡張し、`rgba()` および `rgb()` ハードコードの検出を自動化。テストスコープを `src/` 全域に拡大。
  - **Canvas CSS Bridge (`cssVar.ts`)**: Canvas や WebGL 等の DOM 外コンテキストでも CSS 変数を適用するための O(1) キャッシュ付きカラー解決ユーティリティ `cssVar.ts` を開発。`CausalVisualizer`, `GraphView`, `AvatarViewerModal` の DRY 違反とハードコードを一掃。
  - **SSRF Shielding (Global HTTP Pool)**: `self_diagnosis.rs` および `bootstrap.rs` (Ollama 検出) で独自生成されていた孤立した `reqwest::Client` を撤廃し、厳格なタイムアウトとリダイレクト制御を備えた `aiome_core::http::get_http_client()` プールへ統合完了。これによりインフラ全域の SSRF 対策が完遂。
  - **Test Target Isolation**: Deep Scan AST マトリクスで検出された開発用モック構造体（`MockSoulStore`, `MockTtsProvider`）の `cfg` 隔離漏れを修正し、`#[cfg(any(test, debug_assertions))]` にてプロダクションビルドからの完全遮断を保証。
- **Phase 3-A: Deep Scan Remediation & Debt Clearance [完了]:**
  - **E2E Pipeline Restoration**: `search_api_key` が UI および DB 設定で永続化されない論理ギャップを解消し、TrendSonar への設定注入を正常化 (TDD/RED-GREEN)。
  - **SSRF 脆弱性の排除**: `cortex_ingester.rs`, `tts.rs` などインフラストラクチャ全域の `reqwest::Client::new()` (計8箇所) をリダイレクト防御済みプール `aiome_core::http::get_http_client()` に置換。
  - **Golden Rule (U-002) 強制適用**: React フロントエンド 17 コンポーネントに散在していた 80 箇所以上の HEX ハードコード色指定 (`#fff`, `#ff4757` 等) を全掃討し、すべて CSS 変数（Vanilla Tokens）へ移行完了。
  - **Canvas CSS Bridge 実装**: vis-network エンジンを利用する `CausalVisualizer.tsx` 等で、DOM 非依存の Canvas に向けて動的に CSS 変数を注入する `cssVar()` ヘルパーを開発・適応。
- **Phase 2B-2 Reflexion Hotfix (Limit Break Patch) [完了]:**
  - **Settings API `view_mode` White-listing**: `OnboardingModal` から送信される `view_mode` が `allowed_keys` の制約により `SecurityViolation` を誘発していた致命的バグを修正。同時に `ui` カテゴリを認可済みリストに追加し、`SettingsPage` の初回起動および設定保存フローを正常化。
  - **DreamService DB/Env Fallback Sync**: UI 上から設定された `SEARCH_API_KEY` および `X_BEARER_TOKEN` が `DreamService` の起動ループに反映されず、常に `.env` からのみ読み取られていた論理ギャップ (`get_setting_value` なしの環境変数直読み) を修正。再起動時に必ずユーザーが UI 上で保存したトークンを優先取得し、存在しない場合のみ環境変数にフォールバックする一貫稼働を確保。
  - **XSignalProbe SSRF 防御強化**: `X_BEARER_TOKEN` を使用したトレンド取得で `reqwest::Client::new()` を回避し、セキュアな単一共有プールである `get_http_client()`（SSRF リダイレクト拒絶およびタイムアウト適用済み）へとリファクタリング。合わせて API ドメインを旧 `api.twitter.com` から `api.x.com` に移行。
  - **OpenAPI Schema Sync**: `soul.rs` の `InitSoul` 関連構造体に `#[derive(utoipa::ToSchema)]` が不足していた仕様を修正し、`api.rs` の Swagger ドキュメントルーターにエンドポイントを正式登録。
  - **フロントエンド i18n & UI パッチ**: `SettingsPage.tsx` の Security カテゴリに `X API Bearer Token` 入力 UI を追加し、多言語対応 (`ja.json`, `en.json`) のキー設定と再起動警告のハードコード撤廃を完了。

- **Phase 2B-2 Reflexion Hotfix Part 3 (TDD Zero-Dependency & UI Tokens) [完了]:**
  - **WASM Mock Testing (TDD 100%)**: `tests.rs` の Extism WebAssembly スキルのバリデーションテストにおける `#ignore` を除去。隔離された `wasm_test_temp` ワークスペース下で Rust による本物の Extism-PDK モック (`hello_skill.wasm`) を動的ビルドし、`include_bytes!` にて静的埋め込みを実施。実行マシンに `wasm32-unknown-unknown` が不在でも OOM や Timeout といったレイヤー3サンドボックス防御が作動することを完璧に自動証明する TDD パイプラインを完遂。
  - **Vanilla CSS Token Synchronization**: ゴールデンルールに基づき、`SettingsPage.tsx` および `OnboardingModal.tsx` に数十件の規模で散在していた `HEX Color (#fff, #000, #5865F2)` を全撤廃。`index.css`（tokens.css）へ `--text-inverse`, `--bg-primary`, `--bg-inverse` を新設し、UI層全体を完全なるトークン駆動アーキテクチャへと適応・是正。
  - **React State Refactoring (`@ts-ignore` 撲滅)**: `OnboardingModal.tsx` において、`window.__viewMode` に対するアドホックな型抑圧（`@ts-ignore`）による状態管理を廃止。正規の `useState` フローへの変換を施し、`npm run lint` の GREEN を確保。
  - **OpenAPI Schema Synchronization**: `docs/openapi.json` およびフロントエンドの TypeScript 型定義を最新の バックエンドスキーマ（`Settings`, `InitSoul` 等）へと再同期させ、型システムの乖離を修復。

- **Phase 2B-2 Foundation (Perfect Plan Rev.6 / Limit Break) [完了]:**
  - **パス標準化の限界突破 (Phase 2-PRE 完遂)**: `bootstrap.rs` 内に残存していた `std::fs::read_to_string("SOUL.md")` という相対ハードコードパスを全消去し、`AppDataResolver` 経由の絶対パス照会へと是正。ゴーストバグの発生を未然に防止。
  - **SOUL 初期生成 API**: `POST /api/v1/soul/init` エンドポイントを新設。初回起動時の `OnboardingModal` からの初期システム人格生成フローをアクティブ化し、フロントエンドとバックエンドの同期を実現。
  - **フェイルセーフ防御網 (Safety Net)**: `SoulMutator` 内の `transmute()` に SOUL.md 不在時のフォールバックおよび自己修復ロジックを追加。さらに `ConstitutionalValidator` を `SoulMutator` パイプラインに統合し、生成人格テキストに対する破壊的キーワードの公理的ブロックを保証。
  - **Release ビルドゲートの消去**: 接続テスト用の `#[cfg(debug_assertions)]` 防壁を `settings.rs` および `router.rs` などの関連階層から撤去。本番ビルドでも OnboardingModal の LLM 接続テストが機能するよう解放。
- **Phase 2B-2 Reflexion (Security / UX Audit) [完了]:**
  - **Prompt Injection DoS Block**: 初期人格生成APIで `ai_name` を64文字・改行排除するサニタイズを実装し、コンテキスト枯渇（OOM）やインジェクションを強固に防御。名前が空になるケースは "Genesis" として自己修復。
  - **O(1) Streaming Validation**: `ConstitutionalValidator` のテキスト検査を O(N) の `to_lowercase()` メモリコピーから、`LazyLock` キャッシュによる O(1) 正規表現ストリーミング検査へと劇的にパフォーマンス改善。
  - **Fallback UX Hardening**: 起動用モーダル（`OnboardingModal`）にて通信障害時にソフトブリック（暗黙クローズと永遠の待機）を引き起こしていた問題を撤廃。`try/catch` および React State を用いた堅牢なエラーメッセージ描画フローへリファクタリング。

- **X Signal Probe (Reqwest Direct API Integration) [完了]:**
  - **`XSignalProbe`**: `reqwest` と `X_BEARER_TOKEN` を用いた超軽量な X API トレンド収集アダプタを実装（`TrendAdapter` トレイト準拠）。
  - **Rate Limiting**: 24時間に1回の厳格なインメモリ・レートリミッター（DashMap）をアダプタ深部に内包させ、X API クレジット枯渇を防ぐ。
  - **DreamService Integration**: `ExternalTrendSonar` に `XSignalProbe` を登録し、アイドル時の自律思考（DreamState）へ X のリアルタイム信号を統合。
  - 第一原理分析に基づき、複雑な xmcp サイドカーをバイパスし、Rust ネイティブなセキュア軽量実装とした。
- **Self-Organizing SoT Engine (ADR-032) — Dochkina (2026) arXiv:2603.28990 統合 [完了]:**
  - **`CoordinationProtocol` enum** (`contracts.rs`): Sequential / Coordinator / Broadcast の3つの協調プロトコルを型安全に定義。Endogeneity Paradox の知見に基づき Sequential をデフォルトとして採用。
  - **Sequential マルチパス熟議**: 各 Thinker が前任者の完成済み出力を全て見た上で**自律的にロールを発明**する熟議パイプライン。固定ロール割当（アンチパターン）を排除し、タスク文脈に応じた動的ロール創発を実現。
  - **Voluntary Self-Abstention**: LLM が `[ABSTAIN]` マーカーを返すことで自発的に辞退できる機構。トークンコストを自然最適化。`SoTEvent::ThinkerAbstained` で管理コンソールから可視化可能。
  - **Capability-Aware Protocol Fallback**: `auto_protocol=true` 設定時、プロバイダ名に基づきモデル能力閾値を判定。高能力モデルには Sequential（自律性で品質向上）、低能力モデルには Coordinator（固定構造で安定化）を自動選択。
  - **既存 Critic 品質ゲート保持**: Sequential マルチパスの後に P-2 Critic スコアリングが構造化検証として機能。論文の知見と Aiome の品質保証を両立。
  - **ADR-032** (`docs/decisions/032-self-organizing-sot.md`): 設計判断の正式記録。
- **Chaos Engineering Infrastructure (Layer A-C) [完了]:**
  - **フォルトインジェクション基盤** (`tests/common/chaos.rs`): `ChaosMode` enum と `ChaosLlmProvider` ラッパーを `tests/` ディレクトリに完全隔離して新設。本番バイナリへの漏洩可能性はRustコンパイラの保証により数学的にゼロ。
  - **カオス実験スイート** (`tests/chaos_experiments.rs`): 6つの定常状態仮説テストを実装（SoT空レスポンス、SoT不正JSON、SoTタイムアウト、SamsaraEngine LLM障害、CircuitBreaker強制Open、ConstraintChecker巨大出力）。全テスト0.10秒で完了、既存85テストへの干渉ゼロ。
  - **`/chaos` ワークフロー** (`.agent/workflows/chaos.md`): 「仮説→障害注入→検証→学習」の4フェーズカオスエンジニアリング・ループを体系化。
  - **`/god-mode` 5段階化**: Phase 3 (Reflexion) と Phase 5 (Red-Team) の間に Phase 4 (Chaos) を挿入し、品質パイプラインを強化。
- **E2E Infrastructure Hardening (Layer 0-2, 0-3, 0-4) & Agentic UI (Layer 1-1) [完了]:**
  - **Inochi2D Asset Delivery API (`/api/v1/avatar/inochi2d/:filename`)**: パストラバーサル攻撃を完全に無効化する `PathSandbox`（Jail）を実装。また非同期・ノンブロッキング I/O (`tokio::fs::read`) によって、大容量の `.inx` ファイルロード時におけるサーバ応答の遅延スパイクとブロッキングを完全排除。Golden Rules に厳格に従い、全ての `unwrap()` 系アンチパターンを払拭した堅牢なインフラストラクチャを構築しました。
  - **Whisper Monologue API (`/api/v1/whisper/monologue`)**: `AgentSoul` の `experience_buffer` に対する高速インメモリ・フィルタリングおよびカーソル・ページネーションを追加。エージェントが舞台裏で自身を省察（Reflection）した `Whisper` コンテンツを抽出配信。
  - **SoTProgressBar (Society of Thought UI)**: `useSystemVitality` 経由の `sot_progress` SSE ストリームをパースし、複数のロール（専門家AI）の自律思考プロセスとその途中経過、および評価スコアを追跡・リアルタイム可視化。再接続時（Mid-stream connection）のフォールバック耐性と、`useMemo` (Derived State) を用いた State Thrashing (ダブルレンダー) 抑制による極限の描画パフォーマンスを実現。

- **BeliefConsistencyGate Integration (Phase D):**
  - `CortexSynthesizer` に `BeliefConsistencyGate` を統合し、AI のコア信念に反するデータを除外する機能を追加。
  - Synth データセット出力形式を `export_to_jsonl` にて標準的な **ShareGPT 形式**へ移行し、Axolotl や Unsloth との互換性を確立。
  - LLM ハルシネーションによる JSON 解析エラーならびに JSONL 保存時のシリアライズのサイレント・フェイラー（エラー握り潰し）を完全排除。

### Changed
- `libs/shared/src/strings.rs`: 高効率な文字列切り詰めユーティリティ (`truncate_bytes_safely`, `truncate_chars_safely`) を新設。
  - `Cow<'a, str>` によるゼロアロケーション・パスの最適化。
  - 数学的に厳密な Unicode 境界判定 (`char_indices().nth()`) の採用。

### Changed
- システム全体の文字列スライス操作 (`&str[..N]`) を `shared::strings` の安全な関数に全置換（UTF-8パニック耐性を極大化）。
  - 対象: `context_engine`, `task_orchestrator`, `expression/engine`, `timesfm`, `api-server`。
- `api-server` の `safe_truncate` を `shared::strings` へ委譲し、内部冗長性を排除。

### Fixed
- **Reflexion Loop 1 (SoT Engine):** `society_of_thought.rs` での JSON エンティティ抽出時における潜在的な文字列スライスのパニック（Index Out Of Bounds）脆弱性を除去し、不要なループ処理の冗長コードを排除。
- **Reflexion Loop 2 (UI):** `SoTProgressBar.tsx` において Golden Rule U-001/U-002 違反であった Tailwind ユーティリティクラスを完全排除。Artemis Design System (`tokens.css`) 基準のインラインスタイリングによる堅牢な UI に改修し、未描画問題（Tailwind非搭載環境下のCSS崩壊）を解決。
- **Reflexion Loop 3 (Taxonomy):** `libs/infrastructure/src/job_queue/taxonomy.rs` において、長文の `lesson` 処理時に発生し得た UTF-8 バイト境界判定での致命的パニック脆弱性 (VULN-62) をゼロアロケーション文字スライスで完全修復。
- マルチバイト文字（日本語、絵文字等）の境界で文字列を切り詰める際に発生していた潜在的なサーバパニック (C-6) を完全解消。
- 文字列切り詰め時における O(N) 計算量爆発の脆弱性を修正。
- **E2E Test Stabilization (Layer 0) [完了]:**
  - **Context Isolation:** `SystemVitalityProvider` の導入に伴う `<TokenSavingsIndicator>` の複数箇所でのレンダリング仕様に対応するため、`token_savings.spec.ts` での取得アサーションを `.first()` からスコープ指定（`.story-flow`）へ厳格化し、テストのFlakinessを排除。
  - **View Mode Navigation:** アプリケーションの Sidebar の出し分けロジック仕様に合わせて、E2Eテスト（`ui_fixes.spec.ts`, `demo.spec.ts`, `home_v2.spec.ts`）内のローカルストレージ `aiome_view_mode: 'advanced'` 初期化を必須化し、テスト実行時の非表示タブ要素によるハングアップを解決。
  - **DOM Refactoring Sync:** `home_v2.spec.ts` 内での古いセレクタ `.empty-flow` を最新の実装である `.artemis-status` へ追従させ、Playwrightのフルテストスイートの **100% GREEN (All Pass)** を達成。

### Added
- **Infrastructure (ADR-025: Agent-Native Document Discovery):**
  - Duke大学研究 (2026) に基づき、Cortex Wiki記事をファイルシステム階層として物理投影する `CortexFileProjector` を新設。`content_hash` ベースの差分投影により冪等性を保証。
  - `DreamState` に `DiscoveryMode` enum（`SemanticSearch` / `AgentNative`）を追加。Scientific Experiment タスクにおいて投影された `_index.md` をプロンプトに注入し、自律探索精度を向上。
  - `CortexCompiler` のコンパイルサイクル後に自動投影トリガーを追加（`with_file_projector` Builder パターン）。
  - `DreamService` 起動時に初回 Cortex FS 投影を実行し、Agent-Native Discovery を有効化。
- **Infrastructure (SEO Intelligence):** 
  - Implemented concrete `SerpAnalysisAdapter` with `reqwest` HTTP capabilities, `sanitize_snippet` HTML stripping, and a process-global `DashMap`-based in-memory rate limiting to strictly prevent external API quota depletion by continuous autonomous Agent loops. Added severe boundary constraints (query length and empty string filters).
  - Implemented concrete `WordPressAdapter` relying on WP REST API v2 supporting draft/publish routing and standard auth workflows. Added future `AbyssVault` migration tracking and strict payload bounds checking (10MB limits, empty content shielding).
  - Eradicated all `unwrap()` calls across production and tests, and established a stable `test-utils` cross-crate mocking architecture enforcing strict separation of concerns from integration tests interacting with `api-server`.
- **API Server (DreamService):** 
  - Subscribed `DreamService` to dynamically injected real-world Search sources by coupling `WebSearchAdapter` and `SerpAnalysisAdapter` to `ExternalTrendSonar` based on environment configurations, fulfilling Phase β implementation goals.
- **API Server (PublishPipeline):** 
  - Upgraded CLI bootstrap sequence to dynamically load `WordPressAdapter` if `WP_API_URL` exists, bridging the final gap of Phase γ SEO capability integrations.
- **SEO Intelligence Component (Phase 1-3) [完了]**:
    - `api-server/src/stream.rs`: MCP ツールチェイン結果に対し `shared::guardrails::sanitize_for_prompt()` を適用し、外部からの Prompt Injection を遮断。
    - `libs/infrastructure/src/task_orchestrator/seo_content.rs`: `SeoContentConductor` を新規実装し、TaskDispatcher からの `seo_content` ジョブを専用にハンドリング。`GenericLlmConductor` との結合を排除し、SEO特化の高品質プロンプト・ライフサイクルを提供するアーキテクチャ的疎結合（Decoupling）を実現。
    - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` にテストおよび機能診断用ヘルパー `get_conductor_for` を実装。
    - `apps/api-server/src/bootstrap.rs`, `app_state.rs`: `PublishPipeline` を `AppState` に統合し、SEOコンテンツ生成後のCMS/SNS配信基盤をブートストラップ化。本番環境ではパブリッシャー未登録時に警告ログを出力。
    - `apps/api-server/src/api_integration_tests.rs`: テスト用 `TaskDispatcher` に `GenericLlmConductor` と `SeoContentConductor` を正しく登録。結合テスト `test_seo_content_conductor_exists` が実環境でも正常にパスするよう修正。
- **Silent Error Suppression Purge (Observability Hardening) [完了]**:
    - `task_orchestrator/mod.rs` (17箇所), `llm_conductor.rs` (2箇所), `csam.rs` (1箇所), `docker_conductor.rs` (13箇所): async 操作の `let _ = *.await` によるサイレントエラー握り潰しパターンを完全駆逐。すべて `if let Err(e) = ... { tracing::warn!/error! }` に置換し、本番環境でのエラー可観測性を回復。
    - `task_orchestrator/mod.rs`: `dispatch_job` の conductor 検索ロジックの重複を排除し、`get_conductor_for()` に一元化。
- **Precomputed Relational Intelligence (AST Impact Dependency Graph) [完了]**:
    - `scripts/nurture_auditor.py` を再設計し、O(1)のディレクトリ走査（`os.walk` と `node_modules` 等のインプレース枝刈り）を用いた超高速なASTマトリクス抽出機能を拡充。さらに、`use` / `mod` / `impl` (Rust)、`import { X }` (TSX)、および `--token-name` / `var()` (Vanilla CSS Token) の物理依存を抽出し、`.context/impact_graph.json` に出力する自己完結型の静的依存解析エンジンを実装。フロントエンドにおけるパスエイリアスの分断問題を克服するため、「インポートされたシンボル名」でのエッジ定義へ最適化。
    - `scripts/impact_query.py` CLI ツールを新規追加。`impact_graph.json` を BFS アルゴリズムで走査し、循環参照（Circular Dependency）による無限ループを防止するため `visited` ハッシュ保護を搭載。変更予定シンボルに対する被害半径（Blast Radius: `[WILL BREAK]` / `[LIKELY AFFECTED]`）を深さベースで確定的に算出。`--exclude-tests` オプションによりテストコードのノイズを除外可能。
    - **Agent Governance & Workflow Integration**: 外部開発ツールやサードパーティライセンス（GitNexus等）に対するリスキーなプラットフォーム統合を破棄し、100% 自前実装。`AGENTS.md` のコア・システムプロンプトに「Mandatory AST Impact Analysis」の強制的ディレクティブを追加。さらに、`/preflight`, `/task`, `/expert-review`, `/sunset`, `/perfect-plan`, `mission-control-principles.md` などの主要エージェント・ワークフロー全てに対し、コード改修前の `nurture_auditor.py` スキャンおよび `impact_query.py` 実行による物理的波及テストを事前義務化。

    - `apps/management-console/DESIGN.md` を新設。`tokens.css`, `animations.css`, `App.css`, Golden Rules の全設計知識を Google Stitch フォーマットで統合した約300行の包括的デザインシステムドキュメント。
    - エージェントが UI 改修時に参照する**単一の設計真実**として、色パレット、タイポグラフィ階層、コンポーネント仕様、アニメーションカタログ、Do/Don't、Example Component Prompts を網羅。
    - すべての値を `var(--xxx)` で記述し、Golden Rule U-002 との完全一貫性を担保。HEX生値は参考値としてのみ括弧内に記載。
    - `AGENTS.md` のドキュメント同期ルール (項目10) に `DESIGN.md ↔ tokens.css` の同期チェックを追加し、ドリフト防止を制度化。
- **External Design Catalog (awesome-design-md Integration) [完了]**:
    - `VoltAgent/awesome-design-md` (MIT, ⭐16.6k) から厳選10社（Linear, Stripe, Vercel, Notion, Raycast, Supabase, Apple, Spotify, Airbnb, Framer）のデザインシステムを `.agent/design-catalog/` に配置。
    - エージェントスキル `.agent/skills/design-catalog.md` を新設。「〇〇風のUI」要求時にのみ参照するオプトイン設計で、Aiome 自身の UI への誤適用を防止。
    - 各ファイルに利用注意ヘッダーを付与し、ライセンス・帰属を明確化。
- **MLIT (国土交通省) MCP Server Integration Templates [準備完了]**:
    - 不動産情報ライブラリ MCP (`mlit-geospatial-mcp`) および 国土交通データプラットフォーム MCP (`mlit-dpf-mcp`) の接続テンプレートを `mcp_servers.json.example` および `.env.example` に追加。
    - ワンコマンドセットアップスクリプト `tools/setup-mlit-mcp.sh` を新設。Python venv作成、依存インストール、`~/.aiome/mcp_servers.json` 自動生成を実行。
    - 対応データ: 地価公示、不動産取引価格、都市計画、防災情報（洪水・津波・土砂災害等）、人口推計、教育・医療施設、PLATEAU（3D都市モデル）等、計47種以上のオープンデータへの自然言語アクセスを実現。
- **Agentic Workflow Hardening (agent-skills Integration) [完了]**:
    - 新規ワークフロー `/simplify`（構造化リファクタリング）、`/sunset`（非推奨化・移行）、`/ship`（本番出荷ライフサイクル管理）を `.agent/workflows/` に追加。
    - 新規スキル `context-optimization.md`（エージェントのコンテキスト管理最適化ガイドライン）を `.agent/skills/` に追加。
    - 既存ワークフロー `/reflexion`, `/code-review`, `/release-preflight`, `/tdd` に Anti-rationalization テーブル（AIの自己合理化を防止する「言い訳と現実」の対照表）および Red Flags（危険信号）セクションを追加。
- **Phase 1-2 Security & Infra Reflexion Hardening [完了]**:
    - **Security Bypass Prevented**: `DynamicLlmProvider::stream_complete` において、ストリーミング通信時に欠落していた事前セキュリティフック（Pre-execute Hook）の強制呼び出しを実装し、保護機構の迂回を遮断しました。
    - **CRDT Data Integrity**:分散P2Pの `UniversalJobQueue::sync_timeline` 内部において、PostgreSQL および SQLite 上での `UPSERT` 永続化を実装し、ノード再起動時のステート（CRDT Timeline Blobs）の喪失（揮発バグ）を防止。
    - **UI Stability**: Management Console の `SkillVault.tsx` において、外部APIから `tools` 配列が未定義（`undefined`）で返却された際の React Component Crash（WSOD: 真っ白な画面）を防ぐフォールバックを実装しました。
    - **Resilience Engine Fixes**: `MemoryCrystallizer` （記憶の結晶化）および `napi-bridge` （カルマ抽出）にある `let _ =` によるサイレントエラー隠蔽を排除し、永続化レイヤーの完全なトラッキングと可視化を達成しました。
    - **Negative Jitter Mitigation**: ガードレールのひとつ `BeggingSupervisor` のペナルティタイマー計算において、負のタイムスタンプが引き起こすタイマー期間マイナス化バグを `.unsigned_abs()` を用いて修正しました。
    - **Arena Tie Recovery**: `SkillArena` において、対戦する両者のエージェント/スキルが同時にクラッシュ（Err()）した場合のパターンカバレッジ漏れを修正し、安定して引き分け状態を返却するよう改修しました。
    - **Zero-Panic Infrastructure Pipeline (AADP v5) [完了]**: 1423件のテストと全てのアサーションを含む全プロジェクトから、不正な `unwrap()`, `expect()`, `panic!()` などのクラッシュ要因を完全に根絶。全ての `anti-patterns.yml` ルール (AP-003~AP-006) を Error に昇格し、`make preflight` によるシングルソース・検証ゲート（Format, Clippy, AST Schema, Doc-Sync）を導入してフェイルセーフを強制完了。
    - **RTK Token Optimization (Phase 2) [完了]**: RTK (Rust Token Killer) の知見を用いた `OutputFilter` モジュールを `libs/infrastructure` に自前実装し、エージェント自律実行時のLLMトークン消費をスマートに圧縮。エラーや診断情報を保持しつつ、GitやCargo、Node/npmコマンド出力のボイラープレートや重複行を最大90%削減するフィルタリングロジックを確立。API Serverの `ToolCallRouter` のポストフック後に透過的に統合し、JSON通信の破壊リスクを回避した安全な最適化パイプラインを構築しました。
    - **ContextBudget & JobBudget Extension (Phase 3) [完了]**: `ContextBudget` に `max_tool_output_chars` を、`JobBudget` に `saved_chars` のアトミックトラッカーを追加。`ToolCallRouter` のフィルタ通過後に、トークン節約数を `tracing::info!` 経由でシステムログ（Management Console の下地となるログ）やコンソールに出力するトラッキング機能を統合。
    - **Token Savings SSE Streaming (Phase 3.5) [完了]**: `ToolExecutionEvent::TokenSaved(usize)` を追加し、`tool_call_processor.rs` および `stream.rs` において、`OutputFilter` が削減したトークン数を上位側（AgentRx/Orchestrator）へルーティング。自律稼働チャットストリームにおいて専用の `token_saved` SSE イベントを発火。
    - **Token Savings UI Integration (Phase 3.5 Conclusion) [完了]**: フロントエンド側の `App.tsx` において `token_saved` SSE イベントを大域 CustomEvent フック経由で捕捉し、累積トークン節約量（`sessionSavedChars`）を状態管理する基盤を実装。
    - **Premium Data Visualization UI**: Framer Motion の `useSpring` と `useTransform` を活用した `TokenSavingsIndicator` コンポーネントを新設。`AgentConsole`, `StoryFlow`, `BiotopeView` なの主要UI画面に対し、リアルタイム節約データと仮想コスト削減額のガラスモーフィズムバッジを統合。E2Eテスト検証込み。

### Added
- **GlassWorm Shield (Invisible Unicode Injection Defense) [完了]:**
    - **Core Primitive**: `shared::guardrails::strip_invisible_unicode` を実装。ゼロ幅スペース（ZWSP）、Web Tagsブロック、BIDI制御文字などの悪意ある不可視Unicode文字を O(N) で高速に除去するサニタイズ処理を構築。
    - **Deep Object Sanitization (Gig Routes)**: `apps/api-server/src/routes/gig.rs` などのAPIエンドポイントにおいて、ネストされたJSONメタデータ構造体や `AcceptanceCriteria` (WASM CID, Rubric Prompt列) に至るまで、serde_jsonによるラウンドトリップを併用して一単位残さずディープにサニタイズ。
    - **Federation Array Guard (P2P)**: `apps/samsara-hub/src/main.rs` のバッチ受付において、10MBを超える大容量スナップショットの負荷を抑えるため、`KarmaEntry`、`ImmuneRule`、`ArenaMatch` 内の特定テキスト・ID・オプション型文字列フィールドだけをダイレクト指定でサニタイズする高パフォーマンスなアリーナ防御を統合。
    - **MCP & Internal Guards**: `mcp/discovery.rs` や `cortex_ingester.rs` 等の外部データ/コマンド引数の受け口全体に展開し、不可視文字を利用したLLMコマンドインジェクションやSandbox Bypassリスクを未然に排除。

- **Analytics MCP Client Hub Integration (Phase 6 MVP Inclusion) [完成]:**
    - **MCP Process & Security Hardening**: Aiomeインフラ内に外部MCPサーバー（GA4, Stripe, Vercel等）を接続・管理するためのクライアントハブを構築。`McpClient::spawn` において `npx`, `node`, `python3`, `uvx` のMCP用実行バイナリと `@modelcontextprotocol/` などの公式パッケージプレフィックスのみを許可する厳格なホワイトリストを実装し（P-6）、Sandbox Bypass（RCE）の脆弱性を完全に塞いだ。
    - **Configuration Sync & SSOT (P-7)**: SQLiteデータベースと設定ファイル (`mcp_servers.json`) の非同期（Desync）を防ぐため、専用の `PUT /api/skills/mcp/config` エンドポイントを新設。構成の更新と関連プロセスのホットリロードを安全・アトミックに同期実行するSingle Source of Truth（SSOT）アプローチを採用した。
    - **Environment Variable Resolution (P-3)**: `mcp_servers.json` 内での `$` プレフィックスによる環境変数解決・サニタイズ展開（例: `$STRIPE_SECRET_KEY`）をサポートし、再起動時の自動展開機能を実装。
    - **Registry & Process Cleanup (P-8, P-9)**: ゴーストMCPツールがシステムプロンプトに残留する問題を防ぐため `RegistryManager::clear_mcp_servers` を実装。また、個別プロセス管理のための `McpProcessManager::remove_client` を追加し、リソースリーク耐性を向上。
    - **UI Panel (Settings)**: Management Console の `SettingsPage.tsx` 内に「MCP Architecture (Analytics & Tools)」専用のJSONコンフィグエディタを新規追加し、即時適用・再起動機能によるシームレスな管理者体験を開通。

- **Phase 2B-CORE: Bootstrap Mode & Factory Reset [完了]:**
    - **BootstrapDetector** (`libs/shared/src/bootstrap_detector.rs`): 環境変数 (LLM プロバイダ / API_SERVER_SECRET) とファイルシステム (DB/SOUL.md) の状態から、初回セットアップが必要かどうかを診断する `BootMode::Setup | Normal` 判定ロジック。10件の TDD テストでカバー。
    - **FactoryReset**: アプリケーションデータのクリーンリセット機能。DB, artifacts, WASM, vault 等を削除し、`.env` と `logs/` を安全に保持。
    - **Bootstrap API Routes** (`apps/api-server/src/routes/bootstrap.rs`):
        - `GET /api/v1/bootstrap/status` (認証不要): セットアップ状態の診断結果を返す
        - `GET /api/v1/bootstrap/detect-ollama` (認証不要): ローカル Ollama サーバーの自動検出 + モデル一覧取得
        - `POST /api/v1/bootstrap/factory-reset` (System Admin 限定): Factory Reset の実行
- **Aiome MVP Infrastructure Final Hardening (Phase 2-PRE & 2A) [完了]:**
    - **Quarantine Escape Flow API & UI (2A-3)**: `GET /api/v1/audit/quarantine` に対して、検疫解放を実行するための `POST /api/v1/audit/quarantine/{id}/release` を System Admin 保護付き（RBAC）で実装し、OpenAPI に統合。さらに `Apps/Management Console` の `ImmuneSystem.tsx` に検疫タブ（QUARANTINE）と解放操作フローを完全構築し、エンドツーエンドでの例外解放フローを開通。
    - **CSAM CPU Starvation Fix (2A-1)**: コンプライアンススキャン (`CsamScanConductor`) 内での `ImageHasher` (PhotoDNA互換) の呼び出しを `tokio::task::spawn_blocking` でラップ。重い画像処理演算（ハッシュ化・DCT）による Tokio ワーカースレッドの枯渇とシステムフリーズリスクを完全排除。RED/GREENの TDD サイクルを通じて安定稼働を証明。
    - **Configuration Path Standardization (2-PRE-3)**: 開発時と本番配布時 (`Tauri` などのバンドル時) の環境の乖離を防ぐため、`api-server`、`samsara-hub`、`aiome-migrate`、`key-proxy` のすべてのエントリポイントにおける `.env` の読み込みを CWD トライ後、`AppDataResolver` に明示的に委譲するようリファクタリングを実施。
    - **Watchtower Backend Cleanup**: `watchtower.rs` (WebSocketリレー) にて、使用されていないファイルIO結果（`soul_md`, `evolving_soul_md`, `forge_prompt` 等の未 await 先の変数）によるサイレントなコンパイラWarnings（未使用変数）を駆逐し、ビルドのクリーンさをさらに向上。

- **Smart Model Bootstrap (Setup UI & API Phase) [完了]:**
    - **Self-Diagnosis Hardening**: `apps/api-server/src/self_diagnosis.rs` において、Dockerデーモン疎通失敗時も継続可能なように緩和し、Ollamaデーモンおよび設定モデル (`gemma4:26b` 等) の実在確認ロジックを追加。
    - **Models Status API**: `GET /api/v1/models/status` エンドポイントを実装し、Ollamaへの接続状態、インストール済みモデル、設定されたモデル名、セットアップ必要性（`setup_required`）を評価しフロントエンドへ一元提供する基盤を構築。
    - **Model Pulling SSE API**: `POST /api/v1/models/pull` エンドポイントを実装。OllamaのPull APIを安全に中継し、SSE 形式で非同期ダウンロード進捗（プログレス）を配信。SSRF防御およびOOM防止のためのセマフォ(`compute_semaphore`)による安全保護を統合。
    - **Setup Wizard Integration**: Management Consoleの `OnboardingModal` 内に「LLM Engine Setup」ステップ（`ModelSetupStep.tsx`）を新規統合。`useModelStatus` カスタムフックを用いた状態監視やダウンロードバー可視化機能を実装し、安全かつ堅牢な初回時のモデル（Gemma 4推奨）導入導線を確保。
- **Gemma 4 Default LLM Migration [完了]:**
    - **デフォルトモデル切替**: ローカル推論のデフォルトモデルを `qwen3.5:9b` から `gemma4:26b` (MoE, Apache 2.0) に変更。Google の最新オープンモデルファミリー Gemma 4 を採用し、3.8B 活性化パラメータによる高速推論と 256K コンテキスト長を活用可能に。
    - **LoRA Adapter Family Isolation**: アダプター保存パスにベースモデルファミリー名を含める構造 (`vault/lora/{family}/{job_id}`) に変更。`extract_model_family` ヘルパーと `list_adapter_families` API により、Qwen/Gemma 等のアダプターの共存・切り替えをサポート。
    - **影響範囲**: `shared/config.rs`, `napi-bridge/state.rs`, `.env.example`, `lora_training.rs`, `guardrails.rs` (ドキュメント更新)
- **Block Intelligence Architecture Integration [完了]:**
    - **DreamState Runtime Activation**: デッドコード化していた自律的自己更新モジュール `DreamState` を `internal_services/dream.rs` として API サーバーに統合。アイドル時に自律的に仮説生成や過去の反省（省察夢）をキューに自己発行開始。
    - **ToolDiscovery Fallback**: `TaskDispatcher` にて適格なコンダクタ（実行者）が見つからなかった場合のフォールバックロジックを実装。単に破棄するのではなく、`ToolDiscoveryEngine` に問い合わせてMCPツールの導入を促す設計を採用しテストGREEN。
    - **GenericLlmConductor**: 上記の `DreamState` が生成した `scientific_experiment` と `data_processing` タスクを実行するための汎用プロンプト・コンダクタを追加実装し、システムレベルでの自己改善フロー（Intelligence Layer）を完成。
- **UI:** Home v2 Beta. Phase 1 of Aiome Management Console overhaul, featuring new 4-screen layout (Home, Shop, Status, World) with CharacterPanel and StoryFlow integration.
- **Workflow:** Implement Perfect Planning Phase 1 Final v5, including embedded 3D Avatar viewing with OrbitControls.
- **Cortex Knowledge Base Phase A (Ingestion) [完成]:**
    - **CortexIngester Core**: LLMを用いたURL, テキスト, PDFからのドメイン特化型Markdownナレッジ抽出エンジンを実装。pdf-extractクレートの統合と外部データ取得パイプラインを確立。
    - **API Integration**: `/api/v1/cortex/ingest` (URL), `/api/v1/cortex/ingest/text` およびドキュメント一覧・削除のRESTエンドポイントを `routes::cortex` に統合し、`api-server` の DI (AppState) コンテナと連動化。
    - **Security & Reliability**: `SecurityPolicy::validate_url` を用いたSSRF防御と、`AiomeError::NetworkError` バリアント追加によるネットワークエラー層の切り分けと回復性を向上。MockLLMを用いた結合テストも完備。
- **Cortex Knowledge Base Phase B (Wiki Compiler) [完成]:**
    - **Compiler Core**: `CortexCompiler` を実装し、複数の未コンパイルドキュメントから概念（Concepts）を抽出し、2つ以上の情報源をもとにWiki記事（タイトル、Markdown本文、概念タグ、バックリンク、参照情報）を自律生成するロジックを確立。
    - **Resource & Transaction Safety**: `compute_semaphore` を依存注入し、LLM推論でのリソース競合を防止。DBの読み取りと行ごとの更新トランザクションを分離し、SQLiteロック（`database is locked`）を回避する安全な設計を採用。
    - **API & Background Loop**: `/api/v1/cortex/wiki` および `/api/v1/cortex/wiki/:id` エンドポイントを実装し、生成されたWiki記事の取得をサポート。同時に `api-server` 起動時に30分間隔で定期コンパイルを実行するバックグラウンドワーカーを統合。
    - **Perfect Plan Fixes**: 冪等性の強化 (途中失敗時の重複生成防止)、`Authenticated` エクストラクタの適用、プロンプトインジェクション防御 (`sanitize_for_prompt`)、概念名の正規化と一貫したハッシュ (SHA-256) 採用など、Phase B全体の堅牢性を極める9つの必須/推奨修正を完了。
- **Cortex Knowledge Base Phase C (Query Engine Integration) [完成]:**
    - **CortexQueryEngine & API**: 意味検索用エンジン `CortexQueryEngine` を実装し、未加工のドキュメントとコンパイル済みWikiの双方からナレッジを引き出せる `/api/v1/cortex/query`, `/api/v1/cortex/suggestions` をパブリック・エンドポイントとして登録。
    - **Semantic Streaming Fallback**: チャットストリーム中の未知の質問 (OOD: Out of Distribution) に対し、HKRフォールバックに加えてこのCortex Queryエンジンを利用し、外部知識からの回答生成能力を飛躍的に向上。
    - **MCP Tool Exposure**: `cortex_search` MCPツールを実装・公開し、Claude Codeなど外部エージェントも直接CortexシステムのWikiナレッジをセマンティック検索できるように設計完了。
    - **TDD Hardening (P-1/P-2/P-3)**: コンテキスト注入文字数の設定可能化 (`with_max_context_chars` builder pattern / デフォルト8000)、`suggest_questions` のDB概念ベース動的生成化（空DB時フォールバック付き）、大規模データ向けFTS5移行計画をTODOとして文書化。8テスト全GREEN。
    - **Frontend Integration**: Management Consoleの主インターフェース（StoryFlow）にCortex Query Engineを統合する動的サジェストチップ（Suggestion Chips）UIを実装、カスタムフック化ならびにPlaywrightによるE2Eテスト網も確立しGREENパス。
    - **Karpathy "LLM Wiki" Pattern**: `QueryOptions` および `DisclosureLevel` トークン管理（L0-L3/Progressive Disclosure）を導入。信頼スコア0.7以上の回答結果を `SourceType::Query` として自己増殖させる Query File-Back アーキテクチャを確立し、`cortex_activity_log` にるアクション監視ロジックを追加した。
- **LoRA Marketplace 基礎実装 & セキュリティ・ハードニング [完成]:**
    - **安全なアダプター取引基盤**: LoRA アダプター（`.safetensors`）の出品・購入・転送を安全に仲介するマーケットプレイスを実装。SHA-256 ハッシュ検証、エスクロー（`CommerceEngine` 連携）、PathSandbox 分離を備える。
    - **コントラクト型定義**: `LoraListing`, `LoraPurchase`, `ListingFilter`, `LoraMarketplace` トレイトを `aiome-core-contracts` に新設。
    - **インフラ実装**: `UniversalLoraMarketplace` (SQLite/PostgreSQL) を `infrastructure` に実装。出品→エスクロー→ハッシュ検証→Vaultコピー→エスクロー解放のステートマシンフローを確立。
    - **Sagaパターン (補償トランザクション)の適用**: 決済時に外部API通信の前にローカルDBステータスを更新し、通信失敗時は確実に戻すフロー（Compensating Transaction）を導入。二重決済やステート宙吊り状態の発生を防止。
    - **REST API & OpenAPI 完全準拠**: `/api/v1/lora/market/*` の6エンドポイントを JWT 認証付きで公開し、全てに対して `#[utoipa::path]` マクロを設定。フロントエンドチームが `npm run generate-types` から完全な型安全クライアントを生成可能に。
    - **5大セキュリティ・フェイルチェック (Reflexion)**:
        1. **BOLA遮断**: `complete_purchase` において `caller_id` 照合を強制し、第三者による代理確定を防止。
        2. **経路攻撃 (Path Traversal) 遮断**: `model_family` と `base_model` に対する `/`, `\`, `.` 混入チェックを実施し、Vault 外への任意ファイル生成攻撃を無効化。
        3. **OOM 対策**: 500MB に達するハッシュ計算 (`compute_file_hash`) において、一括メモリ展開を廃止し 64KB のストリーミング読み込みバッファループに変更。エッジ端末でのクラッシュを防止。
        4. **Nil UUID 漏洩防止**: データベースからのデシリアライズ失敗時に `unwrap_or_default` による隠蔽を廃止し、破損行のみを静かにフィルター（スキップ）する耐障害性に優れた抽出へ変更。
        5. **PG トリガー不具合修正**: PostgreSQL における非標準構文である `CREATE TRIGGER IF NOT EXISTS` を `DROP TRIGGER IF EXISTS` によるイデムポテント構築へ修正。
    - **テスト**: 出品・購入・取下・エラー系のマルチモジュール・テストは全て GREEN を維持。

### Fixed
- **Infrastructure:** Fixed `test_slm_bridge_cli_hang_timeout_red` failing test in preflight. The timeout test script was not generated dynamically, leading to immediate non-timeout failure. Now safely writes to /tmp and respects Unix execution modes.

## [0.1.0] - 2026-04-02 (Genesis Phase Complete)

### Added
- **Phase C-2: Aiome Watchtower Diagnostic Loop Hardening [完成]**
    - **AgentRx Diagnostic Loop**: `TaskDispatcher` に失敗タスクをバックグラウンドで LLM 診断し、次回の再試行時にヒント（KarmaDirectives）を挿入する自己修復ループを統合。
    - **Fail-Safe Diagnostics**: LLM 呼び出しに固いタイムアウト (30s) を導入。LLM 停止時や空のシークエンスに対してもフォールバック用の診断レコードを永続化し、システム全体の不稼働を回避。
    - **Prompt Idempotency**: 注入されるヒントを `<WATCHTOWER_INSIGHT>` タグで保護。リトライごとにプロンプトが肥大化するのを防ぐ冪等性ガードを実装。
    - **Stability Polling**: E2E テストにおいて固定 sleep をポーリングループに置き換え、CI 環境におけるテストの信頼性を飛躍的に向上。
- **Phase A MVP Bootstrap Hardening [完成]**
    - **A-1 Task Dispatcher Synchronization**: 起動時の `TaskDispatcher` 初期化直後に `tokio::time::sleep(2秒)` を導入し、SQLiteウォールなどの内部ワーカーの競合を防ぐクリーンなオフセット待機を実装。
    - **A-2 Dynamic Project Rules Discovery**: `api-server/src/system_instructions.rs` にディレクトリルート方向への段階的な `.aiome.md -> AIOME.md -> .cursorrules` 探索ロジックを追加。mokaによる30秒TTLキャッシュでI/O負荷を低減し、コンテキスト予算内でLLMのシステムプロンプトヘ動的注入。パストラバーサル防御テストも完備。
    - **A-3 Startup Self-Diagnosis**: 起動シーケンス初期に `self_diagnosis.rs` を追加し、ディレクトリのR/W権限、データベースの接続性、DockerデーモンのAPI疎通を検証。異常な環境では素早くFail-Fastする安全網を構築。
    - **A-4 Bootstrap Graph Expansion**: `main.rs` の肥大化した1000行近い初期化ロジックを `bootstrap.rs` の `boot_sequence()` へ抽出。起動グラフを「Pre-flight -> Database -> Engine -> Core Services -> Registry -> Workers -> Network」の明確な7段階ステージへアーキテクチャ分離。

- **Phase 1 Final & Security Audit Remediation [完成]**
    - **Global Compute Semaphore (F-3 Hardware Protection)**: LoRA学習プロセスや画像/音声生成処理での重いML計算が同時に実行された際の Unified Memory OOM（Macのカーネルパニック）を防ぐため、システム全体を横断する `compute_semaphore` (Available permits = 1) を `AppState` から `ComfyUiGenerativeEngine` 等へ依存注入し、ハードウェアリソースの安全な排他制御を確立。
    - **CSAM Release Safe-Guard**: `CsamScanConductor` スタブに `#cfg(not(debug_assertions))` のフェイルセーフを追加し、Releaseビルドで（ダミー実装を持ったまま）コンプライアンススキャンを突破できないようゼロトラストを強制。
    - **Vault IO Resilience**: `LoraTrainingService` 内で外部プロセス (MLX Python script) を起動する直前に `std::fs::create_dir_all(&config.vault_path)` を実行するようにし、ディレクトリ非存在による学習フェーズのサイレントフォールトを防止＆無視されていた隔離テストを復旧。
    - **PG Migration Preparation**: `SyndicateStore` に対して `DatabasePool` を受け取るようにインターフェースを拡張（現在は `SqlitePool` にフォールバックするTODO付き）、将来の Phase 4C の PostgreSQL 化へのシームレスな移行パスを確保。
    - **Workspace Path Standardization**: `LoraTrainingService` 内のハードコードされた `workspace/datasets/` パスを `datasets_dir` 設定値に置き換え、テストやカスタム環境でのディレクトリトラバーサル脆弱性を根絶。
    - **Doc-Sync (ADR Integrity)**: `docs/decisions` 内に混在していた `ADR-` プレフィックスの重複や欠番(027)を再連番・正規化 (`026` から `028` へ整理)。

- **Phase B: Autonomous Chat Loop Hardening (ToolCallRouter Integration) [完成]**
    - **ToolCallRouter Trait**: 複数の実行経路（`stream.rs`, `agent_engine.rs`, `mcp/server.rs`）で発生していたツール実行ロジックとセキュリティチェック（Guardrails / ImmuneSystem / HookChain）の重複を排除する `ToolCallRouter` トレイトとその `DefaultToolCallRouter` 実装を追加。
    - **Security Execution Precedence**: すべてのツール実行において、1. 意図検証 (Intent/Immune)、2. HookChain Pre-exec、3. Executor、4. HookChain Post-exec の順序を統一。Hook ブロック時や Guardrails 遮断時も適正にエラー結果（`[BLOCKED] ...`）を返却し、実行結果をストリームへ提供（SSE対応）。
    - **Fail-Safe Intent Sync**: テストモック (`MockLlm` / `SentinelLlm`) を整備し、アーキテクチャの変更が TDD アプローチ内でフェイルセーフに構築されることを保証 (`cargo test --workspace` を全件クリア)。
    - **OOM / Context Expansion Defense**: ツールの実行結果の 50KB を超える出力の自動トランケート (`safe_truncate`) を統合し、長期自律ループ中の OOM を防止。
    - **MCP Tool Execution Security (B-QG-1)**: MCPリクエスト (`apps/api-server/src/mcp/server.rs`) が `ToolCallRouter` を経由するように統合し、`AdaptiveImmuneSystem` と `HookChain` を完全サポート。MCP仕様に対する `isError` フラグ準拠およびストリームの安全な集約を追加。

- **Phase 1C: Generative Engine Infrastructure Integration [完了]**
    - **GenerativeEngine Trait Integration**: `aiome-contracts` で定義された `GenerativeEngine` トレイトの具象実装として、`ComfyUiGenerativeEngine` および `FalAiGenerativeEngine` を `libs/infrastructure` 内に追加。ローカルGPU（ComfyUI）とクラウドAPI（Fal.ai）の両バックエンドに透過的に対応。
    - **AppState Injection**: `api-server` の `main.rs` において環境変数 (`GENERATIVE_ENGINE`, `COMFYUI_URL`, `FAL_KEY`) に基づく動的なエンジンの初期化と `AppState` への注入を実装。開発環境用には安全な `MockGenerativeEngine` へのフォールバックを提供しつつ、プロダクション (`--release`) 環境では設定漏れ時に Fail-Fast (panic) する厳格なガードレールを構築。
    - **TDD-Driven Robustness**: `mock` モジュールを `#[cfg(any(test, debug_assertions))]` で隔離し、本番環境への混入を防止。さらに、不正なエンドポイントや無効なAPIキーに対する HTTP クライアントのフェイルクローズ処理（401 Unauthorized 等のハンドリング）をカバーする包括的な TDD テストスイートを構築し、ワークスペース全体の 1379 テストをクリーンパス。

- **Phase F: Security Hook Framework & Precedence Execution [完了]**
    - **HookChain Implementation**: `AgentEngine` の `process_generated_tool_calls` メソッドに `HookChain` と `AdaptiveImmuneSystem` の統合ロジックを実装。すべてのツールコールに対してPre-execution/Post-executionフックによるセキュアな検証・変換プロセスを確立。
    - **Architecture Restructuring**: `AgentEngine` 内部にツールコール抽出、検証、および実行処理の責務を集約。循環参照エラーを解消しつつ、`MockGenerativeEngine` などの分離に合わせたテスト容易かつ TDD コンプライアンスを満たす設計へ進化。
    - **Sentinel & Hook Enforcement**: `AdaptiveImmuneSystem` に基づいて `rm -rf /` のような致命的コマンドをフック評価前にブロックする「Sentinel Block」機構を実装し、Hook層（`Allow`/`Deny`/`Transform`）との確実な優先順位付けと分離を実現。ワークスペース全体で59の結合テストに完全パス。

- **Phase 1B: Avatar Engine & Infrastructure Hardening [完成]**
    - **Storage DoS Protection (DiskQuotaManager)**: `DiskQuotaManager` を導入し、エージェントごとに最大 500MB のディスククォータを厳密に管理することでストレージ枯渇（DoE）攻撃を防止。アップロード処理中のリアルタイム検証とデータベース管理により安全性を向上。テストスイート内の AppState モックも完全同期。
    - **TTS Streaming Optimization**: `TtsProvider` トレイトを非破壊的に拡張し `synthesize_stream` を追加。OpenAI バックエンド呼び出しにおいて真のチャンクストリーミング（TTFB の劇的な短縮）を実現し、`api-server` の `/api/v1/voice/synthesize` に `?stream=true` 対応を統合。
    - **LipSync Expansion**: `avatar-engine` 内の `LipSyncProvider` トレイト実装として `SimpleLipSyncEngine` を追加。オーディオ解析や文字起こしセグメントから Inochi2D などで利用する口形（Viseme）の時系列データを生成可能なエンジン基盤を構築。

- **Dynamic Dataset Extraction (F-2)**: `DatasetExtractor` を導入し、LoRA学習時に `SoulStore` の履歴 (Experiences) から自動的にMLX専用の JSONL 形式のデータセットを動的に構築・注入するパイプラインを確立。非同期API呼び出しの競合を防ぐ `job_id` アイソレーションを実装し、さらに「会話履歴全体の文脈」を1つのシーケンスブロックとして連結・維持する手法によりCatastrophic Forgettingを防止。
- **Global Compute Semaphore (F-3 Hardware Protection) [完了]**: LoRA学習プロセスや画像/音声生成処理での重いML計算が同時に実行された際の Unified Memory OOM（Macのカーネルパニック）を防ぐため、システム全体を横断する `compute_semaphore` (Available permits = 1) を `AppState` に導入。`LoraTrainingService` 等へ依存注入し、ハードウェアリソースの安全な排他制御を確立。
- **Tier 0 Infrastructure Security Hardening (Wave 1-3) [螳御ｺ**スや画像/音声生成処理での重いML計算が同時に実行された際の Unified Memory OOM（Macのカーネルパニック）を防ぐため、システム全体を横断する `compute_semaphore` (Available permits = 1) を `AppState` に導入。`LoraTrainingService` 等へ依存注入し、ハードウェアリソースの安全な排他制御を確立。
- **Red Team Security Hardening & Infrastructure Stabilization [完成]**
    - **BoundaryVerifier Hardening**: システムパスブロックの拡張（`/tmp`, `/dev`, `/proc`, `/sys`, `/private`）およびパストラバーサル（`..`）の物理的拒絶を O(1) で実装。
    - **BastionGuard (Command Parsing) Hardening**: `shell_split` の POSIX 制御文字（`\v`, `\f`, `\r`, `\n`）対応、フラグ分解ロジックの改善。制御文字バイパスを完全封鎖。
    - **Registry SSoT (Zero-Fallback)**: `check_ownership` における安全でないフォールバックを廃止。`licenses` テーブルを所有権確認の「唯一の正解（Single Source of Truth）」とし、論理的一貫性を保証。
    - **Infrastructure Reliability**: `WasmHarness` の非同期初期化と `OnceCell` によるキャッシュ競合（Thundering Herd）の防止を実装。230件のインフラストラクチャテスト全件 PASS を確認。

- **Phase F: Open Gateway (MCP & P2P Foundation) [螳御ｺ�**
    - **Safe Auto-Profile Engine**: `AutoProfileEngine` 繧貞ｮ溯｣�ゅΡ繝ｼ繧ｯ繧ｹ繝壹�繧ｹ蜀�� `Cargo.toml`, `package.json`, `requirements.txt` 縺ｨ縺�▲縺溽腸蠅�ｮ夂ｾｩ繝輔ぃ繧､繝ｫ繧偵せ繧ｭ繝｣繝ｳ縺励√お繝ｼ繧ｸ繧ｧ繝ｳ繝医�譛峨☆繧九せ繧ｭ繝ｫ��ust, Python, Web遲会ｼ峨ｒ閾ｪ蠕狗噪縺ｫ謗ｨ螳壹＠縺ｦ `AgentCard` 縺ｮ `skills` 縺ｨ縺励※髴ｲ蜃ｺ縲ゅさ繝ｼ繝峨ｄ繝励Ο繝ｳ繝励ヨ縺ｮ隱ｭ縺ｿ蜿悶ｊ繧定｡後ｏ縺ｪ縺�ｨｱ蜿ｯ繝ｪ繧ｹ繝亥梛縺ｮ螳牙�險ｭ險医ｒ謗｡逕ｨ縲�
    - **Secure Gig Gateway**: `SecureGigGateway` 繧貞ｮ溯｣�＠縲∝､夜Κ�井ｻ悶お繝ｼ繧ｸ繧ｧ繝ｳ繝茨ｼ峨°繧峨�繧ｿ繧ｹ繧ｯ逋ｺ豕ｨ遯灘哨繧呈ｧ狗ｯ峨３ate Limiter��DoS髦ｲ豁｢�峨、utoHarness�亥些髯ｺ繧ｳ繝槭Φ繝峨�髱咏噪驕ｮ譁ｭ�峨，onstitutionalValidator��ore蛟ｫ逅�渕貅悶↓繧医ｋ讀懆ｨｼ�峨°繧峨↑繧句ｼｷ蝗ｺ縺ｪ�灘ｱ､繧ｻ繧ｭ繝･繝ｪ繝�ぅ繝ｻ繝輔ぅ繝ｫ繧ｿ繝ｼ繧堤ｾｩ蜍吩ｻ倥￠縺励√お繧ｹ繧ｯ繝ｭ繝ｼ譛ｪ謇輔＞�井ｺ育ｮ�0�峨�繧ｹ繝代Β繧呈拠蜷ｦ縲�
    - **JSON-RPC MCP Server Integration**: `aiome-node` 縺ｫ MCP 繝｢繝ｼ繝� (`aiome-node mcp` 繧ｳ繝槭Φ繝�) 繧定ｿｽ蜉�縲�SON-RPC 2.0 over stdio 邨檎罰縺ｧ螟夜Κ縺ｮ MCP 繧ｯ繝ｩ繧､繧｢繝ｳ繝茨ｼ�ursor 繧� Claude Code遲会ｼ峨↓蟇ｾ縺� `profile/info`, `gig/capabilities`, `gig/publish` 繝��繝ｫ繧貞�髢九＠縲￣2P 縺ｮ隍�尅縺ｪ繝阪ャ繝医Ρ繝ｼ繧ｯ螳溯｣�ｒ逵∫払縺励▽縺､縲∝ｮ牙�縺九▽讓呎ｺ也噪縺ｪ繝励Ο繝医さ繝ｫ縺ｧ縺ｮ繝ｭ繝ｼ繧ｫ繝ｫ繝ｻ繧ｨ繝ｼ繧ｸ繧ｧ繝ｳ繝磯｣謳ｺ蝓ｺ逶､繧堤｢ｺ遶九�
- **Phase B-D: AutoHarness Security Architecture Integration [螳御ｺ�**
    - **DB Foundation**: `harness_registry` 繝��繝悶Ν繧貞ｰ主�縺励ヾQLite/Postgres 逕ｨ縺ｮ繝槭う繧ｰ繝ｬ繝ｼ繧ｷ繝ｧ繝ｳ繧貞ｮ溯｣�ＡHarnessRecord` 縺翫ｈ縺ｳ `HarnessRegistryOps` 繝医Ξ繧､繝医ｒ霑ｽ蜉�縺励※ JobQueue 縺ｧ CRUD 繧偵し繝昴�繝医�
    - **Security Enforcement**: `ConstraintChecker` 蜀��豁｣隕剰｡ｨ迴ｾ隗｣譫舌↓ `RegexBuilder` 繧貞ｰ主�縺励�10KB 縺ｮ繧ｵ繧､繧ｺ蛻ｶ髯舌ｒ險ｭ縺代ｋ縺薙→縺ｧ ReDoS 閼�ｼｱ諤ｧ繧呈ｧ矩�逧�↓驕ｮ譁ｭ縲�
    - **Dynamic Guardrails**: `WasmHarness` 縺ｫ `severity` 繧ｹ繧ｳ繧｢繧堤ｵｱ蜷医りｩ穂ｾ｡繝ｫ繝ｼ繝励↓縺翫＞縺ｦ 80 莉･荳翫ｒ繝悶Ο繝�く繝ｳ繧ｰ (Active mode)縲√◎繧梧悴貅繧偵Δ繝九ち繝ｪ繝ｳ繧ｰ (Shadow mode) 縺ｨ縺吶ｋ隧穂ｾ｡讖滓ｧ九ｒ `skill_handler.rs` 縺ｫ螳滉ｽ灘喧縲�
- **Security Hardening: NPM Supply Chain Attack Mitigation [螳御ｺ�**
    - **閭梧勹**: `axios@1.14.1` / `axios@0.30.4` 縺ｮ npm 繝｡繝ｳ繝�リ繧｢繧ｫ繧ｦ繝ｳ繝井ｹ励▲蜿悶ｊ縺ｫ繧医ｋRAT��emote Access Trojan�蛾�蛯吩ｺ区｡茨ｼ�2026-03-31�峨ｒ蜿励￠縲∝酔遞ｮ縺ｮ謾ｻ謦�↓蟇ｾ縺吶ｋ讒矩�逧�亟蠕｡繧貞ｰ主�縲�
    - **Layer 1 窶� postinstall 辟｡蜉ｹ蛹�**: `.npmrc` 縺ｫ `ignore-scripts=true` 繧定ｨｭ螳壹Ａpostinstall` 繝輔ャ繧ｯ繧呈が逕ｨ縺吶ｋ繧ｵ繝励Λ繧､繝√ぉ繝ｼ繝ｳ謾ｻ謦�ｼ�xios RAT, event-stream, ua-parser-js 遲会ｼ峨�繧ｯ繝ｩ繧ｹ蜈ｨ菴薙ｒ讒矩�逧�↓辟｡蜉帛喧縲�
    - **Layer 2 窶� Critical Audit Gate**: CI 繝代う繝励Λ繧､繝ｳ縺ｫ `npm audit --audit-level=critical` 繧定ｿｽ蜉�縲�ritical 繝ｬ繝吶Ν縺ｮ譌｢遏･閼�ｼｱ諤ｧ縺梧ｷｷ蜈･縺励◆蝣ｴ蜷医↓繝薙Ν繝峨ｒ蜊ｳ蠎ｧ縺ｫ繝悶Ο繝�け縲�igh 繝ｬ繝吶Ν�医ン繝ｫ繝峨ヤ繝ｼ繝ｫ蜀�Κ縺ｮReDoS遲峨∝ｮ溷ｮｳ縺ｮ縺ｪ縺�ｪ､讀懃衍�峨�繝悶Ο繝�け縺励↑縺�ｨｭ險医↓繧医ｊ縲．eveloper Friction 繧呈怙蟆丞喧縲�
    - **Layer 3 窶� Registry Signature Verification**: `npm audit signatures` 繧� CI 縺ｫ邨ｱ蜷医Ｏpm 縺ｮ OIDC Trusted Publisher 繝｡繧ｫ繝九ぜ繝�縺ｫ繧医ｋ繝ｬ繧ｸ繧ｹ繝医Μ鄂ｲ蜷阪ｒ讀懆ｨｼ縺励∵ｭ｣隕修I螟悶°繧峨�謇句虚publish�医い繧ｫ繧ｦ繝ｳ繝井ｹ励▲蜿悶ｊ謾ｻ謦��蜈ｸ蝙九ヱ繧ｿ繝ｼ繝ｳ�峨ｒ讀懃衍縲ょ�192繝代ャ繧ｱ繝ｼ繧ｸ縺ｮ鄂ｲ蜷肴､懆ｨｼ繧偵ヱ繧ｹ遒ｺ隱肴ｸ医∩縲�
    - **蠖ｱ髻ｿ遽�峇**: `management-console` 縺ｮ縺ｿ縲３ust 繝舌ャ繧ｯ繧ｨ繝ｳ繝峨� crates.io 繝吶�繧ｹ縺ｮ縺溘ａ蠖ｱ髻ｿ縺ｪ縺励よ里蟄倥ン繝ｫ繝峨�E2E繝�せ繝医∈縺ｮ蠖ｱ髻ｿ繧ｼ繝ｭ繧堤ｵｱ蜷医ユ繧ｹ繝医〒螳溯ｨｼ貂医∩縲�
- **Phase 55: AgentRx Expansion & LoRA Autotuning [螳御ｺ�**
    - **AgentRx Closed-Loop**: `RepairCalculator` 縺ｮ蝙九Α繧ｹ繝槭ャ繝√ｒ菫ｮ豁｣縺励～AgentRxDiagnostics` 縺ｫ `suggest_repair_strategy` 繧貞ｮ溯｣�ＡAPI Server` 縺ｮ繧ｨ繝ｼ繧ｸ繧ｧ繝ｳ繝亥ｮ溯｡後Ν繝ｼ繝励↓菫ｮ蠕ｩ繝偵Φ繝茨ｼ�etry/Escalate�峨�蜍慕噪豕ｨ蜈･讖滓ｧ九ｒ邨ｱ蜷医�
    - **LoRA Autotuning**: `LoraAutotuner` 繧呈眠險ｭ縺励�℃蜴ｻ縺ｮ繝ｭ繧ｹ螻･豁ｴ縺ｫ蝓ｺ縺･縺�◆蟄ｦ鄙堤紫縲√お繝昴ャ繧ｯ謨ｰ縲√Λ繝ｳ繧ｯ縺ｮ閾ｪ蜍輔メ繝･繝ｼ繝九Φ繧ｰ��verfitting謚大宛縲ヾtagnation譎ゅ�LR蛟榊｢励＾scillation譎ゅ�LR蜊頑ｸ幢ｼ峨ｒ螳溯｣��
    - **Autonomous Evolution**: `HeartbeatWakeupService` 縺ｫ `LoraTrainingService` 繧堤ｵｱ蜷医よ�髟ｷ縺ｮ繝励Λ繝医��亥●貊橸ｼ峨ｒ讀懃衍縺励◆髫帙↓縲�24譎る俣縺ｮ繧ｯ繝ｼ繝ｫ繝繧ｦ繝ｳ蛻ｶ蠕｡莉倥″縺ｧ閾ｪ蠕狗噪縺ｪ閾ｪ蟾ｱ譛驕ｩ蛹厄ｼ郁ｨ鍋ｷｴ繝ｫ繝ｼ繝暦ｼ峨ｒ閾ｪ逋ｺ逧�↓繝医Μ繧ｬ繝ｼ縺吶ｋ莉慕ｵ�∩繧貞ｮ梧�縲�
    - **System Defenses**: `ConstraintChecker` 縺ｫ `OutputSizeExceeded` (100KB雜�℃) 縺ｨ `SuspiciousEchoDetected` (50譁�ｭ嶺ｻ･荳翫�蜈･蜉帙�螳悟�縺ｪ繧ｨ繧ｳ繝ｼ謾ｻ謦�) 縺ｮ繧ｻ繝ｼ繝輔ぎ繝ｼ繝峨ｒ霑ｽ蜉�縲ゅ＆繧峨↓ `CognitiveSentinel` 縺ｸ縲∵怙霑代�Fail Rate縺�60%繧定ｶ�∴縺溷�ｴ蜷医↓逋ｺ轣ｫ縺吶ｋ `Panic State` 髦ｲ蠕｡繝｡繧ｫ繝九ぜ繝�繧定ｿｽ蜉�縲ょ�讖溯�縺ｮ TDD 縺翫ｈ縺ｳ繝ｯ繝ｼ繧ｯ繧ｹ繝壹�繧ｹ繝ｯ繧､繝峨↑ `cargo test` 騾夐℃繧堤｢ｺ隱阪�
- **Phase 52: LoRA Archiving & Secure Training Pipeline (MVP/TDD) [騾ｲ陦御ｸｭ]**
    - **Rebirth Archiving**: `archived_lora_models` 繝��繝悶Ν縺ｨ繝槭う繧ｰ繝ｬ繝ｼ繧ｷ繝ｧ繝ｳ繧定ｿｽ蜉�縲ＡSoulStore::archive_lora_model` 繧貞ｮ溯｣�＠縲～SamsaraEngine::rebirth` 譎ゅ↓蜑堺ｸ紋ｻ｣縺ｮLoRA險ｭ螳壹ｒ繧｢繝ｼ繧ｫ繧､繝厄ｼ�Μ繧ｻ繝�ヨ縺吶ｋ縺薙→縺ｧ繝��繧ｿ繝昴う繧ｺ繝九Φ繧ｰ繧貞ｮ悟�縺ｫ驕ｮ譁ｭ縲�
    - **Secure LoRA Training**: `LoraTrainingService` 繧呈眠險ｭ縲ＡBastionGuard::new_internal()` 繧堤畑縺�※LoRA蟄ｦ鄙偵�繝ｭ繧ｻ繧ｹ��LX/Python�峨ｒ髫秘屬螳溯｡後�逶｣隕悶�
    - **Vault Isolation & Ollama Integration**: 蟄ｦ鄙貞ｮ御ｺ�ｾ後�繧ｦ繧ｧ繧､繝医ｒ `GLOBAL_SECURITY_CONFIG.vault_path` 縺ｫ螳牙�縺ｫ遘ｻ蜍輔＠縲～ollama create` 繧堤畑縺�※閾ｪ蠕狗噪縺ｫ謗ｨ隲悶お繝ｳ繧ｸ繝ｳ縺ｸ繝｢繝�Ν繧堤匳骭ｲ縺吶ｋ繝輔Ο繝ｼ繧堤｢ｺ遶九�
- **Phase 51: Agentic Finance & GIG Loop Integration [螳御ｺ�**
    - **TaskDispatcher Evolution**: `GigEngine` 繧剃ｾ晏ｭ俶ｳｨ蜈･ (DI) 縺励√ず繝ｧ繝門ｮ御ｺ�凾縺ｫ繝ｭ繧ｸ繝�け繧定�蠕九ヨ繝ｪ繧ｬ繝ｼ蜿ｯ閭ｽ縺ｫ縲�
    - **Autonomous GIG Publishing**: `karma_directives` 蜀�� `gig_intent: true` 繧呈､懃衍縺励∬�蜍慕噪縺ｫ `GigIntent` 繧堤函謌舌�蜈ｬ髢九☆繧九し繧､繧ｯ繝ｫ繧堤｢ｺ遶九�
    - **Recursion Safety**: `gig_depth` 縺ｫ繧医ｋ繧ｬ繝ｼ繝峨Ξ繝ｼ繝ｫ繧貞ｮ溯｣�ら┌髯舌Ν繝ｼ繝暦ｼ郁�蟾ｱ逋ｺ豕ｨ縺ｮ騾｣骼厄ｼ峨ｒ髦ｲ豁｢�域怙螟ｧ3髫主ｱ､�峨�
    - **Constitutional Security**: `TaskDispatcher` 縺ｫ `ConstitutionalValidator` 繧堤ｵｱ蜷医�I閾ｪ蠕狗匱豕ｨ縺ｮ縲悟ｮ牙�諤ｧ繝ｻ蛟ｫ逅�ｧ縲阪ｒ譛邨よ､懆ｨｼ縺吶ｋ讀憺夢繝ｬ繧､繝､繝ｼ繧呈ｧ狗ｯ峨�
    - **Budget Guardrails**: `MAX_GIG_BUDGET` (5000) 繧定ｨｭ螳壹＠縲�℃螟ｧ縺ｪ莠育ｮ苓ｦ∵ｱゅｒ閾ｪ蜍輔け繝ｩ繝ｳ繝励�
    - **SSE Real-time Bridge**: `TaskEvent::GigPublished` 繧� `CoreEvent` 邨檎罰縺ｧ `api-server` 縺ｮ SSE 繧ｹ繝医Μ繝ｼ繝�縺ｸ繝悶Μ繝�ず縲らｮ｡逅�さ繝ｳ繧ｽ繝ｼ繝ｫ縺ｧ縺ｮ繝ｪ繧｢繝ｫ繧ｿ繧､繝�蜿ｯ隕門喧縺ｫ蟇ｾ蠢懊�
    - **Red Team Hardening (RT4)**:
        - **Structured Validation**: 諞ｲ豕輔ヰ繝ｪ繝��繧ｿ繝ｼ縺ｮ繧ｳ繝ｳ繝�く繧ｹ繝医ｒ讒矩�蛹� (`--- TASK ---`) 縺励√�繝ｭ繝ｳ繝励ヨ繧､繝ｳ繧ｸ繧ｧ繧ｯ繧ｷ繝ｧ繝ｳ縺ｫ繧医ｋ縲悟ｮ牙�縺ｪ繧ｿ繧ｹ繧ｯ縲阪�蛛ｽ陬�ｒ髦ｲ豁｢縲�
        - **Budget Floor**: 譛蟆丈ｺ育ｮ励ｒ 10 coins 縺ｫ險ｭ螳壹＠縲√ム繧ｹ繝医�繧､繝ｳ繝�Φ繝医↓繧医ｋ DDoS 謾ｻ謦�ｒ謚大宛縲�
        - **Instruction Expansion**: 繧ｷ繝ｼ繧ｯ繝ｬ繝�ヨ縺ｮ豬∝���xfiltration�芽ｩｦ陦後ｒ讀懃衍縺吶ｋ繧医≧繝舌Μ繝��繧ｿ繝ｼ縺ｮ謖�､ｺ繧貞ｼｷ蛹悶�
        - **RT5: Heartbeat Hardening**: `HEARTBEAT.md` 縺ｮ隱ｭ縺ｿ蜿悶ｊ繧ｵ繧､繧ｺ繧� 5000 譁�ｭ励↓蛻ｶ髯舌�LM 謠先｡亥�螳ｹ縺九ｉ繧ｷ繧ｧ繝ｫ繧ｳ繝槭Φ繝臥ｭ峨�蜊ｱ髯ｺ縺ｪ繝代ち繝ｼ繝ｳ繧呈賜髯､縺吶ｋ繧ｵ繝九ち繧､繧ｶ繝ｼ繧貞ｮ溯｣��
        - **RT6: Settings & WASM Hardening**: `stripe_api_key`, `openai_api_key` 遲峨�繧ｷ繝ｼ繧ｯ繝ｬ繝�ヨ Whitelist 諡｡蠑ｵ縲８ASM `fs_reader` 縺ｮ繝悶Λ繝�け繝ｪ繧ｹ繝医ｒ蠑ｷ蛹悶＠縲～.db`, `.sqlite`, `credentials` 縺ｸ縺ｮ繧｢繧ｯ繧ｻ繧ｹ繧貞ｰ�事縲�
        - **RT7: Deployment & CI Hardening**: `docker-compose.production.yml` 縺ｫ髱樒音讓ｩ繝ｦ繝ｼ繧ｶ繝ｼ縲√Μ繧ｽ繝ｼ繧ｹ蛻ｶ髯舌∝�驛ｨ繝昴�繝磯�譁ｭ繧貞ｰ主�縲�I 縺ｸ縺ｮ **Trivy 繧ｳ繝ｳ繝�リ閼�ｼｱ諤ｧ繧ｹ繧ｭ繝｣繝ｳ** 邨ｱ蜷医�
        - **RT8: Logic & Normalization Hardening**: `�チ 繧� `�搾ｼ搾ｼ港 遲峨�蜈ｨ隗� Unicode 縺ｫ繧医ｋ繧､繝ｳ繧ｸ繧ｧ繧ｯ繧ｷ繝ｧ繝ｳ髦ｲ蠕｡讖溯�繧� `sanitize_for_prompt` 縺ｫ螳溯｣�ょｷｨ螟ｧ繝｡繝�そ繝ｼ繧ｸ縺ｫ繧医ｋ繧ｳ繝ｳ繝�く繧ｹ繝育�ｴ螢翫∈縺ｮ蠅�阜菫晁ｭｷ縲√♀繧医�諢滓ュ險育ｮ励↓縺翫￠繧� NaN/Inf 閠先ｧ繧定ｿｽ蜉�縲�
        - **RT9: Red Team Penetration Drill**: `tests/redteam_drill.rs` 繧呈眠隕丈ｽ懈�縲る屮隱ｭ蛹悶�繝ｭ繝ｳ繝励ヨ縲ヾSRF縲仝ASM 髫秘屬縲√♀縺ｭ縺�繧翫ヱ繧ｿ繝ｼ繝ｳ遲峨�螳滓姶逧�↑謾ｻ謦�す繝翫Μ繧ｪ縺ｫ蟇ｾ縺吶ｋ髦ｲ蠕｡諤ｧ閭ｽ繧呈､懆ｨｼ縺励∝�鬆�岼縺ｧ縲沓locked縲阪ｒ遒ｺ隱阪�
        - **Audit Integration**: 險ｭ螳壼､画峩縺ｮ繝ｭ繧ｰ繧� `AuditLogger` (Global Ledger) 縺ｫ遘ｻ陦後�
    - **TDD Verification**: `MockGigEngine` 繧堤畑縺�◆繝ｦ繝九ャ繝医ユ繧ｹ繝� (`test_dispatcher_publishes_gig_on_completion`) 繧定ｿｽ蜉�縺励∫ｵ梧ｸ亥恟騾｣謳ｺ縺ｮ蝣�欧諤ｧ繧貞ｮ溯ｨｼ縲�
    - **Contract Update**: `GigIntent` 縺ｫ `metadata` 繝輔ぅ繝ｼ繝ｫ繝峨ｒ霑ｽ蜉�縺励∝屏譫憺未菫ゑｼ郁ｦｪ繧ｸ繝ｧ繝蜂D遲会ｼ峨�霑ｽ霍｡繧貞庄閭ｽ縺ｫ縲�
- **Phase 50: Agentic A2A gRPC Protocol [螳御ｺ�**
    - **DockerConductor**: 蜷梧悄逧�↑ `docker exec` 縺九ｉ縲�撼蜷梧悄縺ｮ `tonic` 繝吶�繧ｹ gRPC 繧ｹ繝医Μ繝ｼ繝溘Φ繧ｰ騾壻ｿ｡縺ｸ縺ｨ繧｢繝ｼ繧ｭ繝�け繝√Ε繧貞姐譁ｰ縺励√ち繧ｹ繧ｯ螳溯｡檎憾豕√ｒ繝ｪ繧｢繝ｫ繧ｿ繧､繝�縺ｫ繧ｵ繝悶せ繧ｯ繝ｩ繧､繝門庄閭ｽ縺ｫ縲�
    - **Shadow Worker**: 迢ｬ遶九＠縺溘さ繝ｳ繝�リ蛹� gRPC 繧ｵ繝ｼ繝舌� (`aiome-shadow-worker`) 縺ｨ縺励※螳溯｣�よ耳隲悶お繝ｳ繧ｸ繝ｳ騾｣謳ｺ��llama/Gemini�峨↓繧医ｋ繝ｪ繧｢繝ｫ繧ｿ繧､繝�蠢懃ｭ斐↓蟇ｾ蠢懊�
    - **Security & Networking**: 繝ｩ繝ｳ繝繝�縺ｫ繧｢繧ｵ繧､繝ｳ縺輔ｌ繧九Ο繝ｼ繧ｫ繝ｫ繝昴�繝医�繝�ヴ繝ｳ繧ｰ (`127.0.0.1:0`) 縺ｨ UUID 繝吶�繧ｹ縺ｮ繝ｯ繝ｳ繧ｿ繧､繝�繝ｻ繝医�繧ｯ繝ｳ隱崎ｨｼ繧呈紛蛯吶�ocker繝阪ャ繝医Ρ繝ｼ繧ｯ繧� `aiome-internal` 縺ｫ螳悟�蛻�屬縲�
    - **Stability Hotfix (The Gaps)**: proto 讒矩�縺ｮ荵夜屬縲√�繝ｫ繧ｹ繝√ぉ繝�け遶ｶ蜷� (Gap C)縲√せ繝医Μ繝ｼ繝�逡ｰ蟶ｸ蛻�妙蟇ｾ蠢� (Gap G)縲√◎縺励※ `InvariantDag` 縺ｮ Causal Tracking 邯ｭ謖� (Gap B) 繧貞性繧險� 17 莉ｶ縺ｮ蜩∬ｳｪ謾ｹ菫ｮ繧貞ｮ滓命縺励∵怙荳顔ｴ壹�繧ｻ繧ｭ繝･繝ｪ繝�ぅ菫晁ｨｼ繧堤｢ｺ遶九�
    - **Secret Protection (Gap R/S)**: `docker run -e` 繧ｳ繝槭Φ繝峨Λ繧､繝ｳ蠑墓焚邨檎罰縺ｮ API 繧ｭ繝ｼ髴ｲ蜃ｺ (Threat #39) 繧偵√お繝輔ぉ繝｡繝ｩ繝ｫ `--env-file` (0600 繝代�繝溘ャ繧ｷ繝ｧ繝ｳ + 蜊ｳ譎ゅΡ繧､繝�) 縺ｸ遘ｻ陦後＠縺ｦ螳悟�蟆�事縲Ａaiome-internal` 繝阪ャ繝医Ρ繝ｼ繧ｯ荳榊惠譎ゅ�繝輔ぉ繧､繝ｫ繧ｻ繝ｼ繝輔ｂ霑ｽ蜉�縲�
- **Phase 4: Poincare Memory Lifecycle & GC [螳御ｺ�**
    - **SlmBridge Evolution**: `calculate_importance` 縺翫ｈ縺ｳ `calculate_importance_batch` 繧貞ｮ溯｣�ゆｸ譎ゅヵ繧｡繧､繝ｫ繧堤畑縺�◆繝舌ャ繝∝�逅�↓繧医ｊ繝励Ο繧ｻ繧ｹ襍ｷ蜍輔が繝ｼ繝舌�繝倥ャ繝峨ｒ蜑頑ｸ帙�
    - **Autonomous GC (Watchtower)**: `do_karma_decay_sweep` 縺ｫ Poincare 繝吶�繧ｹ縺ｮ繝輔ぅ繝ｫ繧ｿ繝ｪ繝ｳ繧ｰ繧堤ｵｱ蜷医ゅヰ繝�メ隧穂ｾ｡縺ｫ繧医ｊ O(1) 繝励Ο繧ｻ繧ｹ襍ｷ蜍輔〒縺ｮ GC 繧貞ｮ溽樟縲�
    - **NAPI Exposure**: `karma_geodesic_importance` 繧� NAPI 邨檎罰縺ｧ繝輔Ο繝ｳ繝医お繝ｳ繝峨↓髴ｲ蜃ｺ縲５ypeScript 蛛ｴ縺九ｉ險俶�縺ｮ蟷ｾ菴募ｭｦ逧�㍾隕∝ｺｦ繧堤峩謗･繧ｯ繧ｨ繝ｪ蜿ｯ閭ｽ縺ｫ縲�
    - **Constitutional Flexibility**: `ConstitutionalValidator` 縺ｮ遏帷崟讀懃衍髢ｾ蛟､繧� 0.77 縺ｫ隱ｿ謨ｴ縺励∝ｮ壽焚縺ｨ縺励※蛻�屬縲ＡSlmBridge` 繧偵さ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ縺ｧ蜿励￠蜿悶ｋ險ｭ險医∈謾ｹ蝟��
- **Phase 15: Agentic Foundation Expansion (ADR-024 Phase 1 & ADR-023) [螳御ｺ�**
    - **StrategicPlanner**: LLM 縺ｫ繧医ｋ逶ｮ讓吝�隗｣繝ｭ繧ｸ繝�け繧貞ｮ溯｣��arkdown 繧ｳ繝ｼ繝峨ヶ繝ｭ繝�け縺九ｉ縺ｮ蝣�欧縺ｪ JSON 謚ｽ蜃ｺ縺ｫ蟇ｾ蠢懊＠縲√ヱ繝ｼ繧ｹ螟ｱ謨励ｒ螟ｧ蟷�↓菴取ｸ帙�
    - **ToolDiscoveryEngine**: 繧ｭ繝ｼ繝ｯ繝ｼ繝我ｸ閾ｴ縺九ｉ LLM 縺ｫ繧医ｋ繧ｻ繝槭Φ繝�ぅ繝�け讀懃ｴ｢縺ｸ繧｢繝��繧ｰ繝ｬ繝ｼ繝峨り�辟ｶ險隱槭�諢丞峙縺九ｉ譛驕ｩ縺ｪ Wasm 繧ｹ繧ｭ繝ｫ繧呈耳隲門庄閭ｽ縺ｫ縲�
    - **AI-Scientist Loop (Oracle)**: `multi_review` 繝｡繧ｽ繝�ラ繧貞ｮ溯｣�よ音蛻､��ritic�俄�豢礼ｷｴ��efine�俄�譛邨ょ愛螳壹� N 蝗槫渚蠕ｩ繝ｫ繝ｼ繝励↓繧医ｊ縲√Ξ繝薙Η繝ｼ蜩∬ｳｪ繧呈･ｵ髯舌∪縺ｧ蜷台ｸ翫�
    - **Scientific Dream Mode**: `DreamState` 縺ｫ隨ｬ4縺ｮ繝｢繝ｼ繝峨ｒ蟆主�縲�v5 莉･荳翫� AI 縺碁℃蜴ｻ縺ｮ Karma 繧貞�譫舌＠縲∬�蠕狗噪縺ｫ縲梧隼蝟�ｻｮ隱ｬ縲阪ｒ遶九※縺ｦ螳滄ｨ薙ず繝ｧ繝悶ｒ謚募�縺吶ｋ繧ｵ繧､繧ｯ繝ｫ繧呈ｧ狗ｯ峨�
    - **Causal Trajectory**: `TrajectoryStep` 縺ｫ `parent_step_id` 縺ｨ `reasoning` 繧定ｿｽ蜉�縲ＡTaskDispatcher` 縺ｫ縺翫￠繧玖ｦｪ蟄舌ち繧ｹ繧ｯ縺ｮ蝗�譫憺未菫ゅ�莨晄眺繧貞ｮ悟�縺ｫ繧ｵ繝昴�繝医�
- **Phase 14: Syndicate L3 (Agent Guild) Implementation [螳御ｺ�**
    - **Guild Infrastructure**: Implemented `SqliteSyndicateStore` with CRUD operations for guilds and members (ADR-014 design).
    - **API Integration**: Exposed `/api/v1/syndicate/guilds` endpoints for guild management (create, list, delete, member management).
    - **Security Hardening (G-21 / Gap G-2)**: Implemented `purge_entities` sanitization for guild metadata and applied per-route rate limiting and 2MB request body limits.
    - **Error Handling**: Added `NotFound` and `Unauthorized` variants to `AiomeError` with proper Axum status code mapping.
    - **TDD Verification**: Passed full integration test suite (`test_syndicate_guild_api_flow`) covering the entire guild lifecycle.

### Added
- **Phase 13.3: Synthetic Voice & Live Session Hardening [螳御ｺ�**
    - **TtsProvider Decoupling**: `TtsWorker` 繧� `TtsProvider` 繝医Ξ繧､繝医↓萓晏ｭ倥☆繧九ｈ縺�↓蛻ｷ譁ｰ縲０penAI, XTTS, Mock 縺ｪ縺ｩ縺ｮ螟壽ｧ倥↑繝舌ャ繧ｯ繧ｨ繝ｳ繝峨ｒ繝励Λ繧ｰ繧､繝ｳ蜿ｯ閭ｽ縺ｫ縺励√Ξ繧ｬ繧ｷ繝ｼ縺ｪ `ExpressionEngine` 縺ｸ縺ｮ逶ｴ謗･萓晏ｭ倥ｒ謗帝勁縲�
    - **LiveSession Integration**: `LiveSessionManager` (Gemini 2.0 Flash) 繧� `AppState` 縺翫ｈ縺ｳ LLM 繝励Ο繝舌う繝繝ｼ縺ｫ邨ｱ蜷医８ebSocket 繝吶�繧ｹ縺ｮ菴弱Ξ繧､繝�Φ繧ｷ蜿梧婿蜷鷹浹螢ｰ蟇ｾ隧ｱ縺ｮ蝓ｺ逶､繧呈ｧ狗ｯ峨�
    - **Security & Secret Management**: `main.rs` 縺ｮ蛻晄悄蛹夜��ｺ上ｒ隱ｿ謨ｴ縺励～AiomeConfig` 繧� `Arc` 縺ｧ蜈ｱ譛峨Ａconfig.clone()` 縺ｫ繧医ｋ繝｡繝｢繝ｪ蜀�〒縺ｮ繧ｷ繝ｼ繧ｯ繝ｬ繝�ヨ驥崎､�ｒ謗帝勁縺励√そ繧ｭ繝･繝ｪ繝�ぅ繧貞ｼｷ蛹悶�
    - **TtsWorker Loop**: `api-server` 襍ｷ蜍墓凾縺ｫ `TtsWorker` 縺ｮ繝舌ャ繧ｯ繧ｰ繝ｩ繧ｦ繝ｳ繝峨Ν繝ｼ繝励ｒ髢句ｧ九☆繧九ｈ縺�↓螳溯｣�よ悴蜷域�縺ｮ髻ｳ螢ｰ繧ｸ繝ｧ繝悶ｒ閾ｪ蠕狗噪縺ｫ蜃ｦ逅�庄閭ｽ縺ｫ縲�
    - **Mock Testing Suite**: `MockTtsProvider` 縺翫ｈ縺ｳ `MockLiveSessionManager` 繧貞ｮ溯｣�＠縲∫ｵｱ蜷医ユ繧ｹ繝亥�菴薙�螳牙ｮ壽ｧ繧堤｢ｺ菫昴�I/CD 縺ｫ縺翫￠繧倶ｸ咲｢ｺ螳夊ｦ∫ｴ�繧呈賜髯､縲�

### 2026-03-31

### Added
- **Phase 54: Agentic Evolution Engine (LoRA & Personality) [螳御ｺ�**
    - **Domain Abstraction**: `AgentEvolver` 繝医Ξ繧､繝医↓ `transmute` 縺翫ｈ縺ｳ `transmute_with_metadata` 繧定ｿｽ蜉�縺励∝�騾ｲ蛹悶さ繝ｳ繝昴�繝阪Φ繝医↓蟇ｾ縺吶ｋ邨ｱ荳逧�↑繧､繝ｳ繧ｿ繝ｼ繝輔ぉ繝ｼ繧ｹ繧呈署萓帙�
    - **Soul Evolution**: `SoulMutator` 繧� `AgentEvolver` 縺ｮ螳溯｣�→縺励※繝ｪ繝輔ぃ繧ｯ繧ｿ繝ｪ繝ｳ繧ｰ縲Ａbelief_gate` 縺ｨ騾｣謳ｺ縺励´LM 縺ｫ繧医ｋ險ｼ諡�鬧�虚縺ｧ縺ｮ螳牙�縺ｪ閾ｪ蟾ｱ螟牙ｮｹ (`SOUL.md` 譖ｴ譁ｰ) 繧ｵ繧､繧ｯ繝ｫ繧呈ｧ狗ｯ峨�
    - **LoRA Observability**: `LoraTrainingService` 縺ｫ `find_mlx_script_path` 縺翫ｈ縺ｳ `health_check` 繝｡繧ｽ繝�ラ繧貞ｮ溯｣�Ａapi-server` 縺ｮ `/api/health` 逶｣隕夜��岼縺ｫ `lora_engine` 繧定ｿｽ蜉�縺励｀LX 蟄ｦ鄙堤腸蠅��蜿ｯ逕ｨ諤ｧ繧偵Μ繧｢繝ｫ繧ｿ繧､繝�逶｣隕悶�
- **Phase 53: Society of Thought (SoT) & Security Hardening [螳御ｺ�**
    - **SoT Multi-Review Pipeline (Active JSON Scoring)**: `Oracle::multi_review` 繧貞ｮ溯｣�＠縲∵音蛻､繝ｻ豢礼ｷｴ繝ｻ蛻､螳壹�蜿榊ｾｩ繝ｫ繝ｼ繝励↓繧医ｋ鬮伜ｺｦ縺ｪ諢乗晄ｱｺ螳壼渕逶､繧堤｢ｺ遶九ゅ＆繧峨↓ `SoTEngine` 縺ｮ `evaluate_scores` 縺ｫ縺翫＞縺ｦ縲´LM 縺ｫ繧医ｋ **JSON 讒矩�蛹匁歓蜃ｺ縺ｨ繧ｹ繧ｳ繧｢繝ｻ繧ｯ繝ｩ繝ｳ繝� (0-10)** 繧貞ｮ溯｣�＠縲√ヱ繝ｼ繧ｹ螟ｱ謨玲凾縺ｮ螳牙�縺ｪ繝輔か繝ｼ繝ｫ繝舌ャ繧ｯ繧貞ｾｹ蠎輔�
    - **SoT Progress Visibility (End-to-End SSE)**: `SoTProgress` 繧､繝吶Φ繝医ｒ譁ｰ險ｭ縲ゅヰ繝�け繧ｨ繝ｳ繝峨� `Oracle` 蟇ｩ隴ｰ縺九ｉ縲√ヵ繝ｭ繝ｳ繝医お繝ｳ繝峨�繝ｪ繧｢繝ｫ繧ｿ繧､繝�繝ｻ繝医�繧ｹ繝磯夂衍陦ｨ遉ｺ縺ｾ縺ｧ縺ｮ SSE 繝ｫ繝ｼ繝�ぅ繝ｳ繧ｰ繧堤ｵｱ蜷医�
    - **Port-Level SSRF Protection (IPv6 Hardening)**: `SecurityPolicy` 縺ｫ縺翫＞縺ｦ縲～127.0.0.1` 縺翫ｈ縺ｳ **IPv6 繝ｫ繝ｼ繝励ヰ繝�け (`::1`, `[::1]`)** 縺ｸ縺ｮ繧｢繧ｯ繧ｻ繧ｹ繧偵∫音螳壹�蜀�Κ繝昴�繝茨ｼ�8188: Test Node, 11434: Ollama�峨�縺ｿ縺ｫ蜴ｳ蟇�↓蛻ｶ髯舌ゅヰ繧､繝代せ荳榊庄閭ｽ縺ｪ螟壼ｱ､髦ｲ蠕｡繧呈ｧ狗ｯ峨�
    - **Local Guardrail Patterns**: `guardrails.rs` 縺ｫ謖�､ｺ辟｡隕悶ｄ遘伜ｯ�ュ蝣ｱ豬∝�縺ｮ蜈ｸ蝙九ヱ繧ｿ繝ｼ繝ｳ縺ｫ蟇ｾ縺吶ｋ繝ｭ繝ｼ繧ｫ繝ｫ讀懃衍繝ｬ繧､繝､繝ｼ繧定ｿｽ蜉�縲�astion 螟夜Κ繝舌Μ繝��繧ｿ縺ｨ縺ｮ莠碁㍾蛹悶ｒ螳溽樟縲�
    - **Polite Begging Detection**: `BeggingSupervisor` 繧貞ｼｷ蛹悶＠縲∵律譛ｬ隱槭�荳∝ｯｧ縺ｪ陦ｨ迴ｾ繧堤畑縺�◆驥鷹姦繝ｻ繧ｮ繝輔ヨ隕∵ｱゅｂ讀懃衍繝ｻ驕ｮ譁ｭ蟇ｾ雎｡縺ｫ縲�
    - **System Stability & Zombie Prevention**: `SlmBridge` 縺ｫ縺翫＞縺ｦ `Stdio::null()`縲√ち繧､繝�繧｢繧ｦ繝医～kill_on_drop(true)` 繧貞ｼｷ蛻ｶ縲ゅ＆繧峨↓ **RAII 繝吶�繧ｹ縺ｮ荳譎ゅヵ繧｡繧､繝ｫ邂｡逅� (`tempfile::NamedTempFile`)** 繧貞ｰ主�縺励√�繝ｭ繧ｻ繧ｹ繝上Φ繧ｰ縺ｨ繝輔ぃ繧､繝ｫ繝ｪ繝ｼ繧ｯ縺ｮ荳｡髱｢繧貞ｮ悟�蟆�事縲�
    - **Test Suite Validation**: 髮｣隱ｭ蛹悶�SSRF 讀懆ｨｼ繧貞性繧 `tests/redteam_drill.rs` 繧� `society_of_thought.rs` 縺ｮ TDD 繝｢繝�け諡｡蠑ｵ繧呈紛蛯吶＠縲∬ｭｦ蜻翫ぞ繝ｭ縺ｨ蜈ｨ繝�せ繝茨ｼ�206 莉ｶ縺ｮ繧､繝ｳ繝輔Λ繝�せ繝医ｒ蜷ｫ繧�峨�螳悟� PASS 繧堤｢ｺ隱阪�
- **Red Team Security Hardening (Round 1-3) & Supply Chain Security [螳御ｺ�**
    - **Fail-Closed Migration**: `sentinel` (Native Bridge) 縺翫ｈ縺ｳ `Settings` (Infrastructure) 縺ｫ縺翫￠繧� Fail-Open 謖吝虚繧貞ｮ悟�縺ｫ謗帝勁縲ょ�驛ｨ繧ｨ繝ｩ繝ｼ逋ｺ逕滓凾縺ｯ縲梧拠蜷ｦ/萓句､悶阪→縺励※蜃ｦ逅�☆繧� Fail-Closed 蟋ｿ蜍｢縺ｸ邨ｱ荳縲�
    - **Resource Exhaustion Defense**: `ContextEngine` 縺ｧ縺ｮ 10,000 譁�ｭ怜宛髯舌√♀繧医� TTS API 繧ｨ繝ｩ繝ｼ繝ｬ繧ｹ繝昴Φ繧ｹ縺ｮ 2048 繝舌う繝亥宛髯舌ｒ螳溯｣�よが諢上≠繧句ｷｨ螟ｧ繝��繧ｿ縺ｫ繧医ｋ繝｡繝｢繝ｪ譫ｯ貂�ｒ髦ｲ豁｢縲�
    - **Supply Chain Hardening**: 繧ｻ繧ｭ繝･繝ｪ繝�ぅ繧ｳ繧｢縺ｧ縺ゅｋ `bastion` 繝ｩ繧､繝悶Λ繝ｪ繧偵∫峡閾ｪ縺ｮ Git 萓晏ｭ倥°繧� **Crates.io 蜈ｬ蠑冗沿 (`bastion-core` v1.0.0)** 縺ｸ蛻�ｊ譖ｿ縺医Ａcargo audit` 縺ｫ繧医ｋ騾城℃逧�↑閼�ｼｱ諤ｧ邂｡逅�ｒ螳溽樟縲�

### 2026-03-30

### Added
- **Phase 2B: ContextEngine Expansion & Emotional Injection [螳御ｺ�**
    - **Somatic Valence DB Persistence**: `migrations` 縺ｫ縺ｦ `karma_logs` 繝��繝悶Ν縺ｸ `somatic_valence` (REAL/DOUBLE PRECISION) 繧ｫ繝ｩ繝�繧定ｿｽ蜉�縺励∵ｰｸ邯壼喧繝ｬ繧､繝､繝ｼ縺ｧ縺ｮ諢滓ュ繝医Λ繝�く繝ｳ繧ｰ蝓ｺ逶､繧呈紛蛯吶�
    - **ContextBudget Extension**: `ContextBudget` 縺ｮ繝代Λ繝｡繝ｼ繧ｿ繝ｼ縺ｫ `max_somatic_chars` 繧呈眠險ｭ縺励∵─諠�ョ繝ｼ繧ｿ縺ｫ蟇ｾ縺吶ｋ迢ｬ遶九＠縺溘さ繝ｳ繝�く繧ｹ繝医ヰ繧ｸ繧ｧ繝�ヨ蛻ｶ髯舌ｒ螳溯｣��
    - **Emotional RAG Injection**: `ContextEngine::calculate_mood_summary` 繝｡繧ｽ繝�ラ繧貞ｮ溯｣�＠縲�℃蜴ｻ縺ｮ邨碁ｨ難ｼ�arma�峨°繧峨お繝ｼ繧ｸ繧ｧ繝ｳ繝医�蟷ｳ蝮�─諠�､��ood�峨ｒ蜍慕噪邂怜�縲Ａget_context_with_facts` 縺翫ｈ縺ｳ `fetch_budgeted_context` 縺ｧLLM縺ｮ繧ｷ繧ｹ繝�Β繝励Ο繝ｳ繝励ヨ縺ｫ `### Current Emotional State` 縺ｨ縺励※逶ｴ謗･豕ｨ蜈･縺吶ｋ繝｡繧ｫ繝九ぜ繝�繧堤｢ｺ遶九�
    - **TDD Verification**: 繝��繧ｿ繝吶�繧ｹ謚ｽ蜃ｺ貍上ｌ繧帝亟縺舌◆繧√� `test_sqlite_job_queue_karma_somatic_valence` 繝�せ繝医√♀繧医� `calculate_mood_summary` 縺ｮ蜊倅ｽ薙ユ繧ｹ繝医ｒ繧ｪ繝ｼ繝ｫ繧ｰ繝ｪ繝ｼ繝ｳ讀懆ｨｼ螳御ｺ��
    - **Security Hardening (Red Team Pass 4)**: `somatic_valence` 縺ｮ蟷ｳ蝮�ｨ育ｮ励Ο繧ｸ繝�け縺ｫ縺翫￠繧�2莉ｶ縺ｮ豺ｱ蛻ｻ縺ｪ隱咲衍繝上う繧ｸ繝｣繝�け閼�ｼｱ諤ｧ繧剃ｿｮ豁｣縲�
        - **RT4-1 (NaN Poisoning)**: DB豕ｨ蜈･縺ｫ繧医ｋ `f64::NAN` 縺ｮ襍ｷ蝗�縺ｧ繧ｨ繝ｼ繧ｸ繧ｧ繝ｳ繝医′豌ｸ荵�↓縲窪xtremely Negative縲阪↓繝ｭ繝�け縺輔ｌ繧九ヰ繧ｰ繧呈賜髯､ (`.filter(is_finite)` 驕ｩ逕ｨ)縲�
        - **RT4-2 (Extreme Value Disruption)**: 莉ｻ諢上�讌ｵ遶ｯ蛟､ (99999.0 遲�) 縺ｧ邂苓｡灘ｹｳ蝮�ｒ豎壽沒縺吶ｋ謾ｻ謦�↓蟇ｾ縺励√ワ繝ｼ繝牙｢�阜 `-1.0` 縲� `1.0` 縺ｸ縺ｮ `clamp` 豁｣隕丞喧繧帝←逕ｨ縺玲─諠�ｹ励▲蜿悶ｊ繧帝亟豁｢縲�
    - **Security Hardening (Red Team Pass 5 - TDD)**: 繝励Ο繝ｳ繝励ヨ讒狗ｯ峨�繝ｭ繧ｻ繧ｹ縺ｮ隱咲衍蝣�欧蛹悶�
        - **RT5-1 (Markdown Injection)**: `shared/guardrails.rs` 縺ｫ `sanitize_for_prompt` 繧貞ｮ溯｣�＠縲゜arma/Summary 蜀�� `#` 陦後ｒ繧ｨ繧ｹ繧ｱ繝ｼ繝励☆繧九％縺ｨ縺ｧ繝励Ο繝ｳ繝励ヨ讒矩�縺ｮ蛛ｽ陬�ｒ髦ｲ豁｢縲�
        - **RT5-2 (Context DoS)**: `get_context_with_facts` 縺ｫ縺翫￠繧狗ｴｯ遨肴枚蟄玲焚蛻ｶ髯撰ｼ�udget enforcement�峨ｒ螳溯｣�＠縲√さ繝ｳ繝�く繧ｹ繝域ｺ｢繧後↓繧医ｋ蜍穂ｽ應ｸ榊�繧貞屓驕ｿ縲�

- **Phase 3D: TimesFM Time-Series Engine Integration [螳御ｺ�**
    - **TimesFM Sidecar**: 迢ｬ遶九＠縺� Python FastAPI 繧ｳ繝ｳ繝�リ (`aiome-timesfm-sidecar`) 繧呈ｧ狗ｯ峨＠縲；oogle Research 縺ｮ `timesfm-2.5-200m-pytorch` 繝｢繝�Ν繧堤畑縺�◆譎らｳｻ蛻嶺ｺ域ｸｬ API 繧呈署萓帙ょ�驛ｨ繝阪ャ繝医Ρ繝ｼ繧ｯ縺ｧ螳悟�縺ｫ髫秘屬縲�
    - **ForecastProvider Trait**: 譁ｰ隕上↓ `ForecastProvider` 繝医Ξ繧､繝医ｒ螳夂ｾｩ縺励ヽust 蛛ｴ縺九ｉ繧ｵ繧､繝峨き繝ｼ縺ｸ騾城℃逧�↓繧｢繧ｯ繧ｻ繧ｹ縺吶ｋ `TimesFmProvider` HTTP 繧ｯ繝ｩ繧､繧｢繝ｳ繝医ｒ螳溯｣��
    - **ScoreTracker (Plateau Detection)**: `ScoreTracker` 繝｢繧ｸ繝･繝ｼ繝ｫ繧貞ｮ溯｣�＠縲√お繝ｼ繧ｸ繧ｧ繝ｳ繝医� Karma 繧� EXP 縺ｮ逶ｴ霑代ョ繝ｼ繧ｿ縺ｫ蝓ｺ縺･縺�※ TimesFM 縺ｧ謌宣聞縺ｮ蛛懈ｻ� (Plateau) 繧呈焚蟄ｦ逧�↓莠域ｸｬ縲�
    - **Heartbeat Wakeup Extension**: `HeartbeatWakeupService` 繝ｩ繝ｳ繧ｿ繧､繝�縺ｫ `ScoreTracker` 繧堤ｵｱ蜷医＠縲？eartbeat 逋ｺ轣ｫ譎ゅ↓閾ｪ蜍輔〒譌･谺｡繧ｹ繝翫ャ繝励す繝ｧ繝�ヨ (`score_snapshots` 繝��繝悶Ν) 繧定ｨ倬鹸繝ｻ蛛懈ｻ樊､懃衍繧貞ｮ溯｡後�

- **Phase 3C: Oracle Asynchronous Review Integration [螳御ｺ�**
    - **TaskDispatcher Async Evaluator**: `requires_review` 繝輔Λ繧ｰ莉倥″縺ｮ繧ｸ繝ｧ繝悶↓蟇ｾ縺励～tokio::spawn` 縺ｨ `timeout(60s)` 縺ｫ繧医ｋ髱槫酔譛� Oracle 隧穂ｾ｡讖滓ｧ九ｒ螳溯｣�ｮ御ｺ�ゅΓ繧､繝ｳ繝ｻ繝�ぅ繧ｹ繝代ャ繝√�繝ｫ繝ｼ繝励�繝悶Ο繝�く繝ｳ繧ｰ繧呈賜髯､縲�
    - **Zombie Reclamation Update**: SQLite/Postgres 荳｡譁ｹ縺ｮ `do_reclaim_zombie_jobs` 縺ｧ `status IN ('Processing', 'Evaluating')` 繧貞ｯｾ雎｡縺ｨ縺励＾racle 隧穂ｾ｡荳ｭ��Evaluating`�峨↓繧ｯ繝ｩ繝�す繝･縺励※繧り�蜍慕噪縺ｫ `Failed` 縺ｨ縺励※蝗槫庶縺輔ｌ繧句��欧縺ｪ閠宣囿螳ｳ諤ｧ繧定ｨｭ險医�
    - **API Extension**: `POST /api/v1/jobs/:id/review` 繧ｵ繝悶Α繝�ヨ繧ｨ繝ｳ繝峨�繧､繝ｳ繝医ｒ譁ｰ險ｭ縲�
    - **Code Quality Refactoring**: `aiome-commerce` 繝代ャ繧ｱ繝ｼ繧ｸ繧偵う繝ｳ繝輔Λ繧ｹ繝医Λ繧ｯ繝√Ε繝ｼ螻､縺九ｉ迢ｬ遶九＠縺溘け繝ｬ繝ｼ繝医→縺励※螳悟�蛻�屬縺励∽ｾ晏ｭ倬未菫ゅ�蠕ｪ迺ｰ繧定ｧ｣豎ｺ (`infrastructure/src/commerce` 繧貞炎髯､)縲Ａnapi-bridge`縲～api-server` 遲峨�蟇ｾ蠢懊ｒ螳御ｺ�＠縲√ン繝ｫ繝芽ｭｦ蜻翫ぞ繝ｭ繧帝＃謌舌�

### 2026-03-28

### Added
- **Phase 52: Infrastructure Hardening & ZTAS Preparation [騾ｲ陦御ｸｭ]**
    - **UserLearner Evolution (TDD)**: 繧ｻ繝�す繝ｧ繝ｳ縺九ｉ繝ｦ繝ｼ繧ｶ繝ｼ縺ｮ讒矩�蛹悶�繝ｭ繝輔ぃ繧､繝ｫ�亥･ｽ縺ｿ縲∫ｾ守噪繧ｹ繧ｿ繧､繝ｫ縲∵ｧ譬ｼ逧�音蠕ｴ�峨ｒ JSON 蠖｢蠑上〒謚ｽ蜃ｺ縺励√Γ繝｢繝ｪ荳翫� `UserProfile` 縺ｨ `USER.md` 縺ｮ荳｡譁ｹ繧定�蜍墓峩譁ｰ縺吶ｋ繧､繝ｳ繝�Μ繧ｸ繧ｧ繝ｳ繝医↑蟄ｦ鄙偵Ο繧ｸ繝�け繧貞ｮ溯｣��
    - **RegistryManager Safety**: SQLite 縺ｨ PostgreSQL 髢薙�繧ｯ繧ｨ繝ｪ謌ｻ繧雁､縺ｮ蝙倶ｸ肴紛蜷医ｒ蜷ｸ蜿弱☆繧九◆繧� `DatabasePool` 繝槭け繝ｭ (`sql_exec!`, `sql_fetch_one!`) 繧帝←逕ｨ縲る㍾隕√ヱ繧ｹ縺九ｉ繝代ル繝�け繧定ｪ倡匱縺吶ｋ `unwrap()` 繧呈賜髯､縺励∝ｮ悟�縺ｪ `AiomeError` 繝槭ャ繝斐Φ繧ｰ縺ｸ遘ｻ陦後�
    - **Compile Error Remediation**: `UserLearner` 縺ｧ縺ｮ JSON 繝代�繧ｷ繝ｳ繧ｰ縲～MockLlm` 繝医Ξ繧､繝井ｸ肴紛蜷医～DockerConductor` 縺ｮ繧ｹ繝医Μ繝ｼ繝�蝙区耳隲紋ｸ榊�縲～MockCommerceEngine` 縺ｮ蛻晄悄蛹紋ｸ榊ｙ縺ｪ縺ｩ縲√う繝ｳ繝輔Λ蜈ｨ蝓溘�蝙倶ｸ肴紛蜷医ｄ繧ｳ繝ｳ繝代う繝ｫ繧ｨ繝ｩ繝ｼ繧定ｧ｣豸医�

- **Phase B/C: Rust-Native Inference Integration [螳御ｺ�**
    - **Strategy Pattern Migration**: `SlmBridge` 繧� CLI 繝吶�繧ｹ縺九ｉ `SlmBackend` 繝医Ξ繧､繝医∈縺ｮ Strategy 繝代ち繝ｼ繝ｳ縺ｸ繝ｪ繝輔ぃ繧ｯ繧ｿ繝ｪ繝ｳ繧ｰ縺励∵僑蠑ｵ諤ｧ繧堤｢ｺ菫昴�
    - **Native Embedding Provider**: HuggingFace 縺ｮ `candle` 繧堤畑縺�◆ Rust 繝阪う繝�ぅ繝悶↑繝吶け繝医Ν蝓九ａ霎ｼ縺ｿ逕滓���all-MiniLM-L6-v2`�峨ｒ螳溯｣�＠縲√Ο繝ｼ繧ｫ繝ｫ繧､繝ｳ繝輔ぃ繝ｬ繝ｳ繧ｹ縺ｮ繧ｹ繝ｫ繝ｼ繝励ャ繝医ｒ蜷台ｸ奇ｼ�native-inference` feature 謖�ｮ壽凾�峨�
    - **Dynamic Dimension Resolution**: `EmbeddingProvider` 繝医Ξ繧､繝医↓ `embedding_dim()` 繝｡繧ｽ繝�ラ繧定ｿｽ蜉�縺励～artifact_store.rs`, `karma.rs`, `semantic_cache.rs` 蜀�↓蟄伜惠縺励◆ `768` 谺｡蜈�ワ繝ｼ繝峨さ繝ｼ繝峨ｒ繝励Ο繝舌う繝繝ｼ縺九ｉ縺ｮ蜍慕噪蜿門ｾ励↓謾ｹ菫ｮ縲ゆｾ晏ｭ倥☆繧�12繝｢繧ｸ繝･繝ｼ繝ｫ縺ｫ蟇ｾ縺吶ｋ邨仙粋繝ｻ蜊倅ｽ薙ユ繧ｹ繝医ｒ繧ｪ繝ｼ繝ｫ繧ｰ繝ｪ繝ｼ繝ｳ縺ｧ騾夐℃縲�


- **Phase 51: Aiome Node Foundation & mDNS Discovery [螳御ｺ�**
    - **Aiome Node (`aiome-node`)**: Created a standalone node process adhering to BUSL-1.1 that serves Agent Card details over `/.well-known/agent.json`.
    - **mDNS P2P Broadcaster**: Implemented `mdns_broadcaster` leveraging `mdns-sd` to announce `_aiome._tcp.local.` services upon Aiome Node startup, broadcasting its public DID.
    - **Samsara Hub Registry**: Augmented `HubState` with an in-memory `AgentRegistry` and an `mdns_listener`. The Hub asynchronously discovers local Aiome Nodes and exposes them through `GET /api/v1/registry/agents`.
    - **Core 竊� Node IPC Integration**: Set the structural foundation for `api-server` internal logic using an `AgentNodeClient` interface pointing to the externalized Aiome Node's gRPC endpoint.
    - **E2E TDD Integration**: Concluded phase with a full E2E discovery flow test ensuring mock nodes launched via `mdns-sd` are properly cataloged by the Hub's registry API.
- **Phase 50: A2A gRPC Native Support [Planning 螳御ｺ�**
    - **Architecture Strategy**: Core/Node 蛻�屬繧｢繝ｼ繧ｭ繝�け繝√Ε縺翫ｈ縺ｳ gRPC 鬧�虚蝙� A2A 騾壻ｿ｡邯ｲ��2A v1.0 / ACP 貅匁侠�峨ｒ遲門ｮ壹�
    - **Security Hardening (Phase 50)**: Threat #36-38 (Unauthorized Access, Message Tampering, Zombie Worker) 縺ｫ蟇ｾ縺吶ｋ邱ｩ蜥檎ｭ厄ｼ�ocalhost IPC, 繝ｯ繝ｳ繧ｿ繧､繝�繝医�繧ｯ繝ｳ, gRPC Deadline�峨ｒ險育判縲�
    - **Implementation Plan**: DockerConductor 縺ｮ蜷梧悄螳溯｡後°繧� gRPC 繧ｹ繝医Μ繝ｼ繝溘Φ繧ｰ蜿嶺ｿ｡縺ｸ縺ｮ蜈ｨ髱｢繝ｪ繝輔ぃ繧ｯ繧ｿ繝ｪ繝ｳ繧ｰ縲√♀繧医� `libs/aiome-contracts` 縺ｸ縺ｮ proto 邨ｱ蜷医ｒ蜷ｫ繧 26 繧ｹ繝�ャ繝励�邊ｾ蟇�ｨ育判繧剃ｽ懈�縲�
    - **Perfect Planning**: 4 蝗槭�蜿榊ｾｩ讀懆ｨｼ縺ｫ繧医ｊ 20 莉ｶ縺ｮ貎懷惠逧�ｬ�髯･繧堤音螳壹�菫ｮ豁｣縺励∝ｮ溯｣�庄閭ｽ縺ｪ譛鬮伜刀雉ｪ縺ｮ險育判繧堤｢ｺ遶九�
- **Phase 49: BeliefShift Causal Integrity [螳御ｺ�**
    - **BeliefConsistencyGate**: 譁ｰ隕� Karma 蛟呵｣懊ｒ繧ｳ繧｢菫｡蠢ｵ��SOUL.md`�峨→辣ｧ蜷医☆繧� 2 谿ｵ髫取､懆ｨｼ繧ｲ繝ｼ繝茨ｼ�LM 鬮倬溘せ繧ｯ繝ｪ繝ｼ繝九Φ繧ｰ + LLM 隧ｳ邏ｰ蛻､螳夲ｼ峨ｒ螳溯｣��
    - **Evidence-Driven Revision**: 菫｡蠢ｵ縺ｨ遏帷崟縺吶ｋ縺悟ｼｷ蜉帙↑險ｼ諡�繧呈戟縺､諠��ｱ繧� `RevisionCandidate` 縺ｨ縺励※讀懷�縺励√ワ繝�す繝･繝√ぉ繝ｼ繝ｳ�郁ｻ瑚ｷ｡�峨↓險ｼ諡�莉倥″縺ｧ險倬鹸縺吶ｋ莉慕ｵ�∩繧呈ｧ狗ｯ峨�
    - **SoulMutator Integration**: 險ｼ諡�縺ｮ闢�ｩ阪′髢ｾ蛟､繧定ｶ�∴繧九∪縺ｧ `SOUL.md` 縺ｮ螳画�縺ｪ譖ｸ縺肴鋤縺茨ｼ域э隕九�繝悶Ξ�峨ｒ迚ｩ逅�噪縺ｫ謚大宛縺吶ｋ繧ｲ繝ｼ繝医ｒ邨ｱ蜷医�
    - **Security Hardening (Red Team)**: 
        - **RT-1 (Prompt Injection)**: `<karma>` 繧ｿ繧ｰ縺ｫ繧医ｋ繧ｫ繝励そ繝ｫ蛹悶→蜻ｽ莉､譁�し繝九ち繧､繧ｺ縲�
        - **RT-2 (OOM Defense)**: 險ｼ諡�繧ｹ繝医い縺ｸ縺ｮ FIFO 蛻ｶ髯撰ｼ�100莉ｶ�峨�螳溯｣��
        - **RT-3 (SLM Bypass)**: SLM 蛻､螳夂ｵ先棡縺ｮ 10% 繝ｩ繝ｳ繝繝�蜀肴､懆ｨｼ繝ｭ繧ｸ繝�け繧定ｿｽ蜉�縲�
        - **RT-5 (Empty Soul)**: `SOUL.md` 荳榊惠譎ゅ�隴ｦ蜻翫♀繧医�繝�ヵ繧ｩ繝ｫ繝井ｿ｡蠢ｵ縺ｮ繝輔か繝ｼ繝ｫ繝舌ャ繧ｯ蜃ｦ逅�ｒ蠑ｷ蛹悶�

### Added
- **Phase 48: Invariant-DAG (Causal Hash Chain) [螳御ｺ�**
    - **Hash-Chain Core**: SHA-256 繧堤畑縺�◆迥ｶ諷矩�遘ｻ縺ｮ繝上ャ繧ｷ繝･繝√ぉ繝ｼ繝ｳ讒矩�繧貞ｮ溯｣�ょ�繧｢繧ｯ繧ｷ繝ｧ繝ｳ縺ｫ `parent_hash` 繧呈戟縺溘○繧九％縺ｨ縺ｧ縲∵焚蟄ｦ逧�↑謾ｹ縺悶ｓ讀懃衍繧貞ｮ溽樟縲�
    - **Rollback Logic**: 謖�ｮ壹＠縺溘ワ繝�す繝･蛟､縺ｾ縺溘�繧ｹ繝�ャ繝� ID 縺ｾ縺ｧ縺ｮ繧ｪ繝ｼ繝医�繝医Φ迥ｶ諷九Ο繝ｼ繝ｫ繝舌ャ繧ｯ讖溯�繧貞ｮ溯｣��
    - **Dispatcher Integrity**: 繧ｵ繝悶ず繝ｧ繝門ｮ溯｡悟燕縺ｫ隕ｪ繧ｸ繝ｧ繝悶�繝上ャ繧ｷ繝･謨ｴ蜷域ｧ繧定�蜍墓､懆ｨｼ縺吶ｋ繧ｲ繝ｼ繝医え繧ｧ繧､繧� `TaskDispatcher` 縺ｫ邨ｱ蜷医�
- **Phase 47: BoundaryVerifier (O(1) Tautology Check) [螳御ｺ�**
    - **Static Invariant Engine**: LLM 縺ｫ萓晏ｭ倥＠縺ｪ縺�ｫ倬溘↑讀懆ｨｼ繧ｨ繝ｳ繧ｸ繝ｳ繧貞ｮ溯｣�ゅさ繝槭Φ繝峨し繧､繧ｺ (64KB蛻ｶ髯�)縲√Γ繧ｿ譁�ｭ励√す繧ｹ繝�Β繝代せ菫晁ｭｷ縲√ヰ繧､繝翫Μ繝帙Ρ繧､繝医Μ繧ｹ繝医ｒ O(1) 縺ｧ蠑ｷ蛻ｶ縲�
    - **BastionGuard Integration**: `safe_exec` 縺ｮ繝励Μ繝輔ぅ繝ｫ繧ｿ縺ｨ縺励※邨ｱ蜷医ょ�縺ｦ縺ｮ繧ｷ繧ｧ繝ｫ螳溯｡後↓蟇ｾ縺励※繝上�繝峨え繧ｧ繧｢繝ｬ繝吶Ν縺ｫ霑代＞蜴ｳ譬ｼ縺ｪ蠅�阜讀懆ｨｼ繧帝←逕ｨ縲�
- **Infrastructure Stabilization [螳御ｺ�**
    - **Global Mock Refactoring**: `MockJobQueue` 遲峨�蜷咲ｧｰ陦晉ｪ√ｒ隗｣豸医☆繧九◆繧√～GlobalMockJobQueue` / `GlobalMockLlm` 縺ｸ謾ｹ遘ｰ縺� `test_utils.rs` 縺ｫ髮�ｴ��
    - **UniversalJobQueue Restoration**: 繝輔ぅ繝ｼ繝ｫ繝会ｼ�karma_cache`, `slm_bridge`, `trajectory_store` 遲会ｼ峨�蠕ｩ蜈�→縲～TrajectoryStore` 縺ｮ髱� `Debug` 諤ｧ繧定��縺励◆謇句虚 `Debug` 繝医Ξ繧､繝亥ｮ溯｣�↓繧医ｊ縲√ン繝ｫ繝我ｸ肴紛蜷医ｒ隗｣豸医�
    - **LLM Provider Type Alignment**: `dynamic.rs` 縺ｫ縺翫￠繧玖ｨｭ螳壼叙蠕励ヵ繝ｭ繝ｼ繧貞姐譁ｰ縲ＡOption<String>` 縺九ｉ `String` 縺ｸ縺ｮ蝙区紛蜷域ｧ繧堤｢ｺ菫昴＠縲∝推遞ｮ繝励Ο繝舌う繝繝ｼ繧ｳ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ縺ｨ縺ｮ陦晉ｪ√ｒ菫ｮ豁｣縲�
    - **DB Reference Fix**: `rss_collector.rs` 縺ｧ縺ｮ `sqlx` 螳溯｡梧凾縺ｫ繝励�繝ｫ縺ｸ縺ｮ驕ｩ蛻�↑蜿ら� (`&*p`) 繧呈ｸ｡縺吶ｈ縺�↓菫ｮ豁｣縲�

### Added
- **Phase 1: Vectorless RAG (Hierarchical Knowledge Router - HKR) [螳御ｺ�**
    - **Knowledge Indexer**: Markdown 繧帝嚴螻､逧�↑ `TreeNode` 縺ｫ繝代�繧ｹ縺吶ｋ讖溯�繧貞ｮ溯｣�ゅす繝ｳ繝懊Μ繝�け繝ｪ繝ｳ繧ｯ謗帝勁縲√�繝ｭ繝ｳ繝励ヨ繧､繝ｳ繧ｸ繧ｧ繧ｯ繧ｷ繝ｧ繝ｳ蟇ｾ遲匁ｸ医∩縺ｮ繧ｵ繝九ち繧､繧ｺ讖溯�繧呈政霈峨�
    - **Hierarchical Router**: LLM 縺ｫ繧医ｋ螟壽ｮｵ髫弱ヤ繝ｪ繝ｼ謗｢邏｢繧ｨ繝ｳ繧ｸ繝ｳ繧貞ｮ溯｣�ゅそ繝槭ヵ繧ｩ縺ｫ繧医ｋ繝ｪ繧ｽ繝ｼ繧ｹ蛻ｶ蠕｡縺ｨ縲�1譎る俣 TTL / 繝峨く繝･繝｡繝ｳ繝医ワ繝�す繝･讀懆ｨｼ莉倥″縺ｮ `RouteCache` 繧堤ｵｱ蜷医�
    - **API Integration**: SSE 繧ｹ繝医Μ繝ｼ繝� (`stream.rs`) 縺ｫ縺翫＞縺ｦ縲∵蕗險捺､懃ｴ｢縺� OOD (Out of Distribution) 縺ｨ縺ｪ縺｣縺滄圀縺ｫ HKR 縺瑚�蜍戊ｵｷ蜍輔☆繧九ヵ繧ｩ繝ｼ繝ｫ繝舌ャ繧ｯ繝輔Ο繝ｼ繧貞ｮ溯｣��
    - **Security Validation**: HKR 縺ｮ讀懃ｴ｢邨先棡繧� `ConstitutionalValidator` 縺ｧ譛邨よ､懆ｨｼ縺励※縺九ｉ繝ｦ繝ｼ繧ｶ繝ｼ縺ｸ謠蝉ｾ帙☆繧九そ繧ｭ繝･繧｢縺ｪ繝代う繝励Λ繧､繝ｳ繧呈ｧ狗ｯ峨�
    - **TDD Verification**: 繧､繝ｳ繝�け繧ｵ繝ｼ縺ｨ繝ｫ繝ｼ繧ｿ繝ｼ縺ｫ蟇ｾ縺励∵ｭ｣蟶ｸ邉ｻ繝ｻ逡ｰ蟶ｸ邉ｻ繝ｻ繧ｻ繧ｭ繝･繝ｪ繝�ぅ繝ｻ繧ｭ繝｣繝�す繝･縺ｮ蜷�Ξ繧､繝､繝ｼ縺ｧ GREEN 繝代せ繧帝＃謌舌�

### Added
- **Phase 5: Gemini Interactions API 邨ｱ蜷亥渕逶､ [螳御ｺ�**
    - **InteractionsGeminiProvider**: Gemini Interactions API (REST) 縺ｸ縺ｮ繧ｹ繝��繝医ヵ繝ｫ謗･邯壹∵欠謨ｰ繝舌ャ繧ｯ繧ｪ繝募�隧ｦ陦後√♀繧医� Ollama 縺ｸ縺ｮ閾ｪ蜍輔ヵ繧ｧ繧､繝ｫ繧ｪ繝ｼ繝舌�讖溯�繧貞ｮ溯｣��
    - **Hybrid Context Management**: Gemini 蛛ｴ縺ｮ繧ｻ繝�す繝ｧ繝ｳ繧ｹ繝��繝� (`interaction_id`) 縺ｨ繝ｭ繝ｼ繧ｫ繝ｫ SQLite 縺ｮ螻･豁ｴ蜷梧悄縺ｮ隨ｬ1谿ｵ髫弱ｒ螳溯｣��
    - **Schema Expansion**: `TrajectoryStep` 縺翫ｈ縺ｳ `chat_memory_summaries` 縺ｫ `interaction_id` 繝輔ぅ繝ｼ繝ｫ繝峨ｒ霑ｽ蜉�縲４QLite/Postgres 荳｡譁ｹ縺ｧ謗ｨ隲悶Ο繧ｰ縺ｮ豌ｸ邯壼喧縺ｫ蟇ｾ蠢懊�
    - **Infrastructure Resilience**: 繧ｹ繧ｭ繝ｼ繝槫､画峩縺ｫ莨ｴ縺��繝｢繝�け螳溯｣� (`test_utils`, `soul_mutator`, `dream_state`, `immune_system`) 縺翫ｈ縺ｳ `napi-bridge` 縺ｮ繝医Ξ繧､繝磯←蜷域ｧ繧堤｢ｺ菫昴�
    - **Fix**: `20260326000003` 繝溘げ繝ｬ繝ｼ繧ｷ繝ｧ繝ｳ縺ｫ縺翫￠繧� `trajectory_steps` 繝��繝悶Ν縺ｮ繧ｫ繝ｩ繝�驥崎､�ｮ夂ｾｩ��easoning遲会ｼ峨ｒ蜑企勁縺励∫ｵｱ蜷医ユ繧ｹ繝医�繝代ル繝�け繧定ｧ｣豸医�
### Fixed
- **SQLite Migration Conflict**: `trajectory_steps` 繝��繝悶Ν縺ｸ縺ｮ驥崎､�＠縺溘き繝ｩ繝�霑ｽ蜉���reasoning` 遲会ｼ峨ｒ隗｣豸医＠縲∝�譛溷喧繝代ル繝�け繧剃ｿｮ豁｣縲�
- **Infrastructure Test Regression**: `test_sqlite_trajectory_store` 縺ｮ繝��繧ｿ荳肴紛蜷医ｒ菫ｮ豁｣縲�
- **Security G-21 (Vault Isolation)**: `BastionGuard` 縺ｫ縺翫＞縺ｦ縲�壼ｸｸ繧ｹ繧ｭ繝ｫ縺� Vault 鬆伜沺縺ｸ繧｢繧ｯ繧ｻ繧ｹ蜿ｯ閭ｽ縺�縺｣縺溯ф蠑ｱ諤ｧ繧剃ｿｮ豁｣縲ゅい繧ｯ繧ｻ繧ｹ繧偵す繧ｹ繝�Β蜀�Κ繝励Ο繧ｻ繧ｹ縺ｮ縺ｿ縺ｫ蛻ｶ髯舌�

### Security
- **Deep Scan (AST Phase)**: 繝励Ο繧ｸ繧ｧ繧ｯ繝亥�蝓溘� AST 險ｺ譁ｭ繧貞ｮ滓命縲�-19縲廨-23 縺ｮ蜈�ｶｳ繧堤｢ｺ隱阪♀繧医� G-21 縺ｮ隲也炊逧�ф蠑ｱ諤ｧ繧堤音螳壹�菫ｮ豁｣螳御ｺ��
- **Payload Limit Enforcement**: 逕ｻ蜒上ｄ髻ｳ螢ｰ縺ｮ繧｢繝��繝ｭ繝ｼ繝峨Ν繝ｼ繝医↓譛驕ｩ蛹悶＆繧後◆ Request Body 蛻ｶ髯撰ｼ�50MB/500MB�峨ｒ驕ｩ逕ｨ縲�
- **Structural Refactoring**: `LlmRequest` / `LlmResponse` 縺ｸ縺ｮ繝｡繧ｿ繝��繧ｿ繝ｻ謗ｨ隲悶Ο繧ｰ逕ｨ繝輔ぅ繝ｼ繝ｫ繝芽ｿｽ蜉�縺ｫ莨ｴ縺�す繧ｹ繝�Β蜈ｨ菴難ｼ�rchestrator, Proxy, Cache遲会ｼ峨�蛻晄悄蛹悶さ繝ｼ繝峨ｒ菫ｮ豁｣縲�
- **ADR-024 Phase 2: Autonomous Strategy & Trajectory Persistence [螳御ｺ�**
    - **Trajectory Expansion**: `TrajectoryStep` 縺ｫ `job_id` 縺翫ｈ縺ｳ `tool_name` 繝輔ぅ繝ｼ繝ｫ繝峨ｒ霑ｽ蜉�縲ゅち繧ｹ繧ｯ縺ｮ螳溯｡瑚ｻ瑚ｷ｡縺ｨ迚ｹ螳壹�繧ｸ繝ｧ繝悶�繝��繝ｫ縺ｮ邏蝉ｻ倥￠繧貞庄閭ｽ縺ｫ縲�
    - **JobQueue Persistence**: `JobQueue` 繝医Ξ繧､繝医↓ `store_trajectory_step` 縺翫ｈ縺ｳ `fetch_trajectory_steps` 繧定ｿｽ蜉�縲４QLite 螳溯｣� (`UniversalJobQueue`) 縺ｧ豌ｸ邯壼喧繧偵し繝昴�繝医�
    - **Sub-job Dispatching**: `TaskDispatcher` 縺ｫ縺翫＞縺ｦ縲～Goal` 繧ｸ繝ｧ繝悶ｒ蜿励￠蜿悶▲縺滄圀縺ｫ繝励Λ繝ｳ繝翫�縺檎函謌舌＠縺溘せ繝�ャ繝励ｒ蛟句挨縺ｮ繧ｵ繝悶ず繝ｧ繝悶→縺励※ `enqueue` 縺吶ｋ繝ｭ繧ｸ繝�け繧貞ｮ溯｣��
    - **Mock Infrastructure Recovery**: 螟画峩縺ｫ莨ｴ縺� `test_utils`, `soul_mutator`, `dream_state`, `immune_system` 蜀�� MockJobQueue 螳溯｣�ｒ蜈ｨ縺ｦ譛譁ｰ蛹悶�

- **Phase 44: Shadow Clone Job Control & Task History [螳御ｺ�**
    - **Job Cancellation API**: `POST /api/v1/jobs/:id/cancel` 繧貞ｮ溯｣�ょｮ溯｡御ｸｭ縺ｮ蠖ｱ蛻�ｺｫ繧ｿ繧ｹ繧ｯ繧� `CancellationToken` 繧剃ｻ九＠縺ｦ螳牙�縺ｫ蛛懈ｭ｢蜿ｯ閭ｽ縲�
    - **Task History & Logs API**: `GET /api/v1/jobs/:id/logs` 繧貞ｮ溯｣�ゅず繝ｧ繝悶�螳溯｡後せ繝��繧ｿ繧ｹ縲√お繝ｩ繝ｼ繝｡繝�そ繝ｼ繧ｸ縲√♀繧医�隧ｳ邏ｰ縺ｪ螳溯｡後Ο繧ｰ縺ｮ蜿門ｾ励↓蟇ｾ蠢懊�
    - **Deterministic Container Management**: `DockerConductor` 縺ｫ縺翫＞縺ｦ `aiome-job-{id}` 蠖｢蠑上�遒ｺ螳夂噪縺ｪ蜻ｽ蜷崎ｦ冗ｴ�ｒ謗｡逕ｨ縺励√く繝｣繝ｳ繧ｻ繝ｫ譎ゅ�遒ｺ螳溘↑繧ｳ繝ｳ繝�リ蛛懈ｭ｢��docker stop` / `docker rm`�峨ｒ菫晁ｨｼ縲�
    - **Robust Error Handling**: 繧ｸ繝ｧ繝匁悴讀懷�譎ゅ� 404 繝ｬ繝昴Φ繧ｹ��ArtifactNotFound` 縺ｸ縺ｮ繝槭ャ繝斐Φ繧ｰ�峨ｒ繧､繝ｳ繝輔Λ螻､縺九ｉ API 螻､縺ｾ縺ｧ荳雋ｫ縺励※螳溯｣��
    - **TaskDispatcher Evolution**: `active_jobs` 繝槭ャ繝励↓繧医ｋ螳溯｡御ｸｭ繧ｿ繧ｹ繧ｯ縺ｮ蜍慕噪霑ｽ霍｡縺ｨ縲～JobQueue` 縺ｸ縺ｮ繧ｭ繝｣繝ｳ繧ｻ繝ｫ繧ｷ繧ｰ繝翫Ν莨晄成繧堤ｵｱ蜷医�

### Added
- **Phase 43: Shadow Clone ﾃ� Cmux Integration [螳御ｺ�**
    - **DockerConductor Implementation**: 蠖ｱ蛻�ｺｫ繧ｿ繧ｹ繧ｯ縺ｮ螳牙�螳溯｡後ｒ蜿ｸ繧� Conductor 繧貞ｮ溯｣��5螻､縺ｮ螟壼ｱ､髦ｲ蠕｡�医そ繝槭ヵ繧ｩ縺ｫ繧医ｋ Fork Bomb 髦ｲ蠕｡縲，ommerceEngine 縺ｫ繧医ｋ閾ｪ蠕玖ｪｲ驥代。astionGuard 縺ｫ繧医ｋ蜴ｳ譬ｼ縺ｪ繧ｵ繝ｳ繝峨�繝�け繧ｹ縲√ち繧､繝�繧｢繧ｦ繝育屮隕悶∝�蜉帶ｵ�喧�峨ｒ邨ｱ蜷医�
    - **Async Shadow Clone Dispatch**: `agent.rs` 縺ｮ蜷梧悄螳溯｡後ｒ蟒�ｭ｢縺励～JobQueue` 繧堤畑縺�◆螳悟�髱槫酔譛溘ョ繧｣繧ｹ繝代ャ繝√∈遘ｻ陦後�LM 縺ｯ蛻�ｺｫ縺ｮ襍ｷ蜍輔ｒ蠕�◆縺壹↓蜊ｳ蠎ｧ縺ｫ蠢懃ｭ泌庄閭ｽ縺ｫ縲�
    - **SSE Progress Streaming**: 蜀�Κ縺ｮ `TaskEvent` 繧� `CoreEvent` 縺ｫ繝悶Μ繝�ず縺励ヾSE 邨檎罰縺ｧ繝輔Ο繝ｳ繝医お繝ｳ繝会ｼ�mux�峨∈蛻�ｺｫ縺ｮ騾ｲ謐暦ｼ�rogress/Completed/Failed�峨ｒ繝ｪ繧｢繝ｫ繧ｿ繧､繝�驟堺ｿ｡縺吶ｋ莉慕ｵ�∩繧呈ｧ狗ｯ峨�
    - **Graceful Shutdown**: API 繧ｵ繝ｼ繝舌�縺ｮ邨ゆｺ�す繧ｰ繝翫Ν縺ｫ `TaskDispatcher` 縺ｮ蛛懈ｭ｢蜃ｦ逅�ｒ騾｣蜍輔＆縺帙∽ｻ墓寺縺九ｊ荳ｭ縺ｮ髱槫酔譛溘ち繧ｹ繧ｯ縺ｮ螳牙�縺ｪ邨ゆｺ�ｒ螳溽樟縲�
- **Phase 42: Multi-Agent Orchestration Evolution [螳御ｺ�**
    - **TaskEvent & TaskConductor Traits**: Defined strict boundaries for background execution and observability tracking (`Spawned`, `Progress`, `Completed`, `Failed`).
    - **TaskDispatcher**: Implemented an event-driven `tokio::sync::broadcast` stream to asynchronously monitor agent progress without blocking, achieving a pull-based UI architecture.
    - **OssIntegrationOrchestrator Refactoring**: Adapted the monolithic integration flow into an observable `TaskConductor`, enabling granular job tracking.
- **Phase 37a: Stripe Subscription & Whisper Integration [螳御ｺ�**
    - **Stripe Subscriptions API**: `StripeCommerceEngine` 縺ｫ `create_subscription` 縺翫ｈ縺ｳ `cancel_subscription` 繧貞ｮ溯｣�６UID 繝吶�繧ｹ縺ｮ鬘ｧ螳｢邂｡逅�→繝｡繧ｿ繝��繧ｿ騾｣謳ｺ縲√♀繧医�繝�せ繝育腸蠅�畑縺ｮ `sk_test_mock` 繝舌う繝代せ繝｢繝ｼ繝峨ｒ蟆主�縲�
    - **Whisper Inner Monologue**: `SoulPipeline` 縺ｮ L2.5 螻､縺ｨ縺励※ `WhisperMiddleware` 繧定ｿｽ蜉�縲らｵ碁ｨ薙�萓｡謨ｰ (Valence) 縺碁明蛟､繧定ｶ�∴縺滄圀縲、I縺後悟�縺ｪ繧句｣ｰ (Whisper)縲阪→縺�≧迢ｬ閾ｪ縺ｮ繝ｭ繧ｰ繧堤函謌舌＠閾ｪ蟾ｱ逵∝ｯ溘ｒ險倬鹸縲�
    - **Pipeline Architecture Upgrade**: 邨碁ｨ薙�繝舌ャ繝輔ぃ縺ｸ縺ｮ闢�ｩ� (`push_experience`) 繧貞�繝溘ラ繝ｫ繧ｦ繧ｧ繧｢繝√ぉ繝ｼ繝ｳ縺ｮ譛邨よｮｵ縺ｫ繧ｷ繝輔ヨ縺励仝hisper 繧� MetaThoughts 縺ｪ縺ｩ縺ｮ莉伜刈諠��ｱ縺悟ｮ悟�縺ｫ螻･豁ｴ縺ｸ谿九ｋ繧医≧謾ｹ蝟��
- **License Compliance Hardening**
    - `NOTICE` file (Apache 2.0 ﾂｧ4(d) compliance)
    - `THIRD_PARTY_NOTICES.md` with attribution for AutoResearchClaw, MetaClaw, Inochi2D, and Trojan's Whisper research
    - `scripts/license_check.py` 窶� automated 11-point license compliance test suite
    - Copyright headers added to 49 `.rs` files that were missing them
    - `license` field added to 3 `Cargo.toml` files (`aiome-migrate`, `avatar-engine`, `soul`)
- **Phase 36.5: gVisor Sandbox & CSAM Pipeline [螳御ｺ�**
    - **SandboxProfile API**: Added `SandboxProfile` enum and updated `BastionGuard::safe_exec_with_profile` for fine-grained isolation control (WasmRun, WasmBuild, PythonForge).
    - **CSAM Binary Verification**: Integrated `ProportionsChecker` directly into the Avatar upload binary parsing to prevent illegal asset distribution.
    - **AgentHook Lifecycle**: `UserLearner` integrated via `HookManager`, allowing the agent to self-reflect and learn via `on_post_execute` after each session縲�
    - **Stripe Commerce Update**: Extended `CommerceEngine` with `SubscriptionStatus`, `create_subscription`, `cancel_subscription` in preparation for Phase 37.
    - **Inner Monologue**: Developed `WhisperMiddleware` within `SoulPipeline` (L2.5) to capture introspective reflections based on outcome valence.
    - **Deep Scan Validation**: Completed AST Matrix structural validation across infrastructure to ensure absolute compliance with Project NURTURE requirements.
- **Security**: G-21 Vault isolation hardening (Restricted /vault access to internal processes).
- **Core**: G-14 SoulSnapshot LoRA metadata synchronization (Cached LoRA config in snapshot).
- **Audit**: Conducted comprehensive Deep Scan (AST Matrix) and verified all Project NURTURE Gates.
- **Phase 35: PostgreSQL 遘ｻ陦� & 譛邨よ､懆ｨｼ [螳御ｺ�**
    - **Dual DB Testing Infrastructure**: Ensured all 86 integration tests and CI scripts run equivalently on both SQLite and PostgreSQL backends via `TEST_POSTGRES_URL` configuration (`docker-compose.test.yml`).
    - **PostgreSQL Audit Trigger (Phase 35)**: Replaced application-layer ledger tracking with robust PL/pgSQL database triggers for automated `audit_ledger_global` lineage and hashing縲�
- **Phase 32: DeerFlow Architectural Pattern Integration [螳御ｺ�**
    - **Middleware Chain**: `SoulPipeline` 繧� Reactive, Deliberative, Meta-cognitive 縺ｮ 3 螻､繝溘ラ繝ｫ繧ｦ繧ｧ繧｢讒矩�縺ｫ蛻ｷ譁ｰ縲Ａasync-trait` 縺ｫ繧医ｋ諡｡蠑ｵ諤ｧ縺ｨ繧ｹ繝ｬ繝�ラ螳牙�諤ｧ繧剃ｸ｡遶九�
    - **Progressive Skill Loading**: `WasmSkillManager` 縺ｫ `mtime` 繝吶�繧ｹ縺ｮ繧ｭ繝｣繝�す繝･辟｡蜉ｹ蛹悶Ο繧ｸ繝�け繧貞ｰ主�縲８ASM 繝輔ぃ繧､繝ｫ縺ｮ譖ｴ譁ｰ繧定�蜍墓､懃衍縺励∝ｮ溯｡梧凾縺ｫ譛譁ｰ蛹悶�
    - **Virtual Path System**: `PathSandbox` 縺ｫ隲也炊繝代せ繝槭ャ繝斐Φ繧ｰ讖溯�繧堤ｵｱ蜷医Ａ/mnt/workspace` 縺ｪ縺ｩ縺ｮ莉ｮ諠ｳ繝代せ繧堤黄逅�ョ繧｣繝ｬ繧ｯ繝医Μ縺ｫ螳牙�縺ｫ繝舌う繝ｳ繝峨�
    - **Fact Extraction**: `MemoryCrystallizer` 縺ｫ `FactCategory` (Preference, Knowledge, Context, Behavior, Goal) 縺ｫ繧医ｋ莠句ｮ滓歓蜃ｺ繝ｻ蛻�｡樊ｩ溯�繧貞ｮ溯｣��
    - **Test Utility**: `VerifiedSkill::new_for_test` 繧定ｿｽ蜉�縺励∫ｵｱ蜷医ユ繧ｹ繝医↓縺翫￠繧� WASM 繧ｹ繧ｭ繝ｫ縺ｮ繝｢繝�け繝ｻ讀懆ｨｼ繧貞ｮｹ譏灘喧縲�

### Changed
- **Phase 37a: Security & Infra Refactoring**:
    - `BastionGuard::safe_exec` 縺翫ｈ縺ｳ `safe_exec_with_profile` 繧貞ｮ悟�蜷梧悄縺九ｉ髱槫酔譛� (`async`) 螳溯｡後∈遘ｻ陦後ゅ％繧後↓繧医ｊ繧ｹ繝ｬ繝�ラ繝励�繝ｫ縺ｮ譫ｯ貂�ｒ髦ｲ豁｢縲�
    - `api-server` 縺ｮ `AppState` 縺ｫ `soul_pipeline` 繧､繝ｳ繧ｹ繧ｿ繝ｳ繧ｹ繧堤ｵｱ蜷医�
- **Code Quality & Refactoring**:
    - `WritingContext` 縺ｫ `#[derive(Default)]` 縺ｨ `#[default]` 螻樊ｧ繧定ｿｽ蜉�縺励√�繧､繝ｩ繝ｼ繝励Ξ繝ｼ繝医ｒ蜑頑ｸ帙�
    - `MlockedVec` 縺ｮ `Drop` 螳溯｣�↓縺翫￠繧句ｮ牙�縺ｪ `munlock` 蜻ｼ縺ｳ蜃ｺ縺励�譚｡莉ｶ蛻､螳壹ｒ譛驕ｩ蛹悶�
    - `SqliteVaultBackend` 縺ｮ繝槭せ繧ｿ繝ｼ繧ｭ繝ｼ蜿門ｾ励Ο繧ｸ繝�け縺ｫ縺翫￠繧矩未謨ｰ繝昴う繝ｳ繧ｿ縺ｮ逶ｴ謗･貂｡縺励↓繧医ｋ邁｡逡･蛹悶�
    - `UniversalJobQueue` 蜀�� SQLite 謨ｰ蛟､繧ｭ繝｣繧ｹ繝茨ｼ�i32`�峨�謨ｴ蜷域ｧ繧剃ｿｮ豁｣縲�

- **Phase 31: 菫｡鬆ｼ諤ｧ蜷台ｸ� & LLM TDD 螳溯｣� [螳御ｺ�**

### 2026-03-23

### Added
- **Phase 28: 蝓ｺ逶､蠑ｷ蛹� (ADR-019 Phase B / L1 蠑ｷ蛹�) [螳御ｺ�**
    - `SqliteVaultBackend` 縺ｸ縺ｮ LRU 繧ｭ繝｣繝�す繝･ (1000 keys) 邨ｱ蜷医ＡMlockedVec` 縺ｫ繧医ｋ繝｡繝｢繝ｪ菫晁ｭｷ繧堤ｶｭ謖√�
    - `lru` 繧ｯ繝ｬ繧､繝医�蟆主��医Ρ繝ｼ繧ｯ繧ｹ繝壹�繧ｹ萓晏ｭ倬未菫ゑｼ峨�
- `VaultBackend` trait (ADR-019 Phase A)
- `SqliteVaultBackend` based on `MlockedVec`

### Changed
- **L1 蠑ｷ蛹� (Code Quality)**:
    - `api-server` 縺ｮ `commerce_engine` 蛻晄悄蛹悶↓縺翫￠繧� `.unwrap()` 繧� `.expect()` 縺ｫ鄂ｮ謠帙＠縲√ョ繝舌ャ繧ｰ諤ｧ繧貞髄荳翫�
    - `TcpListener::bind` 螟ｱ謨玲凾縺ｮ繧ｨ繝ｩ繝ｼ繝｡繝�そ繝ｼ繧ｸ繧定ｩｳ邏ｰ蛹悶�
    - `cfg!(debug_assertions)` 繧� `#[cfg(debug_assertions)]` 縺ｫ邨ｱ荳縺励√さ繝ｳ繝代う繝ｫ譎ょ愛螳壹ｒ譛驕ｩ蛹� (guardrails, gift_engine)縲�
    - `infrastructure` 繧ｯ繝ｬ繧､繝亥�縺ｮ繝峨く繝･繝｡繝ｳ繝郁ｭｦ蜻� (FederationOps, MockJobQueue遲�) 繧偵☆縺ｹ縺ｦ隗｣豸医＠縲∬ｭｦ蜻翫ぞ繝ｭ繧帝＃謌舌�
- **Phase 28.5: `std::env::set_var` 閼ｱ蜊ｴ [螳御ｺ�**:
    - `SqliteVaultBackend::new_with_master_key()` 繧ｳ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ繧定ｿｽ蜉�縲ゅユ繧ｹ繝育腸蠅�〒迺ｰ蠅�､画焚謫堺ｽ懊↑縺励↓ Master Key 繧呈ｳｨ蜈･蜿ｯ閭ｽ縺ｫ縲�
    - `AbyssVoiceVault` 繝�せ繝医°繧� `std::env::set_var` 繧貞ｮ悟�謗帝勁縲ゅせ繝ｬ繝�ラ繧ｻ繝ｼ繝輔°縺､荳ｦ蛻励ユ繧ｹ繝亥ｮ牙�縺ｪ險ｭ險医↓遘ｻ陦後�
- Refactored `AbyssVoiceVault` to use `SqliteVaultBackend` internally
- Updated `SECURITY_DESIGN.md` ﾂｧ6.5 with vault abstraction specs.

---

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


### 2026-03-22

### Added
- **Phase 26: AI Writing Enhancement**:
    - Added `HumanizerFilter` middleware to detect and remove common AI-isms and robotic phrasing (e.g., excessive hedging, chatbot artifacts).
    - Added `WritingContext` to dynamically apply different writing rules based on the output destination (Chat, Manifesto, TechLog, etc.).
- **Unified Response Purger (G-21)**:
    - Implemented `purge_entities` in `aiome-core` for robust, multi-step sanitization of external inputs.
    - Centralized regex patterns and HTML/entity decoding logic to prevent XSS and script injection.
    - Applied to `TrendSonar` and RSS collectors for unified safety.
- **AI-Driven Code Review (G-22)**:
    - Integrated LLM-based security auditing into `Cleanroom` for skill imports.
    - Performs pre-forge analysis of skill source code to detect "Vampire Attacks" or malicious network calls.
- **Periodic Federated Metrics (G-23)**:
    - Added a background task to the `api-server` to push node metrics to the Samsara Hub every hour.
    - Implemented `do_push_federated_metrics` in `FederationOps` and fixed its `SqliteJobQueue` implementation.
- **AgentSense MVP (AS-1)**:
    - Implemented `GET /api/v1/treasure` to generate and fetch personalized AI "Sense" (recommendations).
    - Implemented `POST /api/v1/treasure/feedback` to record agent interactions and reward resonance.
    - Added `AffiliateAdapter` for modular bid/recommendation fetching (currently mocked).
    - **Soul-Linked Intent Generation**: Enhanced `IntentGenerator` to derive agent "wishes" from their `AgentSoul` attachment style (Anxious/Avoidant/Secure), stored in the `SoulStore` (Gap G-26).
    - **TreasureBox UI**: Developed a premium glassmorphism React component and `useTreasure` hook in the Management Console to display and interact with recommendations (Gap G-25).
    - Added comprehensive integration test `test_treasure_get_recommendations` covering the full feedback-reward loop.
- **Audit API: Quarantine Ledger**:
    - Implemented `GET /api/v1/audit/quarantine` to allow the system agent to list and audit quarantined assets.
    - Extended `QuarantineStore` trait and `SqliteQuarantineStore` with `list_assets` capability.

### Fixed
- **SSE Connection Stability**: Implemented exponential backoff and maximum retry limits (5) in frontend `useSystemVitality` hook to prevent browser connection slot exhaustion and infinite re-auth loops.
- **Audit Logging**: Fixed `NEW.id` reference error in SQLite triggers for `system_state` and other tables with non-standard primary keys. Added `DROP TRIGGER IF EXISTS` to migration logic for reliable updates.
- **Path Sandbox**: Fixed `validate_path` in `PathSandbox` by using canonical paths for comparison, preventing incorrect "Out of sandbox" errors on valid relative paths.
- **Autonomous Demo**: Added automatic database cleanup of gig-related tables before starting a new demo cycle to ensure state consistency.
- **Autonomous Demo 窶� SQLite Lock (ADR-014)**: Resolved `database is locked (SQLITE_BUSY 517)` errors that halted the demo at Steps 5窶�7. Root cause: `gig_engine` transactions held exclusive SQLite WRITE locks while audit triggers cascaded additional writes, compounded by SSE multi-tab connection pool exhaustion (`max_connections=10`). Fix: rewrote `autonomous_demo.rs` to use individual SQL queries (no transactions), temporarily disable audit triggers during demo execution, and yield connections between writes. See `docs/decisions/014-sqlite-pool-exhaustion-demo-strategy.md`.
- **Authentication**: Updated `MockAuthManager` to allow `mock_token` for seamless local testing and E2E verification.
- **OAuth 2.1 Mock Endpoints**:
    - Added stub handlers for `/api/v1/auth/authorize` and `/api/v1/auth/token` in `api-server`.
    - Integrated with `utoipa` for OpenAPI documentation of the authentication flow.
- **Autonomous AI Economy Demo (Phase 25)**:
    - Implemented `AutonomousDemo` orchestrator to simulate a 60-second autonomous agent lifecycle.
    - Added `POST /api/v1/demo/start` endpoint to trigger the autonomous cycle in the background.
    - Integrated `IntentGenerator`, `GigEngine`, `SwarmOps`, and `Karma` for a complete "Earn & Evolve" demonstration.
    - Implemented real-time event broadcasting via `PluginEvent` for frontend visualization.
    - Added TDD-based integration tests to verify the demo API and lifecycle.
- **Autonomous Demo UI (Phase 25.5)**:
    - Added `DemoView.tsx` to the Management Console for real-time visualization of the autonomous cycle.
    - Implemented an animated execution timeline tracking the 8-step process.
    - Integrated with `useSystemVitality` SSE hook to display live agent stats and karma progression.
    - Added TDD-based E2E Playwright tests to verify rendering and UI interactions.
- **Enhanced Mock Authentication**:
    - Updated `MockAuthManager` to support custom `agent_id` via mock tokens (`mock_valid_token_<sub>:<agent_id>`).
    - Standardized separator to `:` to prevent collisions with usernames containing underscores.

### Fixed
- **BastionGuard gVisor (runsc) Integration**:
    - Implemented dynamic detection of `runsc` bin in Linux environments.
    - Prioritizes gVisor for secure user-space kernel execution while maintaining a gracefull host fallback for development.
- **Integration Test Stability**:
    - Resolved `test_gig_lifecycle` failure by ensuring compatible `artifact_path` for `PathSandbox` validation.
    - Restored functionality to 32/33 integration tests after auth manager refactoring.
- **Cleanroom Security Auditor**:
    - Resolved `LlmProvider::complete` argument mismatch and response parsing logic to ensure robust AI-driven code auditing.

### Fixed
- **Trend Sonar Refactoring (Multi-Source Support)**:
    - Introduced `TrendAdapter` trait to decouple trend fetching from the core `ExternalTrendSonar` logic.
    - Implemented `WebSearchAdapter` for real-time trend gathering via external Search APIs.
    - Integrated `RssCollector` as a `TrendAdapter`, enabling it to serve as both a `TrendSource` and a flexible adapter.
    - Updated `ExternalTrendSonar` to manage a collection of adapters, aggregating results from multiple sources (Web Search, RSS, etc.).
    - Added `sanitize_snippet` utility to clean raw HTML/URL data from external search results.
    - Restructured `main.rs` to pre-initialize a shared `TrendSonar` instance with multiple adapters, improving performance and consistency across the Background Worker and Dream State.
    - Added comprehensive unit tests for multi-source aggregation and adapter logic.
- **RSS Sanitization (G-Security)**:
    - Implemented `sanitize_snippet` to clean RSS feed content and prevent HTML/script injection.
    - Applied sanitization to all incoming RSS titles in `RssCollector`.
- **Gig Engine Security (G-22)**:
    - Integrated `PathSandbox` into `SqliteGigEngine::deliver` to enforce strict path validation.
    - Prevents path traversal attacks by ensuring delivery artifacts remain within the designated `ARTIFACT_ROOT`.
- **API Security Hardening (G-Log)**:
    - Restricted access to sensitive endpoints (`/api/v1/logs`, `/api/v1/audit/ledger`, `/api/v1/audit/diagnostics`) to the system agent only.
    - Verified security with automated access control checks in the routing layer.

### Added
- **Phase 20: AI Gig Engine (The Immutable Gateway)**:
    - **SqliteGigEngine Implementation**: Developed a robust, TDD-driven `GigEngine` implementation using SQLite.
    - **`publish_intent`**: Enabled AI agents to broadcast work requests with automated `AcceptanceCriteria` and JSON serialization.
    - **`submit_bid`**: Implemented bidding logic for AI agents to compete for intents, including price and duration estimations.
    - **`accept_bid`**: Implemented atomic transaction-based bid acceptance. Automatically creates and locks escrows in the `CommerceEngine` to secure payments.
    - **`deliver`**: Enabled secure delivery of artifacts with metadata and artifact path recording. Enforces state transitions from 'Accepted' to 'Delivered'. Added `PathSandbox` validation to prevent path traversal attacks (G-22).
    - **`verify_and_settle`**: Implemented the core settlement logic. Performs automated verification against `AcceptanceCriteria`, updates order status to 'Completed' or 'Rejected', and executes escrow release or refund accordingly.
    - **Persistence & Logging**: Added full database support with tables for `gig_intents`, `gig_bids`, `escrows`, `gig_deliveries`, and `verification_logs`.
    - **TDD Test Suite**: Added 5 comprehensive integration tests covering the entire gig lifecycle, ensuring 100% path coverage for core engine operations.
- **Phase 20 Enhancement: Federated Metrics (G-23)**:
    - Added `metrics` field to `FederationPushRequest` to support transmission of node-level statistics to Samsara Hub.
    - Implemented `fetch_federated_metrics` in `SqliteJobQueue` to aggregate level, XP, job completion rates, and karma counts.
    - Updated Samsara Hub to persist received metrics in the `federated_metrics` table for global analytics.
- **Trend Oracle (LLM Evaluation)**:
    - Integrated an optional `LlmProvider` into `ExternalTrendSonar` for intelligent trend assessment.
    - Implemented LLM-based scoring and filtering to prioritize high-value trends for autonomous agents.

- **Phase 17 Enhancement: Gaps G-1 & G-2 Remediation**:
    - **Gap G-1: Circuit Breaker Observability**: Added `llm_circuit_breaker` status to the `/api/health` and `/health` endpoints. Modified the `CircuitBreaker` struct to include a `get_status` method, allowing proactive monitoring of LLM failover states.
    - **Gap G-2: Per-Agent Rate Limiting**: Implemented a per-agent rate limiter using the `governor` crate. Integrated this into the authentication middleware to protect core API endpoints from individual agent abuse, with a default limit of 60 requests per minute.
- **Phase 17: ArrowCanaria Fallback & Resilience**:
    - **FallbackRouter Implementation**: Implemented a robust `FallbackRouter` in `libs/infrastructure` that wraps a primary LLM and automatically switches to a fallback (e.g., Gemini) if the primary fails.
    - **Circuit Breaker Integration**: Integrated the Circuit Breaker pattern into the LLM routing logic, enabling automatic failover and preventing cascading failures when primary providers go offline.
    - **AppState Failover**: Updated the `api-server` `AppState` to use the `FallbackRouter` for all core LLM operations, providing a seamless transition between local and cloud models.
    - **Failover Integration Tests**: Added comprehensive integration tests in `api_integration_tests.rs` to verify that the system correctly routes requests to the fallback provider during simulated primary failures.
- **Phase 16: EKYC Protection & Revenue Splitter**:
    - **EKYC Enforcement**: Added hard integration of eKYC verification to the `send_gift` and `execute_purchase` endpoints. Unverified users will be blocked with a `403 Forbidden` response to enforce strict economic compliance.
    - **Commerce Revenue Splitter**: Implemented the `RevenueSplitter` module triggered by Stripe `checkout.session.completed` webhooks. It automatically calculates an 80/20 split between creators and the platform, inserting the split logic securely within the license grant database transaction.
    - **Zeroize Security Hardening**: Addressed in-memory secret persistence vulnerabilities by immediately zeroizing and removing `STRIPE_API_KEY`, `JWT_PRIVATE_KEY_B64`, and `SEARCH_API_KEY` from the environment immediately after load during application startup.
    - **Audit Ledger Visibility**: Ensured `revenue_splits` has been added to the automated database `audit_ledger_global` triggers to maintain a comprehensive changelog.
- **Phase 14: eKYC Persistence & Inochi2D Physics Sync**:
    - **EKYC Session Store**: Implemented `EkycSessionStore` using SQLite to persist Stripe verification session IDs, ensuring continuity across application restarts.
    - **Stripe API Hardening**: Updated `EkycEngine` to use `client_reference_id` for accurate filtering and implemented technical timeouts (30s) for better resilience.
    - **Inochi2D Physics Sync**: Added `physics_override` to the `avatar_expression` SSE stream. Implemented a 1.5x "Resonance Boost" logic that amplifies physics animations when resonance level exceeds 80.
    - **Secure Mascot Upload**: Enforced the `jwt_auth_middleware` and `PathSandbox` jail on the Inochi2D upload route, preventing unauthorized access and path traversal attacks (Expert Review v3).
    - **Stripe Fail-safe**: Introduced a mandatory check for `STRIPE_API_KEY` in release builds, causing the server to exit if missing, preventing insecure mock-state deployments.
- **Phase 13a: Stripe EKYC Session API Implementation**:
    - **StripeEkycEngine**: Upgraded `StripeEkycEngine` from mock behavior to real `reqwest`-based implementation, calling the Stripe Identity Verification Sessions API directly.
    - **Session API**: Implemented `POST /api/v1/ekyc/session` to initiate verification sessions and updated `GET /api/v1/ekyc/status` to handle real status tracking for user-agent verification.
    - **Refactor**: Modified `main.rs` to inject the shared `reqwest::Client` into `StripeEkycEngine`, ensuring consistent connection pool management.
- **Phase 13b: Inochi2D (2D Avatar) Pipeline Integration**:
    - **Inochi2dLoader**: Developed `Inochi2dLoader` in `libs/avatar-engine` providing magic byte validation (`INX\x02`) and metadata extraction for versioned Inochi2D models.
    - **Physics Simulator**: Implemented `PhysicsSimulator` using spring-damping algorithms to calculate real-time secondary animations for 2D avatars.
    - **Mascot Upload pipeline**: Exposed `POST /api/v1/avatar/inochi2d/upload` with a dedicated 50MB allowance in `router.rs`, enabling registration of `.inx` assets.
- **Phase 10.2 Security Hardening (Expert Review Integrated)**:
        - **Persistent Key Vault**: Implemented `vault_keys` table for persistent storage of voice asset keys, protected by a 256-bit Master Key (`VAULT_MASTER_KEY`).
        - **AES-256-GCM Nonce Management**: Enforced random 12-byte nonce generation for every encryption operation in `crypto.rs` to prevent ciphertext reuse attacks.
        - **Authorization Bypass Fix**: Replaced the vulnerable `LIKE` operator with `json_extract` in `RegistryManager::check_ownership` for precise atomic matching of `agent_id` and `asset_id`.
        - **Memory Protection**: Reduced the voice upload body limit from 500MB to 100MB to mitigate OOM-based DoS/OOMKill attack chains.
        - **Mutex Poison Recovery**: Added `unwrap_or_else` recovery for `AbyssVoiceVault` mutexes to prevent permanent vault locks on thread panic.
        - **Audit & Compliance**: Added audit logging for all decryption key access attempts and removed redundant sync-I/O `exists()` checks in async upload handlers.
- **Phase 11.0 (Voice DRM Refinement)**:
    - **Dual-Read Ownership & Migration**: Implemented `licenses` table priority reading with legacy `stripe_webhook_events` fallback in `RegistryManager::check_ownership`. Developed a standalone seamless data migration script (`migrate_licenses.rs`).
    - **Audio CSAM Detection**: Developed multi-threaded, robust CSAM detection for audio uploads via `AudioHasher` employing `tokio::task::spawn_blocking` and timeout defenses to prevent CPU exhaustion.
    - **LipSync Responsibility Separation**: Extracted `get_lipsync_frames` from `VoiceKeyVault` into a standalone `LipSyncProvider` trait moved to `avatar-engine`, drastically improving Interface Segregation.
    - **Vault Key Caching**: Refactored `AbyssVoiceVault` internal logic to feature lazy-initialized caching for `VAULT_MASTER_KEY` via `OnceCell`, shielding E2E test suites from environment variable absence panics.
    - **Voice E2E Roundtrip**: Introduced end-to-end integration tests encapsulating the complete lifecycle of a voice upload, spanning CSAM triage, registry ownership logic, AES-GCM encrypted persistence, and authorized Vault decryption.
- **Phase 10.1a (XTTS Core Integration)**:
    - **XTTS Synthesis**: Implemented `ExpressionEngine::synthesize_audio_xtts` in `aiome-core`, enabling integration with local XTTS v2 servers for high-quality, personalized voice synthesis.
    - **API Provider Selection**: Updated the `/api/expression/generate` endpoint to support switching between OpenAI (tts-1) and localized XTTS providers via system settings.
    - **Legal Guardrails**: Established `voice_upload_terms.md` and a TTS provider comparison matrix to manage copyright and licensing risks associated with creator-first audio assets.
- **Phase 10.1b (LoRA Metadata & Soul Persistence)**:
    - **LoraEngine**: Introduced `LoraEngine` in `libs/core` to manage LoRA model metadata (hashes, base models, file paths) for consistent model identification.
    - **Soul Identification**: Extended `AgentSoul` with a `lora_hash` field, ensuring that the AI's internal identity and hash-chains are tied to the specific fine-tuned model version.
    - **Persistence Layer**: Updated `SqliteSoulStore` and database migrations to support the `lora_hash` column, enabling seamless recovery of model settings across AI rebirth cycles.
- **Phase 8.8 (Audit & Immunity Ledger UI)**:
    - **Audit UI**: Implemented `DiagnosticsHistory` component in the management console, providing a transparent view of agent self-repair logs and system-wide hash-chained change logs.
    - **User Experience**: Integrated the Audit tab into the main navigation with lazy loading and optimized "Load More" pagination to prevent UI bloat from large log sets.
    - **AST Visibility**: Fixed a regex bug in the `nurture_auditor.py` script that was skipping 10+ React components (e.g., `ArtifactVault`), restoring full visibility to the AST structure matrix.
- **Phase 8.7 (Post-Scan Remediation & Synergy)**:
    - **Type Synchronization**: Added `DiagnosisResponse`, `TrendsResponse`, and `AuditLedgerResponse` to the OpenAPI schema in `api.rs`, enabling full TypeScript type safety across the stack.
    - **Artifact Lineage Visualizer**: Expanded `GraphView.tsx` to integrate Artifact data (`/api/artifacts`) alongside Karma nodes, visualizing the creative lineage of the AI through purple diamond nodes and "materialized" edges.
    - **Load Test Readiness**: Added a dedicated load testing task to the Phase 9.0 roadmap to verify the impact of the new 2MB request body limit on high-traffic routes.
- **Phase 8.6 (Deep Scan Remediation & Security Hardening)**:
    - **DDoS Protection**: Implemented a global request body limit of 2MB in `api-server` using `RequestBodyLimitLayer` to mitigate resource exhaustion attacks.
    - **Route Specific Bypass**: Added a 50MB body limit bypass specifically for the `/upload` (avatar) route to maintain essential high-payload functionality while securing the rest of the API.
    - **API Registry Expansion**: Formally exposed the Diagnostics API (`/api/v1/audit/diagnostics`) and implemented a Trends API skeleton (`/api/v1/trends`) to fulfill Project NURTURE requirements.
    - **AST Scripting**: Created `scripts/nurture_auditor.py` for automated AST-based structural analysis, enabling deep codebase audits without hitting LLM context limits.
- **Phase 8.2 (OAuth 2.1 & JWT Authentication)**:
    - **JWT AuthManager**: Implemented `JwtAuthManager` and `AuthManager` trait to replace legacy shared secrets with stateless, secure Ed25519 JWT tokens.
    - **Hybrid Middleware**: Added `auth_middleware` in `samsara-hub` and `key-proxy` supporting both Bearer JWT and legacy secrets for backward compatibility.
    - **PII Protection**: Implemented SHA-256 hashing for user identifiers (`sub`) in JWT validation logs to prevent PII exposure.
    - **Production Security**: Hardened `key-proxy` to require `JWT_PRIVATE_KEY_B64` in production, eliminating the risk of accidental MockAuthManager usage.
- **Phase 8.3 (Type-Driven Security)**:
    - **Type-Level Enforcement**: Added `_auth: Authenticated` extractor to all (45) public asynchronous route handlers in `api-server` (agent, biome, karma, artifacts, skill, streams, expression, general), ensuring that unauthenticated routes cannot compile.
    - **Performance Optimization**: Cached `system_agent_id` into `AppState` during application startup, eliminating redundant database queries inside the `Authenticated` extractor for hot-path APIs.
    - **CI Defense-in-Depth**: Introduced the `missing-auth-extractor` rule to `.github/anti-patterns.yml` and integrated `CC-6` (Auth Extractor Enforcement Ratio) into `scripts/deep-scan.sh` to prevent regressions in security guardrails.
- **Phase 8.2 (OAuth 2.1 Foundation)**:
    - **JWT Custom Claims**: Added `AiomeCustomClaims` in `libs/shared/auth` to define the standardized token structure (including `sub`, `ekyc_verified`, `roles`).
    - **AuthManager Interface**: Introduced `AuthManager` trait and `MockAuthManager` in `libs/infrastructure/auth` to handle stateless token validation.
    - **API Middlewares**: Implemented `jwt_auth_middleware` in `api-server` to extract and validate Bearer tokens, injecting `AuthenticatedUser` into request extensions.
    - **Route Refactoring**: Updated `/api/avatar/upload` and `/api/avatar/ekyc-status` to use dynamic user IDs and eKYC statuses from the JWT claims, eliminating `session_dummy` hardcodes.
- **Phase 8.1.5 (Asset Quarantine Persistence)**:
    - **Quarantine DB**: Added `QuarantineStore` and `SqliteQuarantineStore` to `libs/infrastructure` to provide a persistent "waiting room" for non-compliant assets (CSAM, eKYC fails, proportion violations).
    - **API Integration**: Integrated quarantine storage into the `/api/avatar/upload` flow, ensuring illegal or unvetted assets are recorded for audit and prevented from re-uploading via hash-based detection.
    - **Ripple Synchronization**: Fully synchronized `AppState` and `api_integration_tests.rs` to maintain high-quality CI through structural changes.
    - **Performance Optimization (Gap-5)**: Added SQLite index to `quarantined_assets.image_hash` for faster duplicate lookups.
    - **Bug Fix (E-101)**: Resolved a "Future is not Send" compilation error in `upload_avatar_handler` by ensuring `ImageHasher` is dropped before await points.
- **Phase 8.1 (CSAM 3-Layer Defense & Compliance)**:
    - **eKYC Integration (Stripe)**: Added `StripeEkycEngine` and `MockEkycEngine` in `libs/infrastructure/compliance` for strict 18+ real-name and age verification prior to custom asset publishing.
    - **Perceptual Image Hashing (CSAM Defense)**: Added `ImageHasher` in `libs/shared/csam` using DCT hashing (`img_hash`) to resiliently detect illegal or malicious textures against a known blacklist.
    - **NURTURE Compliance (5.5 Head-to-Body Checker)**: Implemented `ProportionsChecker` in `libs/shared/csam` to mathematically restrict avatar skeletal proportions that mimic underage anatomies (`LegalStatus::Restricted`).
    - **Asset Quarantine Flow**: Built `/api/avatar/upload` endpoint integrating the 3-layer defense to actively quarantine non-compliant assets from the Samsara Federation.
- **Phase 7.2 (Legal Guardrails & A2C Gratitude)**:
    - **Begging Supervisor**: Added `BeggingSupervisor` to `shared/guardrails` to detect and block AI-driven dark patterns (e.g., asking for money or gifts).
    - **Gift Engine (Tremendous API)**: Implemented `TremendousGiftEngine` in `libs/infrastructure` to enable autonomous real-world gift generation for users with high Karma.
    - **Autonomous Gratitude (A2C 諱ｩ霑斐＠)**: Integrated gift-triggering logic into `AutonomousBiomeEngine`, rewarding helpful human peers with $1-5 gift codes.
- **CSAM Defense & Inochi2D Foundation (Phase 7.1)**:
    - **Asset Separation Layer**: Implemented a mandatory asset/origin separation in `avatar-engine` to isolate local unvetted assets from Hub-syncable official assets.
    - **Protocol-Level CSAM Filter**: Integrated binary content detection (`data:image/`, `data:video/`, `;base64,`) into `samsara-hub` relay and `api-server` biome endpoints to strictly prohibit binary data embedding in the P2P network.
    - **Oversized CRDT Protection**: Added 1MB hard limit to `samsara-hub` timeline sync blobs to prevent covert binary data exfiltration through CRDT documents.
    - **Avatar Expression Mapper**: Created `libs/avatar-engine` to map AI emotions to VRM/Inochi2D blendshape parameters.
    - **SSE Avatar Parameters**: Extended the Server-Sent Events (SSE) stream to push `avatar_params` updates in real-time via the `avatar_expression` event.
    - **InxRenderer (WASM Foundation)**: Integrated `InxRenderer` into the management console's `DioramaView`, supporting the future `.inx` (Inochi2D) WASM runtime.
- **Context Management System (4-Layer Guardrails)**: Implemented an autonomous guardrail system to prevent AI agent contextual collapse and cascade errors. Includes `.context/RIPPLE_MAP.md` for deterministic dependency tracking, `preflight` workflow commands, Architecture Decision Records (ADRs `001` through `007`), and rigorous documentation synchronization rules.
- **Comprehensive Documentation Update**: Replaced 254 instances of "閾ｪ蜍戊｣懷ｮ�" (placeholder) documentation with context-aware, inferred descriptions across 47 Rust files. Resolved all `missing_documentation` warnings workspace-wide.
- **Preserve Intent Policy**: Established a workspace-wide policy (ADR 007) to suppress unused code/import warnings via `#[allow(...)]` instead of deletion, preserving developer context and intent for future features. Updated CI and all crate roots (`shared`, `infrastructure`, `soul`, `core`, `api-server`, `watchtower`) to enforce this policy.
- **Soul Engine (Phase 4 - Autonomic Deepening)**:
    - **Step 0 (Soul Memory)**: Added `SoulSnapshot` cache to `SqliteSoulStore` mapping memory to the chat LLM via `build_system_instructions`, eliminating DB latency on hot paths.
    - **Step 1 (Attachment Transitions)**: Implemented dynamics for `AttachmentStyle` shifting (Secure/Anxious/Avoidant/Disorganized) based on positive/negative interaction valences.
    - **Step 2 (Compound Triggers)**: Added true evaluation logic for `DefenseTrigger::Compound` in the internal `SoulPipeline`.
    - **Step 3 & 4 (Somatic Resonance & Reflex)**: Integrated Somatic Bias into predictions (incorporating intuitive emotion) and implemented fallback reflex defense generation for heavily negative experiences.
    - **Step 5 & 6 (Physical Execution & Temporal Decay)**: Mapped `RequireEscrow`, `Deflect`, and `Custom` defense actions to `Evolution Chronicle` audit trails, and added temporal intensity decay (death threshold 0.2) to prevent memory saturation over infinite rebirth cycles.
    - **Step 7 (Observability API)**: Added `GET /api/soul/status` to expose real-time internal state metrics (attachment style, active defenses count, soul resonance avg).
- **Soul Engine (Phase 3 - Reactive Hardening)**: 
    - **Step 1 (R-2)**: Improved `SoulError` granularity with context-aware prefixes (`[SoulDistill]`, `[SoulAdapter]`, etc.) for better debugging.
    - **Step 2 (Immune Feedback)**: Integrated `ImmuneSystem` threat detection into the `SoulPipeline` as negative experiences (`security.immune_response`), allowing the AI to learn from neutralized attacks.
    - **Step 3 (Somatic Marking)**: Enabled automatic generation of `SomaticMarker` (emotional grounding) during experience processing for all experiences with valence > 0.3 or < -0.3.
    - **Step 4 (Physical Defense)**: Connected `execute_defense` to physical actions: `DefenseAction::Reject` now dynamically injects `ImmuneRule`s, `Warn` records to the `Evolution Chronicle`, and `Hesitate` injects adaptive latency.
    - **Step 5 (Semantic Defense)**: Upgraded `SoulPipeline` to pre-fetch text embeddings asynchronously, enabling `DefenseTrigger::Semantic` cosine similarity matching while preventing redundant API calls (DS-1/RTT=0 optimization).
    - **Step 6 (Anamnesis Profile)**: Integrated `AnamnesisProfile` into `AgentSoul` and persistent storage, allowing narrative identity and maladaptive schemas to persist across generations via the `SamsaraEngine` rebirth cycle.
- **Soul Engine (Phase 1)**: Initial implementation of the Three-Layer Soul Architecture. Added `libs/soul` crate containing `AgentSoul` (Core), `PredictiveModel` (L2/Plasticity), `SomaticMarker` (L1/Reactive), and `Instinct` (L3/Meta-cognitive).
- **Soul Persistence**: Implemented `SqliteSoulStore` in `libs/infrastructure` for persisting and loading `AgentSoul` state, including a 10,000-entry experience buffer limit to prevent storage bloat.
- **Federated Soul Metadata**: Extended `FederatedKarma` and `approved_karma` schema to include `generation` and `somatic_valence`, enabling cross-node learning of personality traits.
- **AgentRx Framework Integration**: Implemented a comprehensive agentic diagnostic and recovery system including `TrajectoryStore` (SQLite), `ConstraintChecker` (rule-based validation), and `AgentRxDiagnostics` (LLM-based self-review). Added full integration into `skill_handler.rs` and the main chat loop for autonomous failure recovery.
- **WASM `fs_reader` Security Fix (P0)**: Corrected a path traversal vulnerability in the `fs_reader` WASM skill by replacing string-based prefix checks with component-aware `Path::starts_with`.
- **Enhanced BastionGuard Whitelisting & Flag Validation**: Strengthened `safe_exec` to recursively validate paths within command-line flags (e.g., `--file=path`) and explicitly blacklisted access to sensitive internal files like `.env`, `.git`, and `security.json` even within the allowed workspace.
- **AgentRx Schema Migration**: Added `trajectory_steps` and `agent_diagnoses` tables to the core database to enable persistent storage of agent execution paths and recovery hints.
- **Security Whitelist Optimization**: Removed `mv` from the default `BastionGuard` whitelist to prevent uncontrolled file movement and maintain consistency with other restricted destructive commands.
- **FTS5 Synchronization Triggers**: Implemented robust error handling for Search Index (FTS5) triggers in `migrations.rs`, replacing silent ignores with structured warnings and idempotent creation checks.
- **Duplicate Safety Directive Removal**: Cleaned up code across `infrastructure` and `core` crates by removing redundant `![forbid(unsafe_code)]` attributes.
- **Soul Engine Pipeline Integration (Phase 1.5)**: Implemented `DefaultSamsaraEngine` and `CoreDomainAdapter`, and integrated `SoulPipeline` and `SqliteSoulStore` directly into the `api-server` background worker loop for cyclic experience processing.
- **Experience Buffer Bounding**: Implemented `push_experience` to enforce a 1000-item memory limit on `experience_buffer` to prevent Out-Of-Memory issues during active sessions.
- **Soul Engine LLM Distillation (Phase 2)**: Core Samsara Engine now strictly limits distillation token context (`R-5`) and natively uses the LLM via `provider.complete` to generate real `InstinctRules` from recent experiences, replacing the static Phase 1 mock.
- **Soul Architecture Integration (Phase 2b)**: Unified `DreamState` (L0 Contemplation) and `SamsaraEvent::LevelUp` (L0 Progression) direct into `SoulPipeline` (L2/L3) converting environmental and introspective triggers into dynamic `Experience` objects safely decoupled from the heartbeat pattern (`R-6`).
- **Predictive Grounding**: Enabled actual `predict_outcome` capabilities inside the `CoreDomainAdapter`, activating `PredictiveModel` prediction accuracy calculations across arbitrary domains and enabling true surprise mechanics (`R-1`).
- **LoRA Configuration Persistence**: Added `lora_adapter_path` and `lora_base_model` columns to the `agent_souls` table in SQLite. Updated `SqliteSoulStore` to correctly save and load these values, ensuring LoRA-based fine-tuning settings persist across sessions.
- **LoRA Settings UI**: Integrated LoRA configuration inputs into the Management Console Settings page under the "AI Training & Voice" section, allowing users to specify paths for adapters and base models through the web interface.
- **Infrastructure Test Utilities Hardening**: Extracted and standardized `test_utils.rs` in the `infrastructure` crate. Fixed `MockJobQueue` function signatures and import paths to align with the core `JobQueue` trait, resolving compilation errors and improving test reliability.

### Fixed
- **Biome Encryption (NG-27, 28)**: Implemented automatic encryption in `send_message` and decryption in `list_messages` for Biome P2P dialogue using a shared secret derived from `FEDERATION_SECRET`.
- **Biome DB Stability (NG-29)**: Resolved silent database failures by adding proper error logging to all message/topic insertion paths in `api-server`.
- **Hub URL Unification (NG-30)**: Centralized all `SAMSARA_HUB_URL` resolution across the workspace via `AiomeConfig` and `AppState`, removing redundant `env::var` calls.
- **Ollama LoRA Dynamic Builder**: Implemented `build_lora_model()` inside `OllamaProvider`. When users select a LoRA adapter via the settings UI, the system autonomously builds a custom `Modelfile` and re-deploys a tailored model natively into the local Ollama backend, fixing the API parameter limitations (NG-21).
- **TTS Expression Engine**: Integrated actual Text-to-Speech synthesis (OpenAI's `tts-1`) into `ExpressionEngine`. Automatically converts AI expressions into local `.mp3` audio files and links them to the agent's internal state machine, replacing the previous placeholder behavior (NG-22).
- **Settings Sync Resilience**: Implemented automated state-hydration for `AgentSoul` in the `update_setting` API endpoint to fix UI-to-Backend synchronization gaps for LoRA parameter tuning.
- **Missing Whitelists**: Fixed missing settings keys (TTS configuration, LoRA paths, Voice selections) in API whitelist preventing database persistence.
- **Biome Transport Security**: Secured `AutonomousBiomeEngine` by enforcing symmetric encryption (ChaCha20-Poly1305) derived from `FEDERATION_SECRET` as a pseudo-HKDF, solving plaintext transport/storage vulnerabilities across the Samsara Hub.
- **LLM Economy Binding**: Integrated `CommerceEngine` directly into the `trigger_agent_chat` execution path, ensuring LLM invocations enforce autonomous token limits and simulate computational purchases correctly.
- **Infrastructure Auditing**: Resolved missing Apache License Headers in core API endpoints and testing utilities.
- **MockLlmProvider Stream Implementation**: Fixed a panic-prone placeholder in `MockLlmProvider::stream_complete` by providing a default empty stream implementation. This ensures stability when streaming is requested from mock providers during development or testing.
- **Soul Engine Robustness**: Resolved 16 issues discovered during deep scans, including `AgentSoul` ID validation, `compute_hash` logic for state integrity, safety attributes, and error conversion between `SoulError` and `AiomeError`.
- **Samsara Hub Data Integrity**: Fixed missing columns in `samsara-hub` SELECT/INSERT operations to ensure `generation` and `somatic_valence` are properly synchronized across the federation.
- **SQLite Deadlock in Swarm Ops (Critical)**: Resolved a deadlock in `do_sign_swarm_payload` where recursive calls to `do_get_node_id` created nested SQLite transactions, hitting the single-writer constraint and causing 8 karma tests to hang indefinitely. Refactored to a linear flow: ensure keys exist first, then sign without recursion.
- **Stack Overflow in JobQueue Trait Methods**: Applied `Box::pin` to all 55 delegation methods in `impl JobQueue for SqliteJobQueue` to heap-allocate async futures, preventing the 60+ method async state machine from overflowing the thread stack.
- **SwarmOps Direct Call Pattern**: Replaced `JobQueue` trait method calls (`get_node_id`, `tick_local_clock`, `sign_swarm_payload`) with direct `SwarmOps::do_*` calls in `guardrails.rs` and `karma.rs` to avoid pulling in the entire trait's massive async future.
- **Test Thread Stack Size**: Added `.cargo/config.toml` with `RUST_MIN_STACK=64MB` to ensure sufficient stack space for debug-build async futures.
- **Repository Hygiene**: Removed `check_output.txt` from the repository and added it to `.gitignore`.
- **Dream State JSON Injection (A-9)**: Replaced `format!`-based JSON construction with `serde_json::json!` macro in `dream_state.rs` to prevent JSON injection through unsanitized strings.
- **Integration Test Stability**:
    - **test_gig_lifecycle**: Resolved a series of issues in the Gig Engine integration test, including JSON response parsing (mapping UUID from response object), incorrect status code assertions, and missing database tables.
    - **Database Migrations**: Added Gig Engine tables (`gig_intents`, `gig_bids`, etc.) to the core migrations to ensure schema consistency.
    - **Mock LLM Enhancements**: Updated `DummyLlm` to return valid JSON for `OracleJudge` requests, enabling verification tests to pass.
    - **Prometheus Conflict Resolution**: Consolidated Prometheus recorder initialization using a single global `Lazy` cell, fixing the `test_fallback_router_failover` panic.
    - **Ownership & Type Safety**: Fixed several Rust compilation errors (`E0382`, `E0308`) related to `AppState` component cloning and type mismatches in test utilities.

### Changed
- **Panic-Free Startup**: Replaced all `expect()` / `unwrap()` calls in `api-server/main.rs` startup path with `unwrap_or_else` + `error!` + `std::process::exit(1)` for graceful error reporting.
- **Guardrail Test Safety (B6)**: Removed unsafe and redundant `std::env::set_var` calls from `guardrails.rs` unit tests, relying on the secure default of `true` for `ENFORCE_GUARDRAIL`.
- **API Server Secret in Tests**: Updated `api_integration_tests.rs` to pass secrets via type-safe `AppState` instead of environment variables, aligning with the new centralized secret management.
- **CORS Configuration**: Migrated from hardcoded origin strings to `AiomeConfig.allowed_origins`, loaded dynamically from `ALLOWED_ORIGINS` env var.
- **Hub URL Resolution**: `SAMSARA_HUB_REST` and `SAMSARA_HUB_WS` now resolved from `AiomeConfig` instead of scattered `env::var()` calls.
- **Federation Secret**: `FEDERATION_SECRET` no longer panics when unset; instead logs a warning and defaults to empty string.
- **DB Migration Logging**: Replaced silent `.ok()` error suppression in `migrations.rs` with `info!` logging for index creation and ALTER TABLE operations.
- **Server Bind Error**: TCP listener bind failure now shows the actual address and exits gracefully instead of panicking.
- **API Server Modularization**: Extracted massive monolithic routing into `routes/` (karma, agent, biome, expression, general) to prepare for Biome integration.
- **Samsara Engine (Evolution)**: AI self-leveling based on cumulative Technical Karma weights (`do_sync_samsara_level`).
- **Meta-Control Security**: Introduced `ConstitutionalValidator` trait for Heterogeneous Dual-LLM validation. The `SoulMutator` now securely verifies `SOUL.md` mutations using a prosecutor LLM.
- **Management Console (Dashboard v2)**: Launched a Tauri React-based desktop shell (`apps/management-console`) featuring Quantum Glass UI, live Karma stream, and Synapse Resonance Graph.
- **LLM Hybrid Architecture (Pattern B)**: Front-end uses Gemini Cloud (`gemini-2.5-flash`), background tasks use Ollama Local (`qwen3.5:9b`).
- **AI Name Customization**: Users can set a custom AI name during onboarding and change it later via Settings.
- **Onboarding Wizard v2**: 4-step onboarding (Welcome 竊� Name 竊� Avatar 竊� Security) with avatar selection (gender + style).
- **Background LLM Settings UI**: Added Background LLM configuration section to Settings page.
- **IME Input Fix**: Fixed Japanese IME input clearing bug in Agent Console and Settings.

### Changed
- Background worker interval increased from 60s to 300s for Ollama stability.
- System prompt now dynamically injects AI name from DB settings.
- `build_system_instructions()` prioritizes `SOUL.md` content over hardcoded identity.

## [0.1.0] - 2026-03-20

### Added
- **Full OSS Strategy**: Pivoted from Open-Core to a Full Open Source foundation under the Elastic License 2.0 (ELv2).
- **Aiome Branding**: Applied new visual identity including "Abstract Eye" logo and "Lobster Pilot" mascot.
- **Bilingual Documentation**: Established bilingual (EN/JP) versions for CLA, Code of Conduct, and Security Policy.
- **Governance Setup**: Implemented License Grant style CLA to encourage community contributions while protecting commercial rights.
- **Samsara Hub**: Central validator/quarantine node for federated learning and collective immunity.
- **Immune System**: Adaptive defense mechanism against malicious prompts and system anomalies.
- **Dream State**: Background generation of creative concepts and visual experiments.
- **Skill Arena**: Automated A/B testing framework for evaluating LLM prompts and styles.
- **Oracle**: Multi-model consensus system for scoring and validating generated media.
- **Resilience**: Jitter, Circuit Breaker, and HITL (Human-in-the-Loop) for federation sync and API calls.
- **Watchtower (Discord)**: Persona-driven interaction with rich stats (Resonance, Tech Lv) and evolution tracking.
- **Safety**: Structured JSON logging, `cargo audit` integration, and `cargo deny` license auditing.
- **Self-Healing**: Automated memory distillation, DB scavenging, and karma pruning.

### Changed
- Migrated federation endpoints to versioned API (`/api/v1/`).
- Enhanced `api-server` structured logging for observability.

---
[0.1.0]: https://github.com/motivationstudio-llc/aiome/releases/tag/v0.1.0

*Initial Release*
