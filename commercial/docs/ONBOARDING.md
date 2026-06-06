# オンボーディングガイド (ONBOARDING)

## 1. 開発環境のセットアップ

### 必須ツール
- **Rust**: 1.75 以上 (rustup)
- **SQLx CLI**: `cargo install sqlx-cli`
- **Docker**: (PostgreSQL 検装用)
- **Tla+ Tools**: (形式検証用、VSCode 拡張推奨)

### 手順
```bash
git clone git@github.com:motivationstudio/modular-open-claw.git
cd modular-open-claw
cargo sqlx database setup
cargo test
```

## 2. アーキテクチャの概要
`docs/adr/` を 001 から順に精読してください。特に以下の 3 点が重要です。
- **Layered Arch**: 依存関係は protocol -> core -> infra -> api。
- **Typestate**: `Transaction<S>` の型パラメータによる状態遷移管理。
- **M2M Economy**: AiomeCoin と CreatorPoints の役割。

## 3. 最初のタスク
1. `nurture-core` の単体テストを一つ追加し、PR を作成する。
2. `clippy` 警告がゼロであることを確認する。
3. 創業者のレビューを受け、Conventional Commits に従いマージ。

## 4. 連絡先
- **CTO (創業者)**: [メールアドレス/Slack/Discord]
