# Stripe設定手順書

Aiomeの自律経済圏（Commerce）機能でStripeを利用するための設定手順です。

## 1. Stripeアカウントの準備とAPIキーの取得
1. [Stripeダッシュボード](https://dashboard.stripe.com/)にログインします（開発時はテストモードを使用してください）。
2. **「開発者」 > 「APIキー」** に移動します。
3. 以下のキーを取得します。
   - **公開可能キー (Publishable key)**
   - **シークレットキー (Secret key)**: `sk_test_...` (テスト用) または `sk_live_...` (本番用)

## 2. 商品と価格 (Price ID) の作成
Proプラン（$9.99/月）の定期支払い用の価格IDを作成します。
1. **「商品」 > 「商品を追加」** に移動します。
2. 商品情報を入力します。
   - **商品名**: `Aiome Pro` (任意の名称)
   - **料金体系**: 定期支払い
   - **価格**: `9.99` (通貨: USD)
   - **請求周期**: 毎月
3. 商品を保存すると、`price_` から始まる **価格ID (Price ID)** が生成されます（例: `price_1Pxxx...`）。
4. このIDをコピーし、環境変数 `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` に設定します。

## 3. Webhook の設定
決済状態やサブスクリプションの更新イベントをリアルタイムに受け取るためのWebhookを設定します。
1. **「開発者」 > 「Webhook」** に移動します。
2. **「エンドポイントを追加」** をクリックします。
3. 以下の項目を設定します。
   - **エンドポイントURL**: 
     - 開発時（ローカル）: `stripe listen --forward-to localhost:3015/api/v1/ekyc/webhook` のようなローカルフォワーディングで取得したアドレス。
     - 本番環境: `https://yourdomain.com/api/v1/ekyc/webhook`
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

## 4. 環境変数の設定
取得したキーを `.env` ファイルに設定します。

```bash
# === Stripe設定 ===
# StripeのAPIキー（未設定の場合は自動でMockモードになります）
STRIPE_API_KEY="sk_test_your_key_here"

# Stripe Webhookの署名シークレット
STRIPE_WEBHOOK_SECRET="whsec_your_secret_here"

# テストモードの有効化 (true/false)
STRIPE_TEST_MODE="true"

# 毎月の定期購読用 Price ID
STRIPE_PRICE_SUBSCRIPTION_MONTHLY="price_your_gold_monthly_id_here"
```
