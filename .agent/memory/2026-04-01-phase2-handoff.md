# 🤝 Handoff to New Session: Aiome MVP Implementation

## 📌 現在のフェーズと直近の成果
- **到達地点**: MVPロードマップの「Perfect Planning検証」が完全終了。`docs/vision/mvp_roadmap.md` が不動の **Rev.5 (最終定本)** となりました。
- **検証結果のハイライト**:
  1. **🚨 デスマーチ爆弾の解除**: `api-server`, `infrastructure`, `shared` に跨る約40箇所の `workspace/xxx` （SQLiteパス含む）ハードコードを発見し、Tauriサンドボックス化時に即死する問題をPhase 2-PREの最優先課題（AppDataResolver等への全面置換）に格上げしました。
  2. **🗑️ 車輪の再開発と無駄の排除**: `KnowledgeIndexer`等の不要なPG移行計画を削除。非MVPコンポーネントである `napi-bridge`, `key-proxy` を保守対象からカット。
  3. **🏛️ モノリス統合**: `watchtower`（Discord/TGボット）をサイドカーバイナリから `api-server` 内への `tokio::spawn` 統合へと設計変更（Tauriのゾンビプロセス管理という鬼門を回避）。

## 🚀 次のセッションでやること（Day 1 Action）

次のチャット（新セッション）では、直ちに以下の実装タスクを開始してください。

**👉 Target: [Phase 2-PRE: Path Standardization & Bootstrap Init]**
1. `libs/shared/src/config.rs` または新規モジュールとして `AppDataResolver` ユーティリティを作成する。
   - 例: `~/Library/Application Support/com.aiome.nexus/` などをベースパスとする。
2. ディープスキャンで特定された40箇所の `workspace/` 実装（`lora_training.rs`, `sqlite_vault_backend.rs`, `main.rs`, `skill_handler.rs`等）を `AppDataResolver` を経由するように書き換える。
3. `HeartbeatWakeupService` を `api-server/src/main.rs` 内のメインTokioループにマウント（初期化）する。
4. `watchtower` を別バイナリではなく、`api-server` 内に非同期タスクとして統合する。

## ⚠️ 守るべき「コンテキスト溢れ対策」3原則
新しいセッションでコンテキストが白紙に戻っても、以下の**MVP鉄の掟**を必ず継承してください：
1. **AST ディープスキャンの義務化**: 新たな実装計画を立てる際は、記憶に頼らず必ず `nurture_auditor.py` を実行し `deep_scan_matrix.md` を正とする。
2. **Drop-Dead GC**: 使わないコード（`napi-bridge`等）は無視し、修正スコープに絶対に入れない。
3. **Rustモノリス至上主義 (MVP限定)**: プロセスを分けない。Tauriのサイドカー管理を極限まで減らし、全て `api-server` に集約する。

---
*Ready to rock. Let's build the MVP.*
