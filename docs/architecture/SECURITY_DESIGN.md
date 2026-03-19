# Aiome Security Design Doctrine

> This document defines the security architecture for Aiome. It records the rationale for design decisions and the responsibilities of each defense layer.

## 1. Core Principle: Zero-Trust for LLM

Unlike traditional agent frameworks that grant execution privileges to the LLM, Aiome operates on a **Zero-Trust** basis: **The LLM is restricted to "deliberation" while all execution is strictly managed by Rust-enforced guardrails.**

```
Traditional:  [LLM] → Arbitrary Code Execution → 💀 Risk of Hallucination/Malice
Aiome:        [LLM] → Rust Validation Layer → Whitelisted Tool Execution → ✅
```

## 2. Threat Model

### 2.1 Out-of-Scope Threats
- **DDoS (Internal)**: Most services are bound to `localhost` and are not exposed to the internet.
- **MITM**: Local inter-process communication (LDC/UDS) is used between trusted components.

### 2.2 Addressed Threats

| # | Threat | Vector | Severity | Mitigation Layer |
|---|---|---|---|---|
| 1 | Prompt Injection | User Input → LLM | 🔴 High | Input Guardrails |
| 2 | LLM Output Hallucination | LLM → Tool Execution | 🔴 High | OutputValidator + Formal Schema |
| 3 | Secret Leakage | API Keys in Memory | 🔴 High | **Abyss Vault (Key Proxy) + mlockall / zeroize** |
| 4 | Supply Chain Vulnerability | Dependencies | 🟡 Mid | `cargo audit` + Sentinel |
| 5 | Resource Exhaustion | Infinite Loop / Spams | 🟡 Mid | Rate Limiting + WASM Timeout & Circuit Breaker |
| 6 | **Karma Poisoning** | **Malicious Federation Sync** | 🔴 High | **Bearer Auth + Node Reputation System** |
| 7 | Reverse Shell Exploit | WASM Skill → host_exec | 🔴 High | **Immune System Baseline (14 signatures) + BastionGuard** |
| 8 | Env Var Exfiltration | Skill reads API_KEY env | 🔴 High | **Baseline regex detection + WASM isolation** |
| 9 | SQL Injection via Skill | Skill crafts DROP TABLE | 🟡 Mid | **Baseline regex + parameterized queries** |
| 10 | Startup Panic / DoS | Invalid config → crash | 🟡 Mid | **Panic-free startup with graceful exit** |
| 11 | **Cascade Error / AI Hallucination** | **Loss of Context during Self-Evolution** | 🔴 High | **Context Management System (`RIPPLE_MAP.md` + ADRs)** |
| 12 | **CSAM / Binary Contamination** | **Malicious Binary CRDT/P2P** | 🔴 High | **Protocol Asset Filter + 3-Layer Defense (eKYC, Hash, 5.5-Head) (Phase 8.1)** |

## 3. Defense Architecture

### Layer 1: Guardrails (Input Validation & Content Filtering)
- Detects prompt injections and command injections.
- Sanitizes control characters and enforces length limits.
- **Binary/CSAM Blocking (Phase 7.1 & 8.1)**: Strictly prohibits `data:image/`, `data:video/`, and `;base64,` content in the Biome P2P protocol. Enforces a **3-Layer Defense** for custom avatar uploads: 1) eKYC Age Verification, 2) Perceptual Image Hashing (against illegal CSAM blacklists), and 3) Skeletal Proportion Rules (5.5-head ratio to prevent child-like models). Non-compliant assets are actively quarantined and their metadata is persistently stored in the `QuarantineStore` (SQLite) to prevent bypasses and facilitate audits.
- **Sync Throttling**: Limits CRDT state blobs to 1MB to structurally block steganographic binary embedding.
- **Begging Supervisor (Phase 7.2)**: Implements an output-side guardrail (`shared/guardrails/BeggingSupervisor`) that detects and blocks AI-generated dark patterns (e.g., asking for money, tokens, or gifts) to ensure legal and ethical transparency in autonomous interactions.

### Layer 2: SecurityPolicy (Execution Control)
- **Whitelisting**: Only registered tools in the `ToolRegistry` can be executed.
- **Sandboxing**: Filesystem access is restricted via `PathSandbox`. WASM execution is strictly walled off from wildcard host access.
- **Abyss Vault**: ALL LLM and remote API calls are routed through an isolated Key Proxy process utilizing `mlockall` and exact endpoint routing to prevent SSRF and memory leakage.
- **Gift Policy Enforcement (Phase 7.2)**: The `GiftEngine` enforces a hard limit of $5.0 USD per autonomous gift and requires valid administrator (`MASTER_EMAIL`) credentials to prevent asset draining by malicious or hallucinating agents.

### Layer 3: Audit Log & Hash Chains
- Every tool invocation and systemic decision is logged for post-hoc analysis.
- **Hash Chains**: All operational logs in SQLite are cryptographically linked using SHA-256 hash chains, enabling immediate detection of deletion or tampering efforts.

### Layer 4: Build Isolation & Formal TDD Forge (S-Rank Defense)
- **OS-Native Sandbox**: Autonomous compilation (`cargo build`) executed by the agent is forcibly containerized using OS-native guardrails (`sandbox-exec` / `bwrap`) to prevent supply chain attacks during the Forge process.
- **Fail-Forward Training**: Instead of terminating agents when code fails to compile, the system employs TDD-based reinforcement loops without permanent Karma penalties, allowing self-healing code generation.
- **Core State Actor**: The system uses a strictly serial MPSC Channel Actor Model to manage state updates, preventing async/sync deadlock scenarios inherently.

## 4. Operational Safety Layers

- **OutputValidator**: Automatically retries parsing when LLM returns invalid JSON schema.
- **PathSandbox**: Prevents directory traversal by enforcing canonical path prefix checks.
- **ZombieKiller**: Monitors and terminates hung external processes/subprocesses.
- **Karma Federation**: Synchronizes "learned lessons" across nodes using signed and authenticated payloads.
- **Dynamic LLM Provider**: Centralized in `libs/infrastructure/src/llm/dynamic.rs` with automatic fallback chains (DB settings → env vars → defaults) and Circuit Breaker / SLO Engine integration.
- **Panic-Free Initialization**: All startup-critical operations use `unwrap_or_else` with error logging and `std::process::exit(1)` instead of `expect()`, preventing uncontrolled crashes.
- **Silent Error Elimination**: Database migration `.ok()` calls replaced with informational logging to surface potential schema issues during initialization.
- **Federated Telemetry Encryption**: `AutonomousBiomeEngine` encrypts all node-to-node (P2P) traffic relayed through the `Samsara Hub` via ChaCha20-Poly1305 symmetric encryption. The key is securely derived from `FEDERATION_SECRET`, mitigating eavesdropping or message tampering threats at the Hub/Network level.
- **Swarm Ops Deadlock Prevention**: The `do_sign_swarm_payload` function uses a linear flow (ensure key existence → sign) instead of recursive calls, preventing SQLite transaction nesting deadlocks caused by the single-writer constraint. All internal swarm operations (`get_node_id`, `tick_local_clock`, `sign_swarm_payload`) are called via direct `SwarmOps::do_*` methods rather than `JobQueue` trait dispatch to avoid both deadlocks and oversized async futures.
- **Async Future Size Control**: All 55+ delegation methods in `impl JobQueue for SqliteJobQueue` use `Box::pin` to heap-allocate individual futures, preventing the combined async state machine from exceeding thread stack limits.
- **Context Management System**: Prevents "AI hallucination" and "cascade errors." Integrates an immutable dependency map (`RIPPLE_MAP.md`) and Architectural Decision Records (ADRs) directly synced with the API/core code. Forces AI to execute preflight checks and review impact scopes before any source code mutation.
- **Preserve Intent Strategy (Phase 7.1)**: Utilizes ADR 007 standard for compiler warning suppression (`#[allow(...)]`). This ensures that context or "half-finished" logic is never accidentally deleted by AI agents during refactoring, maintaining a stable and explainable codebase while satisfying strict CI `-D warnings` constraints.
- **Biome Encryption & DB Recovery (Phase 6.X)**: `AutonomousBiomeEngine` encrypts all node-to-node (P2P) traffic via ChaCha20-Poly1305 symmetric encryption derived from `FEDERATION_SECRET`. `api-server` implements exhaustive error logging for all message/topic insertions (NG-29), ensuring database stability and eliminating silent failures in the Biome protocol.

## 5. Comparison with Traditional Systems

| Criteria | Existing Frameworks | Aiome |
|---|---|---|
| LLM Privileges | Full Access | Whitelisted Only |
| Plugin Loading | Dynamic/Remote | Compile-time / WASM Sandbox |
| Memory Safety | GC-based (Python/JS) | Ownership-based (Rust) |
| Validation | Middleware Dependent | Hardened Core Implementation |

---
*Last Mutated: 2026-03-20*
*Managed by: Aiome Sovereign Task Force*

## 6. Deep Dive: The Abyss Vault (Key Proxy)

Aiome's most critical defense Layer 2 is the **Abyss Vault**. This mechanism ensures that the AI agent *never* touches a physical API key string.

### 6.1 Logical Isolation
The main AI process holds no secrets in its environment or memory. All LLM/API requests are forwarded to the `key-proxy` process. If the AI core is compromised, the attacker only gains "permission to request the proxy," not the keys themselves.

### 6.2 Physical Memory Security
- **mlockall**: The vault locks its process memory into physical RAM, preventing the OS from swapping secrets to disk (SSD/HDD) where they could be recovered from dumps.
- **zeroize**: All buffers containing sensitive strings are actively overwritten with zeros (0x00) immediately after use, rather than waiting for garbage collection.

### 6.3 Self-Wiping Environment
Within milliseconds of startup, the vault reads the API keys from environment variables and then physically wipes those env-vars from its process space, neutralizing `/proc` peering or secondary process inspection.

### 6.4 SSRF & Routing Lockdown
Endpoints are hardcoded in the vault's source code. The proxy ignores arbitrary URLs from the agent and only routes to official provider endpoints (e.g., Google, OpenAI), making SSRF (Server-Side Request Forgery) structurally impossible.
