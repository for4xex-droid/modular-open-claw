# Stripe 本番運用設定ガイド

Stripe の本番アカウント申請承認に伴い、Aiome 課金システムを本番（実決済）モードへ切り替えるための設定手順です。

**最終更新: 2026-07-06** — release_master_plan **R2-1** 手順書（Human 作業の正本）。凍結台帳 **OP-057-R** チェックリストと対応。

> **OP-057-R チェックリスト（本番 env 反映）**
> 1. [ ] api-server 本番ホストに `STRIPE_API_KEY` / `STRIPE_WEBHOOK_SECRET` / `STRIPE_TEST_MODE=false` / `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` を設定（§2）
> 2. [ ] management-console 本番ビルドに `VITE_STRIPE_PRICE_ID` を設定（§2.1）— **api-server と同一 Price ID**
> 3. [ ] Stripe Dashboard Webhook 登録（§3）
> 4. [ ] 本番 API が実 Price ID を返すことを確認（DoD: release_master_plan R2-1）
>
> 決済→Pro 自動有効化（OP-057-R (2)）は 2026-07-05 コード完了。デプロイ前の人間レビューは `OPEN.md` OP-057-R を参照。

---

## 1. Stripe Dashboard での事前準備

### 商品 (Product) と価格 (Price ID) の作成
1. **Stripe Dashboard** (https://dashboard.stripe.com) にログインします。
2. 左メニューの「商品 (Products)」から「商品を追加」をクリックします。
3. 以下の情報を入力します：
   - **商品名**: 例「Aiome Monthly Subscription」
   - **価格モデル**: 「継続 (Recurring)」
   - **請求期間**: 「月次 (Monthly)」
   - **金額**: プラットフォーム規定のサブスクリプション金額（例: ¥2,980/月）
4. 保存後、商品詳細ページに表示される **`price_` から始まる「価格 ID」** (Price ID) をコピーして記録します。
   - ※この値は環境変数 `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` に設定します。

---

## 2. 環境変数の設定 (`.env`)

本番環境のサーバーで `.env` ファイルに以下の本番用設定を追記・変更します。
セキュリティ担保のため、ライブ API キー (`sk_live_` など) やシークレットの取り扱いには十分注意してください。

```bash
# === Stripe Production Configuration ===
# 本番実決済を有効化するための本番用シークレットキー (v2 thin event 自動解決に必要)
STRIPE_API_KEY="sk_live_xxxx" # gitleaks:allow

# Webhook 署名検証用のシークレット (移行時はカンマ区切りで複数指定可能)
STRIPE_WEBHOOK_SECRET="whsec_live_xxxx" # gitleaks:allow

# 本番モードをオンにするため、必ず false に設定
STRIPE_TEST_MODE="false"

# Stripe Dashboard で取得した月額サブスクの価格 ID
STRIPE_PRICE_SUBSCRIPTION_MONTHLY="price_xxxx"
```

### 2.1 management-console フロントエンド（`VITE_STRIPE_PRICE_ID`）

Pro アプリ内 Checkout（`ProUpgradeModal` / `useCheckoutSession`）はビルド時に Vite 環境変数を埋め込みます。**api-server の `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` と同一の Price ID** を設定してください。

| 環境変数 | 設定先 | 用途 |
|---|---|---|
| `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` | api-server 本番 `.env` | Checkout Session 作成・Webhook 照合 |
| `VITE_STRIPE_PRICE_ID` | `apps/management-console/.env` または CI secrets | フロント Checkout リクエストの `price_id` |

```bash
# apps/management-console/.env（本番ビルド / CI）
VITE_STRIPE_PRICE_ID="price_xxxx"   # STRIPE_PRICE_SUBSCRIPTION_MONTHLY と同一値
```

**確定値（2026-07-05、OP-057）**: `price_1TpXFpBcUTwo5TwLmK9SQbKL`（Pro $19.99/月）。開発手順の詳細は [stripe-setup.md](stripe-setup.md) §2.5 を参照。

> [!NOTE]
> Vite 変数はビルド時に静的埋め込みされます。Price ID 変更後は **management-console の再ビルド・再デプロイ** が必要です。

> [!CAUTION]
> - `STRIPE_TEST_MODE="false"` に設定されると、起動時プリフライトチェックで `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` が未設定の場合は**サーバーの起動を拒否**する安全ガードが稼働します。
> - 本番モード下では、テスト用のモック署名 (`whsec_test`) による Webhook リクエストはすべて**厳格に拒否**されます。
> - **Webhook シークレットの移行・ローテーション**: `STRIPE_WEBHOOK_SECRET` はカンマ区切りでの複数設定に対応しています（例: `whsec_live_old...,whsec_live_new...`）。Stripe 側で Webhook 宛先を切り替える際、両方の署名を同時に有効にすることで、ダウンタイムなしに安全にキーのローテーションが行えます。詳細な手順は [api_key_rotation.md](file:///Users/motista/Desktop/antigravity/aiome/docs/operations/api_key_rotation.md) を参照してください。

---

## 3. Webhook エンドポイントの登録と受信イベント

Stripe から決済結果や解約イベントなどをリアルタイムに受信するため、Webhook エンドポイントを登録します。

### 登録手順
1. Stripe Dashboard の「開発者 (Developers)」→「Webhook」メニューへ移動します。
2. 「エンドポイントを追加」をクリックします。
3. 以下の情報を入力します：
   - **エンドポイント URL**: `https://<YOUR_DOMAIN>/api/v1/commerce/webhook`
   - **受信するイベント**: 以下の **7 つのイベント** を必ず選択して追加します。

### 受信すべき必須イベント一覧

| イベント名 | Aiome 側のアクション | 目的 |
|---|---|---|
| `checkout.session.completed` | 新規サブスクリプション開始・ライセンス付与 | 初回購入の完了検知 |
| `invoice.paid` | サブスク継続の支払い成功・アカウント制限解除 | 毎月の支払成功によるアクティブ維持 |
| `invoice.payment_failed` | サブスク決済失敗・アカウント一時停止 (suspend) | 支払不履行によるサービス制限 |
| `customer.subscription.deleted` | サブスクリプション解約・アカウント一時停止 (suspend) | 解約完了時のアクセス剥奪 |
| `customer.subscription.updated` | ステータス変化（`past_due` など）に応じた suspend/unlock 制御 | プランやステータス変更の同期 |
| `charge.dispute.created` | 該当エージェントの即時 suspend + SSE ブロードキャスト | チャージバック（不正利用申告）発生時の安全対策 |
| `checkout.session.expired` | セッション期限切れのログ記録 | 未完了チェックアウトのクリーンアップ |

---

## 4. 本番運用の安全性・堅牢性設計

Aiome 課金システムは、以下の堅牢設計原則 (Security Hardening) に基づいて実装されています：

1. **二重課金防止 (No Double-Billing)**
   - `CostCircuitBreaker` による事前 USD コスト上限チェックと、`calculate_cost_coins` による事後コイン課金は同一ジョブ内で排他的に作用し、二重課金を絶対に引き起こしません。
2. **Webhook の冪等性保証 (Idempotency)**
   - `stripe_webhook_events` テーブルを用いた一意のトランザクション管理により、Stripe からの重複した Webhook 送信による二重ライセンス付与や状態矛盾を自動で防ぎます。
3. **不正・チャージバック対策 (Anti-Dispute Guard)**
   - `charge.dispute.created` を受信した際、即座に該当エージェントのアカウントを `suspend` 状態に遷移させ、SSE 経由で排他的に `dispute_received` イベントをリアルタイムブロードキャストして防御します。
