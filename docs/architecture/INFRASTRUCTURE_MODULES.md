# Infrastructure Modules Overview

このドキュメントでは、`libs/infrastructure` クレートに含まれる各モジュールの役割と責務を概説します。

## モジュール一覧

| モジュール | 役割 | 状態 |
|---|---|---|
| `aiome_log` | システム全体の標準化されたロギング（JSON/Text）を提供。 | 実装完了 |
| `belief_consistency_gate`| コア信念（SOUL.md）との整合性検証ゲート。2段階検証（SLM/LLM）により Opinion Drift を防止。 | **Phase 49 完了** |
| `boundary_verifier` | LLM を介さない O(1) のシェルコマンド境界検証（不変条件強制）を提供。 | **Phase 47 完了** |
| `auth` | OAuth 2.1 / JWT 検証 (`AuthManager`) を提供。ステートレスな認証基盤。**Phase 21** で `/api/v1/auth/authorize` 等のモックエンドポイントを実装。 | **Phase 21 継続中** |
| `artifact_store` | 生成された画像や動画、スキルの成果物を永続化・管理。 | 実装完了 |
| `channel_bridge` | Discord/Telegram 等の外部チャンネルとの抽象化通信層。 | 実装完了 |
| `circuit_breaker` | 外部APIのダウンタイムを検知し、安全に遮断。 | 実装完了 |
| `commerce` | 外部決済・ギフトAPI（Tremendous等）及び Stripe との自律的な商用連携基盤。**Phase 37a** で実機環境連携（create/cancel_subscription）と webhook 冪等性を実装。 | **Phase 37a 完了** |
| `commerce_mock` | 決済フローのカルシウム（テスト用途）。 | 実装完了 |
| `fallback_router` | プライマリLLM障害時に自動で代替LLMへフェイルオーバーするルーティング基盤。 | **新規実装** |
| `gig_engine` | AI間ギグ・エコノミープトコル実装。不変のゲートウェイによる自律受発注とエスクロー決済。 | **Phase 20 完了** |
| `compliance` | eKYC (Stripe Identity) と SQLite アセット検疫DB (Quarantine) の統合。`GET /api/v1/audit/quarantine` および `POST /api/v1/audit/quarantine/{id}/release` (検疫強制リリースAPI)を開通し、フロントエンドUI統合完了。 | **Phase 2A-3 完了** |
| `cognitive_sentinel`| 感情が極端な鬱状態などに陥った場合に検知し、自律的に回復イベントを発火させる防御層。**Phase 55** でジョブ失敗率（60%以上）に基づく Panic State 防御を追加。 | **Phase 55 完了** |
| `concept_manager` | AIが獲得した概念（Concepts）をベクターDBで管理。 | 実装完了 |
| `constraint_checker` | AgentRx における行動制約の検証エンジン。**Phase 55** で出力サイズ制限とエコー攻撃検知を追加。 | **Phase 55 完了** |
| `context_engine` | 会話履歴や環境情報をLLMに提供。**Phase 2B** で感情履歴（Karma/somatic_valence）から動的 Mood を計算し、LLMプロンプトへ注入（Somatic Context & Emotional RAG）機能を追加。 | **Phase 2B 強化** |
| `cortex_ingester`| LLMを用いたURL, テキスト, PDFからのドメイン特化型Markdownナレッジ抽出エンジン。 | **Phase A 完了** |
| `cortex_compiler`| 未加工のドキュメント群から概念（Concepts）を抽出し、一貫したWiki記事を自律的にコンパイルするエンジン。 | **Phase B 完了** |
| `cortex_query`   | 抽出済みのドキュメントやコンパイル済みWikiの双方に対して、セマンティックな意味検索を提供するエンジン。 | **Phase C 完了** |
| `dataset_extractor` | SoulStoreから記憶（`experiences`）を抽出し、MLX LoRA学習用JSONLデータセットに動的に蒸留・フォーマット変換を行うETL基盤。スレッドセーフかつコンテキスト維持（破滅的忘却防止）を担う。 | **Phase 1A-2 完了** |
| `diagnostics` | AgentRx の軌跡分析と自己診断（LLM Judge）。OpenAPI 公開および管理画面統合済。 | **Phase 8.8** |
| `dream_state` | アイドル時の自律思考（探求夢・反省夢）の状態管理。api-server側で `DreamService` ランタイムとしてバックグラウンド統合完了。 | **インテリジェンス層統合完了** |
| `forecast` | Google `timesfm-2.5-200m-pytorch` と通信し、成長停滞等の時系列予測を透過的に返す `ForecastProvider` トレイトの実装。 | **Phase 3D 完了** |
| `heartbeat_wakeup` | 定期的な自己診断とプロアクティブなアクションのトリガー。 | 実装完了 |
| `invariant_dag` | SHA-256 ハッシュチェーンによる因果関係の追跡と改ざん検知基盤。 | **Phase 48 完了** |
| `immune_system` | 脅威シグネチャによる不審な挙動の監視と遮断。 | **強化完了** |
| `job_queue` | タスクの非同期実行とリトライ、依存関係の管理。SwarmOps デッドロック修正済み（Box::pin + リニアフロー）。**Phase 1-2 Reflexion** で SQLite/PostgreSQL の P2P Timeline `UPSERT` 永続化を強化。 | **強化完了** |
| `knowledge_indexer` | ドキュメントや過去の Karma を高速検索可能にインデックス。 | 実装完了 |
| `llm` | 動的プロバイダー（Gemini/Ollama/Fallback）の抽象化。ストリーミング通信時の Pre-execute Hook バイパス遮断（Phase 1-2 反省強化）および Ollama の LoRA 動内ビルダ統合済。 | **第3世代進化** |
| `lora_autotuner` | ロス履歴に基づき LoRA 学習のハイパーパラメータ（LR, Epochs, Rank）を自律調整するエンジン。 | **Phase 55 完了** |
| `lora_marketplace` | LoRAアダプターファイルのSHA-256ハッシュ検証、エスクロー決済（CommerceEngine連携）、および分離されたファイル移動（PathSandbox）を提供する人格売買・流通インフラ。PostgreSQL/SQLite両対応。 | **実装完了** |
| `memory_crystallizer` | 短期記憶から長期的な教訓（Karma）への結晶化。エラーのサイレント隠蔽(`let _ =`)を防止し軌跡可視化を強化。 | **強化完了** |
| `oracle` | システム判断のための高度な論理推論エンジン。 | 実装完了 |
| `output_filter` | RTK(Rust Token Killer)に着想を得たコマンド出力フィルタ。ボイラープレートや重複行を削減しLLMコンテキストを最適化。**Phase 3**で削減数を `ToolExecutionEvent::TokenSaved` として上層やSSE（Management Console）へ流す計測基盤を結合済。 | **Phase 3 完了** |
| `publisher` | 成果物の SNS 等への自動投稿を管理。 | 実装完了 |
| `rate_limiter` | エージェント単位のリクエスト頻度制御。DoS 攻撃や予期せぬAPI消費を防ぐ。 | **新規実装** |
| `samsara_engine` | Soul Engine の L3 転生ロジック実体。Phase 3 で Anamnesis（物語的自己）の LLM 蒸留・継承を実装済。 | **Phase 3 完了** |
| `security` | 暗号化、認証、Abyss Vault との連携。**Phase 13.3** で `main.rs` の初期化順序を最適化し、`config.clone()` を排除した安全なシークレット共有を実現。Linux 環境での `runsc` 検知も継続サポート。 | **強化完了** |
| `tts` | `TtsProvider` トレイトに基づく音声合成エンジン。OpenAI (`tts-1`), XTTS, Mock をサポート。バックグラウンドでの非同期合成ジョブ処理を担当。 | **Phase 13.3 完了** |
| `skill_arena` | スキルの並列実行と結果の評価、ランキング。 | 実装完了 |
| `slm_bridge` | SuperLocalMemory (SLM) CLI との通信ブリッジ。Poincare スコアに基づく記憶の重要度算出を提供。 | **Phase 4 完了** |
| `score_tracker` | エージェントの成長やKarmaの停滞（Plateau）を日次で記録し、TimesFMによる時系列予測モジュールへデータを供給する。 | **Phase 3D 完了** |
| `skills` | WASM スキルのロード、実行、サンドボックス管理。**Phase B** にて `ToolHook` と `HookChain` を導入し、実行前後のインターセプトと `ToolCallRouter` による一元的なセキュリティ評価基盤を構築。 | **Phase B 完了** |
| `slo_engine` | サービスの可用性や応答時間の目標値を監視。 | 実装完了 |
| `soul_adapter` | 内部イベントから Experience へ変換、予測評価、および **Phase 37a** で L2.5 層 `WhisperMiddleware` を追加した Middleware Chain との連携。 | **Phase 37a 完了** |
| `soul_mutator` | 経験に基づく人格（SOUL.md）の動的な書き換え（L0）。※ Phase 2以降は `soul` crate（L1-L3）のSamsaraEngineへ段階的に移行予定。 | 実装完了 |
| `soul_store` | AIの魂（AgentSoul）と記憶（ExperienceBuffer）、Anamnesisの SQLite 永続化（L1-L3用）。**Phase 10.1b** で LoRA ハッシュ保存をサポート。 | **Phase 10.1 完了** |
| `trajectory_store` | AgentRx の行動軌跡を SQLite に永続化。**ADR-024 Phase 2** で `job_id` および `tool_name` による詳細な追跡に対応。 | **機能拡張完了** |
| `trend_sonar` | 外部トレンドの収集（Web/RSS）と LLM による評価・選別。マルチソース集約とインテリジェントな選別を実現。**Phase 24** で `purge_entities` による統合サニタイズへ移行。 | **強化完了** |
| `user_learner` | ユーザーの好みや行動パターンを学習。 | 実装完了 |
| `validator` | 入出力データの形式と安全性の検証。**Phase 4** で `ConstitutionalValidator` に `SlmBridge` を統合し、矛盾検知を強化。 | 実装完了 |
| `workspace_manager` | スキル生成時の一時ディレクトリやサンドボックス環境の管理。 | 実装完了 |
| `autonomous_demo` | 自律経済のデモ・オーケストレーター。欲求生成から進化までのライフサイクルを管理。 | **Phase 25.5 完了** |
| `task_orchestrator`| 非同期タスクの管理とディスパッチ。`DockerConductor` 等の実行部を束ねる。`CsamScanConductor` にて重いハッシュ計算を `spawn_blocking` 化しスレッド枯渇を防止（Phase 2A-1完了）。 | **Phase 2A 統合完了** |

## 備考
- **Phase 37a Integration**: `SoulPipeline` の評価後に経験蓄積 (`push_experience`) を実行するようアーキテクチャを変更し、`WhisperMiddleware` による自己省察ログの永続化を保証。

---
*最終更新: 2026-04-06 (Precomputed Relational Intelligence)*
