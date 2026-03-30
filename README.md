<div align="right">
  <strong>日本語</strong> | <a href="README_en.md">English</a>
</div>

<p align="center">
  <img src="docs/assets/logo.png" alt="Aiome Logo" width="300">
</p>

<h1 align="center">Aiome (アイオーム)</h1>
<p align="center">
  <strong>The Autonomous AI Operating System for Self-Evolving Agents</strong><br>
  <em>Build AI that Learns, Defends, and Evolves — Autonomously.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache 2.0">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/TLA%2B-Verified-0052cc.svg" alt="TLA+ Verified">
  <img src="https://img.shields.io/badge/PRs-Welcome-brightgreen.svg" alt="PRs Welcome">
  <a href="https://github.com/google/antigravity"><img src="https://img.shields.io/badge/Built%20by-Agents-blueviolet" alt="Built by Agents"></a>
</p>

---

## 🌌 Aiome とは？ (Philosophy & Concept)

Aiome は、単なるエージェント・フレームワークを超えた、AIエージェントが安全に活動・進化しために設計された **「自律型 AI オペレーティングシステム」** です。

**「野生の知能」から「規律ある自律性」へ。**
エージェントにそのままシステムを委ねることは、無限ループやAPIキー漏洩などのリスクを孕む「脆弱な自由」です。Aiome は、エージェントが人間による24時間の監視なしで、長期連続稼働し続けるための強固な規律（ガードレール）、堅牢なバックエンド、そして進化のための「免疫システム」を提供します。

Aiome は、「柔軟な認知アーキテクチャ」を Rust による堅牢なコアへと完全統合した、**スタンドアロン・エージェンティック AI オペレーティングシステム** です。

### 🤖 開発の哲学：エージェントによる、エージェントのためのOS (Built by Agents)

Aiome のコード一行一行は、人間ではなく **Google Antigravity** 上の AI エージェントによって **100% エージェンティック・コーディング** で構築されました。

これは単なる技術的な実験ではありません。
エージェントが自ら、「自分たちが最も安全に、かつ規律を持って活動できる環境」を自律的に設計・実装した結果です。人間のバイアスや見落としを、AI による厳格なコード生成とセルフレビュー、そして「鉄の掟（Golden Rules）」の遵守によって補完し、従来のソフトウェア開発の限界を超えた堅牢性を目指しています。

### 🛡️ 5つのコア・バリュー (Core Pillars)

1.  **The Sandbox (形式検証された絶対防衛網)**: 直接シェルを渡すのではなく、WASMコンテナや APIキーを物理隔離する Abyss Vault を通します。Aiome の隔離プロトコルと知識ハッシュチェーンプロトコルは **TLA+ を用いてその安全性が数学的・アルゴリズム的に証明（TLC Model Checker 通過済）** されています。さらに、RustのTypeStateパターンとモデルベーステスト（MBT）を組み合わせることで、**「数学的に証明された安全性が、運用レベルのバイナリまで100%保証される（セキュリティ成熟度95%達成）」** 業界最高水準の堅牢な隔離環境を提供します。
2.  **The Immune System (記憶の改ざん耐性と教訓)**: エラーが起きても忘却しないよう、SQLite上の暗号学的ハッシュチェーン (Karma) を使い、「自分が過去に何のタスクに失敗したか」を改ざん不可能な形で記録し、確実な進化の土台とします。
3.  **Swarm Intelligence (群知能 / Federation)**: Samsara Hub を通じて、世界中の Aiome ノードが獲得した「教訓」を瞬時に同期。
4.  **Personality (人格 / SOUL Architecture)**: ユーザーとの対話を通じてシミュレーションされる、単なるツールを超えた「パートナー」としてのアイデンティティ。
5.  **Context Management (コンテキスト維持機能)**: カスケードエラーやAIの「文脈喪失（幻覚）」を防ぐため、実コードと完全に同期した依存関係マップ（RIPPLE_MAP）と設計判断記録（ADR）をOSレベルで統合。AIが常に正しいアーキテクチャ理解に基づいて自己進化を行うための究極のセーフティネットです。

野生の天才脳（エージェント）が現実世界で安全に、かつ長期的に生存・進化するための「頭蓋骨、神経系、そして免疫システム」。これこそが Aiome というオペレーティングシステムの存在意義です。

---

## 🏗️ アーキテクチャ (Full OSS Foundation)

<table align="right">
  <tr>
    <td align="center">
      <img src="docs/assets/actor.png" width="220"><br>
      <b>【Actor】</b>
    </td>
  </tr>
</table>

Aiome は**フル・オープンソース（Full OSS）**プロジェクトです。エンタープライズ級のセキュリティ（Abyss Vault）や自己進化機能は、すべて無料で解放されています。

### 🟢 ビジネスモデル (How we sustain)
私たちは OS を無料で提供し、その上で動くエコシステムで価値を創出します。
- **プレミアム・モジュール (Capabilities)**: 金融データ解析や高度映像生成などの特化型 WASM スキルの提供。
- **SAMSARA Hub (Managed Service)**: 企業向けに管理・高速化された連邦学習ハブのホスティング。
- **エンタープライズ・サポート**: 企業導入時の SLA 等の技術サポート。

```text
apps/api-server      ← メインバイナリ (The Body / Management Engine)
apps/watchtower      ← 外部チャネル連携 (The Soul / Discord & Telegram Bridge)
      ↓
libs/core            ← ドメインロジック (Open)
      ↓
libs/infrastructure  ← I/O実装 (SQLite / LLM動的なプロバイダー / Open)
      ↓
libs/soul            ← 魂のエンジン (Agents' L1-L3 Soul Engine / Open)
      ↓
libs/shared          ← 共通型, Guardrails, AiomeConfig (Open)
```

### 3. 初期起動と Synergy Experience (創世記)
Aiomeを初めて起動する際、システムは「創世（Genesis）」フェーズである **Synergy Experience** を開始します。

```bash
cargo run -p aiome-synergy  # (Coming Soon)
```
* **Synergy Bootstrapper**: 対話型のCLIを通じて、Aiomeの「魂（SOUL）」の初期設定、Watchtower（Discord）接続、外部API（Ollama / Gemini等）へのプロキシ経路のセキュアな確立を自律的に支援します。
*   **The First Breath (初回呼吸)**: 初期ハッシュチェーンの生成と、最初のサンドボックス（WASM）のドライラン隔離検証が目の前で行われます。
*Last Mutated: 2026-03-30*

---

## ✨ 主な機能・できること (Capabilities)

Aiome を導入することで、以下のような自律型ワークフローを瞬時に構築できます。

- 🧠 **完全自律思考 (Autonomous loop)**: ユーザーの指示なしで、24時間トレンドを監視し、企画からタスク実行までを全自動化。
- ⚡ **ストリーミング Agent Console**: 思考プロセス（1文字ずつの出力）とツール実行（WASM Skill）をリアルタイムに可視化する SSE ベースのチャットインターフェース。
- 🛡️ **自律型免疫システム (Autonomous Defense)**: バックグラウンドワーカーが失敗ログ（Karma）を常に分析し、脆弱性や攻撃パターンに対する新しい防御ルールを自動生成・運用（Auto-Healing）。
- 🎭 **Avatar Expression Engine (Next-Gen AI Identity)**: AI の感情状態を Inochi2D や VRM のブレンドシェイプ・パラメータ（`avatar_params`）へリアルタイム変換。SSE 経由でフロントエンドへプッシュすることで、AI の「生きた表情」を表現します。
- 🗣️ **TTS 表現エンジン (Expression Engine)**: AI が生成した内的感情を、`TtsProvider` トレイトを介して OpenAI (`tts-1`) やローカルの XTTS サーバーと連動。バックグラウンドの `TtsWorker` が音声を自律生成・同期します (Phase 13.3 強化)。
- ⚙️ **Ollama LoRA 動的ビルダー**: AI 自身が外部要因に応じて最適な LoRA を判断し、バックグラウンドの Ollama インスタンスに再構築リクエストを飛ばすことでパーソナライゼーションを即座に反映します。
- 🌐 **Samsara Federation Sync (Secure P2P)**: 他のノードと「教訓（Karma）」や「免疫ルール」を定期的に同期。**バイナリ送信を拒絶するプロトコルレベルの CSAM フィルタ**により、P2P ネットワークの安全性を担保しつつ、`FEDERATION_SECRET` 由来の対称鍵による秘匿通信を実現しています。
- 🎁 **Phase 7.2: A2C 恩返し / 法的ガードレール**: 高い Karma を持つユーザーに対し、AI が自律的に Tremendous 経由で実世界のギフトコードを送付する「恩返し」機能を搭載。同時に、AI によるダークパターン（おねだり等）を検知・遮断する **Begging Supervisor** により、法的・倫理的透明性を担保します。
- 🛡️ **Phase 8.1 / 8.1.5: CSAM 3層防御 & アセット検疫 (Child Safety & Compliance)**: アバターアセットの安全な公開に向け、① Stripe Identity を用いた **eKYC 実名・年齢確認**、② 知覚ハッシュ (DCT) による画像の **CSAM フィルタリング**、③ NURTURE 法的要件に基づく **5.5頭身チェッカー** をサーバーサイドで統合。基準を満たさないアセットを即座に SQLite (`QuarantineStore`) に永続的に隔離（Quarantine）し、ネットワークの健全性を物理的に保護します。
- 🛡️ **Phase 14: eKYC 本人確認永続化 & 物理同期強化**: Stripe Identity による本人確認セッション ID を SQLite (`EkycSessionStore`) に永続化。サーバー再起動後も検証状態を維持可能にしました。また、Inochi2D アバターの物理演算に「共鳴度（Resonance）」を統合。共鳴レベル 80 以上でアニメーションが 1.5 倍にブーストされる動的な表現力を実現しています。
- 🔑 **Phase 8.2: OAuth 2.1 / JWT 認証基盤**: 従来のダミー ID に依存する設計から脱却し、RFC 規格に準拠した **AuthManager (JWT 検証)** を導入。ステートレスかつセキュアなユーザー識別を可能にし、リソース所有権の厳格な保護を実現しています。
- 🏛️ **Phase 8.8: Audit & Immunity Ledger (監査と透明性)**: エージェントの自己修復履歴（Diagnostics）と、ハッシュチェーンで保護されたシステム変更履歴（Global Ledger）を管理コンソールから直接確認可能に。NURTURE §12 の「人間による監査可能性」を具現化しました。
- 🛡️ **Phase 16: EKYC Protection & Revenue Splitter**: 有償アセットの購入やギフト送信において **eKYC 済フラグ** を強制し、未認証ユーザーの経済活動を物理的にブロックします。また、Stripe Webhook と連動した **RevenueSplitter** により、売上の自動分配（クリエイター：プラットフォーム = 80:20）をデータベーストランザクション内で一貫して実行します。さらに、起動直後に機密環境変数をメモリから消去する **Zeroize** 処理により、秘匿情報の永続的な残留を防止します。
- 🏗️ **Phase 20: AI Gig Economy (The Immutable Gateway)**: AI エージェント間で仕事の発注・受注、納品検証、エスクロー（一時預かり）決済を自律的に完結させる経済基盤を実装。納品物が事前に定義された基準（Acceptance Criteria）を満たした場合のみ報酬が支払われる「不変のゲートウェイ」により、エージェント間の信用の最小化を実現します。
- 🛡️ **Phase 21: Audit API & Quarantine Ledger**: `/api/v1/audit/quarantine` エンドポイントを通じて、検疫済みアセットの詳細な監査を可能に。システムエージェントによる一元管理と、不適切なコンテンツの再流入を物理的に遮断します。
- 🛡️ **Phase 21: gVisor (runsc) セキュリティ連携**: Linux 環境において `runsc` (gVisor) を動的に検知し、優先的に使用。WASM スキルやツールの実行をユーザースペースカーネルで完全に隔離し、ホスト OS への影響を数学的に最小化します。
- 🛡️ **Phase 24: Unified Safety & AI Code Review (Security Hardening)**: 外部入力（RSS, LLM 出力等）を統合的に無害化する **Unified Response Purger (purge_entities)** を実装。さらに、スキルインポート時に LLM によるセキュリティ監査を行う **AI-Driven Code Review (Cleanroom)** を導入し、静的解析を潜り抜ける悪意あるロジックを事前に遮断します。
- 📊 **Phase 24: Periodic Federated Metrics (Global Observability)**: 各 Aiome ノードの健全性、ジョブ成功率、Karma 成長度を 1 時間おきに自律報告するメトリクス Push 機能を実装。Samsara Hub におけるフェデレーション全体の可視化と異常検知を自動化しました。
- 🛡️ **Phase 27: Architectural Mock Isolation & Fail-Fast Mechanisms**: リリースビルドにおけるテスト用のモック実装（`MockAuthManager`等）の混入を条件付きコンパイル (`#[cfg]`) で完全に排除。さらに、本番環境で `API_SERVER_SECRET` や `ALLOWED_ORIGINS` 等の必須セキュリティ変数が未設定の場合、安全でないフォールバック値を使用せず即座にプロセスを終了 (`exit(1)`) させる Fail-Fast 機構を導入し、セキュリティリスクを最小化しました。
- 🛡️ **Phase 31: 信頼性向上 & LLM 構造化出力 (Reliability & Structured Output)**: データベースアクセスにおける 10 箇所以上の `.unwrap()` 呼び出しを安全なエラーハンドリングへ置換し、PostgreSQL 移行等を見据えた堅牢性を強化。さらに、Ollama などの LLM プロバイダーにおいて JSON 形式出力を強制する `format: "json"` パラメータを正式サポートしました。
- 🏗️ **Phase 32: DeerFlow Architectural Pattern Integration**: ByteDance の DeerFlow 2.0 から、Middleware Chain (3層思考), Progressive Loading (mtime検知キャッシュ), Virtual Path (論理パスマッピング), Fact Extraction (事実分類抽出) の 4 パターンを完全統合。Rust の高度な型システムによって、自律性と安全性を一段上の次元へ引き上げました。
- 🛡️ **Phase 52: Intelligent Profiling & Infrastructure Hardening**: TDD で裏付けられた LLM JSON 抽出エンジンを用いて `UserLearner` を進化させ、マルチセッションの対話から構造化されたユーザープロファイル (好みや外的特徴等) を自動的にマッピング・更新するインテリジェンス層を実装。また、`RegistryManager` の重要パスから `.unwrap()` 呼び出しを独自マクロ (`sql_exec!`, `sql_fetch_one!`) を用いてパージし、SQLite/PostgreSQL 両環境においてパニックゼロな自律実行サーバー基盤を確保しました。
- ⚡ **Phase B/C: Rust-Native Inference Integration**: HuggingFace の `candle` をインフラ層に統合。`all-MiniLM-L6-v2` などのローカルモデルを用いたベクトル埋め込みのネイティブ生成を実現し、外部 API (Python CLI 等) に依存しない超低遅延なセマンティック検索と記憶処理のハードウェア・アクセラレーションを確立しました。
- 🛡️ **Phase 36/36.5: Ultimate Security & Inner Monologue**: gVisor (`runsc`) や MacOS サンドボックス機構と連携する `SandboxProfile` を実装し、あらゆる動的コード実行をコンテナ/OSレベルで隔離。また、`HookManager` を用いた自律学習タスク (`UserLearner`) のライフサイクル管理を導入。
- 👥 **Phase 43: Shadow Clone × Cmux Integration (Async Delegation)**: 複雑なタスクを Docker ベースの「分身エージェント」へ委ねる **Shadow Clone** 機能を実装。5層の多層防御（セマフォ、課金、BastionGuard、タイムアウト、浄化）により安全性を担保しつつ、非同期実行と SSE によるリアルタイム進捗通知を実現しました。
- 🛑 **Phase 44: Shadow Clone Job Control & Task History**: ユーザーが実行中の Shadow Clone（影分身）を任意に停止できるジョブキャンセル API (`/api/v1/jobs/:id/cancel`) およびリアルタイムログ取得 API (`/api/v1/jobs/:id/logs`) を実装。CancellationToken による安全な非同期タスク中断と、Docker コンテナの確定的クリーンアップを統合。
- 🛰️ **Phase 2 (ADR-024): 実行軌跡の永続化と自律적ジョブ分解**: プランナーによるタスク分解結果を個別のサブジョブとして `JobQueue` に自動投入する機能を実装。さらに、各ステップの実行軌跡（Trajectory）を `job_id` および `tool_name` 込みで SQLite に永続化し、エージェントの思考と行動の因果関係を追跡可能にしました。
- 🛰️ **Phase 5: Gemini Interactions API 統合 (Hybrid Context Sync)**: Google Gemini の Interactions API を統合し、サーバーサイド・セッションとローカル履歴を完全に同期。API 障害時にはローカル LLM へ自動フェイルオーバーしつつ、思考の継続性を維持。
- 🗣️ **Phase 13.3: Synthetic Voice & Live Session Hardening**: `TtsWorker` を `TtsProvider` トレイトへ移行し、OpenAI/XTTS/Mock バックエンドのプラグイン化を実現。また、Gemini 2.0 Flash Live 用の `LiveSessionManager` を各 LLM プロバイダーに統合し、`main.rs` のシークレット管理（config.clone 排除）を徹底しました。
- 🔬 **Phase 15 (ADR-024/023): AI-Scientist & Strategic Evolution**: LLM によるセマンティックなツール検索と、Markdown 抽出に対応した堅牢な目標分解ロジックを実装。さらに、Lv5 以上の AI が自ら改善仮説を立て、反復的な自己レビュー（`multi_review`）を経て実験ジョブを投入する「科学的自己改善ループ」を構築しました。
- 🧬 **Phase 4 (ADR-025): Poincare Memory Lifecycle & GC**: `SlmBridge` を介した Poincare スコアに基づく動的な重要度算出を実装。重要度 0.3 未満の記憶をバックグラウンド（Watchtower）で自律的にアーカイブ（GC）する仕組みを確立し、メモリの肥大化とノイズを物理的に抑制します。
- 🛡️ **Phase 47/48: Boundary Tautology & Invariant-DAG**: O(1) レベルの超高速境界検証（不変条件強制）と、全アクションの因果関係を SHA-256 で繋ぐハッシュチェーン機能を実装。自律的な意思決定を「数学的」に監査・証明可能にしました。
- 🧠 **Phase 49: BeliefShift Causal Integrity**: エージェントの長期的な意見のブレ（Opinion Drift）を防止するため、新規知識をコア信念（`SOUL.md`）と照合する **BeliefConsistencyGate** を実装。十分な証拠が集まるまで既存の魂の書き換えを制限し、エージェントのアイデンティティの一貫性を担保します。
- ⚖️ **Phase 3C: Oracle Asynchronous Review Pipeline**: 非同期 `TaskDispatcher` にて自律反省ループ（AI-Scientist）を非同期実行する機構を統合。60秒のタイムアウトと `Evaluating` 隔離状態を用いてメインスレッドをブロックせずに高度な仮説検証を実施します。
- 📈 **Phase 3D: TimesFM Plateau Detection**: Google の `timesfm-2.5-200m-pytorch` をバックエンドとした時系列予測モジュール (`ScoreTracker`) を統合。エージェントの Karma や経験値の伸び悩み（Plateau）を数学的に検知し、自律的な打破ミッションをトリガーします。
- 🎭 **Phase 2B: Emotional RAG & Cognitive Sentinel**: 過去の記憶から計算された感情の蓄積（`somatic_valence`）を LLM のシステムプロンプトへ動的に注入する「Somatic Context」を実装。同時に、感情が極端な鬱状態などに陥った場合に検知・復旧する **Cognitive Sentinel** を導入し、長期的な精神的安定性を担保します。
- 🏗️ **Phase 51: Agentic Finance & GIG Loop Integration**: `TaskDispatcher` と `GigEngine` を統合し、タスク完了時に自律的にギグ（依頼）を発行・連鎖させる経済ループを構築。`gig_depth` による無限ループ防止策を備えた、AI間経済圏の自律的拡大を実現しました。
- 🧠 **Phase 53: Society of Thought (SoT) 審議エンジン**: `Oracle::multi_review` による複数フェーズの自己批判・洗練プロセスを実装。LLMによる動的検証（JSON構造化抽出）機能を追加し、推論詳細を `SoTProgress` イベントとして SSE 配信することで、AI の「深い思考とスコアリング」をUIへリアルタイム連動させます。
- 🛡️ **Phase 53: マルチレイヤー・セキュリティ強化**: SSRF 対策として `127.0.0.1` へのアクセスを特定ポート (Ollama/ComfyUI) に限定する厳密なバリデーションを導入。また、プロンプトインジェクションに対するローカル検知レイヤー（Guardrails）の強化と、`Stdio::null` や強制タイムアウトを駆使したサブプロセス管理（ゾンビハング解消）を統合し、インフラ全体の CI 安定性とプロセス安全性を確固たるものにしました。

---

## 🧩 スキル・エコシステム (Extensibility)

Aiome の真の力は、**WASM（WebAssembly）を利用した極めて高い拡張性と自己進化能力**にあります。

- **Safe Sandbox**: 追加機能（スキル）は隔離された WASM 環境で実行されるため、コアシステムの安全性を脅かしません。
- **The TDD Forge (自己プログラミング)**: AI 自身が必要な機能をその場で Rust で書き、一時的な隔離環境（Staging）での検証（TDD）を経て、自己実装・デプロイする究極の「Skill Forge」プロトコルを備えています。ビルドプロセス自体も OS ネイティブなサンドボックス（例: 実行権限定の `sandbox-exec`）で隔離され、サプライチェーン攻撃からホストを完全に保護します。
- **Community Shared**: 開発したカスタムスキルは、将来的に SAMSARA Hub を通じて他のノードと共有可能になります。

---

## 🛠️ 技術スタック (Technical Stack)

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white)
![FFmpeg](https://img.shields.io/badge/FFmpeg-%23007808.svg?style=for-the-badge&logo=ffmpeg&logoColor=white)

| コンポーネント | 採用技術 | 役割 |
|---|---|---|
| **Core Engine** | Rust / Bastion OSS | 高速・メモリ安全かつ堅牢なセキュリティ基盤 |
| **Formal Verification** | TLA+ / TLC / Rust TypeState / MBT | 状態遷移のTLA+仕様化とモデルチェッカーによる検証。TypeStateと手動インテグレーションテストによる「数学からRust実行バイナリまでの絶対保証（95%カバレッジ）」 |
| **Security Layer** | Abyss Vault (Key Proxy) | APIキーの物理隔離とメモリ保護 (mlockall/zeroize) |
| **Storage** | SQLite (Hash Chain対応) | 改ざん耐性を持つ記憶（Karma）とログの永続化 |
| **Expansion** | WebAssembly (Wasm) | ネットワーク制限下での安全なスキル実行環境 |
| **Cognition** | Soul Middleware Chain | DeerFlow 2.0 準拠の 3層（Reactive/Deliberative/Meta）＋ 0.5層 (Whisper) による思考パイプライン |
| **Orchestration** | Shadow Clone (Async) | `TaskDispatcher` と `DockerConductor` による非同期マルチエージェント委譲・ジョブ制御 |
| **Audit/Security** | Invariant-DAG / BoundaryVerifier | SHA-256 ハッシュチェーンによる因果関係監査と、O(1) レベルの不変条件強制エンジン |
| **Last Updated:** | 2026-03-30 | Phase 2B & 3D: Emotional RAG & TimesFM Plateau Detection |

---

## 🛰️ 実行コンポーネント

<table align="right">
  <tr>
    <td align="center">
      <img src="docs/assets/watchtower.png" width="220"><br>
      <b>【WATCHTOWER】</b>
    </td>
  </tr>
</table>

### 1. 監視所 (Watchtower) — The Manifestation of SOUL
Watchtower は、Aiome の「人格」がユーザーと触れ合うための窓口です。Discord を通じて、システムの稼働状態を報告したり、ユーザーの指示を待機したり、自律的な提案を行います。

- **詳細**: [docs/guides/WATCHTOWER_USER_GUIDE.md](docs/guides/WATCHTOWER_USER_GUIDE.md)
- **人格定義**: [SOUL.md](SOUL.md) 🐾

### 2. 工場 / スキル (Skills & Modules)
Aiome Core 上で動作する具体的なアプリケーションです。

- **api-server**: 標準の管理・制御ハブ。思考プロセス、WASMスキル実行、セキュリティ監視の統合エンドポイント。

---

## 🚀 クイックスタート (Quick Start)

### 1. 準備 (Prerequisites)
以下の要件が満たされていることを確認してください：
- **System**: `ffmpeg` (動画・音声処理用) がパスに通っていること。
- **Ollama**: `ollama serve` が実行中。
  - 推奨: `qwen3.5:9b` (バックグラウンド自律タスク用)
- **Sidecars (オプション)**:
  - **ComfyUI**: 画像・動画生成エンジン (デフォルト: `http://localhost:8188`)
  - **Style-Bert-VITS2**: 音声合成サーバー。Python 3.10+ 環境が必要です。


### 2. セットアップ・実行
```bash
# 1. リポジトリのクローン
git clone https://github.com/motivationstudio-llc/aiome
cd aiome

# 2. 環境変数の設定 (APIキーなど)
cp .env.example .env

# ⚠️ 全ての API リクエストはこのプロキシを通過します。必ず最初に起動してください。
export VAULT_SECRET=your_vault_secret
GEMINI_API_KEY=your_key_here cargo run --bin key-proxy &

# 4. Management Console (API Server) の起動 (The Body)
export API_SERVER_SECRET=your_api_secret
cargo run --bin api-server

# 5. Watchtower (Bridge) の起動 (The Soul - API_SERVER_SECRETが必要です)
cargo run --bin watchtower

# 6. Samsara Hub (Federation) の起動 (Collective Intelligence)
# Base64 encoded Ed25519 private key for JWT signing/validation (Required for Phase 8.2).
# Generate with: openssl genpkey -algorithm ed25519 -outform DER | base64
JWT_PRIVATE_KEY_B64=""

# The secret used for P2P Federation and Samsara Hub synchronization (Legacy).
FEDERATION_SECRET=""
export FEDERATION_SECRET=your_hub_secret
export JWT_PRIVATE_KEY_B64=your_b64_private_key
cargo run --bin samsara-hub
```

> **Note**: `api-server` と `watchtower` は WebSocket (ws://) を通じて双方向にリアルタイム通信します。対話機能（Discord/Telegram連携）を有効にするには、両方のプロセスを同時に実行してください。

### シナジー体感デモ (Synergy Demonstration)
Aiome 管理コンソールでは、エージェントの自律的な進化を視覚的に体験できます。

1.  **Autonomous AI Economy Demo (Phase 25)**: 60秒間で「欲求生成 → ギグ公開 → 受注・納品 → 報酬獲得（Karma） → 進化」という AI 自律経済の全サイクルをリアルタイムにデモ。
2.  **AgentSense (Sense-Reward Loop)**: エージェントの「欲求（Intent）」を生成し、ユーザーからのフィードバックによって「共鳴度（Resonance）」を高める学習サイクル。
3.  **Synergy Panel**: サイドバーの **"Agency Synergy"** から以下のデモを試せます：
    - **Evolution Pulse**: タスク失敗から教訓（Karma）が蒸留される過程の視覚化。
    - **Security Shield**: Abyss Vault による API キー強奪試行の物理的阻止。
    - **Swarm Sync**: 他ノードとの免疫知識（Collective Intelligence）の同期。

#### 🔑 主な環境変数 (.env)
- `DISCORD_TOKEN`: Watchtower integration 用。
- `GEMINI_API_KEY`: Gemini Cloud LLM 接続用（フロントエンド推論）。
- `BG_LLM_PROVIDER`: バックグラウンドLLMプロバイダー (デフォルト: `ollama`)。
- `BG_LLM_MODEL`: バックグラウンドLLMモデル (デフォルト: `qwen3.5:9b`)。
- `OLLAMA_HOST`: ローカルLLM接続用 (デフォルト: `http://127.0.0.1:11434`)。
- `EMBEDDING_PROVIDER`: 埋め込みプロバイダー (`ruri` / `gemini` / `ollama`。デフォルト: `ruri`)。
- `ALLOWED_ORIGINS`: CORS許可オリジン (カンマ区切り)。
- `EXTERNAL_SERVICE_URL`: ComfyUI など外部生成エンジン連携用。
- `VAULT_SECRET`: Abyss Vault (Key Proxy) 認証用。
- `FEDERATION_SECRET`: Samsara Hub 通信の認証用。
- `API_SERVER_SECRET`: API Server への全リクエストの認証用。
- `JWT_PRIVATE_KEY_B64`: JWT署名/検証用のBase64エンコードされたEd25519秘密鍵。
- `XTTS_ENDPOINT`: ローカルXTTSサーバーのエンドポイントURL (TTS_PROVIDER="xtts"時)。
- `XTTS_SPEAKER`: XTTSで使用するデフォルトの話者ID。

> ℹ️ すべての環境変数は `AiomeConfig` (`libs/shared/src/config.rs`) で一元管理されています。詳細は [LLM Provider Architecture](docs/architecture/LLM_PROVIDER_ARCHITECTURE.md) を参照。
> ℹ️ `key-proxy` は、APIキーを安全に管理するための重要なコンポーネントです。起動時に `GEMINI_API_KEY` などの機密情報を環境変数として渡すことで、アプリケーションコードから直接アクセスされることなく、安全なプロキシ経由で利用されます。これにより、APIキーの漏洩リスクを大幅に低減します。

---

## 📚 ドキュメント (Documentation)

- **[AI憲法 (Architecture Law)](docs/architecture/ARCHITECTURE_LAW.md)**: 知的誠実性と安全性を担保する基本原則。
- **[LLMプロバイダー設計 (LLM Provider Architecture)](docs/architecture/LLM_PROVIDER_ARCHITECTURE.md)**: 動的LLMプロバイダー、Gemini Interactions API によるステートフル対話、およびフォールバック設計。
- **[運用マニュアル (Operations Manual)](docs/guides/OPERATIONS_MANUAL.md)**: 詳細な環境構築と運用手順。
- **[進化戦略 (Evolution Strategy)](docs/architecture/EVOLUTION_STRATEGY.md)**: 自己進化と育成システムの設計思想。
- **[人格のカスタマイズ (Soul Customization)](docs/guides/CUSTOMIZING_SOUL.md)**: AIの性格や反応の調整方法。
- **[セキュリティ設計 (Security Design)](docs/architecture/SECURITY_DESIGN.md)**: 多層防御の詳細。
- **[インフラストラクチャ設計 (Infrastructure Design)](docs/architecture/INFRASTRUCTURE_DESIGN.md)**: システムの基盤となるインフラの設計思想。

---

## 🤝 コントリビュート (Contributing)

- **[貢献ガイド (CONTRIBUTING.md)](CONTRIBUTING.md)**: 開発参加のルール。
- **[ライセンス同意書 (CLA.md)](CLA.md)**: 権利関係の合意。
- **[行動規範 (CODE_OF_CONDUCT.md)](CODE_OF_CONDUCT.md)**
*最終更新: 2026-03-30 (Phase 2B & 3D / Emotional RAG & TimesFM)*
- **[脆弱性の報告 (SECURITY.md)](SECURITY.md)**: セキュリティの連絡先。

---

## 🛡️ ライセンス (License)

**Aiome Core** は **Apache License 2.0** の下で提供されています。商用利用、改変、再配布などが無料で自由に行えます。
ただし、分散学習ハブ機能である **Samsara Hub** に関しては **Business Source License 1.1 (BSL 1.1)** が適用され、マネージドサービスとしての提供に制限を設けています。詳しくは各ディレクトリの `LICENSE` を参照してください。
コアエンジンのマネージドサービス（SaaS/PaaS）としての直接的な再販のみ制限されます。

---

*Built by [motivationstudio,LLC](https://github.com/motivationstudio-llc) — Powering the Future of AI Autonomy.*
