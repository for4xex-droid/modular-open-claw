# Project NURTURE Requirements v2（実態正本）

> v1 要件定義書の改訂版。詳細計画: `docs/roadmaps/nurture_quality_max_plan.md`

## v1 → v2 改訂表

| v1 | v2（実態） |
|---|---|
| Nemotron-3 + NeMo Guardrails (Docker) | マルチプロバイダ + 独自 Guardrails（BeggingSupervisor 等） |
| 中央サーバー = AWS/PostgreSQL | SQLite ローカル + PostgreSQL 本番 |
| クリエイター報酬 = 法定通貨支払い | CP→AiomeCoin のみ（**ADR-052**） |
| ツール名 `search_marketplace`/`buy_item` | `marketplace_search`/`marketplace_buy` |
| P2P 送金 | **デフォルトブロック**（`allow_p2p_transfer: false`） |
| CSAM 頭身 0.20 閾値 | **1/5.5**、fail-closed、自己申告バイパス禁止 |

## 実装済み拡張（v1 未記載）

エスクロー（24h TTL）、冪等性ゲート、Merkle 監査、Federation BFT、ウォッシュトレード防止、SurpriseEngine、月次支出上限（ADR-050）。

**2026-07-07 追記（品質最大化 v4）**: wishlist（`nurture_wishlist` / `GET /commerce/wishlist`、購入成功時消込み）、ledger `memo` 列、再購入ブロック（ライセンス + 24h）、BoneChecker fail-closed（1/5.5）、Management Console `CoinBalanceProvider`、A2C `NURTURE_A2C_DRY_RUN`（デフォルト dry-run）。

## スコープ外（ADR-052）

- CP の法定通貨出金 / 外部ギフト変換
- Tremendous は **A2C 恩返し専用**（ドライラン→Human 確認→有効化）
