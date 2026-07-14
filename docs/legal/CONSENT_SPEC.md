# 同意取得（Click-wrap）仕様書

**版**: v1.0
**最終更新日**: 2026-07-14
**目的**: 利用規約への同意の証拠力を確保し、規約改訂時に「どの版に同意したか」を追跡可能にする。

## 1. 規約バージョン定数

* 正本: `docs/legal/TERMS_OF_SERVICE.md` ヘッダの「版」（現行 **v2.1**）。
* フロントエンド定数: `apps/management-console/src/config.ts` の `TOS_VERSION`（正本と同一文字列）。
* LP 表示層: `docs/landing/src/components/LegalPages.tsx` の `lastUpdated` は正本の「最終更新日」と一致させる（乖離検知テストの対象）。

## 2. SetupWizard（初回同意）

| 項目 | 仕様 |
|---|---|
| 表示 | Step 1 に規約の要約 + **全文リンク**（`https://aiome.dev/terms` / `https://aiome.dev/privacy`、`target="_blank"`） |
| 同意 | チェックボックス（デフォルト OFF）。ON にしない限り「次へ」非活性（既存実装を維持） |
| 送信 | `POST /api/v1/setup/init` の payload に `tos_version: TOS_VERSION` を追加 |
| 保存 | api-server `setup_init` が settings に保存: `tos_accepted="true"`（既存）+ `tos_accepted_version`（版文字列）+ `tos_accepted_at`（UTC ISO8601） |

## 3. ProUpgradeModal（課金前の情報提供）

特商法（2022 年改正・定期購入の最終確認画面表示義務）への対応として、Checkout へ遷移する画面に以下を表示する。

| 項目 | 仕様 |
|---|---|
| 更新条件の明示 | 価格表示の直下に 1 行: 「$19.99/月・自動更新。いつでも解約できます（期間末まで利用可）」（i18n: `pro.renewalNotice`） |
| 法務リンク | モーダル下部に 利用規約 / 特定商取引法に基づく表記 / 解約・返金ポリシー への外部リンク（i18n: `pro.legalTerms` / `pro.legalTokushoho` / `pro.legalCancellation`） |
| リンク先 | `https://aiome.dev/terms` / `https://aiome.dev/tokushoho` / `https://aiome.dev/cancellation` |

> 注: Stripe Checkout 自体も金額・周期を表示するが、当社導線側でも表示することで打ち消し表示の弱さを補強する。

## 4. 規約改訂時の再同意

* 不利益変更を伴う改訂時は、`tos_accepted_version` と `TOS_VERSION` の不一致を検出して再同意モーダルを表示する（**将来実装**。現時点ではバージョン記録のみを先行実装し、比較ロジックは規約改訂の発生時に追加する）。

## 5. 監査

* 同意記録は settings（`legal` カテゴリ）に保存され、DB バックアップに含まれる。
* Negative: `tos_accepted=false` での `setup_init` は 400 拒否（既存実装・テスト済）。
