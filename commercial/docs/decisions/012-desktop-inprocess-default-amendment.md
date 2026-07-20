# ADR 012 Amendment: Desktop 既定 InProcess

## Date
2026-07-21

## Status
**Accepted**（Human Accept 2026-07-21）

## Relates
- Parent: [`012-agenthook-fire-path-unification.md`](012-agenthook-fire-path-unification.md)（Accepted）
- Plan: `docs/roadmaps/desktop_inprocess_default_plan.md` v1.3

## Context
Parent ADR は InProcess 時の sidecar 非起動と **D3（CommerceEngine DI しない）** を固定した。  
公式 Desktop の**製品既定**を Local sidecar から InProcess へ移すにあたり、経済経路を自己 HTTP（`NURTURE_API_URL=http://127.0.0.1:3015`）+ JWT 外 `/internal` に固定する。

## Decision
1. **Desktop 通常起動の正本は InProcess**（P2 既定フリップ）。
2. **D3 維持** — CommerceEngine の in-process DI は行わない。課金・forget・DLQ は既存 S2S（Bearer secret + OXP）の自己 HTTP。
3. **`/internal` は JWT `auth_middleware` の外**に `nest` する。Plugin `nurture_routes` / `merge_routes` 配下に載せない。
4. **Local escape** は `NURTURE_MODE=local`。公式パッケージから nurture-api sidecar を除外する（P3 / Q3）。

## Consequences
- Positive: 偽成功（URL 未設定 skip）を構造排除。Hook 二重発火は Parent D1 で排除済み。
- Negative: 自己 HTTP のデッドロックリスク（G11）— 監視し、必要なら oneshot 内部呼び出し（P5-a）。
- Neutral: Parent Decision 2「Sidecar がデフォルト」は **Desktop 製品既定については本 Amendment で上書き**（サーバ/開発 Local は明示モード）。
