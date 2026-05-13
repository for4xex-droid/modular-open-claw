# ADR 041: Sovereign Verifier Productionization & KANI_STUB_MODE Deprecation

## Status
Accepted

## Context
As part of the Aiome infrastructure security hardening (Phase D), the goal was to transition the Sovereign Verifiers (Kani and OxiLean) from development stubs to production-ready enforcement. Historically, the `KANI_STUB_MODE` environment variable was used extensively across the workspace (8 separate occurrences) to bypass formal verification steps, avoid external dependencies (e.g., Podman) during CI, and relax development constraints (like HTTP vs HTTPS in the commerce module).

However, retaining `KANI_STUB_MODE` in the codebase violates the "Zero-Panic & Total Verification" policy. It creates a vector for bypassing security sandboxes, such as the Aegis Sentinel patch verification and OxiLean CI/CD checks.

## Decision
1. **Total Deprecation of `KANI_STUB_MODE`**: All 8 references to `KANI_STUB_MODE` have been permanently removed from the codebase:
   - `apps/api-server/src/routes/commerce.rs`: Replaced with strict `AIOME_DEV_MODE` for HTTP fallback.
   - `apps/management-console/playwright.config.ts`: Substituted with `AIOME_DEV_MODE: '1'`.
   - `libs/infrastructure/src/dream_state.rs`: Removed testing workarounds, enforcing actual failure states when Podman is unavailable.
   - `libs/infrastructure/src/aegis/prover.rs`: Removed the stub bypass (`if KANI_STUB_MODE == "true" { return Ok(true) }`).

2. **Podman-backed Verification Enforcement**: `AegisProver::verify_with_kani` now mandatorily executes the Kani verifier within a rootless Podman container (`aiome/kani-verifier:latest`), adhering to ADR-040. If the environment lacks Podman or the container image, the verifier accurately fails, preventing unverified patches from entering production.

3. **OxiLean Verification Hardening**: The `shadow-worker` integration tests have been validated to successfully run against the `OxiLeanProofService` while `KANI_STUB_MODE` is inactive, guaranteeing that OxiLean checks are functioning correctly under real conditions.

## Consequences
### Positive
- **Guaranteed Formal Verification**: LLM-generated patches cannot bypass Kani model checking.
- **Reduced Attack Surface**: Attack vectors leveraging the test-only `KANI_STUB_MODE` flag in production are eliminated.
- **Architectural Clarity**: Testing mode relies exclusively on `AIOME_DEV_MODE` or native Rust `#[cfg(test)]`, streamlining configuration.

### Negative
- **CI/CD Requirements**: Environments that test Aegis patching logic must now explicitly support Podman rootless execution and pre-pull the `aiome/kani-verifier:latest` image, which increases CI complexity and execution time.

## Related
- ADR-040: Aegis Prover — Kani Rust Verifier Sandbox Architecture
- ADR-038: OxiLean Kernel Integration
