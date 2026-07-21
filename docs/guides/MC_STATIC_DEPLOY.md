# Management Console static 配信ガイド

**正本（決定・DoD・非スコープ）**: [`docs/roadmaps/mc_static_deploy_plan.md`](../roadmaps/mc_static_deploy_plan.md)（OP-087）  
**Image SSOT（bind-mount 撤去）**: [`docs/decisions/055-mc-static-image-ssot.md`](../decisions/055-mc-static-image-ssot.md)（ADR-055・実行は Human ゲート）

## 事実（短く）

- ソース SSOT: `apps/management-console`
- ユーザーに届く UI（暫定）: **ホスト** `apps/api-server/static`（本番 compose が bind-mount。ADR-055 実行まで）
- distroless イメージ内の `dist` COPY だけでは、bind-mount がある限り **UI は更新されない**
- git は `apps/api-server/static/` を **全無視**（OP-087 Q6）。追跡スタブは `apps/api-server/static.stub/index.html` のみ

## ローカル / ホスト同期（Path B）

```bash
# ビルド込み（既定）
./scripts/sync_mc_static.sh

# 既存 dist のみ
SKIP_BUILD=1 ./scripts/sync_mc_static.sh

# 宛先指定（リモート可）
DEST=user@host:/app/aiome/apps/api-server/static ./scripts/sync_mc_static.sh
```

TDD / Verification Protocol:

```bash
./scripts/test_sync_mc_static.sh
```

ソースツリーをホストへ運ぶ場合は既存のまま:

```bash
DEST=user@host:/app/aiome ./scripts/sync_production_sources.sh
```

その後ホスト上で `sync_mc_static.sh` するか、ローカルで sync 済み `static` が allowlist 経由で運ばれる点に注意。

## 本番（Human 許可 → Agent 実行可）

1. Human: 「MC static を同期してよい」／「OP-087 P4 を許可」と明示  
2. Agent/Human: Path B（上記スクリプト。リモート例 `DEST=user@host:/app/aiome/apps/api-server/static`）  
3. 同期前にホストで `static.bak-*` を取る（スクリプトはリモート bak を自動作成しない）  
4. Human: ブラウザで白画面なし・Checkout 等を確認  

`docker compose build api-server`（Path A）はバイナリ更新用。**FE 完了の定義に使わない**（ADR-055 実行前）。

**注意**: Path B 後の `apps/api-server/static/` は gitignored。コミット対象にしない。スタブ文言の正本は `static.stub/index.html`。

## ADR-055 実行後（Image SSOT）

Human が ADR-055 実行を許可したあと:

- compose の static bind-mount を外す  
- FE リリース完了 = Path A（イメージに `dist` 同梱）  
- Path B の位置づけはガイド／計画の更新コミットで再定義する  

## 関連

- NT-1 distroless: [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](HUMAN_PUBLIC_BETA_RUNBOOK.md)
- 課金 closeout の static 注記: [`billing_closeout_plan.md`](../roadmaps/billing_closeout_plan.md)
