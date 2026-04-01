# ADR 028: Unified ToolCallRouter for Security Execution Precedence

## Status
Accepted

## Date
2026-04-01

## Context
In the `aiome` codebase, the execution of WASM tools and built-in skills occurred across two completely separate paradigms:
1. **Synchronous/Batch Execution** (`apps/api-server/src/agent_engine.rs`): Utilized `process_generated_tool_calls`, where `ToolHook` chains and intent verifiers (`AdaptiveImmuneSystem`) were manually invoked prior to `execute_skill`.
2. **Asynchronous/Streaming Execution** (`apps/api-server/src/stream.rs`): Tool execution occurred as Server-Sent Events (SSE) were yielded. Pre-execution checks, such as `Guardrails` logic, were either duplicated or awkwardly integrated into the SSE streaming closure, leading to architectural bypasses.

This bifurcation inherently violated the **Zero-Trust for LLM** core principle. We needed a mechanism that mathematically guarantees that every tool call, irrespective of its execution site (SSE stream or sequential loop), runs through a unified security and hook lifecycle before being parsed or executed.

## Decision
We implemented a centralized architectural barrier, `ToolCallRouter` (and its concrete implementation `DefaultToolCallRouter`), which subsumes the direct call paths to `execute_skill` and `ToolFactory::parse`.

The router enforces the following **Security Execution Precedence**:
1. `evaluate_security(tool_name, tool_args)`: Intercepts raw tool strings. Runs Local Guardrails and the `AdaptiveImmuneSystem` (Fail-Closed).
2. `execute_skill(tool_name, tool_args)`: Only called if Step 1 succeeds. This method wraps the actual WASM logic execution between `ToolHook::pre_execute` and `ToolHook::post_execute`.

All call sites in `stream.rs` and `agent_engine.rs` were modified to exclusively depend on the `ToolCallRouter` trait. 

## Consequences

### Positive
- **Architectural Invincibility**: Eliminates the "split-brain" execution model. Security policies and constraints (`Guardrails`, `ImmuneSystem`, `Hooks`) cannot be bypassed by newly introduced asynchronous endpoints.
- **Fail-Closed Execution**: If `Guardrails` detects an injection attempt, the SSE stream or task immediately halts and returns an error without ever attempting to parse the malicious JSON.
- **Maintainability**: Duplicated tool preparation code in `stream.rs` and `agent_engine.rs` was deleted, drastically reducing the cognitive load required to understand the execution loop. `process_generated_tool_calls` was simplified from 6 arguments to 4 arguments by delegating logic to the router.

### Negative
- All mock implementations and unit tests heavily test `ToolCallRouter`. To suppress clippy warnings, we had to introduce `#[cfg(test)]` bypass blocks within `tool_call_router.rs` to skip `Arc` initialization of complex systems like `SamsaraHub` during simple tests.

## See Also
- [SECURITY_DESIGN.md](../SECURITY_DESIGN.md) (Layer 2: Unified Precedence)
- [INFRASTRUCTURE_MODULES.md](../INFRASTRUCTURE_MODULES.md) (skills module HookChain)
