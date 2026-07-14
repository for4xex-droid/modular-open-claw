# docs/legal — 法務文書の正本管理

**このディレクトリが全法務文書の正本（Single Source of Truth）である。** LP（`docs/landing/src/components/LegalPages.tsx`）は表示層であり、正本から反映する。

## 対応表

| 正本 | 版 | 表示層 | 状態 |
|---|---|---|---|
| `TERMS_OF_SERVICE.md` | v2.1 | LP `/terms`（`TermsPage`） | 公開 |
| `PRIVACY_POLICY.md` | v2.0 | LP `/privacy`（`PrivacyPage`） | 公開 |
| `TOKUSHOHO.md` | v2.2 | LP `/tokushoho`（`TokushohoPage`） | 公開 |
| `CANCELLATION_POLICY.md` | v1.1 | LP `/cancellation`（`CancellationPage`） | 公開 |
| `COMPLIANCE_CHECKLIST.md` | v1.0 | なし（必須記載チェック） | 内部 |
| `KC_LEGAL_POSITION.md` | v1.0 | なし（社内正本） | 内部 |
| `CONSENT_SPEC.md` | v1.0 | なし（実装仕様） | 内部 |
| `REVIEW_PACKAGE.md` | v1.1 | なし（任意相談用・**必須ゲートではない**） | 内部 |
| `commerce_policy.md` | 1.0.0 | なし | **凍結**（有償 KC・マーケット開放時に改訂） |
| `voice_upload_terms.md` | — | なし | **凍結**（同上） |
| `../../commercial/docs/legal/TOS_CORE_DRAFT.md` | 0.1.0-draft | なし | **凍結**（KC チャージ Phase 2 用） |
| `../../commercial/docs/legal/CHARGE_UX_CONSENT.md` | 0.1.0-draft | なし | **凍結**（同上） |

## 更新フロー

1. 正本（本ディレクトリの `.md`）を改訂し、ヘッダの「版」「最終更新日」を更新する。
2. LP `LegalPages.tsx` に文言を反映し、`lastUpdated` を正本の最終更新日と一致させる。
3. `LegalPages.sync.test.tsx`（乖離検知テスト）が正本ヘッダと LP の `lastUpdated` を照合する。乖離があるとテストが FAIL する。
4. 規約（TERMS）の版を上げた場合は `apps/management-console/src/config.ts` の `TOS_VERSION` も更新する（現行 **v2.1**。`CONSENT_SPEC.md` §1）。
5. 不利益変更は効力発生日の 30 日前周知（TERMS §8）。

## 禁止事項

* LP 側だけを直接編集すること（正本と乖離する）。
* 「Karma Coins の販売」「チャージ」を前提とする文言の復活（`KC_LEGAL_POSITION.md` §3 の封印監査ポイントに抵触）。
* 実装のない「無料トライアル」等を広告・UI に謳うこと（景表法リスク。`COMPLIANCE_CHECKLIST.md` §5）。
