# Management Console static 配信ガイド

**正本（決定・DoD・非スコープ）**: [`docs/roadmaps/mc_static_deploy_plan.md`](../roadmaps/mc_static_deploy_plan.md)（OP-087）

## 事実（短く）

- ソース SSOT: `apps/management-console`
- ユーザーに届く UI: **ホスト** `apps/api-server/static`（本番 compose が bind-mount）
- distroless イメージ内の `dist` COPY だけでは、bind-mount がある限り **UI は更新されない**
- git に `static/assets` を載せない。追跡 `index.html` はスタブ（製品 UI ではない）

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

`docker compose build api-server`（Path A）はバイナリ更新用。**FE 完了の定義に使わない。**

**注意**: ローカル Path B 後、作業ツリーの `static/index.html` は Vite shell になり得る。コミット前にスタブへ戻す（`git show` の P3 スタブ / `git restore`）。assets は git に載せない。

## 関連

- NT-1 distroless: [`HUMAN_PUBLIC_BETA_RUNBOOK.md`](HUMAN_PUBLIC_BETA_RUNBOOK.md)
- 課金 closeout の static 注記: [`billing_closeout_plan.md`](../roadmaps/billing_closeout_plan.md)
