# Stripe 本番運用設定ガイド

Stripe の本番アカウント申請承認に伴い、Aiome 課金システムを本番（実決済）モードへ切り替えるための設定手順です。

**最終更新: 2026-07-16** — OP-084 向け: Pro 価格表記を **$19.99 USD/月** に同期 + §5 監視手順。release_master_plan **R2-1** / NT-1 / OP-084 L3 の正本。

> **OP-057-R チェックリスト（本番反映）**
> 0. [ ] **環境**: 本番に distroless イメージをデプロイ（[`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-1 **Step 0** を実行。`restart` だけではイメージは更新されません）
> 1. [ ] **秘密**: AbyssVault に `STRIPE_API_KEY` / `STRIPE_WEBHOOK_SECRET` を格納し、api-server を再起動（§2.A）
> 2. [ ] **非秘密**: `STRIPE_TEST_MODE=false` / `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` をホスト env または compose パススルーで設定（§2.B）
> 3. [ ] ホストに `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` を設定（§2.B）。`VITE_STRIPE_PRICE_ID` は任意（未設定時は `price_gold_monthly` エイリアス → §2.1）
> 4. [ ] Stripe Dashboard Webhook 登録（§3）
> 5. [ ] 本番 API が実 Price ID を返すこと + **テスト決済 1 件で Pro unlock**（DoD: R2-1 / NT-1）
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
   - **金額**: **$19.99 USD / 月**（Aiome Pro。JPY 併記しない — 為替誤表示防止。正本: `docs/legal/TOKUSHOHO.md`）
4. 保存後、商品詳細ページに表示される **`price_` から始まる「価格 ID」** (Price ID) をコピーして記録します。
   - ※この値は環境変数 `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` に設定します。

---

## 2. 秘密情報と非秘密設定の分離

本番 compose（`docker-compose.production.yml`）は **「No API keys in environment」** 設計です。  
api-server は起動時に `shared::security::fetch_and_inject_secrets()` で key-proxy（AbyssVault）から許可キーを注入します（`ALLOWED_VAULT_SECRETS` に `STRIPE_API_KEY` / `STRIPE_WEBHOOK_SECRET` 済み）。

| 種別 | 変数 | 正本の格納先 | compose への直書き |
|---|---|---|---|
| **秘密** | `STRIPE_API_KEY` | **AbyssVault**（**推奨: MC GUI** / 上級: `abyss-vault set`） | **禁止**（Zero-Trust） |
| **秘密** | `STRIPE_WEBHOOK_SECRET` | **AbyssVault**（同上） | フォールバックとして env 可（空推奨） |
| **非秘密** | `STRIPE_TEST_MODE` | ホスト `.env` / compose パススルー | 可（Vault 対象外） |
| **非秘密** | `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` | ホスト `.env` / compose パススルー | 可（Vault 対象外） |

詳細な Vault 操作は [api_key_rotation.md](api_key_rotation.md)（**§B GUI 推奨** / §C CLI）を参照してください。Human 向けコピペは [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-1。  
**本番 MC（Vault GUI）**: `docker-compose.production.yml` の **api-server = `docker/distroless.Dockerfile`（実施済）**。Human は [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-1 **Step 0** で `build`/`up`/稼働 Labels 確認してから GUI に入ること（`restart` だけでは旧イメージのまま）。

### 2.A 秘密（必須・AbyssVault）

#### 推奨 — Management Console GUI（本番 compose）

本番 MC に管理者ログイン（通常 `https://<YOUR_DOMAIN>/`。quickstart の 1420 ではない）→ **まもる・整える → 設定 → Abyss Vault シークレットマネージャ** で `STRIPE_API_KEY` / `STRIPE_WEBHOOK_SECRET` を設定します。API は key-proxy 経由のため、**本番 api-server が読む Vault DB と同一**です（[api_key_rotation.md](api_key_rotation.md) §B）。

#### 上級 — CLI（同一 DB が保証できるときのみ）

ホストで `cargo run --bin abyss-vault` する場合、`ABYSS_VAULT_PATH` / `VAULT_MASTER_PASSWORD` / `CELL_ID` が **key-proxy コンテナと同一**でないと、別 DB に書いて「入ったつもり」になります。迷ったら GUI のみ。

```bash
# 本番 live/test キーを Vault に格納（値はシェル履歴に残さないよう対話入力推奨）
cargo run --bin abyss-vault -- set STRIPE_API_KEY
cargo run --bin abyss-vault -- set STRIPE_WEBHOOK_SECRET

# 確認（値は表示されない想定の status / list）
cargo run --bin abyss-vault -- status
```

反映は **起動時のみ**です。格納後は api-server（および key-proxy 依存サービス）を **再起動**してください。

> [!CAUTION]
> 本番 compose の `api-server.environment` に `STRIPE_API_KEY` を追加しないでください。既存の Zero-Trust 設計（key-proxy 経由）と矛盾し、平文キー拡散のリスクがあります。

### 2.B 非秘密（必須・env / compose）

ホストの `.env`、または `docker-compose.production.yml` の api-server パススルーに以下を設定します。

```bash
# === Stripe Production — non-secrets ===
STRIPE_TEST_MODE="false"
STRIPE_PRICE_SUBSCRIPTION_MONTHLY="price_xxxx"
```

> [!IMPORTANT]
> compose で `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` を未設定のまま起動すると、変数は空文字 `""` としてコンテナに入ります。`STRIPE_TEST_MODE=false` かつ Stripe キー注入済みの場合、api-server の preflight が起動を拒否します（Fail-Closed）。必ず実 Price ID をホスト env に入れてから `docker compose up` してください。
>
> `STRIPE_WEBHOOK_SECRET` も同様に、Vault 未設定かつ compose/host env が空だと署名検証不能になります。Vault に値がある場合は起動時 `fetch_and_inject_secrets` が空の compose env を上書きします。Nurture（`STRIPE_SECRET_KEY`）は別系統のため、本手順の api-server Vault 設定だけでは Nurture 側は埋まりません。

（任意）開発・非 compose ホスト向けのフォールバックとして `.env` に秘密を書く場合は、平文の長期保管を避け、可能な限り Vault へ移行してください。

```bash
# 非推奨（本番 compose）: 平文 .env への live キー直書き
# STRIPE_API_KEY="sk_live_xxxx"   # → 代わりに §2.A
# STRIPE_WEBHOOK_SECRET="whsec_live_xxxx"  # → 代わりに §2.A
```

### 2.1 management-console フロントエンド（`VITE_STRIPE_PRICE_ID`）

Pro アプリ内 Checkout（`ProUpgradeModal` / `useCheckoutSession`）はビルド時に Vite 環境変数を埋め込みます。

| 環境変数 | 設定先 | 用途 |
|---|---|---|
| `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` | api-server 非秘密 env / compose | Checkout Session 作成・Webhook 照合（**必須**） |
| `VITE_STRIPE_PRICE_ID` | `apps/management-console/.env` または CI secrets | フロント Checkout リクエストの `price_id`（任意） |

**エイリアス（本番 Docker 向け）**: `VITE_STRIPE_PRICE_ID` 未設定時、フロントはデフォルト `price_gold_monthly`（`config.ts`）を送ります。api-server はこれをホストの `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` に解決します。したがって **ホスト Price が正しければ、VITE 未焼き込みでも Checkout は動作**します。厳密に同一文字列をフロントに焼き込む場合のみ再ビルドしてください。

```bash
# apps/management-console/.env（任意・本番ビルド / CI）
VITE_STRIPE_PRICE_ID="price_xxxx"   # STRIPE_PRICE_SUBSCRIPTION_MONTHLY と同一値
```

**確定値（2026-07-05、OP-057）**: `price_1TpXFpBcUTwo5TwLmK9SQbKL`（Pro $19.99/月）。開発手順の詳細は [stripe-setup.md](stripe-setup.md) §2.5 を参照。

> [!NOTE]
> Vite 変数を使う場合、変更後は **management-console の再ビルド・再デプロイ** が必要です。エイリアス経路のみなら api-server 側のホスト Price 更新で足ります。

> [!CAUTION]
> - `STRIPE_TEST_MODE="false"` に設定されると、起動時プリフライトチェックで `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` が未設定の場合は**サーバーの起動を拒否**する安全ガードが稼働します。
> - 本番モード下では、テスト用のモック署名 (`whsec_test`) による Webhook リクエストはすべて**厳格に拒否**されます。
> - **Webhook シークレットの移行・ローテーション**: `STRIPE_WEBHOOK_SECRET` はカンマ区切りでの複数設定に対応しています。詳細は [api_key_rotation.md](api_key_rotation.md) を参照してください。

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

## 5. Live 後の監視（OP-084 L5-2）

| 頻度 | 確認先 | 見るもの |
|---|---|---|
| 切替直後 24h | Stripe Dashboard → Developers → Webhooks | 失敗配信（4xx/5xx）・署名エラー |
| 切替直後 24h | api-server ログ | `commerce/webhook` 拒否・unlock/suspend |
| 週次（任意） | Stripe Dashboard → Payments / Customers | 異常返金・dispute |
| アラート（Human） | Stripe Dashboard → Settings → Notifications | 決済失敗・dispute メールを有効化 |

緊急停止手順: [`docs/releases/NT6_R5_ROLLBACK_DRAFT.md`](../releases/NT6_R5_ROLLBACK_DRAFT.md)「Live 課金停止」。
