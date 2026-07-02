# エラーバジェットと障害復旧計画

## 1. SLA (Service Level Agreement) 目標
買収側チームおよび利用ユーザーへの安定性の約束として、以下の SLO (Service Level Objectives) を掲げます。

- **可用性 (Availability)**: 99.9% 以上
- **トランザクション成功率**: 99.95% 以上
- **P99 レイテンシ**: < 200ms（決済操作）

## 2. 耐障害性設計 (Resilience)

### 2.1 指数バックオフ
ネットワークエラーや DB ロック（SQLite `BUSY`）に対し、100ms, 200ms, 400ms の指数バックオフリトライを実施。

### 2.2 冪等性保証
すべてのトランザクションはユニークな `idempotency_key` を付与し、不慮のリトライによる二重決済を物理的に防止。内部 S2S API（`/internal/transfer`, `/internal/instant-refund`, `/internal/withdraw-points`）も `IdempotencyGate` 経由で同一キーの重複実行を拒否し、失敗時はキーを解放して正当なリトライを許可する。

### 2.3 暴走ストッパー (EconomyInterceptor)
単一取引額、日次合計、残高のしきい値に基づき、ロジック上の「暴走」から資産を守る物理的な防壁（Guardrail）を実装。

## 3. 災害復旧 (DR) 戦略

### 3.1 目標値
- **RPO (Recovery Point Objective)**: 1時間以内（データの最大損失時間）
- **RTO (Recovery Time Objective)**: 4時間以内（サービス復旧までの最大時間）

### 3.2 復旧手順
1. IaC (Terraform) により新規インフラをプロビジョニング。
2. バックアップからデータベースをリストア。
3. デッドマンズスイッチ（`DEADMANS_SWITCH.md`）記載の秘密鍵等を KMS から展開。
4. プロパティベーステスト実行により、復旧後の整合性を確認。
