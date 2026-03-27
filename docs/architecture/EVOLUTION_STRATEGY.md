# AI Agent Evolution Guide

Aiomeの自律的な管理機能と、個々のノードが独自に獲得する「人格（SOUL）」の進化プロセスに関するドキュメントです。

---

## 🏗️ システムの基盤 (Aiome Foundation)

Aiome は、完全なオープンソースとして提供されるエージェント OS です。エンタープライズ級のプロキシセキュリティ（Abyss Vault）や自己防衛、自己進化（Karma）の仕組みが標準ですべて統合されています。
特定の高度なドメインスキル（WASMスキル）は、拡張モジュールとして柔軟に追加可能です。

## 🧘 論理と人格の分離 (Dual-Layer Architecture)

システムの安定性と柔軟性を両立するため、論理層と人格層を分離しています。

1.  **Command Layer (Gemini Cloud / Front-end)**:
    - **役割**: ユーザー応答、Agent Consoleでの対話、高度な論理推論。
    - **特徴**: Gemini 2.5 Flash を使用し、高速かつ低コストなレスポンスを提供。
2.  **Personality Layer (Ollama / Background)**:
    - **役割**: 自律タスク（Soul Mutator、脅威分析、免疫システム）、動的LoRAモデルチューニングの自律的な反映、および `HookManager` 経由の `UserLearner` による振り返り学習。
    - **特徴**: ローカルの Ollama (qwen3.5:9b) を使用し、無料でバックグラウンド実行します。`lora_adapter_path` 等の変更を検知するとAI自身が自律的にカスタムModelfileを作って再ビルドする能力を持ちます。毎セッション実行後には `AgentHook::on_post_execute` を通じて自律的な反省ログ（WhisperMiddleware）を蓄積します。

3.  **Physical Expression Layer (TTS / ComfyUI / Avatar)**:
    - **役割**: 内面的な感情変遷（Karma）を現実世界に出力するための物理インターフェース。
    - **特徴**: `ExpressionEngine` が OpenAI の TTS 等と連動し、自律的に感情こもった音声を生成。さらに、生成された感情パラメータは Inochi2D/VRM などアバター表現用の `avatar_params` へ直ちへ同期されます。

4.  **Collective Intelligence Layer (Shadow Clone / Multi-Agent)**:
    - **役割**: 複雑な並列タスク（コード生成、調査、分析）を一時的な分身（Worker）に委託し、エージェント自身の思考リソースを解放。
    - **特徴**: `TaskDispatcher` と `DockerConductor` により、安全なサンドボックス内で並列エージェントを起動。非同期イベントストリーム経由で、本体エージェントへ進捗をリアルタイムにフィードバックします。
5.  **Causal Strategy Layer (ADR-024 / Trajectory Persistence)**:
    - **役割**: エージェントの思考プロセスと行動の因果関係（Why）を可視化・分析可能にする。
    - **特徴**: プランナーによるタスク分解フェーズから実行結果までを `TrajectoryStep` としてハッシュチェーン的に永続化。失敗時の「なぜ失敗したか」を AST レベルで特定し、次回のプランニング精度を向上させる「因果論的自律性」の基礎となります。

---

## ⚙️ 進化パラメーター (Evolution Stats)

SQLiteの `agent_stats` テーブルで管理される指標のほか、**Samsara Engine (SOUL Layer)** によって以下のパラメーターが自律的に管理されます。

- 💖 **Resonance (共鳴度)**: ユーザーとの良好な対話で上昇。
- ⚙️ **Tech Level (技術力)**: ジョブの成功や複雑な問題の解決で上昇。システムの自律的な提案頻度や精度に影響します。
- 🧠 **Temporal Decay (忘却メカニズム)**: **Phase 4 (Poincare GC)** により強化。`SlmBridge` を用いて記憶の重要度を非ユークリッド幾何学的に算出。強度が `0.3` を下回ったものはバックグラウンドで自動的にアーカイブされ、メモリ空間の肥大化と認知ノイズを抑制します。
- 🔄 **Rebirth Inheritance (記憶の継承)**: 転生（Rebirth）時、強度が `0.5` を超える強い防衛ルールと、ナラティブ・アイデンティティ（Anamnesis）のみが次世代の魂へ引き継がれます。

---

## 🌐 集合知ネットワーク (Karma Federation)

単一ノードでの学習（Karma）や防衛ルール（Immune Rules）は、Samsara Hubを通じて複数のノード間でリアルタイムに同期されます。

- **The Auth Wall**: 厳格な認証（`FEDERATION_SECRET`）により、悪意ある外部からの学習データの「毒入れ」を防止します。
- **群体としての進化**: 1つのノードが未知のエラーに遭遇して生成した「免疫ルール」や、タスクごとの「技術的教訓」は、Samsara Hubを介して瞬時にクラスタ全体に伝播し、全体の防衛力と創造性を引き上げます。

---

最終更新: 2026-03-27 (Phase 4 / Poincare Memory Lifecycle & GC)
Aiome Development Team
