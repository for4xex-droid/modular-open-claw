# ADR-054: Error Hierarchy (Three Layers)

**Status**: Accepted  
**Date**: 2026-07-10  
**Accepted**: 2026-07-20  
**Related**: OP-051, [`error_handling.md`](../architecture/error_handling.md) §2, TECH_DEBT Top5 plan v1.3, [`op051_error_hierarchy_plan.md`](../roadmaps/op051_error_hierarchy_plan.md)

## Context

Aiome currently mixes ~10 error types (`AiomeError`, `SoulError`, `X402Error`, `CsamError`, …) with widespread `anyhow` in infrastructure. OP-051 asked for a three-layer Decision before any bulk rewrite. Bulk replacement is explicitly forbidden until this ADR is approved.

## Decision

| Layer | Role | Canonical types |
|-------|------|-----------------|
| **1 Domain / Boundary** | Crosses API or crate boundaries; HTTP-safe via `IntoResponse` / CWE-209 sanitize | `AiomeError` (`libs/aiome-contracts`) |
| **2 Subsystem** | Domain-specific, kept local; convert with `From` at boundary | Existing: `SoulError`, `X402Error`, `CsamError`, `NurtureError`, … — **no new types** (`error_handling.md` §3) |
| **3 Internal** | Implementation detail inside a module | `anyhow::Error` / ad-hoc; must map to Layer 1 before leaving the crate |

### Rules

1. Prefer adding a variant to `AiomeError` over inventing a new public error enum.
2. Subsystem errors that reach `api-server` must implement `From<…> for AiomeError` (or map via existing helpers such as commerce `map_infra_err`).
3. `anyhow` is allowed only inside Layer 3; do not return `anyhow::Result` from public trait methods in `aiome-core-contracts`.
4. No mechanical “replace all anyhow with AiomeError” pass without a follow-up implementation plan after this ADR is Accepted.

## Consequences

- **Accepted (2026-07-20)**: Enables phased OP-051 per [`op051_error_hierarchy_plan.md`](../roadmaps/op051_error_hierarchy_plan.md). Each phase requires explicit「実装しろ」before code changes.
- **Rejected / Deferred**: Status remains Proposed; code stays as documented in `error_handling.md` §2.

## Out of scope

- Immediate code migration
- Skills-module refactor riding this ADR
- Changing Safety-Critical commerce/auth error surfaces without separate review
