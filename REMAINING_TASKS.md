# 📋 プロジェクト全体 残存タスク集約レポート (REMAINING_TASKS.md)

> [!IMPORTANT]
> **本ファイルは 2026-07-03 をもって [OPEN.md](OPEN.md) に統合されました（Aiome 側タスクは OP-xxx として移入済み）。**
> 以後の追跡・更新は OPEN.md で行ってください。本ファイルは Project-Nurture 側タスク（セクション3）の参照用スナップショットとしてのみ保持します。

**最終更新日**: 2026-07-02
**対象プロジェクト**: `aiome` (OSS) & `Project-Nurture` (商用拡張モジュール)

---

## 1. 直前の開発で完了した内容
直前のセッション（`7618a248-2c65-47f3-801a-2863f34e0366`）では、以下の項目が実装・検証され、`main` リモートリポジトリにプッシュされています。

* **MCPツールバジェット制限警告機能の実装**
  * 同一ターン内のツール呼び出しバジェット制限警告の追加。
  * 対象ファイル: [apps/api-server/src/mcp/client.rs](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/mcp/client.rs)
* **X (旧Twitter) 公式MCPクライアントへの移行**
  * 公式名前空間への移行とトレンドワード収集・分析アダプターの新規作成。
  * 対象ファイル: [libs/shared/src/mcp_constants.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/shared/src/mcp_constants.rs), [apps/api-server/src/internal_services/x_mcp_trend.rs](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/internal_services/x_mcp_trend.rs)
* **`quick-xml` 脆弱性の一時的な無視設定**
  * Pre-push hook でのブロックを回避するため、`.cargo/audit.toml` に無視設定を追加。
  * 対象ファイル: [.cargo/audit.toml](file:///Users/motista/Desktop/antigravity/aiome/.cargo/audit.toml)
* **ARCHITECTURE.md の自動同期**
  * 不要となった環境変数記述 of 削除に伴うドキュメント更新。
  * 対象ファイル: [ARCHITECTURE.md](file:///Users/motista/Desktop/antigravity/aiome/ARCHITECTURE.md)

---

## 2. Aiome 側の残存タスク (技術的負債・インフラ)

[TECH_DEBT_AUDIT.md](file:///Users/motista/Desktop/antigravity/aiome/TECH_DEBT_AUDIT.md) および [MEMORY.md](file:///Users/motista/Desktop/antigravity/aiome/MEMORY.md) に基づくオープン項目です。

### 🔴 優先度：高 (Priority 1-3)
* **Tauriアップストリーム追従に伴う `quick-xml` 脆弱性の解決**
  * **現状**: `tauri` -> `plist` の依存バージョンが固定されているため、一時的に `RUSTSEC-2026-0195`, `RUSTSEC-2026-0194` を無視しています。
  * **タスク**: 将来的に Tauri 側で依存関係がアップデートされた際、`.cargo/audit.toml` から無視設定を削除し、`cargo update -p quick-xml` を実行します。
* **フロントエンド型安全性 (`as any`) の解消**
  * **対象**: [WorkflowBuilder.tsx:101,234,271](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/components/WorkflowBuilder.tsx#L101), [workflowConverter.ts:139](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/lib/workflowConverter.ts#L139) (計4箇所)
  * **タスク**: 型キャストを廃止し、厳密な型定義または型ガードを導入します。
* **`biome-popup-entry.tsx` の HEX カラー直書きの解消 (U-002違反)**
  * **対象**: [biome-popup-entry.tsx:36](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src/biome-popup-entry.tsx#L36) (背景色 `#030712`)
  * **タスク**: CSS 変数 `var(--bg-primary)` 等のテーマ同期トークンに置換します。

### 🟡 優先度：中 (Priority 4-5 & その他)
* **`skills/mod.rs` (1,134行) God Module の分解**
  * **タスク**: スキル登録、正規表現マッチング、ディスパッチの各責務を別ファイルに分離・整理します。
* **Error型定義の統一 (10種類 → 3階層)**
  * **タスク**: `thiserror` (7ファイル) と `anyhow` (47ファイル) が混在しているものを、明確な3階層のエラー構造に統一します。
* **`deep-scan.sh` の CRATES 設定修正**
  * **対象**: `scripts/deep-scan.sh:48`
  * **タスク**: 既に廃止され `api-server` に統合された `apps/watchtower` のパスをスキャン対象から除外します。
* **`unwrap_or_else(|_| loop {})` 回避策の修正 (Dim 10 違反)**
  * **対象**: [skills/mod.rs:163-164](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/skills/mod.rs#L163)
  * **タスク**: パニック検出回避のための意図的な CPU 100% 無限ループ処理を、安全な `LazyLock::new` または `Result` によるエラー伝搬に変更します。
* **JobQueue トレイトの API 乖離の解消**
  * **タスク**: `UniversalJobQueue` にのみ定義されている多数の補助メソッドについて、トレイト側 (`traits.rs`) に引き上げるか、`crate` プライベート化します。
* **`immune_system.rs` 内の巨大 MockJQ の共有化**
  * **タスク**: テスト用 MockJQ (約700行) を `libs/test-utils` などへ切り出し、テスト間で再利用可能にします。
* **`CausalVisualizer` (軌跡グラフ) のフロントエンド実装**
  * **タスク**: `management-console` 側に AI の思考プロセス（軌跡データ）を視覚化する Trajectory Graph コンポーネントを構築します。

---

## 3. Project-Nurture 側の残存タスク (経済・コンプライアンス)

[UNCERTAINTY_BREAKTHROUGH.md](file:///Users/motista/Desktop/antigravity/Project-Nurture/docs/UNCERTAINTY_BREAKTHROUGH.md) および [DEVELOPMENT_GUIDE.md](file:///Users/motista/Desktop/antigravity/Project-Nurture/DEVELOPMENT_GUIDE.md) に基づくオープン項目です。

### 🛡️ 技術的ギャップ・設計タスク
* **経済トランザクションの TLA+ 形式仕様の策定**
  * **タスク**: 二重通貨の ACID トランザクション（残高減算 + Creator Points 加算のアトミック性）の安全性を数学的に証明する `NurtureEconomyProtocol.tla` を策定し、TLC で検証します。
* **VRM レンダラーと LLM 推論の VRAM 競合調停の実装**
  * **タスク**: LLM 推論（Ollama）実行時に Tauri IPC 経由で WebView (Three.js) の描画を 15fps に制限し、完了後に 60fps へ復帰させるセマフォ制御を実装します。
* **On-memory DRM の Tauri プロセス連携実装**
  * **タスク**: VRM ファイルの復号時にディスクへ平文を露出させず、Tauri v2 の Custom Protocol (`tauri://vrm/`) 経由で WebView 側へバイナリストリーミングする設計の検証・実装を行います。
* **Saga パターンの複雑性と補償ロールバック設計の強化**
  * **タスク**: `commerce-protocol/transaction.rs` の型状態に `Compensable` を追加し、アペンドオンリーの `CompensationLog` による安全なロールバックを `proptest` 等で保証します。
* **高度な金融工学 (ZKP, CRDT) のローカルPoC**
  * **タスク**: 将来的な分散経済圏構築を見据え、線形型(CoinQuantum)、ZKP (Bellman)、CRDT などの適用可能性検証およびローカル PoC 設計を行います。
* **CSAM ボーン解析チェッカーの実稼働調整**
  * **タスク**: 多様な VRM モデルを用いた実機テストを行い、頭身比率判定の閾値や判定ロジックを微調整します。

### ⚖️ 法務・運営・外部依存タスク
* **資金決済法「自家型前払式支払手段」の対応**
  * **タスク**: 未使用残高 1,000万円超過を防ぐためのハードキャップ（`MAX_TOTAL_OUTSTANDING_COINS`）の設計・実装、および超過時のチャージ停止とアラート表示機能。将来の届出プロセス書類・会計処理の整理を行います。
* **特定商取引法に基づく表記ページの作成**
  * **タスク**: プラットフォーム上でのデジタルコンテンツ販売に必要な特商法表記ページをフロントエンドへ追加します。
* **AI の「自律購買度」ポリシーの実装**
  * **タスク**: 完全自律 / ユーザー承認制 / ハイブリッドの各モードを `PurchasePolicy` enum で制御し、`NURTURE_AUTO_PURCHASE_LIMIT` 閾値を管理画面から設定可能にします。
* **Creator Points の報酬（ギフトカード）変換機能の構築**
  * **タスク**: 1,000 CP = ¥500 デジタルギフトといった段階制変換を行う `CreatorPointsPolicy` の実装、および Tremendous/giftee API 連携（日本国内向けの二重化対応）を行います。
* **クリエイター向けコールドスタート対策**
  * **タスク**: 公式ベースボディおよび公式パーツの制作・初期マーケットプレイス在庫としての配備を行います。

---

## 4. 目視検証・その他のオープン項目
* **BiomeBackground + alpha:false 修正のブラウザ動作検証**
  * **内容**: R3F v9 の DPR ズレおよび CSS 背景アルファ合成バグに対する修正が、フロントエンドで正常にレンダリングされているか、目視での検証が必要です。
