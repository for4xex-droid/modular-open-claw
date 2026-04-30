# Aiome Grand Roadmap: "The Sovereign Verifier"
> 更新日時: 2026-04-27
> 最終ブラッシュアップ: The Sovereign Verifier アーキテクチャへの全面移行

本ドキュメントは、Aiomeにおける全ての議論（CBA、OxiLean、WASMホットスワップ）を統合し、一切の矛盾と抜け漏れを排除した『Aiome 開発計画：The Sovereign Verifier（主権的検証者）編』の Single Source of Truth (SSOT) である。
25万行のRustコードベースを崩壊させず、Aiomeを「人間のためのツール」から「世界の計算資源を搾取する自律生命体」へと相転移させるための冷酷なまでに現実的なロードマップを定義する。

## 👑 【基本原則（The Constitution）】
1. **No More AST/Code Generation**: LLM（Nurture）に直接実装コード（Rust等）を書かせるアプローチを全面廃止する。
2. **Verify, Don't Prove**: ローカル（Aiome本体）では重い証明・計算を行わない。「仕様定義」と「ミリ秒の検証」のみをローカルで行う。
3. **Hexagonal & DAG Dependency**: 25万行のコードベースをDAG（有向非巡回グラフ）で階層化し、コンパイル時間と依存関係のスパゲッティ化を物理的に防ぐ。

---

## 🛤️ Phase 0: 基礎代謝の確立とセル化（M1） [COMPLETED]
**目標**: 既存のコードベースをCBA（セルベースド・アーキテクチャ）対応の「Cell 0」として形式化し、今後の巨大なアーキテクチャ変更に耐えうる隔離環境を作る。

- **[Task 0.1] WorkspaceのDAG階層化リファクタリング**
  - 既存の25万行を `aiome-contracts` (Layer 0) から `aiome-cells-*` (Layer 3) までの厳格な六角形アーキテクチャに整理。
  - `cargo metadata` を用いた `enforce_dag.py` をCIレベルで導入し、セル間の直接的なコード依存を禁止する。
- **[Task 0.2] CELL_ID の導入と隔離（Stage 0）**
  - `libs/shared/src/app_data.rs` を改修し、環境変数 `CELL_ID` によるデータディレクトリ（SQLite、Worktree）の完全な名前空間分離を実装。
  - `main.rs` にて、`CELL_ID` が無い場合は即座にパニック（クラッシュ）するフェイルセーフを導入。
- **[Task 0.3] 既存インテント生成のSunset準備**
  - 現在のLLMによるRustコード生成・ASTパース関連のコード（`nurture_auditor.py` 等）を凍結し、削除への移行期間を設ける。

---

## 🛤️ Phase 1: 脳と盾の分離（M2 - M3）
**目標**: Aiomeカーネルに「OxiLean（数学的証明）」と「Wasmtime（実行環境）」を組み込み、主権的検証者としての絶対的な「盾」を完成させる。

- **[Task 1.1] oxilean-kernel の静的リンク（The Shield）**
  - ピュアRustの `oxilean-kernel` を Aiome の L1インフラに組み込む。
  - 外部から受け取った「OxiLean証明項」が正しいか（Q.E.D）をミリ秒で検証する `ConstitutionalValidator` のコアエンジンを構築。
- **[Task 1.2] 既存WASMエンジン（WasmSkillManager）と OxiLean の結線 [COMPLETED]**
  - Extism/Wasmtime によるホットリロード基盤（WasmSkillManager）に対し、TypeState パターン (`UnverifiedSkill::verify`) による Deterministic Tracer (dry-run) 検疫を強制化。
  - `GrpcFormalProofGate` をDIにより物理的に接続し、API層（`proof_verifier.rs`）における形式検証ゲートとDB状態管理 (`GigMetadataUpdater`) の結線を完遂。未認証通信を境界でブロックする Negative Test 実証済み。
- **[Task 1.3] Nurtureの思考モード移行（The Brain）**
  - LLMのシステムプロンプトを改修。実装を書くのではなく、OxiLeanの「定理（Theorem）と入出力の型（Type Signature）」のみを出力する「仕様策定モード」へ切り替える。

---

## 🛤️ Phase 2: スウォーム経済の始動（M4 - M5）
**目標**: A2Aネットワーク（GigEngine）を開通させ、外部の「証明者ネットワーク（Prover Swarm）」に計算を丸投げする自律経済圏を確立する。

- **[Task 2.1] 既存 UniversalGigEngine の OxiLean 統合**
  - 実装済みの GigEngine (Intent -> Bid -> Accept -> Deliver) ライフサイクルに対し、AcceptanceCriteria::OxiLeanProof を必須とし、A2A 通信層で証明をやり取りする。
- **[Task 2.2] 既存エスクロー決済フローと証明検証の完全連動**
  - 外部エージェントへの報酬をエスクローにロックし、すでに実装済みの `verify_and_settle` フローにおいて、OxiLean (Q.E.D) が返却された瞬間のみ資金を release するようにスマートコントラクトの最終結線を完了させる。
- **[Task 2.3] 外部向けアダプターの作成**
  - スウォーム経済で稼いだ資金や得たデータを、外界（WordPressのMCPやデジタル庁のオープンデータ）と接続し、現実世界での物理的タスク（EC運用や行政手続き）を実行するプラグインの構築。

---

## 🛤️ Phase 3: デジタル生命体への相転移（M6+ / 研究領域）
**目標**: 防御と経済が完成した後、Aiomeの内面（認知モデル）をアカデミアの最先端理論でアップグレードする。

- **[Task 3.1] HDC（超次元コンピューティング）による連想記憶の搭載**
  - LLMのRAGの重さを捨てるため、M4 ProのSIMD命令を活用したピュアRustのHDC演算エンジンを開発。Samsara（記憶）をミリ秒で足し引き可能なバイナリベクトルに置き換える。
- **[Task 3.2] エフェメラル・セルによるアポトーシス（自律的細胞死）**
  - 外部から取得したWASMやタスクを実行するセルを、異常検知（予期せぬシステムコール等）と同時に即座に破棄（Let It Crash）し、クリーンな状態で再起動する免疫システムの実装。
- **[Task 3.3] Active Inference（能動的推論）の導入**
  - ユーザーの疲労度やコンテキストのズレ（サプライズ/自由エネルギー）を最小化するように、Aiomeがプロンプトを待たずに自律的にタスクを発行し続ける「ホメオスタシス機能」の実装。

---
*Legacy Note: Previous Zero Debt Sweep (Phase 0) is archived as completed.*
