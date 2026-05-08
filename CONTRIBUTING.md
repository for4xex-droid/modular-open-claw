# コントリビューションガイド (CONTRIBUTING)

Aiome（The Autonomous AI Operating System）へのコントリビューションをご検討いただきありがとうございます！
本プロジェクトは、堅牢な自律型エージェントの基盤構築を目指しており、「Zero-Panic」や「TDD主導」といった厳密な品質基準を設けています。以下のガイドラインに沿って開発を進めてください。

## 1. 開発環境のセットアップ

Aiome の開発を始めるための要件と手順です。

### 必須要件
- **Rust**: `1.75.0` 以上 (rustup 経由でのインストールを推奨)
- **Node.js**: `v20` 以上 (フロントエンド、MCPサーバー開発用)
- **データベース**: SQLite3 (開発環境の DB はメモリまたはローカルファイルで動作します)

### セットアップ手順
```bash
# 1. リポジトリのクローン
git clone https://github.com/motivationstudio-llc/aiome.git
cd aiome

# 2. Rust 依存関係のビルドとテスト
cargo build --workspace
cargo test --workspace

# 3. フロントエンド/MCPの依存関係のインストール
npm install
```
*※ DB マイグレーションはサーバー起動時（またはテスト実行時）に `sqlx::migrate!` によって自動的に適用されます。*

## 2. 開発フローと TDD (Test-Driven Development) 原則

Aiome では、コードの堅牢性を保証するために **TDD（テスト駆動開発）** を必須としています。

1. **ベースライン確認**: 開発を始める前に `/preflight` コマンド相当の確認を行い、`cargo test` が現状で 100% GREEN であることを確認してください。
2. **ブランチの作成**: `main` ブランチから機能ブランチ（`feat/your-feature-name`）を作成します。
3. **テストの記述 (RED)**: 実装前に、要件を満たすか検証するテスト（正常系および異常系）を先に記述し、失敗することを確認します。
4. **実装 (GREEN)**: テストを通すための最小限のコードを実装します。
5. **異常注入 (Negative Test) の必須化**: 単にエラーが出ない（exit code 0）ことを以て「機能している」と見なすことは**厳禁**です。意図的に不正な入力や障害を注入し、システムが正しくそれを検知・拒否するかを必ずテストしてください。
6. **リファクタリング**: コードを整理し、再度 `cargo test --workspace` が通ることを確認します。
7. **PR の作成**: 新規ファイルや重要な設計判断があった場合は、`.context/RIPPLE_MAP.md` および `docs/decisions/` (ADR) を更新した上で Pull Request を作成してください。

## 3. アーキテクチャとコーディング規約

### 🚫 厳格な Zero-Panic ポリシー
Aiome は常時稼働する自律システムです。**パブリック API や業務ロジック内で `unwrap()` や `expect()` を使用することは禁止されています。**
エラーは必ず `Result` 型で返し、`?` オペレータを用いて上位層に伝播させてください。

### 🛡️ Error 型の統一
エラーは `AiomeError` または各コンポーネント固有の `AppError`（`thiserror`を利用）へ変換し、インフラストラクチャレベルでのエラー原因を正確にトレースできるようにしてください。

## 4. コミット規約

[Conventional Commits](https://www.conventionalcommits.org/) を採用しています。
- `feat`: 新機能
- `fix`: バグ修正
- `docs`: ドキュメントの変更（README, CHANGELOG など）
- `refactor`: コード変更（バグ修正や機能追加を含まない構造変更）
- `test`: テストの追加・修正
- `chore`: ビルドプロセスや補助ツールの変更

コミットメッセージ例: `fix(api-server): resolve federation jitter sync panic`

## 5. レビュー基準

PR がマージされるためには、以下の品質ゲートをすべて通過する必要があります。

- `unwrap()` / `expect()` をパブリック API で使用していないか。
- `missing_docs` 警告が出ていないか。
- 新規ファイルや重要な設計判断をした場合、`.context/RIPPLE_MAP.md` と `docs/decisions/` (ADR) に記録されているか。
- ファイルヘッダーに適切なライセンス表記（下記セクション参照）が含まれているか。
- `build.rs` を新規追加・変更していないか（セキュリティ上の重要項目。変更が必要な場合は事前に Issue で議論してください）。
- `cargo check --workspace --tests && cargo test --workspace` が 100% GREEN であるか。
- 創業者によるコードレビューで Approve を得ているか。

## 6. ライセンスの取り扱い

Aiome はモジュールによって異なるライセンスが適用されています。新規ファイルを作成する際は、必ずファイルヘッダーに以下の適切なライセンス表記を含めてください。

- **`libs/core`, `libs/contracts` など**: Apache License 2.0 (オープンソース基盤)
- **`apps/api-server`, `apps/samsara-hub` など**: Business Source License 1.1 (コアシステム・サーバー実装)

## 7. セキュリティ

脆弱性を発見した場合は、GitHub Issue で公開せず、[CTOへの直接連絡手段（設定されたセキュリティメールなど）] へ報告してください。

## 8. MCP エコシステムと自動化

Aiome では、GitHub 上の Issue 管理や情報収集を Model Context Protocol (MCP) を通じて自動化しています。

### GitHub MCP のセットアップ
1. `.env` ファイルに `GITHUB_PERSONAL_ACCESS_TOKEN` を設定してください（`.env.example` の `MCP Integrations` セクション参照）。
2. Aiome を起動すると、`discovery.rs` により `mcp_servers.json` が自動生成され、GitHub MCP が利用可能になります。
3. エージェントはこれを利用して、Issue の作成、コードレビューの自動化、およびタスクのトラッキングを自律的に行います。

ご不明な点があれば、Issue や Discussions でお気軽にご質問ください！
