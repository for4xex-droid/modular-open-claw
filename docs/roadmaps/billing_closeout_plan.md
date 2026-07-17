# 課金クローズアウト計画（v1.3・L5-3 2026-07-18）

- **ステータス**: Agent R1a–R3 **完了** / Human H1–H4 **PASS** / **L5-3 台帳クローズ済**（2026-07-18）。任意残は **R4** のみ
- **完了記録**: R1a=`3ab70c5e` / R1b=`main-DrW5KfL_.js` / R2=§7.D / R3=`ad9461a5` / H4 PASS（L4-3=`subscription.deleted`→suspend）/ L5-3=本更新
- **継承元**: `billing_hardening_deploy`（A/B-1/B-2 済）+ `live_billing_open_plan.md` v1.2
- **目的**: 抜け漏れ・重複・車輪の再発明なしに、残作業だけを閉じる
- **v1.3**: H4→L5-3 完了を §0 に反映。R4 は次回 api-server rebuild 時の任意同梱。

## 0. 事実（実行後・2026-07-18）

| 項目 | 状態 | 根拠 |
|---|---|---|
| Portal API/FE | ✅ 再実装不要 | `customer-portal/create` + `handlePortal` / OP-010 CLOSED |
| Trialing + pick_preferred | ✅ | `stripe/mod.rs` / `expression.rs` / FE `isPro` |
| MC static + api-server | ✅ | health 200 / Portal 401 / `main-DrW5KfL_.js` |
| fail-closed `isLoading` 初期 true | ✅ 本番 rsync 済（R1b） | LockedOverlay-BLJhwnQo.js 配信 |
| §7 Portal Human DoD | ✅ PASS | `stripe-production-setup.md` §7.D + R2 追記 |
| LP Cancellation | ✅ main + GH Pages | 「サブスク管理」なし / sync テスト PASS / Actions success |
| A2A compose | ✅ 本番済 | `A2A_NODE_TOKEN=${A2A_AUTH_TOKEN}` |
| A2A 空文字 FATAL（config filter） | ⏳ コードは main 済・**本番イメージ未同梱** | R4 任意（次回 rebuild） |
| OP-084 L3-2〜L4 / L5-3 | ✅ | H4 PASS 2026-07-18 + OPEN クローズ |
| OPEN 過大表記 | ✅ 訂正済→クローズ | OP-084 を ✅ 解決へ移管 |

**スコープ外**: Portal 再構築、OP-083-C/D、有償 KC、自律購買、subscription SSE push、Vault/鍵の Agent 操作。

## 1. レーン分離

```
Agent（R1a→R1b→R2→R3→R4任意）     Human（別承認・live plan §6）
R1a LP commit/push → GH Pages   ║  H1 L3-2 Vault live 鍵
R1b MC build → static 再 rsync  ║  H2 L3-3 env（要なら recreate/build）
R2 UI/Portal 検証記録（≠ L4）   ║  H3 L3-4 Webhook 7 events
R3 OPEN/CHANGELOG/live plan 訂正║  H4 L4 実決済 Verification Protocol
R4 shared 空文字ガード同梱      ║  → 報告後 Agent: L5-3 台帳クローズ
```

- 並列可: R1a と R1b、R1* と H1 準備。
- 直列: **R1b → R2**（fail-closed を本番で見るため）。**H4 → L5-3**。
- R3 は H4 を待たない（過大表記の先行訂正）。

## 2. Agent レーン（DoD）

| ID | 作業 | 検証 | 禁止 |
|---|---|---|---|
| **R1a** | `LegalPages.tsx` + `LegalPages.sync.test.tsx`（必要なら `docs/legal` は既同期確認のみ）を commit → `main` push | CI `deploy-landing` 成功。公開確認は **curl HTML だけでは不足**（SPA）→ Actions 緑 + ローカル `npm test -- LegalPages.sync` PASS。任意: 公開 JS bundle に「サブスク管理」無し | `docs/landing/dist` を commit しない / Portal 再実装しない |
| **R1b** | `npm ci && npm run build` → `apps/api-server/static` へコピー → 本番 rsync（`static.bak-*` 退避後）。**reflexion(6) FE を載せる** | 本番 bundle ハッシュ更新。ハードリロード後 MC 動作 | static を git commit しない |
| **R2** | **UI ゲート検証**（Live 切替検証ではない）: Free/Pro CTA、Portal、LockedOverlay 初回 loading、未認証 Portal 401 | Positive/Negative/Revert を §7.D 追記 or CHANGELOG。失敗時 static ロールバック | H4（実カード決済）と混同しない / 新機能追加しない |
| **R3** | 下記「訂正文」を OPEN / CHANGELOG / live plan に適用。本計画へのリンクを OPEN 実装手順に追加可 | OPEN と live plan §6 が矛盾しない | 「L1–L4 済」再記載 / L5-3 先行クローズ |
| **R4** | （任意）次回 api-server rebuild 時に `config.rs` 空文字ガード同梱 | 空 TOKEN → FATAL（Negative） | 鍵を出力しない |

### R3 訂正文（OPEN OP-084 1行の置換指針）

**履歴（R3 時点）**: 済=L1–L2/L3-1/§7/R1–R2、残=L3-2〜L4→L5-3。  
**2026-07-18**: L3–L5 完了。OPEN OP-084 ✅。任意残=R4 のみ。

### R3 で直す live plan 陳腐化（再実装なし・docs のみ）

1. §6 L3-3: `restart` → 「イメージ更新時は distroless rebuild + `--force-recreate`。env のみなら restart 可」。
2. §0.2 B-2: 🔴 → ✅（L1-1 / VoiceStore Pro 明示で解消）。行番号参照の再実装はしない。

### コミット境界（指示時）

| バッチ | 含める | 除外 |
|---|---|---|
| LP | `docs/landing/src/**`、sync テスト | `docs/landing/dist` |
| 台帳 | OPEN / CHANGELOG / live plan / 本計画 | — |
| コード | FE/BE/i18n/compose/`config.rs`/commerce テスト | `apps/api-server/static/*`、秘密、無関係 wrangler |

## 3. Human レーン（手順の正本は live plan §6 — ここでは重複しない）

| ID | 要約 | 本計画との関係 |
|---|---|---|
| H1–H3 | Vault / env / Webhook | Agent 非関与。H2 は R3 訂正後の recreate 注記に従う |
| H4 | 実カード Verification Protocol | **R2 の代替にならない** |
| L5-3 | OPEN クローズ | ✅ 2026-07-18（H4 PASS 後） |

## 4. 成功基準

1. ✅ 公開 LP が「お支払い管理」のみ（「サブスク管理」再混入なし）。
2. ✅ 本番 MC に reflexion(6) fail-closed が載っている（R1b）。
3. ✅ R2 検証記録あり + **H4 PASS（2026-07-18）**。
4. ✅ OPEN が live plan と一致（OP-084 クローズ）。
5. ✅ Portal/Trialing の新規コードが増えていない。
6. 任意: R4 で A2A 空文字ガードを本番イメージに同梱。

## 5. /perfect-plan 第2周（v1.1）

| Gate | 結果 |
|---|---|
| 1 構造 | PASS — 新規モジュールなし。抜けは **R1b（FE 再 rsync）** のみ追加 |
| 2 NURTURE | PASS — §5 文言同期。経済/認証の新経路なし |
| 3 波及 | PASS — LP sync・OPEN・live plan docs。Mock/portal 再実装不要 |
| 4 Red Team | PATCH→反映 — 最悪=R2 を H4 と誤認 / 前提=B-1 で fail-closed 済（偽）/ やらない=LP 旧文言残 |
| 5 順序 | PASS — **R1b→R2** を強制。R3 先行訂正。L5-3 は H4 後 |

**判定: ✅ PASS（v1.1 を実行正本とする）**
