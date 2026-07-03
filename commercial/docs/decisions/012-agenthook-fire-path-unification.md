# ADR 012: AgentHook 発火経路の一本化（In-Process vs Sidecar）

## Date
2026-07-04

## Status
Accepted

## Context
Nurture の AgentHook（`on_transaction_completed` → KarmaForge 合成）は現在2系統存在する:

1. **nurture-api サイドカー内** — Webhook 経由で `state.rs` L151–155 の Hook インスタンスが発火
2. **api-server in-process** — W-3 で `NurturePlugin` を `plugin_registry.register()` し、api-server の `HookManager` 経由で `trigger_job_completed` 等から発火

両方が同時に有効だと KarmaForge 合成が二重実行されるリスクがある。

## Decision
1. **`NURTURE_IN_PROCESS=true` モードでは nurture-api サイドカーを起動しない** — Tauri の `resolve_nurture_mode()` に `InProcess` variant を追加し、`Local` より優先判定する（将来タスク）。api-server 単体で Plugin ルート・Hook・MCP が完結する。
2. **Sidecar モード（デフォルト）では in-process 登録を行わない** — 現行の HTTP プロキシ経由 CommerceEngine + サイドカー内 Webhook Hook を正とする。
3. **CommerceEngine の in-process DI は行わない** — 二重台帳リスク回避。HTTP プロキシ Factory を正とする（W-3 スコープ外）。

## Consequences

### Positive
- Hook 二重発火を構造的に排除
- デプロイモードが3段階に整理: Mock（$0）→ InProcess / Sidecar Local → Cloud

### Negative
- Tauri 側の `InProcess` variant 追加は別タスク（OPEN.md 起票）
- 開発者は `NURTURE_IN_PROCESS` と sidecar 起動の排他を理解する必要がある
