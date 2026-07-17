# 実課金オープン計画（Stripe Live Switch Plan v1.2）

- **ステータス**: **✅ L0〜L5 完了（2026-07-18）** — Human H4 PASS（L3-2〜L4）+ Agent L5-3 台帳クローズ。本番は方針 B（Live）。任意フォロー: `billing_closeout_plan` **R4**（A2A 空文字ガードの次回 api-server rebuild）
- **対応 OP**: **OP-084**（OPEN.md ✅ 解決・2026-07-18）
- **目的**: Public Beta（v1.2.0、Stripe 方針 A = Test）から、**実通貨決済（方針 B = Live）** へ安全に切り替えるための全タスクを検証・定義する
- **正本関係**: タスク台帳は `OPEN.md`。切替手順のコピペ正本は [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](../guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) NT-1（方針 B 分岐）+ [`stripe-production-setup.md`](../operations/stripe-production-setup.md)。本計画は「Live 固有の追加タスク・順序・DoD」を定義する層

### 進捗（2026-07-18 クローズ）

| Phase | 状態 | 根拠 |
|---|---|---|
| L0-1 | ✅ 記録訂正 2026-07-16 | LP Link `aFa00i9cEaVE4ay4y9f7i03` = **`livemode=true`**。L3 完了により API/Webhook も Live 整合 |
| L0-2 | ✅ | スコープ = Pro $19.99/月のみ（§1） |
| L1-1 | ✅ | VoiceStore: 「Pro に登録する」+ KC 無償明記（OP-085） |
| L1-2 | ✅ | AiaaOnboardingWizard: `useAgentIdentity`（zero-UUID 禁止） |
| L1-3 | ✅ | 現行 LP CTA = 上記 Live Payment Link（差し替え不要） |
| L1-4 | ✅ | LP/MC `$19.99` 整合 |
| L1-5 | — | 任意・非ブロッカー（表示撤去済） |
| L2-1〜3 | ✅ | OP-085（COMPLIANCE_CHECKLIST ✅） |
| L2-4 | ✅ **2026-07-16 Human** | Live / 特商法一致 / Successful payments ON / card fail ON / Radar 既定 / Tax 見送り |
| L3-1 | ✅ **既存** | Live Product / Price **`price_1TpXFpBcUTwo5TwLmK9SQbKL`** / Payment Link 上記 |
| L3-2〜4 | ✅ **2026-07-17 Human+Agent** | Vault `sk_live`/`whsec`・env Live・正本 Webhook 7 イベント・legacy disabled |
| L4 | ✅ **2026-07-17〜18 Human** | L4-1 unlock / L4-2 偽署名 400 / L4-3 deleted→suspend / L4-4 返金+cancel→Free |
| L5-1 | ✅ | `NT6_R5_ROLLBACK_DRAFT.md` に Live 課金停止を追記 |
| L5-2 | ✅ 手順 | `stripe-production-setup.md` §5。Dashboard アラート設定は Human |
| L5-3 | ✅ **2026-07-18** | OPEN / CHANGELOG / RIPPLE_MAP / release_master_plan / closeout 計画同期 |

---

## 0. 検証結果サマリー（実コード照合・2026-07-14）

### 0.1 Live 対応済み（コード変更不要）

| 項目 | 根拠 |
|---|---|
| Fail-Closed 起動ガード（`STRIPE_TEST_MODE=false` + キー注入済 + Price 未設定 → 起動拒否） | `apps/api-server/src/bootstrap/preflight.rs` `validate_stripe_production_price_id`（単体テスト 4 本） |
| `price_gold_monthly` エイリアス → ホスト `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` 解決 | `apps/api-server/src/routes/commerce.rs` `resolve_price_id`（checkout / subscription 両経路） |
| Webhook 閉ループ: `invoice.paid`→unlock / `invoice.payment_failed`→suspend / `charge.dispute.created`→即 suspend + SSE | `commerce_webhook/invoice.rs` / `stripe.rs`（NT-4 E2E PASS 済） |
| Webhook 冪等性（`stripe_webhook_events`）・本番でのモック署名 `whsec_test` 拒否 | `commerce_webhook/stripe.rs`（NT-6 Negative 記録済） |
| リダイレクト URL の https + `ALLOWED_ORIGINS` ホワイトリスト検証 | `routes/commerce_helpers.rs` `validate_redirect_url` |
| 秘密の Zero-Trust 管理（AbyssVault → `fetch_and_inject_secrets`、compose 直書き禁止） | `docker-compose.production.yml` + `stripe-production-setup.md` §2 |
| RBAC / eKYC ゲート（subscription 作成は eKYC 必須、IDOR 拒否） | `commerce.rs` `create_subscription` / `create_checkout_session` |
| 有償 KC・自律購買の封印（`execute_autonomous_purchase` は Mock 封印、OP-011 で封印維持判定済） | `libs/aiome-commerce/src/stripe/mod.rs` `is_mock` / R3-3 |

### 0.2 Live オープンのブロッカー（本計画で解消する）

| # | 問題 | 根拠 | 影響 |
|---|---|---|---|
| B-1 | **LP Payment Link は Live 確定**（`plink_…` / `livemode=true`）。API/Webhook も Live 整合済（2026-07-18） | `Pricing.tsx` + Stripe API | ✅ **L3–L4 で解消** |
| B-2 | **VoiceStore「チャージ」導線の誤認リスク**（~~旧~~）→ **✅ L1-1 で解消**（Pro 登録明示 + KC 無償明記。Portal CTA 出し分けは 2026-07-17 MC 反映済） | `VoiceStore.tsx` / L1-1 | ✅ |
| B-3 | **AiaaOnboardingWizard の死に導線**。ダミー `agent_id`（zero UUID）で Checkout 要求 → RBAC `req.agent_id != auth.agent_id` で 403 確定 | `AiaaOnboardingWizard.tsx` L50 / `commerce.rs` L629 | 🟠 |
| B-4 | **特商法表記が実態と不一致**。LP `TokushohoPage` は「KC 購入画面」前提の記述だが、Live スコープは Pro サブスクのみ。月額サブスクの解約手順（Customer Portal）・課金タイミングの記載がない。「理由の如何を問わず一切返金しない」の消費者契約法上の妥当性も未確認 | `LegalPages.tsx` L105/L154/L166 | 🔴 Human 法務 |
| B-5 | Stripe Live アカウント側の整備（ビジネスプロフィール・領収書メール・税務処理の要否）が未実施 | Stripe Dashboard（Human） | 🟠 |

---

## 1. スコープ宣言（L0 で固定）

**Live オープン対象は「Pro 月額サブスクリプション（$19.99/月）」のみ。**

以下は引き続き封印・スコープ外（`release_master_plan.md` §0 の宣言を Live 後も維持）:

- 有償 KC チャージ（資金決済法・前払式支払手段の論点があるため、法務完了まで開かない）
- 有償スキル/ボイス販売・マーケット α（特商法 + 資金決済法）
- `execute_autonomous_purchase` 封印解除（OP-011、ポストリリース判定済み）

---

## 2. フェーズ構成

```
L0 スコープ固定 + B-1 緊急確認（Human+Main）
        │
L1 UI/導線整備（Main — Safety-Critical 隣接、承認必須）── L2 法務・表記（Human 律速、並行可）
        │                                                      │
        └────────────────┬─────────────────────────────────────┘
                         │
L3 本番切替（Human — Runbook NT-1 方針 B）
                         │
L4 検証（Verification Protocol 3 段階）
                         │
L5 運用・ロールバック整備 → 台帳クローズ
```

### Phase L0 — スコープ固定と緊急確認（0.5 日）

| ID | タスク | 担当 | DoD |
|---|---|---|---|
| L0-1 | **B-1**: LP Payment Link の mode を Stripe Dashboard で確認。live リンクなら**即時に無効化 or Coming Soon 差し替え**（Test 構成のまま実決済を受けない） | **Human**（確認）→ Main（差し替え） | link mode 記録 + live なら遮断完了 |
| L0-2 | 本計画のスコープ宣言（§1）をユーザーが承認 | **Human** | 「実装しろ」承認 |

### Phase L1 — UI/導線整備（1〜1.5 日、Main。フロントのみだが課金導線のため要承認）

| ID | タスク | 対応 | DoD |
|---|---|---|---|
| L1-1 | **B-2**: VoiceStore の「チャージ」導線を整理 — Pro サブスク Checkout であることを明示（文言変更）or Pro 誘導（`openProUpgradeModal`）へ統一。KC 有償チャージを想起させる表現を排除 | B-2 | 「月額 Pro に登録」等の明示文言・Jest PASS |
| L1-2 | **B-3**: AiaaOnboardingWizard の Checkout を実 `agent_id`（`useAgentIdentity`）に修正 or Live スコープ外として導線を無効化 | B-3 | 403 死に導線ゼロ（E2E or 手動確認） |
| L1-3 | LP Pricing CTA を live Payment Link（L3-1 で作成）へ差し替え + `Pricing.link.test.tsx` 更新 | B-1 | LP ビルド + link テスト PASS |
| L1-4 | 価格表示の整合確認（$19.99: LP / ProUpgradeModal / README / MESSAGING.md）— 表示のみ、差分があれば同期 | — | `rg "19.99"` 突合ゼロ差分 |
| L1-5 | （任意）トライアル実装: 実装するなら `trial_period_days` + 表示復帰。**現状は表示撤去済み**のため Live ブロッカーではない | G-10 解消済（表示側） | — |

### Phase L2 — 法務・表記（Human 律速、L1 と並行）

> **詳細展開の正本**: [`legal_docs_plan.md`](legal_docs_plan.md)（**OP-085**）+ [`COMPLIANCE_CHECKLIST.md`](../legal/COMPLIANCE_CHECKLIST.md)。**弁護士レビューは必須ではない**。OP-085 のチェックリスト ✅ 維持 + Human 開示能力・デプロイ目視で本フェーズ充足。

| ID | タスク | 担当 | DoD |
|---|---|---|---|
| L2-1 | **B-4**: 特商法表記の全面改訂 — 実オファー（Pro サブスクのみ）に一致させる。事業者名/所在地/連絡先/販売価格（$19.99/月）/支払時期/提供時期/**解約方法（Customer Portal → 期間末解約）**/返金ポリシー | **Human**（内容確定）→ Main（`LegalPages.tsx` 反映） | 専門家 or ユーザー最終確認 |
| L2-2 | 「一切返金不可」ポリシーの消費者契約法・Stripe 規約（サブスク解約権）との整合確認 | **Human** | 文言確定 |
| L2-3 | KC = 無償ポイントのままであることの利用規約上の明文化（資金決済法回避スコープの文書化） | Main（ドラフト）→ **Human** | LegalPages + docs/legal 同期 |
| L2-4 | **B-5**: Stripe Live アカウント整備 — ビジネスプロフィール・カスタマーメール（領収書）・Radar 既定・税務（Stripe Tax 要否判断） | **Human** | Dashboard 設定完了 |

### Phase L3 — 本番切替（0.5 日、Human。正本 = Runbook NT-1 方針 B）

| ID | タスク | DoD |
|---|---|---|
| L3-1 | Stripe **Live mode** で Product/Price 作成（Pro $19.99/月）→ live Price ID 記録 + live Payment Link 作成（LP 用） | `price_...`（live）記録 |
| L3-2 | AbyssVault（MC GUI）へ `STRIPE_API_KEY`（`sk_live_...`）/ `STRIPE_WEBHOOK_SECRET`（live `whsec_...`）格納 | Vault status 確認 |
| L3-3 | ホスト env: `STRIPE_TEST_MODE=false` + `STRIPE_PRICE_SUBSCRIPTION_MONTHLY=<live price>` → api-server 再起動。**preflight が起動を許可すること**（Price 空なら拒否 = 正常） | 起動ログ確認 |
| L3-4 | Stripe Dashboard（**Live mode**）で Webhook 登録: `https://<domain>/api/v1/commerce/webhook` + 必須 7 イベント | 登録 + 疎通 |
| L3-5 | （任意）`VITE_STRIPE_PRICE_ID` 焼き込み — エイリアス経路で不要。焼き込む場合のみ MC 再ビルド | — |

### Phase L4 — 検証（Verification Protocol 3 段階・絶対遵守）

| ID | タスク | 種別 | DoD |
|---|---|---|---|
| L4-1 | 実カードで少額決済 1 件 → `checkout.session.completed` → `invoice.paid` → **Pro unlock（MCP suspend 解除）** を確認 | Positive | unlock ログ + UI Pro 表示 |
| L4-2 | 不正署名 Webhook（改ざんペイロード）が 4xx 拒否されること + `whsec_test` 拒否を Live 構成で再確認 | **Negative** | 拒否ログ |
| L4-3 | `invoice.payment_failed` 相当（Stripe CLI trigger 不可のため、テストクロックまたは Dashboard からのサブスク失効操作）→ suspend 確認 | **Negative** | suspend ログ |
| L4-4 | L4-1 の決済を Dashboard から**返金 + サブスク即時キャンセル** → suspend 反映を確認し、正常状態へ復帰 | Revert | 返金完了 + 台帳記録 |

> exit code 0 / 「決済が通った」だけでは検証と見なさない。L4-2/L4-3 の Negative を省略した Live オープンは禁止。

### Phase L5 — 運用・ロールバック（0.5 日）

| ID | タスク | 担当 | DoD |
|---|---|---|---|
| L5-1 | `docs/releases/NT6_R5_ROLLBACK_DRAFT.md` に **Live 課金停止手順**を追記（`STRIPE_TEST_MODE=true` 戻し + LP CTA 差し替え + 既存 subscriber の扱い） | Main | 文書化 |
| L5-2 | 監視: Webhook 失敗（4xx/5xx）ログの定期確認手順 + Stripe Dashboard アラート設定 | Main（手順）+ **Human**（設定） | 手順書 1 節 |
| L5-3 | 台帳クローズ: OPEN.md OP-084 / CHANGELOG / RIPPLE_MAP / `release_master_plan.md`「方針 B」フォロー消し込み | Main | docs-sync |

---

## 3. 工数と律速

| フェーズ | 見積 | 律速 |
|---|---|---|
| L0 | 0.5 日 | Human（Dashboard 確認） |
| L1 | 1〜1.5 日 | Main（承認後） |
| L2 | 数日〜 | **Human 法務が全体の律速** |
| L3 | 0.5 日 | Human |
| L4 | 0.5 日 | Human + Main |
| L5 | 0.5 日 | Main |
| **合計** | 実働 3〜4 日 + 法務確定待ち | |

## 4. リスクと対策

| リスク | 対策 |
|---|---|
| B-1 が live リンクで、公開 LP から既に実決済可能 | **L0-1 を計画承認前でも即実施可能な緊急項目とする**（確認は読み取りのみ） |
| 法務（L2）が長期化し切替が止まる | L1/L3 準備を先行完了させ、L2 確定を唯一のゲートに集約 |
| Live 検証で実カード決済が必要 | 少額 1 件 + 即返金（L4-4）で露出最小化。返金手数料は許容コストとして記録 |
| チャージバック発生 | dispute → 即 suspend は実装済み。Radar 既定 + L5-2 監視でカバー |
| 「チャージ」文言の既存スクショ資材（NT-5）との乖離 | L1-1 の文言変更後、該当スクショのみ再撮影要否を判定 |

## 5. /perfect-plan 検証結果（自己適用）

- **Gate 1 構造スキャン**: ✅ 変更対象（`VoiceStore.tsx` / `AiaaOnboardingWizard.tsx` / `Pricing.tsx` / `LegalPages.tsx`）は全て実在をコードで確認済み。新規モジュール作成なし（二重実装リスクなし）
- **Gate 2 要件カバレッジ**: §2 経済台帳（KC 封印維持で影響遮断）・§4 セキュリティ（既存ガード変更なし）・§5 法的リスク（L2 で正面対応）をカバー。§3/§6/§7/§8 は影響なし
- **Gate 3 依存関係**: `STRIPE_PRICE_ID` 参照 4 箇所（App.tsx / NurtureDashboard / VoiceStore / AiaaOnboardingWizard）を全て計画に収録。`NurtureDashboard.test.tsx` / `Pricing.link.test.tsx` の追随を DoD に明記
- **Gate 4 悪魔の弁護人**: (1) 最悪シナリオ = B-1 が live で無提供課金 → L0-1 を最優先化 (2) 暗黙の前提 = 「返金不可ポリシーが適法」→ L2-2 で明示検証 (3) やらない選択肢 = 方針 A 継続はゼロ収益だが事故ゼロ。**L2 完了までは実質「やらない」を維持する構造**にした
- **Gate 5 実行順序**: L3-1（live Price 作成）が L1-3（LP 差し替え）の前提のため、L1-3 のみ L3-1 後に実施。他に循環なし

---

---

## 6. Human 実行チェックリスト — L2-4 / L3 / L4（✅ 2026-07-18 クローズ）

> L3-2〜4 / L4 は Human 実施済。Agent は L5-3 台帳クローズ済。運用監視は §5 / `stripe-production-setup.md` §5。

### L2-4 — Live アカウント整備 ✅ 2026-07-16 Human 報告

> 顧客メール設定の正本手順: [`stripe-production-setup.md`](../operations/stripe-production-setup.md) **§6**。

- [x] Dashboard **Live**
- [x] 事業名・住所・メール・URL が特商法と一致
- [x] 領収書メール ON（Successful payments）
- [x] 支払失敗メール ON（Send emails when card payments fail）
- [x] Radar = 既定
- [x] Tax = 見送り

### L3 — 方針 B 切替（正本: Runbook NT-1 + `stripe-production-setup.md`）

- [x] **L3-1** 既存確認（2026-07-16 CLI `--live`）:
  - Product: `Aiome Autonomous Pro（最新）`（active）
  - Price: `price_1TpXFpBcUTwo5TwLmK9SQbKL` = **1999 USD / month**（`livemode=true`）
  - Payment Link: `https://buy.stripe.com/aFa00i9cEaVE4ay4y9f7i03`（`livemode=true`・LP 掲載済）
  - 旧 $9.99 / 非 active Product は触らない（archive 済み想定）
- [x] **L3-2** 本番 MC Abyss Vault に `STRIPE_API_KEY=sk_live_…` / `STRIPE_WEBHOOK_SECRET=whsec_…`（2026-07-17 Human・チャット禁止）
- [x] **L3-3** ホスト `STRIPE_TEST_MODE=false` + `STRIPE_PRICE_SUBSCRIPTION_MONTHLY=price_1TpXFpBcUTwo5TwLmK9SQbKL` → api-server restart（Vault 再注入・health 200）
- [x] **L3-4** Live Webhook URL = `https://app.aiome.dev/api/v1/commerce/webhook` + **7 イベント**。legacy `we_1TlVbZ…`（workers.dev）**disabled**（2026-07-17 Human+Agent 照合）
- [x] **L1-3** LP CTA 差し替え不要（上記 Link が既に Live）
- [x] **Agent 2026-07-16**: forwarder `FORWARD_URL` を app.aiome.dev に修正・切替スクリプト追加

### L4 — 検証（省略禁止）✅ 2026-07-17〜18 Human PASS

- [x] **L4-1 Positive**: 実カード 1 件 → `checkout.session.completed` / `invoice.paid` → Pro unlock（本番ログ Verified）
- [x] **L4-2 Negative**: 偽署名 Webhook → 400（Agent 再確認）
- [x] **L4-3 Negative**: `customer.subscription.deleted` → suspend（payment_failed 相当・本番ログ）
- [x] **L4-4 Revert**: 返金/クレジットノート + 即時キャンセル → Pro→Free（UI + ログ）

*本計画の実行はフェーズごとにユーザーの明示承認を得ること（AGENTS.md Scope Lock / Safety-Critical Zone）。*
