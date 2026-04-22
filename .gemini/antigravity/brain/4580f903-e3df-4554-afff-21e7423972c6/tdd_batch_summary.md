# OxiLean Security & Integration Patches (TDD Phase 4)

## Critical Fixes (Security & Legal)
1. **Timing Attack Vulnerability**: Replaced `ends_with` auth token checks with `subtle::ConstantTimeEq` across `proof_service.rs` and `main.rs`.
2. **Legal Compliance**: Dropped the standard Apache-2.0 `LICENSE` in `vendor/oxilean-kernel` and created `THIRD_PARTY_LICENSES.md` in root.

## Architecture Synchronization
3. **FormalProofGate**: Created Phase 2 stub interface in `aiome-contracts/src/proof.rs`.
4. **Agent Hooks**: Appended `on_proof_completed` to `AgentHook` trait in `error.rs`/`security.rs`.
5. **Nurture Plugin Target (TDD)**: Extended Nurture's `plugin.rs` replacing the `DummyJobQueue` with standard implementation structure via `RealJobQueue` and instantiated `NurtureAgentHook` unittests ensuring full `on_proof_completed` integration.

## Tests & CI/CD Patches
6. **Local Preflight Validation**: Inserted `vendor/oxilean-kernel` execution into `scripts/test_all.sh`.
7. **CI Pipeline Alignment**: Introduced explicit `OxiLean` execution commands into `.github/workflows/formal-verify.yml`.
8. **Docker Cleanup**: Corrected `Dockerfile.shadow-worker` duplicated caching commands and optimized memory context limits with properly configured `.dockerignore`.

## Documentation Checks
9. **ADR Implementation**: Persisted ADR-038 outlining `shadow-worker` structural bounds and scaling properties.
10. **System Memory Updates**: Documented current session outputs inside `memory/2026-04-22.md` and successfully synced `Project-Nurture/CHANGELOG.md`.
