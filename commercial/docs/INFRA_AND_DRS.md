# インフラ基盤と災害復旧計画

## 1. IaC (Infrastructure as Code)
- 本番インフラは `Terraform` を用いて完全自動定義されています。
- 手動プロビジョニングは禁止。
- 変更は PR ベースで行い、CI 経由でのみ反映。

## 2. 災害復旧 (DR) 戦略
- **RPO (Recovery Point Objective)**: 1時間。
  - PostgreSQL の WAL アーカイブを利用した PITR（Point-in-Time Recovery）。
- **RTO (Recovery Time Objective)**: 4時間。
  - Terraform による全環境のゼロからの再構築。

## 3. リージョン冗長性
- Phase 3（PMI +12〜18ヶ月）において、AWS 東京リージョンと大阪リージョンを用いたマルチリージョン構成を計画中。
