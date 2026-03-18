# コントリビューションガイド (CONTRIBUTING)

## 1. コミット規約
[Conventional Commits](https://www.conventionalcommits.org/) を採用しています。
- `feat`: 新機能
- `fix`: バグ修正
- `docs`: ドキュメントのみの変更
- `refactor`: コード変更（機能・修正なし）
- `test`: テスト追加・変更

## 2. 開発フロー
1. コード変更を行う前に、プロジェクトルートで `/preflight` を実行し影響範囲とベースラインテストを確認する。
2. `main` ブランチから機能ブランチ（`feat/some-feature`）を作成。
3. コードを書き、テストを通す（`cargo check --workspace --tests && cargo test --workspace`）。
4. ドキュメント更新：新規構造体・ファイルがある場合は必ず `.context/RIPPLE_MAP.md` を更新する。
5. PR を作成し、CI がパスすることを確認。
6. 創業者によるコードレビューを受け、Approve を得る。

## 3. レビュー基準
- `unwrap()` / `expect()` をパブリックAPIで使用していないか。
- `missing_docs` 警告が出ていないか。
- 新規ファイルや重要な設計判断をした場合、`.context/RIPPLE_MAP.md` と `docs/decisions/` (ADR) に記録されているか。（コンテキストの維持）
- ファイルヘッダーにライセンス（Apache 2.0、またはSamsara Hubの場合はBSL 1.1）が記載されているか。
- `build.rs` を新規追加・変更していないか（セキュリティ上の重要項目）。

## 4. セキュリティ
脆弱性を発見した場合は、issue ではなく、[CTOへの連絡手段] へ直接報告してください。
