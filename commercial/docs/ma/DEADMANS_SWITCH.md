# 事業継続計画（DEADMANS SWITCH）

## 1. コンテキスト
本ドキュメントは、創業者（私）が事故等の理由で突発的に業務不能となった場合でも、買収先企業が 72 時間以内に事業を継続・復旧できるようにするためのガイドです。

## 2. 資産の所在

### ソースコード
- **GitHub Organization**: `motivationstudio-llc`
- **権限設定**: 買収完了後、速やかに買収側管理者へ Owner 権限を譲渡。

### シークレット管理 (KMS)
- **AWS KMS / HashiCorp Vault**: 本番環境の `API_SERVER_SECRET`, `STRIPE_API_KEY` 等はここで管理されている。
- **マスターキー**: [買収先指名担当者] のみがアクセス可能な物理金庫/セーフティボックスに復元コードを保管済み。

### インフラ (IaC)
- **Terraform 構成**: `/infra` ディレクトリにすべて格納。
- **デプロイ**: `cargo make deploy` コマンド一つで全環境が再構築可能。

## 3. 緊急アクションプラン (72時間)
1. `docs/ONBOARDING.md` に従い、開発者用環境を構築。
2. `cargo check && cargo test` により、コードベースの整合性を確認。
3. `docs/adr/` の 8 つのファイルを精読し、設計意図を把握。
4. GitHub Actions の過去ログから直近 30 日間のデプロイ履歴を確認。
5. 既存の `EconomyInterceptor` の設定値（支出制限）を確認し、パニックモードに入っていないかチェック。
