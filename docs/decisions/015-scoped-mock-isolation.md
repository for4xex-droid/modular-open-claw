# ADR-015: Scoped Mock Object Isolation

## Context
During a security deep scan, it was identified that multiple mock implementations (e.g., `MockAuthManager`, `MockCommerceEngine`, `MockEkycEngine`) were included in the production release builds. Although these were not active by default, their presence increased the attack surface (binary analysis, potential execution if environment variables were misconfigured).

## Decision
We will strictly isolate all mock implementations to development (debug) and test environments using Rust's conditional compilation features.

1.  **Strict Isolation**: All mock structures, their trait implementations, and their associated module definitions must be wrapped with `#[cfg(any(test, debug_assertions))]`.
2.  **Explicit Scoping in Main**: The initialization logic in `api-server` and other entry points will use `#[cfg(debug_assertions)]` to branch between production-grade implementations and mocks.
3.  **Fatal Failures in Production**: If a required secure component (like `StripeCommerceEngine` or `JwtAuthManager`) cannot be initialized in a release build (e.g., due to missing environment variables), the system MUST panic or exit with a fatal error rather than silent substitution with a mock.
4.  **Module-Level Guard**: Where possible, entire modules (e.g., `commerce_mock`) will be conditionally compiled at the crate root to ensure they do not even contribute to the release binary's symbol table.

## Status
Accepted

## Consequences
- **Security**: Significantly reduced attack surface in production binaries.
- **Reliability**: Prevents the catastrophic scenario where a production environment accidentally runs with mock security/commerce logic.
- **Developer Experience**: Developers still have access to easy "zero-config" local execution since `debug_assertions` are enabled in standard `cargo run`.
- **Build Process**: Release checks must now explicitly run `cargo check --release` to ensure that conditional compilation doesn't break the release build due to missing types.
