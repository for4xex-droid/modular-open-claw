# NT-6 / R5-3 — Public Beta ロールバック草案

> **作成**: 2026-07-14（NT-6 Part A）  
> **用途**: リリース草案 / Issue 転記用。空欄禁止の実体。  
> **非目標**: 公開承認（R5-5）・タグ打ち。

## ロールバック

### Feature Flag / 課金停止
- Stripe Dashboard で Webhook を一時無効化、または対象 Price を archive して新規課金を止める。
- 本番 compose は `STRIPE_*` を key-proxy / ホスト `.env` 経由で注入（`docker-compose.production.yml` — キー直書き禁止）。
- 緊急時は `STRIPE_TEST_MODE=true` へ戻し、MC / Nurture の Pro 導線を止める。
- Vault: `VAULT_MASTER_PASSWORD` / `VAULT_SECRET` はホスト `.env` のみ。漏洩時は Vault 内 Stripe 秘密のローテーション + key-proxy 再注入。

### Live 課金停止（OP-084 / 方針 B → 緊急停止）

> 正本手順の補足。秘密はチャット・Issue に書かない。

1. **新規課金の遮断（即時）**
   - Stripe Dashboard（**Live**）→ Payment Link / Price を **archive or deactivate**
   - LP CTA: `docs/landing/src/components/Pricing.tsx` の `buy.stripe.com/...` を Coming Soon または Test 用に差し替え → Deploy Landing Page
2. **API の Fail-Closed 退避**
   - ホスト env: `STRIPE_TEST_MODE=true`（または Live Price を外して preflight で起動拒否させる）
   - api-server 再起動。Vault の `sk_live_` は残してよいが、新規 Checkout を止めるなら Dashboard 側 archive を優先
3. **既存 subscriber の扱い**
   - 既に Live で契約中の顧客は Stripe Customer Portal / Dashboard で個別対応（即時キャンセル vs 期間末）。返金は `CANCELLATION_POLICY.md` に従う
   - Webhook は残す場合: `customer.subscription.deleted` → suspend が既実装のため、キャンセル後の Pro 剥奪は自動
4. **監視**
   - Stripe Dashboard → Developers → Webhooks → 失敗配信を確認
   - api-server ログで `commerce/webhook` 4xx/5xx を確認

### git revert
- タグ付与前: 問題コミットを `git revert <sha>`（単一）または範囲 revert。force-push 禁止。
- タグ付与後: hotfix ブランチ → PR → **新タグ**。既存タグは動かさない。
- 正本ブランチ: `main`（`origin/main`）。

### DB 復元
- 手順正本: [`docs/operations/BACKUP.md`](../operations/BACKUP.md)
- Pre-migration: `*.pre_migration.bak` から復元（BACKUP.md「Restoring a Pre-Migration Snapshot」）。
- 定期バックアップ: `scripts/backup.sh` の tar から復元（同「Restoring from an Automated Tar Archive」）。
- Postgres 検証スタック: `scripts/verify-production-postgres.sh`（検証用のみ。本番データを破壊試験しない）。

### 公開中止条件（NT-6）
- release-preflight いずれか FAIL（特に gitleaks / DAG / ignored / vendor除外追跡 >2500 / サイズ >75MB）
- Human の「公開してよい」未受領
