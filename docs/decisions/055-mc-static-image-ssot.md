# ADR-055: MC static — Image SSOT（bind-mount 撤去方針）

**Status**: Accepted（**実行は Human ゲート**）  
**Date**: 2026-07-22  
**Related**: OP-087 Q5、[`mc_static_deploy_plan.md`](../roadmaps/mc_static_deploy_plan.md)、[`MC_STATIC_DEPLOY.md`](../guides/MC_STATIC_DEPLOY.md)、`docker-compose.production.yml`

## Context

本番 compose は `./apps/api-server/static` を api-server コンテナへ **read-only bind-mount** する。そのため distroless イメージ内へ COPY した `dist/`（Path A）は、ホスト側 static が存在する限り **ユーザー向け UI に勝たない**。

現行の UI 更新正本は Path B（`scripts/sync_mc_static.sh` → ホスト static）である。ホットパッチが容易な一方、イメージとホストのドリフト・「rebuild したら UI も直る」誤解のリスクが残る。

[`mc_static_deploy_plan.md`](../roadmaps/mc_static_deploy_plan.md) §0 Q-B は「今四半期は bind-mount 維持。撤去は別 ADR」とロック済み。本 ADR がその別 ADR である。

## Decision

1. **目標状態（Image SSOT）**  
   本番のユーザー向け MC UI は **コンテナイメージ内** `apps/api-server/static`（distroless COPY）を正本とする。  
   `docker-compose.production.yml` の  
   `./apps/api-server/static:/app/apps/api-server/static:ro`  
   は **撤去する**（実行時）。

2. **移行までの暫定（現行維持）**  
   Human が本 ADR の **実行ゲート**を明示するまで、bind-mount と Path B を維持する。  
   Path A 単独を「FE リリース完了」と呼ぶことは引き続き禁止。

3. **実行ゲート（すべて満たすこと）**  
   - Human が「ADR-055 を実行してよい／bind-mount を外してよい」と明示する  
   - Path A（distroless rebuild）のパイプラインが、リリース対象の `management-console` `dist/` をイメージに同梱することを運用で確認済み  
   - ロールバック手順が文書化されている（直前イメージタグへ戻す、または一時的に bind-mount を再追加）

4. **実行時の作業（ゲート後・別コミット）**  
   - compose から当該 volume 行を削除  
   - [`MC_STATIC_DEPLOY.md`](../guides/MC_STATIC_DEPLOY.md) / 計画書の「bind-mount が勝つ」記述を Image SSOT 前提に更新  
   - Path B は「イメージ焼き込み前の検証／緊急ホットフィックス」に降格するか廃止を Human と再確認  

5. **やらないこと（本 ADR 単体では）**  
   - 本 Accepted だけでは compose を変更しない  
   - CSP / Cache-Control（計画 §8 Q2/Q3）や CI artifact→rsync（Q7）の同梱

## Consequences

- **Accepted**: OP-087 Q5 クローズ。Image SSOT への意図とゲート条件が文書正本になる。  
- **実行前**: 運用は Path B + bind-mount のまま（ホットパッチ利便を維持）。  
- **実行後**: FE リリース完了の定義は Path A（イメージ）に寄せられる。ホスト `static/` は開発用・同期検証用に残しうるが、本番配信の SSOT ではなくなる。

## Alternatives considered

| 案 | 却下理由 |
|----|----------|
| 永続 bind-mount のみ | イメージと UI の永久ドリフト・誤解が残る |
| 即時撤去（ゲートなし） | §0 Q-B と衝突。FE 同梱確認前に本番白画面リスク |
| 独立 console サービス | 計画外（compose に無し）。本 ADR の範囲を超える |
