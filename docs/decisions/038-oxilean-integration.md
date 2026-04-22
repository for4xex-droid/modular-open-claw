# ADR 038: OxiLean Kernel Integration

## Context
As the Aiome ecosystem moves toward robust, verifiable execution of AI-generated constraints (Guardrails) and Logic (WASM Skills), we must trust that the generated proofs strictly adhere to mathematically sound properties. Traditional theorem provers are large, typically C++ or Lean based, and difficult to isolate in our zero-trust Rust architecture.

The OxiLean Kernel is our pure Rust solution. However, since the Proof Verification process is computationally heavy and involves untrusted outputs from LLMs, there is a substantial risk of Denials of Service (DoS) or Kernel panics. 

## Decision
1. **Separation of Concerns (TCB)**: OxiLean will be included as an excluded workspace crate (`vendor/oxilean-kernel`). It acts as our Trusted Computing Base (TCB) with zero external dependencies, verifiable via `cargo clippy` and `cargo test`.
2. **Shadow Worker**: A dedicated gRPC service container, `shadow-worker`, handles verification and isolates the host system.
3. **4-Layer Defense Architecture**:
    * **Auth (L1)**: gRPC `authorization` header with Constant-Time String comparison to prevent timing attacks.
    * **Semaphore (L2)**: Prevents CPU exhaustion by limiting concurrent verification threads (`OXILEAN_PROOF_SEMAPHORE_PERMITS`).
    * **Timeout (L3)**: Halting Problem mitigation via `tokio::time::timeout` (`OXILEAN_PROOF_TIMEOUT_SECS`).
    * **Panic Isolation (L4)**: `catch_unwind` wraps the synchronous kernel proof evaluation in block execution, protecting the gRPC Server process from crashing if malformed proofs trigger Rust panics.
4. **Nurture Synergy (FormalProofGate & KarmaForge)**: Upon successful verification (via Phase 2 implementation of `FormalProofGate`), the `AgentHook::on_proof_completed` executes, passing synthesis events to Project-Nurture's `KarmaForge` to build quantifiable "Proof Power" trust metrics.

## Consequences
- **Positive**: Complete fault isolation. Aiome core infrastructure (`api-server`, `samsara-hub`) cannot be crashed by malicious proofs.
- **Positive**: Formal proofs integrate seamlessly with Nurture's economic/reputation model.
- **Negative**: Increased complexity. The `shadow-worker` needs separate scaling configurations and monitoring within Docker Swarm / local Compose.
- **Negative**: Adds overhead to local testing environments due to new compilation profiles and `.env` setup.

## Future Plans (Phase 2 & 3)
- Implement `FormalProofGate` bridging inside `WasmSkillManager`.
- Add `cargo test` explicitly in CI validation loop (currently bypassed as an excluded workspace member).
- Connect `cross_synthesize` stub to process proper Economic Ledger entries in Nurture API upon successful proofs.
