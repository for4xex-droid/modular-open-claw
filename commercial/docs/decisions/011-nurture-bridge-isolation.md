# ADR 011: Nurture Bridge Isolation and Zero-Panic Defense-in-Depth

## Date
2026-05-14

## Status
Accepted

## Context
Nurture API (Economy & Asset Management) was previously tightly coupled with the broader Aiome workspace infrastructure. Direct dependencies on `aiome-core`, `aiome-contracts`, and `infrastructure` exposed Nurture to downstream cascading changes, compilation breakages (due to trait modifications like `MockLlmProvider` or `CommerceEngine`), and potential IP leakage across the two ecosystems.
Furthermore, the lack of strict HTTP security headers (CORS, CSP) and the presence of scattered inline middleware configuration in `main.rs` hindered testability and production-readiness.

## Decision
1. **Physical Interface Separation (`nurture-bridge`)**: We established the `nurture-bridge` crate as the single, authoritative gateway for all Aiome-provided shared infrastructure (e.g., job queues, auth, LLM providers). 188 occurrences of direct Aiome dependencies across 24 files were mechanically refactored to route through `nurture-bridge`.
2. **Zero-Panic Policy Enforcement**: Conducted a full static audit ensuring 0 `unwrap()`, `expect()`, or `panic!` calls exist in production logic (`apps/nurture-api/src/` and `libs/nurture-infra/src/`). All errors now gracefully propagate using `NurtureError` and log securely via `tracing`.
3. **Defense-in-Depth via TDD (`middleware.rs`)**: Abstracted all global API defenses (Rate Limiting, Payload Size Limit, Strict Security Headers, and CORS) into `apply_security_middlewares`. Validated via strict TDD (`security_headers_test.rs`).

## Consequences
### Positive
- **IP Protection & Build Stability**: Aiome's internal structural changes no longer break Nurture's build, provided the `nurture-bridge` contract is upheld. Future refactors or relocations of Aiome contracts are fully insulated from Nurture's application layer.
- **Resilience**: The complete eradication of unhandled panics and `unwrap` calls guarantees a Crash-Only but highly available runtime architecture.
- **Security Posture**: Automatic prevention of MIME-sniffing, XSS, and Clickjacking at the framework boundary without developer intervention.
- **Domain Alignment**: Promotes Domain-Driven Design (DDD) by explicitly defining the interface boundaries between the API and the underlying foundational platform.

### Negative
- **Maintenance Overhead**: Adding a new Aiome infrastructure feature requires an explicit re-export mapping in `nurture-bridge/src/lib.rs`.
- **Infrastructure Coupling**: `nurture-infra` must maintain a direct dependency on `aiome-core-contracts` because routing its infrastructure dependencies through `nurture-bridge` introduces an unavoidable circular dependency (`nurture-bridge` -> `infrastructure` -> `nurture-bridge`).
- **Protobuf Dependency**: gRPC clients within `nurture-api` still require a direct dependency on `aiome-core-contracts` with `features = ["grpc"]` due to complex macro-generated protobuf structures that are impractical to re-export.
