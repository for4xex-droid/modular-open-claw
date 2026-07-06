# ADR-051: Stripe Customer Portal 統合 — OP-010 クローズ判定

## ステータス
Accepted — **OP-010 CLOSED**（2026-07-06、release_master_plan R2-5 照合）

## コンテキスト
HANDOVER.md P1-1 / OPEN.md OP-010 は「Stripe Customer Portal 統合（クレート追加・ポータル URL 生成エンドポイント新設）」を未完了として起票していた。release_master_plan R2-5 では、フロントエンド `handlePortal` の存在を前提に、バックエンドが Mock か実 Stripe Billing Portal API かを照合し、Mock なら実装・実装済みならクローズと定義した。

## 照合結果（2026-07-06）

| レイヤ | 実体 | 判定 |
|---|---|---|
| **API** | `POST /api/v1/commerce/customer-portal/create` — `apps/api-server/src/routes/commerce.rs` `create_portal_session` | 実装済み（JWT 認証・IDOR 防止・`return_url` ホワイトリスト検証 SEC-2） |
| **Commerce エンジン** | `libs/aiome-commerce/src/stripe/mod.rs` `create_portal_session` | **実 Stripe Billing Portal API**（`CreateBillingPortalSession`）。`is_mock=true` 時のみ固定 URL を返す開発用分岐 |
| **フロントエンド** | `apps/management-console/src/hooks/useCheckoutSession.ts` `handlePortal` | 実装済み（認証付き POST → 返却 URL へリダイレクト）。VoiceStore 等から呼び出し |
| **テスト** | `apps/api-server/src/api_integration_tests/commerce.rs` | `test_customer_portal_session_success` / `_idor_rejection` / `_invalid_return_url` — Positive + Negative カバー |
| **OpenAPI** | `docs/openapi.json` `/api/v1/commerce/customer-portal/create` | 登録済み |

Mock スタブのみで本番相当のポータル URL を返す経路は存在しない（本番 `is_mock=false` では Stripe API 呼び出し必須）。

## 決定
- **OP-010 を CLOSED とする。** 新規実装・クレート追加は不要。
- Public Beta リリース判定: Customer Portal は Day 1 機能として利用可能（Stripe Customer が `stripe_customers` に存在する agent が前提。未登録時は `NotFound` を返す Fail-Closed 設計）。

## 影響
- **ポジティブ**: R2-5 の DoD（照合レポート → 判定）を満たし、サブスク管理 UI（解約・支払方法変更）導線が本番 Stripe と接続済みであることを文書化。
- **残タスク**: 本番 env 反映（`STRIPE_API_KEY` 等）は OP-057-R (1) / R2-1 の Human 作業として継続。Portal 機能自体の追加実装はスコープ外。

## 参照
- `docs/roadmaps/release_master_plan.md` R2-5
- `docs/operations/stripe-setup.md` §2（Price ID 整合）
- CHANGELOG — Customer Portal エンドポイント・`handlePortal` 統合履歴
