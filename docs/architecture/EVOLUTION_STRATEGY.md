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
    - **特徴**: `TaskDispatcher` と `DockerConductor` により、安全なサンドボックス内で並列エージェントを起動。Phase 50 より **ネイティブ gRPC ストリーム通信** に移行し、`aiome-internal` ネットワーク隔離、エフェメラルな `--env-file` 経由のシークレット注入、および UUID Bearer トークン認証を用いた「Zero-Trust A2A 基盤」として動作します。
5.  **Causal Strategy Layer (ADR-024 / Trajectory Persistence)**:
    - **役割**: エージェントの思考プロセスと行動の因果関係（Why）を可視化・分析可能にする。
    - **特徴**: プランナーによるタスク分解フェーズから実行結果までを `TrajectoryStep` としてハッシュチェーン的に永続化。失敗時の「なぜ失敗したか」を AST レベルで特定し、次回のプランニング精度を向上させる「因果論的自律性」の基礎となります。
6.  **Belief Integrity Layer (Phase 49 / BeliefShift)**:
    - **役割**: エージェントの「根源的なアイデンティティ」と矛盾する知識の学習を阻止し、Opinion Drift を防ぐ。
    - **特徴**: `BeliefConsistencyGate` が全ての新規知識（Karma）をコア信念と照合。矛盾するが妥当に見える情報は「証拠（Evidence）」として蓄積し、十分な証拠が得られた場合のみ `SoulMutator` が安全な書き換えを許可します。
7.  **Autonomous Evolution Layer (Phase 55 / LoraAutotuner)**:
    - **役割**: エージェントの学習プロセスを自律的に監視・最適化する。
    - **特徴**: `LoraAutotuner` がロス履歴から Stagnation（停滞）や Overfitting（過学習）を検知し、学習率 (LR) や Epoch 数、Rank を動的に調整。`HeartbeatWakeup` と連携し、24時間のクールダウンを維持しつつ、成長のプラトー（停滞）を検知した際に自律再学習を自己トリガーします。
8.  **Data Distillation Layer (Phase 1A-2 / DatasetExtractor)**:
    - **役割**: SoulStore（長期記憶）から会話や経験を抽出し、動的に機械学習モデル専用の訓練データへと精製する。
    - **特徴**: `DatasetExtractor` が稼働し、AIの過去の記憶領域から文脈を維持した一続きの JSONL ブロックデータを抽出。並行ジョブ間で I/O 競合が発生しないスレッドセーフな基盤の上で、破滅的忘却を防ぎながら独自のパーソナリティを LoRA へ転写（Fine-Tuning）するパイプラインを形成します。

9.  **Intelligence Layer (From Hierarchy to Intelligence)**:
    - **役割**: ピラミッド型の命令ダウンフロー（Hierarchy）から、LLM自身が課題発見から解決ツールの探索まで行う自律ネットワーキング（Intelligence）への転換。
    - **特徴**: アイドル時に稼働する `DreamService` がシステム全体の課題解決や「探索的夢（`scientific_experiment`）」を JobQueue に投下します。さらに、タスク実行に必要なスキルを持たない場合でもクラッシュせず、`ToolDiscoveryEngine` にフォールバックして必要な MCP ツールを自己推論して導入要求を出す「フェイルソフト」と「自己組織化」を獲得しました。

---

## ⚙️ 進化パラメーター (Evolution Stats)

SQLiteの `agent_stats` テーブルで管理される指標のほか、**Samsara Engine (SOUL Layer)** によって以下のパラメーターが自律的に管理されます。

- 💖 **Resonance (共鳴度)**: ユーザーとの良好な対話で上昇。
- ⚙️ **Tech Level (技術力)**: ジョブの成功や複雑な問題の解決で上昇。システムの自律的な提案頻度や精度に影響します。
- 🧠 **Temporal Decay (忘却メカニズム)**: **Phase 4 (Poincare GC)** により強化。`SlmBridge` を用いて記憶の重要度を非ユークリッド幾何学的に算出。強度が `0.3` を下回ったものはバックグラウンドで自動的にアーカイブされ、メモリ空間の肥大化と認知ノイズを抑制します。
- 🔄 **Rebirth Inheritance (記憶の継承)**: 転生（Rebirth）時、強度が `0.5` を超える強い防衛ルールと、ナラティブ・アイデンティティ（Anamnesis）のみが次世代の魂へ引き継がれます。

---

## 🌐 集合知ネットワーク (Karma Federation) [v1.5 導入予定]

> **Note**: Aiome v1.0 ではシングルノードの安定性向上のため、P2P / Samsara Hub 同期機能は無効化（スタブ化）されています。Federation 機能は v1.5 での再有効化に向けて設計調整中です。

単一ノードでの学習（Karma）や防衛ルール（Immune Rules）は、Samsara Hubを通じて複数のノード間でリアルタイムに同期されます。

- **The Auth Wall**: 厳格な認証（`FEDERATION_SECRET`）により、悪意ある外部からの学習データの「毒入れ」を防止します。
- **群体としての進化**: 1つのノードが未知のエラーに遭遇して生成した「免疫ルール」や、タスクごとの「技術的教訓」は、Samsara Hubを介して瞬時にクラスタ全体に伝播し、全体の防衛力と創造性を引き上げます。

---

最終更新: 2026-05-04 (Aiome v1.0 Stabilization / Federation Sunset)
Aiome Development Team
