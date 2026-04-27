---
description: CodeQLによるinter-procedural taint trackingを実行。ハンドラ引数→fs/process sinkの全経路を自動検出します。
---

# /codeql-scan - CodeQL Taint Analysis

CodeQL 2.25.x を用いた **inter-procedural taint tracking** を実行し、HTTPハンドラの引数から `std::fs` / `std::process::Command` 等の危険なシンクに到達する未サニタイズ経路を検出します。

## 前提条件

- `codeql` CLI がインストールされていること（`brew install codeql`）
- Rust ツールチェーン（nightly）が利用可能であること
- `codeql-custom/` ディレクトリにクエリパックが存在すること

## 実行手順

### Step 1: CodeQL Taint Analysis の実行
// turbo
adversarial テストクレートまたは本番コードに対してスキャンを実行します。
```bash
bash scripts/codeql-scan.sh
```

### Step 2: 結果の確認
生成されたレポートを読み込みます。

エージェントは以下のファイルを `view_file` で確認してください：
- `docs/architecture/codeql_taint_summary.md` — 人間可読なサマリ
- `docs/architecture/codeql_taint_report.sarif` — 機械可読な SARIF（必要に応じて）

### Step 3: 分析レポート

エージェントは検出結果に基づき、以下を報告します：

1. **検出された脆弱性の一覧**（ファイル名、行番号、データフローステップ数）
2. **各脆弱性の具体的な修正案**（サニタイズ関数の挿入箇所）
3. **Security Gate 判定**: CI で使用する場合の PASS/FAIL 判定

### 出力フォーマット例
```markdown
# 🔍 CodeQL Taint Analysis Report
- Total Findings: 3
- Severity: Critical (9.0)

| # | File | Line | Steps | Description |
|---|------|------|-------|-------------|
| 1 | src/handlers/upload.rs | L42 | 5 | User input reaches fs::write |
| 2 | src/api/admin.rs | L128 | 3 | Command injection via Command::new |
```

## 対象シンク一覧

| カテゴリ | 関数 | CWE |
|---------|------|-----|
| Path Injection | `std::fs::write`, `read`, `remove_file`, `copy`, `rename` | CWE-022 |
| Command Injection | `Command::new`, `::arg`, `::spawn`, `::output` | CWE-078 |

## カスタマイズ

新しいシンクを追加するには `codeql-custom/AiomeTaintTracking.ql` の `DangerousSink` クラスを編集してください。

---
**[注意]**: このワークフローは `scripts/codeql-scan.sh` を呼び出します。GitHub Actions では `.github/workflows/ci.yml` の `codeql-analysis` ジョブが同等の処理を行います。
