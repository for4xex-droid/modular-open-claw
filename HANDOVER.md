# HANDOVER: Aiome First Penguin (v1.0) Release

## 🎯 現在のゴールとステータス
- **目標**: Stripe 本番承認に伴う、最高の安全性・堅牢性・自立性を保証した Stripe 本番運用対応の実装と検証。
- **現在の状況**: P0-BLOCKER の実装および検証が TDD に基づいて 100% 完了しました。手数料率 15% への統一、主要 API 封印解除、Dispute 対策・イベント拡充 Webhook 実装、および Price ID 環境変数化（堅牢パターン）を含むすべての機能テストが GREEN でパスしています。

## 🚨 完了済みのインフラ安定化・本番課金タスク
1. **手数料率 15% への統一 (P0-B0)**: [commerce_policy.md](docs/legal/commerce_policy.md) 記載を手数料率 15% へ修正・整合完了。
2. **ADR-042 課金ロジック検証・コミット (P0-B1)**: `CostCircuitBreaker` (事前上限ゲート) と `calculate_cost_coins` (事後コイン課金) が同一ジョブに二重計上しないことを整合検証・確定完了。
3. **Stripe 主要 API 封印解除 (P0-B2)**: サブスクリプション解約 (`cancel_subscription`) およびステータス取得 (`get_subscription_status`) API を unseal し、`async-stripe` 本番 API と `SubscriptionStatus` 拡張 enum へのマッピングを完了。
4. **Webhook ハンドラ拡充 (P0-B3)**: `customer.subscription.deleted`, `customer.subscription.updated` (status `past_due`/`unpaid`), `charge.dispute.created` (Dispute / 不正利用申告検知時の即時 suspend & `dispute_received` SSE 排他ブロードキャスト), `checkout.session.expired` に対する Webhook 高信頼処理を実装完了。
5. **Price ID 環境変数化・堅牢パターン (P0-C2)**: 起動時に `STRIPE_PRICE_SUBSCRIPTION_MONTHLY` が未設定の場合に起動を拒否する堅牢プリフライト検証を実装。ルートハンドラにおいて `"price_gold_monthly"` エイリアスを自動で本番価格 ID にマッピングして Stripe に送信する置換ロジックを TDD 実装完了。
6. **本番 Stripe 設定ガイド & 例追加 (P0-B4)**: [stripe-production-setup.md](docs/operations/stripe-production-setup.md) の新規作成、および [.env.example](.env.example) への本番価格 ID プレースホルダーの追記完了。
7. **Nurture SQLite WAL モード & busy_timeout**: [db.rs](libs/shared/src/db.rs#L58-L59) にてすでに実装済みであることを確認し、DONE とマーク。
8. **Nurture Task Supervisor**: 既存のリトライ + Reflexion + Watchtower パイプラインがすでに稼働中であることを確認し、DONE とマーク。

## ⏸️ 凍結事項（DEFERRED）- 絶対遵守
> [!CAUTION]
> **OGP 画像 (`og:image`) およびプロモーション動画の埋め込みタスクは完全凍結中**です。
> ユーザーから「完成版のロゴ・音声素材」が提供されるまで、コードの変更を一切行ってはなりません。仮画像やプレースホルダーでの代用は厳禁です。

## ⏩ 次期リリースでのネクストアクション (P1 項目)
最新の実行計画は **[implementation_plan.md](.agent/workflows/implementation_plan.md)** にて定義されています。
- **P1-1: Stripe Customer Portal 統合**
  - `libs/aiome-commerce/Cargo.toml` に `async-stripe-billing-portal` クレートを追加し、ポータル URL 生成エンドポイントを新設。ロゴ解凍後に Stripe Dashboard からブランディングをカスタマイズ。
- **P1-4: `execute_autonomous_purchase` の封印解除**
  - Nurture Ledger への Coin Charge Relay と連携し、`/api/v1/commerce/purchase` のスタブ解除と Nurture /internal/purchase へのプロキシを実装。

## 🛡️ 開発原則（AGENTS.md）
- **Scope Lock 原則**: 各フェーズ（計画・設計・実装・検証）の境界を厳守すること。
- **Verification Protocol**: 実装完了時は必ず **(1) Positive Test (正常系)** → **(2) Negative Test (異常注入)** → **(3) Revert & Report (復旧検証)** の3段階の検証を実行すること。
