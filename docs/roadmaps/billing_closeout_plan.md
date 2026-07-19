# 課金クローズアウト計画（v1.5・R4 完了 2026-07-18）

- **ステータス**: R1a–R4 / H1–H4 / L5-3 **完了**（closeout クローズ）
- **完了記録**: R1a=`3ab70c5e` / R1b=`main-DrW5KfL_.js` / R2=§7.D / R3=`ad9461a5` / H4+L5-3=`190e2686` / **R4=本番 distroless rebuild + Verification Protocol PASS（2026-07-18）**
- **継承元**: `live_billing_open_plan.md` v1.2 + NT-1 Step 0（`HUMAN_PUBLIC_BETA_RUNBOOK.md`）
- **目的**: 抜け漏れ・重複・車輪の再発明なしに残作業を閉じる
- **v1.5**: R4 実行結果・インシデント（compose 上書き）・ホスト hotfix を記録。/reflexion で台帳矛盾・死参照・`GENERATIVE_ENGINE` デフォルト誤称を是正

## 0. 事実（実行後・2026-07-18）

| 項目 | 状態 | 根拠 |
|---|---|---|
| Portal / Trialing / fail-closed FE | ✅ | R1–R2 |
| OP-084 Live（L3–L5） | ✅ | H4 + L5-3 |
| compose `A2A_NODE_TOKEN=${A2A_AUTH_TOKEN}` | ✅ | production compose |
| 空文字ガード **コード** | ✅ main | `libs/shared/src/config.rs`（`ad9461a5`） |
| 空文字ガード **本番イメージ** | ✅ | rebuild 後 `security.distroless=true` / User `65532` |
| R4 Verification Protocol | ✅ | Positive health 200 / Negative FATAL ログ / Revert health 200 |
| `GENERATIVE_ENGINE` パススルー | ✅ | compose `${GENERATIVE_ENGINE}`（ホスト `.env` 必須・デフォルトなし） |

**スコープ外**: Portal 再構築、OP-083、有償 KC、whitespace trim、shadow-worker rebuild、本番 Postgres 移行（別件）

**コミット除外**: `apps/api-server/static/*`（R1b 配信物。ローカル `index.html` が Vite SPA でも git HEAD の旧 HTML との差分はコミットしない。本番へは rsync のみ）。以降の正本: [`mc_static_deploy_plan.md`](mc_static_deploy_plan.md)（OP-087）

## 1. R4 実行結果（2026-07-18）

| ステップ | 結果 |
|---|---|
| R4-0 | AUTH len=30 / ソース rsync で filter 投入 / distroless 確認 |
| R4-1 | `build api-server` + `force-recreate --no-deps` → `security.distroless=true` / User 65532 |
| R4-2 Positive | health **200** / Listening |
| R4-2 Negative | override `A2A_NODE_TOKEN=` → **FATAL** `A2A_NODE_TOKEN must be set` / Restarting(1) |
| R4-2 Revert | override 除去 → Up / health **200** |
| R4-3 | 台帳更新（本ファイル + OPEN / CHANGELOG / RIPPLE_MAP） |

### インシデント（R4-1 直後）と対処

| 事象 | 原因 | 対処 |
|---|---|---|
| SQLite code 14 / Restarting | 本番へ `rsync --delete` で **Postgres 前提 compose** を上書き。ホストは SQLite（`data/api/aiome.db`）・`POSTGRES_PASSWORD` 未設定・postgres 未稼働 | ホスト compose から `AIOME_DB_PATH=postgres…` と api-server の `depends_on: postgres` を除去（bak 保管） |
| GenerativeEngine FATAL | compose が `GENERATIVE_ENGINE` をパススルーしていなかった（`.env` には設定あり） | `GENERATIVE_ENGINE=${GENERATIVE_ENGINE}` を api-server environment に追加（repo + ホスト） |

**教訓**: R4 のソース同期は **compose を無差別 `--delete` しない**。バイナリ同梱に必要なツリー（`libs/` / `apps/api-server` / `apps/management-console` / `docker/`）に限定する。

## 2. R4 手順正本（再実行用）

1. **R4-0**: `A2A_AUTH_TOKEN` 非空（長さのみ確認・値は出さない）/ `config.rs` の empty filter がソースにある / イメージが distroless（`security.distroless=true`）
2. **R4-1**: NT-1 Step 0 と同じく `docker compose -f docker-compose.production.yml build api-server` → `up -d --force-recreate --no-deps --no-build api-server`（`restart` 禁止）
3. **R4-2**:
   - Positive: `https://app.aiome.dev/health` → 200
   - Negative: api-server **のみ** で `A2A_NODE_TOKEN=` override（`A2A_AUTH_TOKEN` を空にしない）→ FATAL ログ
   - Revert: override 除去 → health 200
4. **R4-3**: OPEN / CHANGELOG / RIPPLE_MAP / 本計画を同期

## 3. 成功基準

1. ✅ 公開 LP が「お支払い管理」のみ（「サブスク管理」再混入なし）
2. ✅ 本番 MC に fail-closed FE が載っている（R1b）
3. ✅ R2 検証記録あり + **H4 PASS（2026-07-18）**
4. ✅ OPEN が live plan と一致（OP-084 クローズ）
5. ✅ Portal/Trialing の新規コードが増えていない
6. ✅ R4: 空文字ガード本番同梱 + P/N/R PASS + 台帳更新

## 4. /perfect-plan 第3周（v1.4）— 実行前 PASS

（実行前 Gate 1–5 は PASS。実行後の差分は §1 インシデントと `/reflexion` 是正のみ。）

## 5. 履歴

| 版 | 内容 |
|---|---|
| v1.1–1.3 | R1–R3 / H4 / L5-3 |
| v1.4 | R4 実行可能化（NT-1 委譲 + P/N/R） |
| v1.5 | R4 実行完了・compose 事故と hotfix・/reflexion 是正 |
