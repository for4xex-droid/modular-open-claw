---
name: sqlx-migration-safety
description: DB スキーマ変更・sqlx マイグレーション追加・migration テスト修正時に読む。SQLite/PostgreSQL 二重同期、適用済みファイル編集禁止、TempDir スコープの安全規約。アプリ層のみの変更では不要。
---

# SQLx Migration Safety

スキーマ変更は **OSS 本線**（`libs/infrastructure/migrations/{sqlite,postgres}/`）と **commercial 配下**（`commercial/migrations/{sqlite,postgres}/`）の両系統を意識し、適用済み SQL の直接編集は行いません。詳細は golden-rules **B-003**（`.agent/skills/docs-ui-ux-golden-rules.md`）の拡張版です。

## 発動条件

- テーブル・カラム・インデックスを追加・変更するとき
- `sqlx::migrate!` 実行テストが `VersionMismatch` や `readonly database (1032)` で失敗するとき
- 既存マイグレーションの「ついで修正」を検討するとき

## 手順

1. **二重追加**: SQLite 用 `libs/infrastructure/migrations/sqlite/<timestamp>_<name>.sql` と PostgreSQL 用 `libs/infrastructure/migrations/postgres/` に同名ファイルを追加する（commercial 専用変更は `commercial/migrations/` も同期）
2. **適用入口**: SQLite 実行は `libs/infrastructure/src/job_queue/migrations.rs` の `sqlx::migrate!("migrations/sqlite")` 経由
3. **修正方針**: 既適用ファイルは触らず、新規タイムスタンプのマイグレーションで ALTER/UPDATE する（実例: `commercial/migrations/sqlite/20260702000000_payout_amount_usd_to_cents.sql`）
4. **テスト**: `tempdir`（または同等の一時 DB ディレクトリ）を DB 接続より長いスコープで保持する

## 良い例 / 悪い例

```sql
-- ✅ 新規ファイルで列型を修正（既適用 20260426142940 は不変）
ALTER TABLE nurture_payout_requests RENAME COLUMN amount_usd TO amount_usd_cents;
```

```rust
// ❌ TempDir を即 drop — 接続中に DB ファイル削除 → readonly database (1032)
let pool = SqlitePoolOptions::new()
    .connect(&format!("sqlite:{}/test.db", tempdir().path().display())).await?;
// tempdir がここで drop される

// ✅ テスト関数末尾まで TempDir を保持
let dir = tempdir().unwrap();
let pool = connect(dir.path()).await?;
// ... assert ...
```

## 完了条件

- **Positive**: `cargo test --workspace` で migration 関連テストが GREEN
- **Negative Test**: 適用済み `.sql` に1文字追加 → `VersionMismatch` を確認 → 変更を revert → 再テスト GREEN
- **Revert**: チェックサム検証用の改変は必ず元に戻す

> 出典: CHANGELOG [Unreleased]「既適用マイグレーションの直接編集は VersionMismatch を引き起こす」、memory/2026-04-17.md「tempdir スコープ外解放 → readonly database (1032)」、memory/2026-05-01.md
