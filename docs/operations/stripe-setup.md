# Stripe設定手順書

Aiomeの自律経済圏（Commerce）機能でStripeを利用するための設定手順です。

**最終更新: 2026-07-10**

> **本番（live）**: 秘密は AbyssVault、非秘密は env/compose。手順の正本は [`stripe-production-setup.md`](stripe-production-setup.md)（near_term NT-1）。本ドキュメントは主に **開発・テストモード** 向けです。

## 1. Stripeアカウントの準備とAPIキーの取得
1. [Stripeダッシュボード](https://dashboard.stripe.com/)にログインします（開発時はテストモードを使用してください）。
2. **「開発者」 > 「APIキー」** に移動します。
3. 以下のキーを取得します。
   - **公開可能キー (Publishable key)**
   - **シークレットキー (Secret key)**: `sk_test_...` (テスト用) または `sk_live_...` (本番用。本番格納は Vault — 上記リンク)

## 2. 商品と価格 (Price ID) の作成
Proプラン（$19.99/月）の定期支払い用の価格IDを作成します。
1. **「商品」 > 「商品を追加」** に移動します。
2. 商品情報を入力します。
   - **商品名**: `Aiome Autonomous Pro`（Checkout 表示名と揃える）
   - **料金体系**: 定期支払い
   - **価格**: `19.99` (通貨: USD)
   - **請求周期**: 毎月
   - **無料トライアル**: **現時点では未設定（アプリ/LP でも謳わない）**。Dashboard でトライアルを付ける場合は、Price / Payment Link / アプリ表示を必ず一致させること
3. 商品を保存すると、`price_` から始まる **価格ID (Price ID)** が生成されます（例: `price_1Pxxx...`）。
4. このIDをコピーし、環境変数 `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` および管理コンソールの `VITE_STRIPE_PRICE_ID` に設定します。

## 2.5 LP Payment Link（aiome.dev Pro CTA）

LP の「プロへアップグレード」は Stripe Payment Link を直リンクしています（`docs/landing/src/components/Pricing.tsx`）。

| 項目 | 値 |
|---|---|
| **現行 Payment Link** | `https://buy.stripe.com/aFa00i9cEaVE4ay4y9f7i03` |
| **Payment Link ID** | `plink_1TpXHCBcUTwo5TwLnO1BJneY` |
| **Price ID（Pro $19.99/月）** | `price_1TpXFpBcUTwo5TwLmK9SQbKL` |
| **mode** | **Live**（`livemode=true`。OP-084 L3-1 CLI 2026-07-16 + L4 実カード 2026-07-17〜18。詳細は [`MESSAGING.md`](../marketing/MESSAGING.md) §9） |
| **価格** | $19.99/月（自動更新・いつでも解約可。**無料トライアルは現時点で提供しない**） |
| **旧 Link（参照禁止）** | `https://buy.stripe.com/aFa9AS1Kc1l47mK3u5f7i01` — Stripe 側 **inactive**（2026-07-05 以降 LP は上記 Live URL のみ） |

**本番反映**: `docs/landing/**` 変更は `main` push → `.github/workflows/deploy-landing.yml` が GitHub Pages（https://aiome.dev）へデプロイ。**LP CTA 配線の正本は `Pricing.tsx`**。URL 差し替え時は MESSAGING §9 / 本節 / `Pricing.link.test.tsx` を同期すること。

**検証**: 新 Link をブラウザで開き、Checkout に「Aiome Autonomous Pro」「$19.99/month」が表示され、**意図しない「14 days free」が残っていないこと**を確認（残っていれば Dashboard でトライアルを外すか、文書側をトライアルありに合わせる）。日本 IP では JCT 込み **$21.99/月** 表示の場合あり。

**Price ID 整合（2026-07-05 確定）**: 上記 Payment Link に紐づく Price ID は **`price_1TpXFpBcUTwo5TwLmK9SQbKL`**。以下に同一値を設定すること（OP-057）。

| 環境変数 | 設定先 |
|---|---|
| `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` | api-server `.env` |
| `VITE_STRIPE_PRICE_ID` | management-console ビルド時（`.env` または CI secrets） |

## 3. Webhook の設定
決済状態やサブスクリプションの更新イベントをリアルタイムに受け取るためのWebhookを設定します。
1. **「開発者」 > 「Webhook」** に移動します。
2. **「エンドポイントを追加」** をクリックします。
3. 以下の項目を設定します。
   - **エンドポイントURL**: 
     - 開発時（ローカル）: `stripe listen --forward-to localhost:3015/api/v1/commerce/webhook` のようなローカルフォワーディングで取得したアドレス。
     - 本番環境: `https://yourdomain.com/api/v1/commerce/webhook`
   - **送信するイベント**: 以下の8つのイベントを選択して登録します。
     - `checkout.session.completed` (チェックアウト完了、ライセンス付与)
     - `invoice.paid` (サブスクリプション更新成功)
     - `invoice.payment_failed` (決済失敗)
     - `customer.subscription.deleted` (サブスクリプション削除・解約)
     - `customer.subscription.updated` (サブスクリプション更新)
     - `charge.dispute.created` (チャージバック警告)
     - `checkout.session.expired` (セッション期限切れ)
4. 登録後、**「署名シークレット」 (Signing secret)** を表示し、`whsec_...` から始まるキーをコピーします。
5. このキーを環境変数 `STRIPE_WEBHOOK_SECRET` に設定します。
   - **移行・ローテーション時**: Stripe Webhook v2 移行やキー更新の際は、カンマ区切りで複数のシークレットを指定できます（例: `whsec_old...,whsec_new...`）。これにより新旧の Webhook イベントを同時に処理し、ダウンタイムゼロで移行可能です。

## 4. 環境変数の設定
取得したキーを `.env` ファイルに設定します。

```bash
# === Stripe設定 ===
# StripeのAPIキー（未設定の場合は自動でMockモードになります。v2 thin event 解決時に使用されます）
STRIPE_API_KEY="sk_test_your_key_here"

# Stripe Webhookの署名シークレット (移行時はカンマ区切りで複数指定可能)
STRIPE_WEBHOOK_SECRET="whsec_your_secret_here"

# テストモードの有効化 (true/false)
STRIPE_TEST_MODE="true"

# 毎月の定期購読用 Price ID（Payment Link / ProUpgradeModal と同一 Price を指定）
STRIPE_PRICE_SUBSCRIPTION_MONTHLY="price_your_gold_monthly_id_here"
```
