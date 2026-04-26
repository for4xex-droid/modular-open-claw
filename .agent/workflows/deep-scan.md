---
description: 最高レベルのコードスキャンを実行。ASTマトリクス抽出し要件との差異を自律分析します。
---

# /deep-scan - ディープスキャン・コマンド実行プラン

このワークフローは、巨大なプロジェクト全域をコンテキストに過不足なく読み込ませ、セキュリティ、要件（Project NURTURE 等）、そしてアーキテクチャのギャップを最高精度でスキャンするために設計されています。

## 目的
人間の目とLLMのコンテキスト制限（Token Limit）を突破するため、AST（構文木ベース）で抽出されたハイライト情報を用いて俯瞰的な監査を行います。

## 実行手順

### Step 1: ディープスキャンの実行
// turbo
セグメント化されたディープスキャンと AST Taint Analysis を一括実行します。
```bash
bash scripts/deep-scan.sh
```

### Step 2: 監査マトリクスとTaintレポートの読み込み
自動生成されたマニフェストを読み込みます。

```bash
# cat docs/architecture/deep_scan_matrix.md
# cat docs/architecture/taint_analysis_report.md
# ツール (view_file) で読み込み
```

### Step 3: レポート生成
エージェントは読み込んだマトリクスから、以下について分析・レポートを行います：

1. **実装済みの主要構造・APIエンドポイントの総覧**
2. **Project NURTURE 要件に対する「不足機能（Gaps）」の特定**
3. **セキュリティチェック** (不要な公開 API や不審な構造体がないか)

### 出力フォーマット例
```markdown
# 📡 Aiome Deep Scan レポート
- **スキャン対象**: {アプリ数} Apps, {ライブラリ数} Libs
- **実装状況**:
  - `StructName` は存在しますが、要件で定義された `CapabilityX` に対応するエンドポイントが不足しています。
- **推奨アクション**:
  1. `libs/domain_x` クレートの作成
  2. `App/routes/y.rs` のエンドポイント追加
```

---
**[注意]**: 将来の Phase 8 において、本ワークフローは Aiome 自身にネイティブな `WASM Skill (Self-Auditor)` として統合される計画です。それまでは本コマンドを手動実行して下さい。