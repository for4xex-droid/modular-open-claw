# コード出自証明書 (CODE PROVENANCE)

## 1. コード生成ポリシー
Project NURTURE のソースコードは、創業者（人間）の監督下で、先進的な AI コーディングアシスタント（Google Gemini / Antigravity）を活用して構築されています。

## 2. ライセンスの帰属
- **AI ツール**: Google Gemini (Antigravity), GitHub Copilot
- **著作権帰属**: 生成されたすべてのコードの著作権は motivationstudio, LLC に帰属します（各ツールの商用利用規約に基づく）。
- **外部コード**: Stack Overflow 等からのコピペは禁止されています。すべての外部ライブラリは `Cargo.toml` を通じて導入され、`cargo-deny` によってライセンスが監査されています。

## 3. 品質保証プロセス
1. **プロンプトエンジニアリング**: 創業者が ADR（Architecture Decision Records）に基づき、明確な技術仕様を指示。
2. **AI 生成**: AI が Rust の安全性規約（ownership, type safety）に準拠したコードを生成。
3. **人間によるレビュー**: 創業者が全コミットを精読。
4. **自動検証**: CI パイプライン（clippy, test, cargo-audit, TLA+）による 5 層のガードレールを通過時のみマージ。

## 4. 証拠アーティファクト
- 全 Git コミット履歴
- GitHub Actions の CI 実行ログ（365日間保存）
- ScanCode による深層スキャンレポート
- cargo-deny 監査レポート
