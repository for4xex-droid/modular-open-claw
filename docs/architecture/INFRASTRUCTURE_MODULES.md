# Infrastructure Modules Overview

このドキュメントでは、`libs/infrastructure` クレートに含まれる各モジュールの役割と責務を概説します。

## モジュール一覧

| モジュール | 役割 | 状態 |
|---|---|---|
| `aiome_log` | システム全体の標準化されたロギング（JSON/Text）を提供。 | 実装完了 |
| `artifact_store` | 生成された画像や動画、スキルの成果物を永続化・管理。 | 実装完了 |
| `channel_bridge` | Discord/Telegram 等の外部チャンネルとの抽象化通信層。 | 実装完了 |
| `circuit_breaker` | 外部APIのダウンタイムを検知し、安全に遮断。 | 実装完了 |
| `commerce_mock` | 決済フローのシミュレーション（テスト用途）。 | 実装完了 |
| `concept_manager` | AIが獲得した概念（Concepts）をベクターDBで管理。 | 実装完了 |
| `constraint_checker` | AgentRx における行動制約の検証エンジン。 | **新規実装** |
| `context_engine` | 会話履歴や環境情報をLLMに提供するコンテキスト生成。 | 実装完了 |
| `diagnostics` | AgentRx の軌跡分析と自己診断（LLM Judge）。 | **新規実装** |
| `dream_state` | アイドル時の自律思考（思考整理・企画立案）の状態管理。 | 実装完了 |
| `heartbeat_wakeup` | 定期的な自己診断とプロアクティブなアクションのトリガー。 | 実装完了 |
| `immune_system` | 脅威シグネチャによる不審な挙動の監視と遮断。 | **強化完了** |
| `job_queue` | タスクの非同期実行とリトライ、依存関係の管理。SwarmOps デッドロック修正済み（Box::pin + リニアフロー）。 | **強化完了** |
| `knowledge_indexer` | ドキュメントや過去の Karma を高速検索可能にインデックス。 | 実装完了 |
| `llm` | 動的プロバイダー（Gemini/Ollama/Fallback）の抽象化。 | **第3世代進化** |
| `memory_crystallizer` | 短期記憶から長期的な教訓（Karma）への結晶化。 | 実装完了 |
| `oracle` | システム判断のための高度な論理推論エンジン。 | 実装完了 |
| `publisher` | 成果物の SNS 等への自動投稿を管理。 | 実装完了 |
| `security` | 暗号化、認証、Abyss Vault との連携。 | 実装完了 |
| `skill_arena` | スキルの並列実行と結果の評価、ランキング。 | 実装完了 |
| `skills` | WASM スキルのロード、実行、サンドボックス管理。 | 実装完了 |
| `slo_engine` | サービスの可用性や応答時間の目標値を監視。 | 実装完了 |
| `soul_mutator` | 経験に基づく人格（SOUL）の動的な書き換え。 | 実装完了 |
| `trajectory_store` | AgentRx の行動軌跡を SQLite に永続化。 | **新規実装** |
| `trend_sonar` | 外部トレンドの収集と、それに基づく企画の提案。 | 実装完了 |
| `user_learner` | ユーザーの好みや行動パターンを学習。 | 実装完了 |
| `validator` | 入出力データの形式と安全性の検証。 | 実装完了 |
| `workspace_manager` | スキル生成時の一時ディレクトリやサンドボックス環境の管理。 | 実装完了 |

---
*最終更新: 2026-03-18*
