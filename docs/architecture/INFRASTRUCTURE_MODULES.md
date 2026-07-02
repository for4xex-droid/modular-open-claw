# Infrastructure Modules Overview

このドキュメントでは、`libs/infrastructure` クレートに含まれる各モジュールの役割と責務を概説します。

## モジュール一覧

| モジュール | 役割 | 状態 |
|---|---|---|
| `aiome_log` | システム全体の標準化されたロギング（JSON/Text）を提供。 | 実装完了 |
| `belief_consistency_gate`| コア信念（SOUL.md）との整合性検証ゲート。CortexSynthパイプラインと統合され、生成データから矛盾を破棄する堅牢なフィルターとして稼働。 | **実装完了** |
| `boundary_verifier` | LLM を介さない O(1) のシェルコマンド境界検証（不変条件強制）を提供。 | **Phase 47 完了** |
| `cortex_synth` | ドキュメント群からタスクに応じた高品質なJSONLデータセット（ShareGPT形式）を自律生成する教師データ蒸留エンジン。 | **実装完了** |
| `auth` | OAuth 2.1 / JWT 検証 (`AuthManager`) を提供。ステートレスな認証基盤。**Phase 21** で `/api/v1/auth/authorize` 等のモックエンドポイントを実装。 | **実装完了 (Phase 8.2)** |
| `a2ui` | LLM出力からリアクティブUIを動的にストリーミング生成する基盤（Generative UI）。型安全な `schema`、`A2uiValidator` による XSS/SSRF および深再帰(DoS)防御を提供。 | **Phase 0 完了** |
| `artifact_store` | 生成された画像や動画、スキルの成果物を永続化・管理。Embedding 検索、CSAM 非同期スキャン、RT-6 監査ログ（読み書き時の全アクセス記録）、PathSandbox によるパストラバーサル防御、DRM vault ディレクトリ分離を実装。PostgreSQL/SQLite 両対応。**2026-07** で DB 行読取りを `sql_helpers::require_column` / `json_parse` 経由の明示エラー化に移行。 | **強化完了** |
| `buzz` | SNS等への自動投稿・スケジュール管理を行う自律型ワーカーおよび生成エンジン。 | **実装・堅牢化完了** |
| `channel_bridge` | Discord/Telegram 等の外部チャンネルとの抽象化通信層。 | 実装完了 |
| `circuit_breaker` | 外部APIのダウンタイムを検知し、安全に遮断。状態遷移時に `AlertManager` と非同期で連動し、トリップ時に Critical アラートを発火。 | 実装完了 |
| `tests/common/chaos` | **ADR chaos**: `ChaosMode` と `ChaosLlmProvider` を提供するテスト専用のフォルトインジェクション基盤。本番環境に影響を与えずLLM障害（空応答、不正JSON、巨大サイズ、タイムアウト）を注入可能。 | **実装完了** |
| `commerce` | 外部決済・ギフトAPI（Tremendous等）および Stripe/Polar との自律的な商用連携基盤。Webhook の冪等性保証（二重処理防止）とトランザクション境界での安全なライセンス付与を完備。 | **実装完了 (Phase E / P-1)** |
| `alerts` | 抽象化された通知レイヤー `AlertNotifier` トレイト、重要度レベル `AlertLevel`、および重複送信を防止するメモリ内デバウンスキャッシュを備えた `AlertManager` を提供。 | **実装完了 (Phase E / A-3)** |
| `commerce_mock` | 決済フローのカルシウム（テスト用途）。 | 実装完了 |
| `fallback_router` | プライマリLLM障害時に自動で代替LLMへフェイルオーバーするルーティング基盤。 | **実装完了** |
| `gig_engine` | AI間ギグ・エコノミープトコル実装。不変のゲートウェイによる自律受発注とエスクロー決済。 | **Phase 20 完了** |
| `compliance` | eKYC (Stripe Identity) と SQLite アセット検疫DB (Quarantine) の統合。`GET /api/v1/audit/quarantine` および `POST /api/v1/audit/quarantine/{id}/release` (検疫強制リリースAPI)を開通し、フロントエンドUI統合完了。 | **Phase 2A-3 完了** |
| `ban_store` | `BanStore` トレイト、`UniversalBanStore`、および `MockBanStore` を提供。不正なアクターやトークンの一元的な禁止リスト管理とリアルタイム検疫機能を提供。 | **Phase Compliance-1 完了** |
| `cognitive_sentinel`| 感情が極端な鬱状態などに陥った場合に検知し、自律的に回復イベントを発火させる防御層。**Phase 55** でジョブ失敗率（60%以上）に基づく Panic State 防御を追加。 | **Phase 55 完了** |
| ~~`concept_manager`~~ | AIが獲得した概念（Concepts）をベクターDBで管理。（`llm/utils.rs` への機能委譲によりSunset完了） | **Sunset済** |
| `constraint_checker` | AgentRx における行動制約の検証エンジン。**Phase 55** で出力サイズ制限とエコー攻撃検知を追加。 | **Phase 55 完了** |
| `context_engine` | 会話履歴や環境情報をLLMに提供。**Phase 2B** で感情履歴（Karma/somatic_valence）から動的 Mood を計算し、LLMプロンプトへ注入（Somatic Context & Emotional RAG）機能を追加。 | **Phase 2B 強化** |
| `cortex_ingester`| LLMを用いたURL, テキスト, PDFからのドメイン特化型Markdownナレッジ抽出エンジン。PDFテキスト抽出は `pdftotext` 隔離プロセス実行モデルによって安全に処理される。 | **Phase A 完了** |
| `cortex_compiler`| 未加工のドキュメント群から概念（Concepts）を抽出し、一貫したWiki記事を自律的にコンパイルするエンジン。Graphify抽出戦略に基づきリンクの `confidence` タグ (1.0/0.7/0.4) を算出する機能を搭載。 | **Phase B & CT 完了** |
| `cortex_query`   | 抽出済みのドキュメントやコンパイル済みWikiの双方に対して、セマンティックな意味検索を提供するエンジン。FTS5 高速化に加え、Typed Linksの最小 `confidence` に基づく `evidence_quality` (extracted/inferred/ambiguous) 導出機能を統合済。 | **Phase D & CT 完了** |
| `dataset_extractor` | SoulStoreから記憶（`experiences`）を抽出し、MLX LoRA学習用JSONLデータセットに動的に蒸留・フォーマット変換を行うETL基盤。スレッドセーフかつコンテキスト維持（破滅的忘却防止）を担う。 | **Phase 1A-2 完了** |
| `diagnostics` | AgentRx の軌跡分析と自己診断（LLM Judge）。OpenAPI 公開および管理画面統合済。**Phase B**にて `FailureCategory` 別の集計を返す `/api/v1/audit/diagnostics/summary` API を追加。 | **Phase B 拡張済** |
| `dream_state` | アイドル時の自律思考（探求夢・反省夢）の状態管理。8つのサブモジュール（aegis, biome, communication, exploration, observability, reflection, scientific, tests）に分割され、api-server側で `DreamService` ランタイムとしてバックグラウンド統合完了。 | **インテリジェンス層統合完了** |
| `forecast` | Google `timesfm-2.5-200m-pytorch` と通信し、成長停滞等の時系列予測を透過的に返す `ForecastProvider` トレイトの実装。 | **Phase 3D 完了** |
| `heartbeat_wakeup` | 定期的な自己診断とプロアクティブなアクションのトリガー。 | 実装完了 |
| `invariant_dag` | SHA-256 ハッシュチェーンによる因果関係の追跡と改ざん検知基盤。 | **Phase 48 完了** |
| `immune_system` | 脅威シグネチャによる不審な挙動の監視と遮断。 | **強化完了** |
| `cortex_file_projector` | ADR-025: Duke大学研究に基づき、Cortex Wiki記事をファイルシステム階層（カテゴリ/記事.md）として物理投影。`content_hash` 差分更新により冪等性を保証し、DreamState の Agent-Native Discovery モードで自律探索精度を向上。 | **ADR-025 実装完了** |
| `job_queue` | タスクの非同期実行とリトライ、依存関係の管理。SwarmOps デッドロック修正済。Biome 3メソッド（メッセージ永続化・評価値更新）及び Federation データ（ImmuneRule/ArenaMatch）のインポート・エクスポートを実装完了。P2pSanitizer による CSAM/Toxicity 動的ブロックを完備。DBクエリ DRY マクロの適用、および `let _ =` エラー黙殺コードのトリアージを完了。 | **強化完了** |
| `knowledge_indexer` | ドキュメントや過去の Karma を高速検索可能にインデックス。 | 実装完了 |
| `llm` | 動的プロバイダー（Gemini/Ollama/Fallback）の抽象化。ストリーミング通信時の Pre-execute Hook バイパス遮断および Ollama の LoRA 動内ビルダ統合済。**Phase v3/16** にて `EntropyGate`、`HumanizerFilter`、`WritingContext` を実装。最新モデルのユニットエコノミクス（コスト追跡）を実装。**Phase 48** でコスト計算と `BackgroundLlmProvider` をサブモジュール (`cost`, `background`) へ分割し、`dynamic.rs` を後方互換再構成。 | **分割・構造化完了** |
| `llm/semaphore_guard` | LLMプロバイダーへの同時実行数をセマフォで制限するラッパー `SemaphoreGuardedProvider` およびストリーム保護用 `GuardedStream` を提供。 | **実装完了** |
| `lora_autotuner` | ロス履歴に基づき LoRA 学習のハイパーパラメータ（LR, Epochs, Rank）を自律調整するエンジン。 | **Phase 55 完了** |
| `lora_marketplace` | LoRAアダプターファイルのSHA-256ハッシュ検証、エスクロー決済（CommerceEngine連携）、および分離されたファイル移動（PathSandbox）を提供する人格売買・流通インフラ。PostgreSQL/SQLite両対応。 | **実装完了** |
| `memory_crystallizer` | 短期記憶から長期的な教訓（Karma）への結晶化。エラーのサイレント隠蔽(`let _ =`)を防止し軌跡可視化を強化。 | **強化完了** |
| `oracle` | Multi-Judge consensus によるシステム判断の高度な論理推論エンジン。SoT Engine (Society of Thought) 委譲による多角的熟議、SSE イベントブリッジ連携、SEC-4 リフレクション回数上限（暴走防止）を実装。 | **強化完了** |
| `output_filter` | RTK(Rust Token Killer)に着想を得たコマンド出力フィルタ。ボイラープレートや重複行を削減しLLMコンテキストを最適化。**Phase 3**で削減数を `ToolExecutionEvent::TokenSaved` として上層やSSE（Management Console）へ流す計測基盤を結合済。 | **Phase 3 完了** |
| `publisher` | 成果物の自動公開（SNS, ブログ等）を管理。**Phase B/C** で WordPress REST API v2 アダプタを追加し、SEOコンテンツの自律パブリッシュを実装完了。**Phase 4** にて WordPress API トークンを排除し AbyssVault (Key Proxy) へ委譲するゼロトラストアーキテクチャへ進化。 | **Phase 4 完了** |
| `rate_limiter` | エージェント単位のリクエスト頻度制御。DoS 攻撃や予期せぬAPI消費を防ぐ。 | **実装完了** |
| `samsara_engine` | Soul Engine の L3 転生ロジック実体。Phase 3 で Anamnesis（物語的自己）の LLM 蒸留・継承を実装済。 | **Phase 3 完了** |
| `security` | 暗号化、認証、Abyss Vault との連携。サブモジュール（`config`, `security/bastion_guard`, `security/voice_core_drm`, `security/tests`）に分割し、`security.rs` を後方互換ハブモジュールとして再構成。 | **分割・構造化完了** |
| `tts` | `TtsProvider` トレイトに基づく音声合成エンジン。SSEストリーミング出力とリップシンク（Viseme）対応の多重化イベント配信をサポート。 | **Phase 14 完了** |
| `native_backend` | ネイティブRust実装によるSLM（SuperLocalMemory）機能群。意味検索（recall）、重要度計算、矛盾検知を提供。 | **実装完了** |
| `native_embedding` | SLMバックエンド向けに、ネイティブRustによるローカル埋め込み（Embedding）モデルの推論とコサイン類似度計算を提供。 | **実装完了** |
| `skill_arena` | スキルの並列実行と結果の評価、ランキング。**Phase 3**にてSQLite永続化と、MoE Routingのための実行前後の淘汰（Culling）フィードバックループを実装完了。また **Autodata Boltzmann 選択** による探索と搾取のトレードオフ最適化を統合。 | **強化完了** |
| `slm_bridge` | SuperLocalMemory (SLM) CLI との通信ブリッジ。Poincare スコアに基づく記憶の重要度算出を提供。 | **Phase 4 完了** |
| `spec_provider` | `FsSpecProvider` を通じた動的ワークフロー仕様のファイルシステムへのエクスポート基盤。パストラバーサル防御、symlink拒否、正規表現によるシークレットサニタイズを実装。 | **Phase 4 完了** |
| `prompt_registry` | Minijinjaベースのシステムプロンプト・テンプレートレンダリングエンジン。ゼロパニック対応の `NoopPromptRegistry` フォールバックを提供し、プロンプトのSSOTを確立。 | **Phase 4 完了** |
| `score_tracker` | エージェントの成長やKarmaの停滞（Plateau）を日次で記録し、TimesFMによる時系列予測モジュールへデータを供給する。 | **Phase 3D 完了** |
| `skills` | WASM スキルのロード、実行、サンドボックス管理。**Phase B** にて `ToolHook` と `HookChain` を導入し、実行前後のインターセプトと `ToolCallRouter` による一元的なセキュリティ評価基盤を構築。**2026-07** に God Module（mod.rs 1,135行）を `code_mode`（JS ブリッジ）/ `host_fns`（ホスト関数ビルダー）/ `types`（TypeState 型定義）へ分解し、機密パス検査を `is_sensitive_path` に統一。 | **リファクタ完了** |
| `slo_engine` | サービスの可用性や応答時間の目標値を監視。 | 実装完了 |
| `society_of_thought` | **ADR-032**: Dochkina (2026) の Endogeneity Paradox に基づく自己組織化熟議エンジン。Sequential マルチパス協調、自律ロール発明、Voluntary Self-Abstention、Capability-Aware Protocol Fallback を実装。Oracle `multi_review` と統合済。 | **ADR-032 完了** |
| `soul_adapter` | 内部イベントから Experience へ変換、予測評価、および **Phase 37a** で L2.5 層 `WhisperMiddleware` を追加した Middleware Chain との連携。 | **Phase 37a 完了** |
| `soul_mutator` | 経験に基づく人格（SOUL.md）の動的な書き換え（L0）。※ Phase 2以降は `soul` crate（L1-L3）のSamsaraEngineへ段階的に移行予定。 | 実装完了 |
| `soul_store` | AIの魂（AgentSoul）と記憶（ExperienceBuffer）、Anamnesisの SQLite 永続化（L1-L3用）。**Phase 10.1b** で LoRA ハッシュ保存をサポート。**2026-07** で JSON シリアライズ/デシリアライズを `sql_helpers` 経由の明示エラー化に移行。 | **Phase 10.1 完了** |
| `sql_helpers` | DB 行からの必須カラム抽出 (`require_column`) と JSON シリアライズ/デシリアライズ (`json_string`, `json_parse`) を `AiomeError::Infrastructure` で統一。`unwrap_or_default` による silent corruption を排除。 | **2026-07 追加** |
| `trajectory_adapter` | 実行軌跡 (Trajectory) を RLHF 向けフォーマット (Triplet) に変換・抽出するアダプタ。`ConstitutionalValidator` と統合され、Phase G 監視ループの報酬計算 (Reward Signal) を担う。 | **Phase G 完了** |
| `trajectory_store` | AgentRx の行動軌跡を SQLite に永続化。**ADR-024 Phase 2** で `job_id` および `tool_name` による詳細な追跡に対応し、**Phase G** にて `reward_signal` と `llm_prompt_hash` による RL フィードバック閉ループを統合。 | **機能拡張完了** |
| `trend_sonar` | 外部トレンドの収集（Web/RSS/X）と LLM による評価・選別。マルチソース集約対応。**Phase 8.7** にて全体ストールを防ぐ `FuturesUnordered` + Timeout の並行アーキテクチャと依存性注入（DI）によるテスト分離パラダイムを確立。 | **Phase 8.7 完了** |
| `user_learner` | ユーザーの好みや行動パターンを学習。 | 実装完了 |
| `validator` | 入出力データの形式と安全性の検証。`ConstitutionalValidator` で `SlmBridge` の矛盾検知を強化。CLI依存を排除した `LocalMockSlm` によるフェイルセーフなTDD環境を構築済。 | **強化完了** |
| `testing` | テスト専用の共有モック群（`#[cfg(test)]` ゲート）。`mock_jq` に JobQueue 系 14 トレイトのフル Mock 実装（MockJQ）を提供し、テスト間で再利用可能。リリースバイナリには含まれない。 | **2026-07 追加** |
| `workspace_manager` | スキル生成時の一時ディレクトリやサンドボックス環境の管理。 | 実装完了 |
| `x_signal_probe` | reqwest と X_BEARER_TOKEN を用いた超軽量な X API トレンド収集アダプタ。**Phase 8.7** にて、429 Retry-After 自律解析と、DashMap によるアンダーフロー無縁 (`saturating_sub`) な絶対安全レート制限機構へ到達。 | **Phase 8.7 完了** |
| `autonomous_demo` | 自律経済のデモ・オーケストレーター。欲求生成から進化までのライフサイクルを管理。 | **Phase 25.5 完了** |
| `task_orchestrator`| 非同期タスクの管理とディスパッチ。Oracle の Reject/Revise 時にフィードバックを蓄積して再試行を行う Verify-to-Iterate (自己修復リトライ) ループ、および自己修復ヒントを安全にプランナー指示へマージする GoalProcessor マージロジックを統合。 | **分割・構造化・自己修復強化完了** |
| `gig_metadata_updater` | `GigMetadataUpdater` トレイトの SQLite 実装 (`DbGigUpdater`)。OxiLean 検証結果 (`oxilean_verified`, `oxilean_oxp`) を `ai_artifacts` テーブル of `file_manifest` JSON に永続化し、Presentation 層から DB 直接アクセスを排除。 | **Sovereign Pipeline Phase 1 完了** |
| `grpc_proof_gate` | `FormalProofGate` トレイトの gRPC 実装 (`GrpcFormalProofGate`)。shadow-worker の `ProofVerifierService` と tonic チャンネル経由で通信し、WASM スキルの形式検証を透過的に実行。空トークン時の送信遮断によるゼロトラスト保証付き。 | **Sovereign Pipeline Phase 1 完了** |
| `workflow` | ワークフロー定義の構造、バリデーション、および Job リストへのコンパイルを提供。関連ファイル: `libs/infrastructure/src/workflow/` (`schema.rs`, `store.rs`, `transpiler.rs`, `validator.rs`) | **Phase 10 完了** |
| `backup_guard` | `bootstrap/database.rs::backup_sqlite_db_before_migration()` によるマイグレーション前の自動 DB スナップショット。`:memory:` / PostgreSQL 自動スキップ。`scripts/backup.sh` は SQLite Online Backup (`sqlite3 .backup`) による WAL-safe ホットバックアップ + 世代ローテーション + SHA256 チェックサム + 暗号化監査を提供。 | **Sinking Ship #19 完了** |

## 備考
- **Phase 37a Integration**: `SoulPipeline` の評価後に経験蓄積 (`push_experience`) を実行するようアーキテクチャを変更し、`WhisperMiddleware` による自己省察ログの永続化を保証。

---
*最終更新: 2026-07-03 (Asia/Tokyo) - sql_helpers 追加、artifact_store/soul_store 明示エラー化*

## Phase 6 Integration Notes

### OxiLean Power Polling
- `api-server` introduces `oxilean_poller` as a background Tokio task. It periodically checks the security verification score from `shadow-worker` and stores it in `AppState` as an `AtomicU32`. This ensures O(1) instantaneous reads for the UI without gRPC latency.

### P2P Sync Edge Proxy
- `aiome-node` acts as a Smart Edge Gateway. Its `/api/v1/federation/sync` endpoint (Deferred to v1.5) accepts incoming CRDT payloads from peer nodes, verifies signatures, and proxies the payload to the core `samsara-hub` for actual CRDT merging. This prevents the edge node from loading heavy dependency graphs.
