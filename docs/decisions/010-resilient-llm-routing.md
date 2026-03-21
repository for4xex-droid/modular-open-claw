# ADR 010: Resilient LLM Routing (FallbackRouter & Circuit Breaker)

## Status
Accepted

## Context
As the Aiome system integrates with multiple LLM providers (ArrowCanaria, Gemini, Ollama, etc.), the availability and latency of these external services become critical vulnerabilities. A failure in the primary LLM provider (e.g., due to API outages, rate limits, or network instability) can lead to a complete collapse of the AI agent's cognition (Contextual Collapse) and stop all autonomous background tasks.

Previously, the `api-server` relied on a single `provider` instance. If this provider failed, the request would error out, potentially triggering cascade failures in the Job Queue or UI.

## Decision
We implemented a **Resilient LLM Routing** architecture using the `FallbackRouter` and the **Circuit Breaker** pattern.

1.  **FallbackRouter**: A decorator that wraps a `primary` and a `fallback` LLM provider. It implements the `LlmProvider` trait and delegates calls based on the health of the primary provider.
2.  **Circuit Breaker**: The `FallbackRouter` uses a `CircuitBreaker` to monitor the health of the primary LLM. If the failure threshold (default: 3) is reached, the circuit opens, and subsequent requests bypass the primary entirely for a reset timeout period (default: 60s).
3.  **Safe Default Response**: If both the primary and fallback providers fail, the `FallbackRouter` returns a pre-defined "Safe Default Response" (a neutral JSON message) to prevent the system from crashing and provide a graceful degradation of service.
4.  **Arc-based Sharing**: The `FallbackRouter` and its inner providers are managed via `Arc<dyn LlmProvider>`, ensuring efficient sharing across multiple threads and background workers.

```rust
// In main.rs
let primary = provider.clone();
let fallback = bg_provider.clone(); // Usually Gemini or Ollama
let resilient_provider = Arc::new(FallbackRouter::new(
    primary,
    fallback,
    3, // failure threshold
));
state.provider = Component::new(resilient_provider);
```

## Consequences
- **Positive**: High Availability. The system can survive outages of the primary LLM provider by automatically switching to a secondary one.
- **Positive**: System Stability. The Circuit Breaker prevents "hammering" a failing service, allowing it time to recover and protecting the system from cascading latency.
- **Positive**: Graceful Degradation. The safe default response ensures the AI remains "polite" even under extreme service failure conditions.
- **Negative**: Increased Complexity. Debugging LLM issues now requires checking the state of the router and identifying which provider actually fulfilled the request.
- **Negative**: Latency Overhead. A single failure on the primary still incurs the primary's timeout before falling back.

## Future Considerations
- **Dynamic Re-weighting**: In the future, we could implement a more sophisticated router that selects providers based on real-time latency or cost metrics rather than a simple primary/fallback relationship.
- **Observability**: Expose the Circuit Breaker state (CLOSED/OPEN/HALF-OPEN) via the `/api/health` or a dedicated diagnostics endpoint to provide visibility into the health of the LLM ecosystem.
- **Per-Task Fallback**: Allow certain critical tasks (e.g., security audits) to use a different fallback strategy than casual chat.
