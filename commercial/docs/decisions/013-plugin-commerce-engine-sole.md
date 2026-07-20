# ADR 013: InProcess 時 Plugin CommerceEngine を唯一正本にする（C1'）

## Date
2026-07-21

## Status
**Accepted**（Human「P5-c を実装しろ」2026-07-21）

## Relates
- Parent: [`012-agenthook-fire-path-unification.md`](012-agenthook-fire-path-unification.md) Decision 3
- Amendment: [`012-desktop-inprocess-default-amendment.md`](012-desktop-inprocess-default-amendment.md)（D3 維持を本 ADR で Desktop InProcess に限り上書き）
- Plan: `docs/roadmaps/op088_p5_polish_plan.md` §5（C1'）

## Context
OP-088 Ship では経済を自己 HTTP + JWT 外 `/internal` に固定し、CommerceEngine DI を後回しにした（D3）。  
P5-a で forget / monthly-limit / coin-charge は oneshot 化した。残る Stripe Factory 自己 HTTP と、gig/docker/browser/marketplace が Factory `Arc` を捕捉する二重台帳を、InProcess では Plugin `NurtureCommerceBridge` 一本に揃える。

## Decision
1. **C1' boot**: `event_sender` + `auth_manager` 生成直後（Factory より前）に InProcess なら `create_plugin` を登録し、その `commerce_engine()` を gig / docker / browser / AppState / marketplace に渡す。
2. **InProcess では `CommerceEngineFactory`（Stripe self-HTTP）を生成しない。**
3. **Local / Cloud は Factory 維持**（本 ADR の対象外）。
4. **禁止**: assemble 後の `AppState.commerce_engine` だけ置換 / core 全体より前の Plugin / Bridge 不足を埋める第2 Engine。
5. **Fiat スコープ**: Bridge が封印する checkout/portal/subscription は AppState 経路では Fail（Err）。Desktop の Fiat は Nurture Plugin ルートを正とする。台帳（escrow / validate_activity 等）は Bridge 実体で充足（c0 突合済み）。
6. **二重登録禁止**: C1' で登録済みなら `register_in_process_plugins` は no-op。

## Consequences
- Positive: InProcess の台帳系が単一 Arc。Stripe self-HTTP / G11 残りを構造排除。
- Negative: AppState 経由の Stripe Fiat API は InProcess で使えない（Plugin 経路へ誘導）。
- Neutral: ADR-012 Amendment D3 は「Ship 時点」の記述として残し、Desktop InProcess の DI は本 ADR が正本。
