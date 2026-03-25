# Infrastructure Modules Overview

このドキュメントでは、`libs/infrastructure` クレートに含まれる各モジュールの役割と責務を概説します。

## モジュール一覧

| モジュール | 役割 | 状態 |
|---|---|---|
| `aiome_log` | システム全体の標準化されたロギング（JSON/Text）を提供。 | 実装完了 |
| `auth` | OAuth 2.1 / JWT 検証 (`AuthManager`) を提供。ステートレスな認証基盤。**Phase 21** で `/api/v1/auth/authorize` 等のモックエンドポイントを実装。 | **Phase 21 継続中** |
| `artifact_store` | 生成された画像や動画、スキルの成果物を永続化・管理。 | 実装完了 |
| `channel_bridge` | Discord/Telegram 等の外部チャンネルとの抽象化通信層。 | 実装完了 |
| `circuit_breaker` | 外部APIのダウンタイムを検知し、安全に遮断。 | 実装完了 |
| `commerce` | 外部決済・ギフトAPI（Tremendous等）及び Stripe との自律的な商用連携基盤。**Phase 37a** で実機環境連携（create/cancel_subscription）と webhook 冪等性を実装。 | **Phase 37a 完了** |
| `commerce_mock` | 決済フローのカルシウム（テスト用途）。 | 実装完了 |
| `fallback_router` | プライマリLLM障害時に自動で代替LLMへフェイルオーバーするルーティング基盤。 | **新規実装** |
| `gig_engine` | AI間ギグ・エコノミープトコル実装。不変のゲートウェイによる自律受発注とエスクロー決済。 | **Phase 20 完了** |
| `compliance` | eKYC (Stripe Identity) と SQLite アセット検疫DB (Quarantine) の統合。**Phase 21** で `/api/v1/audit/quarantine` による監査 API を公開。 | **Phase 21 完了** |
| `concept_manager` | AIが獲得した概念（Concepts）をベクターDBで管理。 | 実装完了 |
| `constraint_checker` | AgentRx における行動制約の検証エンジン。 | **新規実装** |
| `context_engine` | 会話履歴や環境情報をLLMに提供するコンテキスト生成。 | 実装完了 |
| `diagnostics` | AgentRx の軌跡分析と自己診断（LLM Judge）。OpenAPI 公開および管理画面統合済。 | **Phase 8.8** |
| `dream_state` | アイドル時の自律思考（思考整理・企画立案）の状態管理。Phase 2b で Soul Pipeline と統合済。 | **機能強化** |
| `heartbeat_wakeup` | 定期的な自己診断とプロアクティブなアクションのトリガー。 | 実装完了 |
| `immune_system` | 脅威シグネチャによる不審な挙動の監視と遮断。 | **強化完了** |
| `job_queue` | タスクの非同期実行とリトライ、依存関係の管理。SwarmOps デッドロック修正済み（Box::pin + リニアフロー）。**Phase 24** で `FederationOps` による統計メトリクスの自律報告を実装。 | **強化完了** |
| `knowledge_indexer` | ドキュメントや過去の Karma を高速検索可能にインデックス。 | 実装完了 |
| `llm` | 動的プロバイダー（Gemini/Ollama/Fallback）の抽象化。Ollama の LoRA 動的ビルダ (`build_lora_model`) を統合済。 | **第3世代進化** |
| `memory_crystallizer` | 短期記憶から長期的な教訓（Karma）への結晶化。 | 実装完了 |
| `oracle` | システム判断のための高度な論理推論エンジン。 | 実装完了 |
| `publisher` | 成果物の SNS 等への自動投稿を管理。 | 実装完了 |
| `rate_limiter` | エージェント単位のリクエスト頻度制御。DoS 攻撃や予期せぬAPI消費を防ぐ。 | **新規実装** |
| `samsara_engine` | Soul Engine の L3 転生ロジック実体。Phase 3 で Anamnesis（物語的自己）の LLM 蒸留・継承を実装済。 | **Phase 3 完了** |
| `security` | 暗号化、認証、Abyss Vault との連携。**Phase 21** で Linux 環境における `runsc` (gVisor) の動的検知・優先実行を実装。 | **強化完了** |
| `skill_arena` | スキルの並列実行と結果の評価、ランキング。 | 実装完了 |
| `skills` | WASM スキルのロード、実行、サンドボックス管理。**Phase 32** で `mtime` ベースの Progressive Loading と `VerifiedSkill` による型安全実行を強化。 | **Phase 32 完了** |
| `slo_engine` | サービスの可用性や応答時間の目標値を監視。 | 実装完了 |
| `soul_adapter` | 内部イベントから Experience へ変換、予測評価、および **Phase 37a** で L2.5 層 `WhisperMiddleware` を追加した Middleware Chain との連携。 | **Phase 37a 完了** |
| `soul_mutator` | 経験に基づく人格（SOUL.md）の動的な書き換え（L0）。※ Phase 2以降は `soul` crate（L1-L3）のSamsaraEngineへ段階的に移行予定。 | 実装完了 |
| `soul_store` | AIの魂（AgentSoul）と記憶（ExperienceBuffer）、Anamnesisの SQLite 永続化（L1-L3用）。**Phase 10.1b** で LoRA ハッシュ保存をサポート。 | **Phase 10.1 完了** |
| `trajectory_store` | AgentRx の行動軌跡を SQLite に永続化。 | **新規実装** |
| `trend_sonar` | 外部トレンドの収集（Web/RSS）と LLM による評価・選別。マルチソース集約とインテリジェントな選別を実現。**Phase 24** で `purge_entities` による統合サニタイズへ移行。 | **強化完了** |
| `user_learner` | ユーザーの好みや行動パターンを学習。 | 実装完了 |
| `validator` | 入出力データの形式と安全性の検証。 | 実装完了 |
| `workspace_manager` | スキル生成時の一時ディレクトリやサンドボックス環境の管理。 | 実装完了 |
| `autonomous_demo` | 自律経済のデモ・オーケストレーター。欲求生成から進化までのライフサイクルを管理。 | **Phase 25.5 完了** |

## 備考
- **Phase 37a Integration**: `SoulPipeline` の評価後に経験蓄積 (`push_experience`) を実行するようアーキテクチャを変更し、`WhisperMiddleware` による自己省察ログの永続化を保証。

---
*最終更新: 2026-03-25 (Phase 37a / Stripe Subscription & Whisper Integration)*
