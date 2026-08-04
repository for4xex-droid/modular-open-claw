# ADR 058: Intelligent LLM Routing (TaskTier Selection)

## Status
Accepted

## Context
ADR-010 introduced `FallbackRouter` for **availability** (primary → fallback on failure). Production also needs **cost-aware tier selection**: route simple chat prompts to local-first Fast tier while reserving Smart tier for structured or high-stakes requests.

Existing assets:
- `TaskTier` (`Fast` / `Smart`) in `aiome-contracts` — designed for dispatcher routing, currently unwired.
- `fast_provider` — local-first chain for background components; chat path uses separate `router_provider`.
- `CostCircuitBreaker`, `EvaluationLogger`, `SemanticCache` — observability and budget layers already exist.

## Decision

1. **IntelligentRouter** — new `LlmProvider` decorator on the chat path only (`router_provider`), inserted between `EntropyGate` and inner `FallbackRouter` chains.
2. **TaskTier mapping** — `Fast` → `FallbackRouter(local_provider, bg_provider)`; `Smart` → `FallbackRouter(primary, bg_provider)` after KeyProxy resolution.
3. **FallbackRouter unchanged in role** — still handles failover; IntelligentRouter handles tier **selection** only.
4. **`LLM_ROUTE_MODE=legacy`** (default) — always Smart chain; zero behavior change until opt-in.
5. **`stream_complete`** — always Smart chain (BackgroundLlm has no streaming).
6. **Sticky tier** — first decision stored in request metadata so EntropyGate re-asks do not flip tiers. Only internally re-injected `route_tier_locked` is trusted; `HumanizerFilter` (chat boundary) strips client-supplied `route_tier` / `route_tier_locked` before the inner stack.
7. **Budget degrade** (rules mode) — when `CostCircuitBreaker` is tripped, force Fast tier via `LocalCostBypassProvider` instead of rejecting local inference.
8. **Cache placement (review fix)** — `CachingLlmProvider` sits **outside** `EntropyGate` and is DI'd **only in `LLM_ROUTE_MODE=rules`**. Cache stores EG-validated responses only; hit path skips EG (acceptable because write path already validated). Semantic (embedding) match is disabled for chat cache; keys are framed SHA-256 over `channel_id` + full messages + format/temperature/max_tokens (`route_*` metadata excluded). Missing `channel_id` → cache bypass (Fail-Closed). `ContextEngine::prepare_hybrid_request` injects `channel_id`.
9. **Local pin** — Fast-tier / budget-degrade `BackgroundLlmProvider` instances set `pin_local=true` so DB/env cannot self-promote them to cloud. Cloud Failover via `FallbackRouter` remains governed by `local_fallback_policy` (chat `cheap_chain` respects `LocalOnly`).

## Non-Goals
- RouteLLM / ML classifiers
- Soul or Playbook per-agent policies
- Load balancing across equivalent cloud models
- Changes to `fast_provider` consumers
- HierarchicalRouter reuse

## Consequences
- **Positive**: Cost reduction on simple chat without sacrificing Smart tier for JSON/security tasks.
- **Positive**: Reuses TaskTier, FallbackRouter, cost/eval infrastructure — no parallel router service.
- **Negative**: Chat cheap tier and `fast_provider` share local Ollama (no shared semaphore in Phase 1).
- **Negative**: `stream.rs` billing still uses decorator `name()` — resolved via metadata in eval logs (G-12 follow-up).

## Related
- ADR-010: Resilient LLM Routing
- ADR-021: Prompt Caching (provider-side; orthogonal to SemanticCache)
- `docs/roadmaps/intelligent_llm_router_plan.md`
