# 🔍 Aiome 技術的負債監査レポート

**監査日**: 2026-06-29 (v8.0 — Verify-to-Iterate 統合 & 新規認証・パニック負債の検出)
**前回監査日**: 2026-06-10 (v7.2)
**対象コードベース**: **152k LOC** (Rust ~128k + TypeScript ~24k)
**監査ツール**: `cargo audit`, `enforce_unwrap_deny.py`, `deep-scan.sh`, Git hotspot analysis, grep-based deep scan
**分析コミット**: `1020e30c`

---

## 1. Executive Summary

本監査フェーズにおいて、以前の主要な負債の解消状況を再スキャンするとともに、直近のコミット（`Zero-Metadata Commune Protocol v2` や `FormalProofGate` 統合）により新たに侵入した技術的負債を検知しました。

直近の `Verify-to-Iterate Loop` 統合に伴い、以前ハードコードされていたリトライ上限数 `3` が `ContextBudget::max_job_retries` として設定値化され、アーキテクチャの柔軟性が向上しました。一方で、新しく追加された API エンドポイント（Stripe Webhook や OAuth 関連）および Vault 管理エンドポイントにおいて、**Type-Driven Security (CC-6) の静的ルール違反（`Authenticated` 抽出子または `// auth-exempt` コメントの欠落）**が 3 ファイル（合計 6 ハンドラ）で検出されました。これらはミドルウェアによる物理保護は有効ですが、型検査およびドキュメンテーションの観点から修正が必要です。

また、Zero-Panic Policy（アンラップ/パニックの禁止）に関して、WASMスキル管理内の LazyLock 正規表現パースエラー時の `unreachable!()`（5箇所）および Tauri アプリビルド失敗時の `panic!()`（1箇所）が残存しており、これらを安全なエラー伝搬へと移行する必要があります。

---

## 2. Top 5 Priorities

| # | 負債 | 深刻度 | 影響 | 見積もり | Status |
|---|---|---|---|---|---|
| **P1** | **[NEW] Type-Driven Security (CC-6) 違反の解消** | 🔴 | `stripe.rs`, `auth.rs`, `vault.rs` での `// auth-exempt` 欠落および Authenticated 型抽出子欠損による静的チェックエラーの是正。 | 2h | `[NEW]` |
| **P2** | **[NEW] Zero-Panic Policy 違反 (unreachable/panic)** | 🔴 | `skills/mod.rs` (unreachable 5箇所) および `src-tauri/src/lib.rs` (panic 1箇所) の不安全な強制クラッシュの除去。 | 2h | `[NEW]` |
| **P3** | `mcp/discovery.rs` (1,113行) God Module + ハードコード OAuth URL 20+ | 🟡 | OAuth エンドポイント、トークン交換、MCP テンプレートが1ファイルに密結合。URL 変更時のリグレッションリスク。 | 6h | — |
| **P4** | **[RESOLVED] JobQueue リトライ上限のハードコード** | 🟢 | `core_ops.rs:585` の `count >= 3` が `ContextBudget::max_job_retries` に設定値化され解消。 | — | `[RESOLVED]` |
| **P5** | Error 型の統一 (10種類 → 3階層) | 🟡 | `thiserror` 7ファイル vs `anyhow` 47ファイルの混在。ただし `error.rs` の変換層 (22テスト) は模範的な設計。 | 6h | — |

---

## 3. Quick Wins（1時間以内で修正可能）

| # | 修正内容 | ファイル | 効果 | Status |
|---|---|---|---|---|
| **QW-1** | `.env.example` の不足キー追加 (`NURTURE_DRM_MASTER_KEY`, `BIOME_HUB_WHITELIST`) | `.env.example` | 新規 onboarding の完全自動化および環境変数一貫性の是正 | `[NEW]` |
| **QW-2** | `commerce_webhook/stripe.rs` に `// auth-exempt` 追加 | `commerce_webhook/stripe.rs:33` | CC-6 違反の解消（Stripe側Webhookのため免除） | `[NEW]` |
| **QW-3** | `auth.rs` の OAuth エンドポイントに `// auth-exempt` 追加 | `auth.rs:63, 118` | CC-6 違反の解消（ログインフローのため免除） | `[NEW]` |
| **QW-4** | `vault.rs` の管理者用APIに `_auth: Authenticated` を注入 | `vault.rs:38, 76, 118` | CC-6 違反の解消（管理者認証の型安全性の保証） | `[NEW]` |
| **QW-5** | `skills/mod.rs` の LazyLock 正規表現 `unreachable!()` を `unwrap_or_else` に置換 | `skills/mod.rs:165-187` | 静的パターン失敗時にもパニックさせず安全フォールバック | `[NEW]` |
| **QW-6** | `src-tauri/src/lib.rs` の Tauri ビルド時の `panic!` を Result 伝搬に変更 | `src-tauri/src/lib.rs:329` | デスクトップアプリ起動エラーを graceful に報告 | `[NEW]` |

---

## 4. Findings Table（12次元別）

### Dimension 1: Architectural Decay（アーキテクチャの腐敗）
- **God Module 残存状況**:
  - 1,000行を超えるモジュールは `society_of_thought.rs` (1,106行), `lora_marketplace.rs` (1,081行), `lora_training.rs` (1,010行) のみ。動作が安定しているため現時点では現状維持（DEFERRED）。

### Dimension 3: Type & Contract Debt（型・契約の負債）
- **[NEW] JobQueue トレイトの API 乖離 (CC-1)**:
  - `JobQueue` トレイトの定義（[traits.rs:300-370](file:///Users/motista/Desktop/antigravity/aiome/libs/aiome-core-contracts/src/traits.rs)）に対し、実装側の `UniversalJobQueue` にのみ定義されている補助的パブリックメソッドが多数存在し、インターフェース契約が形骸化しています。
  - **対策**: 将来的に trait 定義を整理するか、不要な pub 指定を削る必要があります。

### Dimension 5: Dependency & Config Debt（依存関係・設定の負債）
- **[NEW] .env.example 不一致 (CC-4)**:
  - `libs/infrastructure/src/compliance/quarantine.rs` などで参照されている `NURTURE_DRM_MASTER_KEY` と `BIOME_HUB_WHITELIST` が `.env.example` から漏れています。

### Dimension 8: Security Hygiene
- **[NEW] Type-Driven Security 違反 (CC-6)**:
  - 以下のエンドポイントで認証抽出子または免除タグが欠損しています。
    - [stripe.rs:33](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/commerce_webhook/stripe.rs#L33): `pub async fn stripe_webhook` (外部連携Webhook) → `// auth-exempt` コメントが必要
    - [auth.rs:63](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/auth.rs#L63): `pub async fn authorize_handler` (OAuth) → `// auth-exempt`
    - [auth.rs:118](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/auth.rs#L118): `pub async fn token_handler` (OAuth) → `// auth-exempt`
    - [vault.rs:38](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/vault.rs#L38): `pub async fn vault_status` (管理API) → `_auth: Authenticated`
    - [vault.rs:76](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/vault.rs#L76): `pub async fn vault_upsert` (管理API) → `_auth: Authenticated`
    - [vault.rs:118](file:///Users/motista/Desktop/antigravity/aiome/apps/api-server/src/routes/vault.rs#L118): `pub async fn vault_delete` (管理API) → `_auth: Authenticated`

### Dimension 10: Zero-Panic Policy 形骸化 (Aiome固有)
- **[NEW] 不適切な unreachable! / panic! 適用**:
  - [skills/mod.rs:165-187](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/skills/mod.rs#L165): `LazyLock` 正規表現コンパイルエラーのハンドリングで `unreachable!()` を 5 箇所使用。
  - [src-tauri/src/lib.rs:329](file:///Users/motista/Desktop/antigravity/aiome/apps/management-console/src-tauri/src/lib.rs#L329): Tauriアプリの初期ビルド失敗時に `panic!()` を使用。

---

## 5. メトリクス推移

| 指標 | v7.0 (2026-06-10) | v7.2 (2026-06-10) | v8.0 (2026-06-29) | トレンド |
|---|---|---|---|---|
| 総 LOC | 152k | 152k | **152k** | → |
| Rust テスト数 | 4,459 | 4,459 | **4,524** (1,103 passed) | 増加 ✅ |
| TS テストファイル | 63 | 63 | **63** | → |
| U-002 違反 (TSX) | 0 | 0 | **0** | 完全解消維持 ✅ |
| CC-6 違反 (Auth) | 0 | 0 | **6** | 一時的悪化 ⚠️ (新API追加に伴う) |
| God Module (1k+ 行) | 3 | 3 | **3** | 維持 |
| `as any` 本番使用 | 1 | 1 | **1** | 維持 |
| `let _ =` 黙殺 (要対処) | 0 | 0 | **0** | 完全解消維持 ✅ |
| JobQueue ハードコードリトライ | 存在 | 存在 | **解消** | 改善 ✅ |

---

*Generated by `/tech-debt-audit` workflow — 2026-06-29 v8.0 (Verify-to-Iterate 統合及び新規負債の特定完了)*
