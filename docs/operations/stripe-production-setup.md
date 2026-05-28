# Stripe 本番運用設定ガイド

Stripe の本番アカウント申請承認に伴い、Aiome 課金システムを本番（実決済）モードへ切り替えるための設定手順です。

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
# 本番実決済を有効化するための本番用シークレットキー
STRIPE_API_KEY="sk_live_xxxx" # gitleaks:allow

# Webhook 署名検証用のシークレット
STRIPE_WEBHOOK_SECRET="whsec_live_xxxx" # gitleaks:allow

# 本番モードをオンにするため、必ず false に設定
STRIPE_TEST_MODE="false"

# Stripe Dashboard で取得した月額サブスクの価格 ID
STRIPE_PRICE_SUBSCRIPTION_MONTHLY="price_xxxx"
```

> [!CAUTION]
> - `STRIPE_TEST_MODE="false"` に設定されると、起動時プリフライトチェックで `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` が未設定の場合は**サーバーの起動を拒否**する安全ガードが稼働します。
> - 本番モード下では、テスト用のモック署名 (`whsec_test`) による Webhook リクエストはすべて**厳格に拒否**されます。

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
