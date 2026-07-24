# OP-094 — GitHub ↔ LP 公開面整合（v1.0）

- **ステータス**: **✅ 2026-07-25 完了**
- **OPEN**: OP-094
- **正本計画**: Cursor plan `lp_github_整合修正` v1.0（plan ファイル自体は編集しない）

## 目的

Release / MESSAGING / COMPLIANCE / README と OP-084 Live（2026-07-18）・LP Payment Link の事実を一致させる。

## 結果

| フェーズ | 結果 |
|---|---|
| P0 | Payment Link **Live**（`livemode=true`。OP-084 L3-1 CLI + L4 実カード） |
| P1 | MESSAGING §9 / COMPLIANCE §7 / README・README_en 同期 |
| P2 | `gh release edit v1.2.0` — soft-launch 誤記除去（タグ不動）。正本: [`v1.2.0-release-body-op094.md`](../releases/v1.2.0-release-body-op094.md) |
| P3a | LP CTA 変更なし（Live 確定）。`Pricing.link.test` PASS |
| P4 | OPEN / CHANGELOG / RIPPLE |

## /reflexion（2026-07-25）

- OP-057-R 後回し表記・旧 Link 陳腐文言・COMPLIANCE §7 見出しを修正
- MESSAGING §9 にセルフホスト Pro 解錠手順（`stripe-setup.md`）を復帰。「既知ギャップ」見出しを解消済みに変更
- COMPLIANCE 最終更新日・`stripe-setup.md` §2.5 Live/Deploy 文言・README セルフホスト Pro 解錠を同期（3回目）
- MESSAGING 文書ヘッダ日付・OPEN OP-085 解決行の Live 整合（4回目）

## スコープ外（意図的 defer）

- Desktop LP 掲載
- SPA HTTP 404 インフラ改修（404.html 既存）
- commerce / deploy-landing workflow 改変
- Payment Link livemode CI
