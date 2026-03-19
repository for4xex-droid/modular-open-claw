# ADR 008: Type-Driven Security (Auth Extractor Enforcement)

## Status
Accepted

## Context
As the `api-server` routing logic expanded to include modular route handlers (`agent.rs`, `biome.rs`, `karma.rs`, etc.), the system relied primarily on a global middleware (`auth_middleware`) defined at the `Router` level to restrict unauthorized access. 

While effective, middleware configuration resides entirely within `main.rs`. When maintaining or modifying route handlers, it is easy for developers to accidentally omit a handler from the protected router scope, exposing insecure endpoints to the public internet. The disconnect between a route's logic and its security assumptions introduces cognitive overhead and regression risks.

## Decision
We implemented **Type-Driven Security** by enforcing authentication directly at the compile-time type level.

All protected asynchronous request handlers now require the `crate::auth::Authenticated` type as a parameter in their signature. Since Axum's `FromRequestParts` trait powers this extractor, any handler missing this structural dependency will fail to evaluate the authentication state correctly if we bypass the router guard, and more importantly, it makes the constraint self-evident in the signature.

```rust
// Previous pattern (Implicit security)
pub async fn trigger_agent_chat(
    State(state): State<AppState>,
    Json(payload): Json<AgentChatRequest>,
) -> Result<Json<serde_json::Value>, AppError> { ... }

// New pattern (Type-driven security)
pub async fn trigger_agent_chat(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated, // Explicit security boundary
    Json(payload): Json<AgentChatRequest>,
) -> Result<Json<serde_json::Value>, AppError> { ... }
```

### Defense in Depth Integrations
1. **Middlewares**: The global `auth_middleware` remains in place to prevent the extractor from redundant parsing, executing the defense-in-depth paradigm.
2. **AppState Optimization**: `system_agent_id` is now injected dynamically into `AppState` at startup. This bypasses the need for the `Authenticated` extractor to query the `job_queue` on every request when falling back to system logic, reducing latency.
3. **CI Assertions**: We've added `CC-6: Type-Driven Security` to `scripts/deep-scan.sh` and a `missing-auth-extractor` rule to `.github/anti-patterns.yml`. This automatically enforces that all `pub async fn` route handlers request `Authenticated`.

## Consequences
- **Positive**: High resistance to accidental route exposure. Handlers clearly declare their security assumptions, satisfying zero-trust architectural boundaries.
- **Positive**: CI acts as an automated security architect preventing silent security bypasses during refactoring loops.
- **Negative**: Adds verbose boilerplate (`_auth: crate::auth::Authenticated` or `auth: crate::auth::Authenticated`) to all route handlers that may not otherwise utilize the embedded `agent_id`.
- **Risk (SPOF)**: The current Type-Driven Security model relies on a single shared secret (`api_server_secret`). While the *presence* of authentication is guaranteed by the type system, the *strength* of it remains a Single Point of Failure. If the shared secret is leaked, all endpoints are compromised.

## Future Considerations
- **JWT / RBAC Integration (Phase 8.2)**: To mitigate the SPOF risk, we plan to shift from a shared secret to scoped JSON Web Tokens (JWT) with Role-Based Access Control (RBAC).
- **ProtectedRouter Wrapper (Typestate Pattern)**: The current implementation enforces constraints via CI (`deep-scan.sh CC-6`) rather than strict compiler typestate on the `Router`. In the future, building a `ProtectedRouter<S>` wrapper could enforce compile-time errors for handlers missing `Authenticated` without relying on CI scripts.
