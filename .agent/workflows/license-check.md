---
description: ライセンスコンプライアンスの検証と外部OSSの帰属表示管理
---

# /license-check - ライセンスコンプライアンス 📜

外部OSSの利用状況を監査し、ライセンス義務への準拠を検証・維持するワークフローです。

## いつ使うか
- 新しい外部OSSライブラリ・ツール・論文を参照した時
- リリース前のライセンス準拠チェック
- 新しい `.rs` ファイルや `Cargo.toml` を追加した後
- CIで自動実行（プリコミットフック等）

---

## 実行手順

### ステップ1: 自動テスト実行
// turbo
```bash
python3 scripts/license_check.py
```

11項目の自動チェック:
1. ✅ `LICENSE` ファイル存在 & ライセンス種別（BUSL 1.1、Change License: Apache 2.0）
2. ✅ `NOTICE` ファイル存在 & 内容検証
3. ✅ `THIRD_PARTY_NOTICES.md` 存在 & 必須エントリ
4. ✅ 全 `.rs` ファイルに著作権ヘッダー
5. ✅ 全 `Cargo.toml` に `license` フィールド

### ステップ2: 結果判定
- **全PASS** → ステップ5（定期メンテナンス）へ
- **FAIL あり** → ステップ3（修正）へ

### ステップ3: 不足分の修正

#### 著作権ヘッダーが欠落している場合
新規 `.rs` ファイルの先頭に以下を追加:
```rust
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
```

#### Cargo.toml に license がない場合
`[package]` セクションに追加:
```toml
license = "BUSL-1.1"
```

#### 新しい外部OSSを追加した場合
`THIRD_PARTY_NOTICES.md` に以下の形式で追記:
```markdown
### [プロジェクト名]
- **URL**: https://github.com/...
- **License**: [ライセンス種別]
- **Usage**: [利用形態の説明]
- **Copyright**: [著作権者]
```

同時に `scripts/license_check.py` の `required_entries` リストにも追加:
```python
required_entries = [
    ("プロジェクト名", "ライセンス種別"),
    # ...
]
```

### ステップ4: 修正後の再検証
// turbo
```bash
python3 scripts/license_check.py
```
全PASSを確認。

### ステップ5: 定期メンテナンス

#### Cargo 依存ライセンス一覧の更新
```bash
cargo install cargo-license
cargo license --json > docs/licenses.json
```

#### npm 依存ライセンスの監査
```bash
cd apps/management-console && npx license-checker --summary
```

---

## ライセンス義務の判定基準

新しい外部OSSを利用する際は、以下の表で義務レベルを判定:

| 利用形態 | 義務 | 対応 |
|---------|------|------|
| ① ソースコードをコピー/移植 | 🔴 必須 | 著作権表示 + ライセンス全文を `THIRD_PARTY_NOTICES.md` に記載 |
| ② 設計パターン/アルゴリズムを参考 | 🟡 推奨 | 謝辞を `THIRD_PARTY_NOTICES.md` に記載 |
| ③ CLI/API として外部呼び出し | 🟢 推奨 | 帰属表示を `THIRD_PARTY_NOTICES.md` に記載 |
| ④ Cargo/npm 依存 | 🟢 自動 | `cargo license` で管理、バイナリ配布時に一括表示 |

## ライセンス互換性チェック

Aiome は **BUSL 1.1**（2030-04-01 に Apache 2.0 へ自動移行）です。依存として取り込む外部 OSS のライセンスとの互換性:

| ライセンス | 互換性 | 注意 |
|----------|-------|------|
| MIT | ✅ 互換 | |
| BSD 2/3-Clause | ✅ 互換 | |
| Apache 2.0 | ✅ 互換 | |
| MPL 2.0 | ⚠️ 条件付き | ファイルレベルのコピーレフト |
| LGPL 2.1/3.0 | ⚠️ 条件付き | 動的リンクのみ OK |
| GPL 2.0/3.0 | ❌ 非互換 | **絶対に混入させない** |
| AGPL 3.0 | ❌ 非互換 | **絶対に混入させない** |
| SSPL | ❌ 非互換 | **絶対に混入させない** |
| CC BY 4.0 | ✅ 互換 | 論文・テキストに適用、コードには不適 |

> ⚠️ **GPL/AGPL/SSPL ライセンスの依存を検出した場合は即座にアラートを上げること。**
> これらはコピーレフト義務によりプロジェクト全体を汚染するリスクがあり、BUSL 1.1 とも将来の Apache 2.0 移行とも両立しません。

---

## 関連ファイル
- `LICENSE` — プロジェクトライセンス (BUSL 1.1、Change Date: 2030-04-01 → Apache 2.0)
- `NOTICE` — 帰属表示（Apache 2.0 §4(d) 形式）
- `THIRD_PARTY_NOTICES.md` — 外部OSS帰属詳細
- `scripts/license_check.py` — 自動検証スクリプト (11テスト)

## 関連ワークフロー
- `/code-review` — コードレビュー時にライセンスヘッダーも確認
- `/preflight` — コード変更前の影響確認
- `/docs-sync` — ドキュメント同期（CHANGELOG等）