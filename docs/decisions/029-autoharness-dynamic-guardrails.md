# ADR 029: AutoHarness Dynamic Guardrails

## 1. Status
Accepted

## 2. Context
The Aiome infrastructure requires resilient and self-adaptive security guardrails to monitor and filter agent behaviors dynamically. Initially, guardrails were hardcoded with static severity thresholds (e.g., a static `80` severity). Furthermore, regex evaluations within the `ConstraintChecker` were vulnerable to ReDoS due to literal expansion without size limits. This created an inflexible security posture that could not effectively differentiate between minor policy violations (which may only warrant logging for later LLM adaptation) and critical security breaches (which must be blocked immediately).

We needed an adaptive protection mechanism—AutoHarness—that allowed dynamic fetching of constraint rules (harnesses) from a registry, parsing their severities, and structurally blocking ReDoS attacks.

## 3. Decision
We decided to implement the AutoHarness Dynamic Guardrails system focusing on three main pillars:
1. **Harness Registry Database (`harness_registry`)**: Introduced a unified database layer via `HarnessRegistryOps` on the `JobQueue` trait to allow runtime fetching, updating, and storing of `HarnessRecord` structs.
2. **ReDoS Immunity via `RegexBuilder`**: Refactored `ConstraintChecker::evaluate_step_with_harnesses` to build regular expressions using `RegexBuilder::new(regex).size_limit(10 * 1024)`. Any expression exceeding a 10KB state machine limit is safely rejected, blocking ReDoS.
3. **Active and Shadow Modes**: Enhanced the `ActionHarness` trait to include a `severity()` method. In the main `skill_handler`, the system fetches harnesses mapped to the agent/skill. If an evaluated breach has a severity of `>= 80`, it operates in **Active Mode** (the action is strictly blocked). If `< 80`, it operates in **Shadow Mode** (the action is allowed, but the violation is recorded to the trajectory for `AgentRx` telemetry).

## 4. Consequences
### Positive
- **Adaptive Telemetry**: Agent mistakes can now be safely logged (Shadow) rather than silently ignored or violently blocked, drastically improving the feedback loop for the `AgentRx` self-healing infrastructure.
- **Structural Immunity**: ReDoS attacks from maliciously crafted prompt injections mimicking harness structures are mathematically prevented.
- **Extensible Registry**: Admins can hot-reload WASM security harnesses or update threshold configs via the `HarnessRegistryOps` database layer without recompiling the system.

### Negative
- **WASM Payload Overhead**: Injecting multiple WASM harnesses per step evaluation introduces memory and instantiation overhead, which may require later optimization via WASM byte caching or instantiation pooling.
- **Mock Implementation Burden**: Expanding `JobQueue` required updating 5+ mock structures across the entire unit test suite (`tts_worker.rs`, `immune_system.rs`, `dream_state.rs`), temporarily destabilizing CI.

## 5. References
- PR / Task: Phase B-D AutoHarness Security Architecture Integration
- Documentation: RIPPLE_MAP.md (AutoHarness Phase B-D)
