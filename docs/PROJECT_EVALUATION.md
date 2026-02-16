# ShortsFactory プロジェクト全体評価レポート

> 2026-02-17 時点でのコードベース全体を監査し、強み・弱点・改善点を評価する。

## 総合評価

```
 設計思想    ██████████ 10/10  ← 市場の競合より圧倒的に優れている
 セキュリティ ████████░░  8/10  ← 3層防御 + Sentinel CI、商用水準に近い
 実装進捗    ███░░░░░░░  3/10  ← Phase 1 完了のみ、中身はまだスカスカ
 テスト      ██████░░░░  6/10  ← shared に 15 テスト。他は 0
 ドキュメント ████████░░  8/10  ← セキュリティ設計書など充実
 運用準備    ██░░░░░░░░  2/10  ← 設定ハードコード、エラー処理不足
```

**一言でいうと: 「骨格は芸術品、筋肉はまだついていない」**

---

## ✅ 強み（競合優位性）

### 1. アーキテクチャ設計が非常に堅固
```
apps/shorts-factory (Body)
      ↓
libs/core           (Domain Interface)
      ↓  
libs/infrastructure (I/O Implementation)
      ↓
libs/shared         (Common Types + Security)
```
依存方向が厳格に守られており、Golden Rule に完全準拠。**これだけで多くのPythonベースの競合を凌駕している。**

### 2. セキュリティが「後付け」ではなく「設計に組み込まれている」
- Guardrails（入力検証）、SecurityPolicy（実行制御）、Sentinel（CI静的解析）の3層。
- **外部Skillという攻撃面自体が存在しない**のは、他のエージェントフレームワークにない根本的優位性。

### 3. LLM接続が動作している
- rig-core v0.30 との接続を確立済み。Qwen が Rust の中で思考・応答することを実証。

---

## ❌ 致命的な弱点（今すぐ認識すべき）

### 1. `libs/core` と `libs/infrastructure` が完全に空

```rust
// libs/core/src/lib.rs — 現在の中身
// (空ファイル)

// libs/infrastructure/src/lib.rs — 現在の中身
// (空ファイル)
```

アーキテクチャの最も重要な2層が**未実装**。トレイト定義（`TrendSource`, `VideoGenerator` 等）が存在しないため、ShortsFactory は現在「考えるだけで何もできない脳」の状態。

> [!CAUTION]
> **影響**: Phase 2 の全ツール（TrendSonar, ComfyBridge, MediaForge, FactoryLog）が実装不可能。

### 2. `api-server` と `shorts-factory` に関連性がない

`api-server` は CodeWiki ダッシュボード（ドキュメント表示用の Web UI）であり、ShortsFactory の動画生産とは**まったく無関係**。将来的に統合するのか、それとも別プロジェクトとして分離するのかを決める必要がある。

### 3. OpenClaw 設定ファイルが未設定のテンプレートのまま

| ファイル | 状態 |
|---|---|
| `IDENTITY.md` | テンプレートのまま（名前もVibeも未設定） |
| `SOUL.md` | テンプレートのまま |
| `TOOLS.md` | テンプレートのまま |
| `USER.md` | 未確認 |

これらは OpenClaw の動作に直接影響はしないが、**商品として見せたときの印象が「まだ何も始まっていない」になる**。

---

## 🟡 改善が必要な点（優先順列付き）

| # | 項目 | 深刻度 | 対処フェーズ |
|---|---|---|---|
| 1 | `thiserror` でエラー型を定義 | 🔴 | Phase 2 |
| 2 | `config.toml` による設定外部化 | 🔴 | Phase 2 |
| 3 | `libs/core` にトレイト定義を実装 | 🔴 | Phase 2 |
| 4 | `libs/infrastructure` に具体実装 | 🔴 | Phase 2 |
| 5 | `api-server` の位置付けを決定 | 🟡 | Phase 3 |
| 6 | 統合テスト追加（カバレッジ 80%+） | 🟡 | Phase 2-3 |
| 7 | ログ構造化（JSON） | 🟡 | Phase 3 |
| 8 | README.md（プロジェクト説明） | 🟡 | いつでも |
| 9 | `cargo-deny`（ライセンスチェック） | 🟢 | リリース前 |
| 10 | ダッシュボード認証 | 🟢 | 商用化時 |

---

## 📊 競合との比較

| 項目 | LangChain (Python) | AutoGen (Microsoft) | **ShortsFactory (Rust)** |
|---|---|---|---|
| メモリ安全性 | ❌ GC依存 | ❌ GC依存 | ✅ コンパイル時保証 |
| LLM実行権限 | ⚠️ 広い | ⚠️ 広い | ✅ ホワイトリスト制限 |
| 外部プラグイン | ⚠️ 動的ロード | ⚠️ 動的ロード | ✅ 排除（コンパイル必須） |
| 実装の成熟度 | ✅ 豊富 | ✅ 豊富 | ❌ **Phase 1 のみ** |
| エコシステム | ✅ 巨大 | ✅ 大きい | ❌ **自前のみ** |
| パフォーマンス | ⚠️ Python | ⚠️ Python | ✅ ネイティブ |

**結論: 設計面では優位だが、実装面では大幅に遅れている。**

---

## 🎯 推奨アクションプラン

### 短期（Phase 2: 次のセッション）
1. `libs/core/src/lib.rs` にトレイト定義（`TrendSource`, `VideoGenerator`, `MediaEditor`, `FactoryLogger`）
2. `libs/shared/src/config.rs` に `config.toml` 読み込みを追加
3. `libs/infrastructure` に最低1つの具体実装（ComfyBridge 推奨）
4. `thiserror` でドメインエラー型を定義

### 中期（Phase 3）
1. rig-rs `Tool` トレイトで ComfyBridge をエージェントに装着
2. ターン制リソース管理の実装
3. 1本の動画の E2E フロー（企画 → 生成 → 編集）を完成

### 長期（商用化前）
1. README.md の整備
2. ライセンス監査
3. ダッシュボード認証
4. テストカバレッジ 80%+

---

*評価日: 2026-02-17*
