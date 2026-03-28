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
| 13 | **Session Hijacking / Weak Auth** | **Bearer Token Brute Force / Static IDs** | 🔴 High | **OAuth 2.1 / JWT AuthManager + extension-based User Extractors (Phase 8.2)** |
| 14 | **Global API DoS / OOM** | **Oversized Request Body (Global)** | 🟡 Mid | **Global 2MB Limit (RequestBodyLimitLayer) + 50MB Avatar Bypass (Phase 8.6)** |
| 15 | **Stripe API Key Leak / Null** | **Empty key in Release build** | 🔴 High | **Mandatory Release-build Env Check (Fail-safe) (Phase 14)** |
| 16 | **Unauthorized Economic Activity** | **Unverified users sending gifts/buying** | 🔴 High | **Mandatory eKYC Verified Claim Enforcement (403 Forbidden) (Phase 16)** |
| 17 | **Revenue Split Inconsistency** | **License grant without accounting** | 🔴 High | **Atomic Transaction: RevenueSplitter + License Grant (Phase 16)** |
| 18 | **Persistent Env Var Secrets** | **Secrets visible in /proc/pid/environ** | 🔴 High | **Immediate std::env::remove_var (Zeroize) after Startup Load (Phase 16)** |
| 19 | **Web/RSS Content Injection** | **Malicious RSS/Search snippets** | 🔴 High | **Unified Response Purger (purge_entities) (Phase 24)** |
| 20 | **Malicious Skill Import** | **Skill crafts backdoors/vampire attacks** | 🔴 High | **AI-Driven Code Audit (Cleanroom) (Phase 24)** |
| 21 | **Federation Blind Spot** | **Node health/metrics unseen by Hub** | 🟡 Mid | **Periodic Federated Metrics Push (Samsara Hub) (Phase 24)** |
| 22 | **Unauthorized Demo Access** | **Unauthenticated demo trigger** | 🔴 High | **Bearer Auth + MockAuthManager (Phase 25)** |
| 23 | **SQLite Pool Exhaustion** | **Multi-tab SSE + gig_engine Transactions** | 🔴 High | **Transaction-free Demo SQL (ADR-014) + Audit Trigger Suspension (Phase 25.5)** |
| 24 | **Testing Mocks in Production** | **Mock object leakage (e.g. MockLlmProvider)** | 🔴 High | **Conditional Compilation (#[cfg]) Isolation (Phase 27)** |
| 25 | **Insecure Default Secrets** | **Fallback API_SERVER_SECRET** | 🔴 High | **Mandatory Release-build Env Check (Fail-safe) (Phase 27)** |
| 26 | **Unauthorized Frontend Requests**| **CORS misconfiguration (AllowOrigin::any)** | 🔴 High | **Strict ALLOWED_ORIGINS Enforcement (Phase 27)** |
| 27 | **Runtime Database Panic** | **unwrap() on Pool in PG mode** | 🟡 Mid | **Safe DatabasePool Getters (Phase 31)** |
| 28 | LLM Format Mismatch | JSON expected, Text received | 🟡 Mid | **LLM Structured Output (format: json) (Phase 31)** |
| 29 | Shadow Clone Hijacking | Docker Bomb / Secret Exfiltration | 🔴 High | **5-Layer Shadow Defense (Semaphore, Commerce, Bastion, Timeout, Purge) (Phase 43)** |
| 30 | **Gemini Session Hijacking**| **Context Poisoning via interaction_id** | 🔴 High | **Interaction ID Validation + Trait-Based Provider Isolation (Phase 5)** |
| 31 | **Secret Duplication in Memory**| **Secrets cloned via config.clone()** | 🔴 High | **Arc<AiomeConfig> sharing & immediate env::remove (Phase 13.3)** |
| 32 | **Memory Bloat & Cognitive Noise** | **Endless ingestion of low-value artifacts** | 🟡 Mid | **Poincare-based Autonomous GC (Phase 4)** |
| 33 | **Boundary Violation** | **Malformed Shell Command Injection** | 🔴 High | **BoundaryVerifier (O(1) Tautology) (Phase 47)** |
| 34 | **Causal Tampering** | **Job Graph Transformation / History Deletion** | 🔴 High | **Invariant-DAG (SHA-256 Hash Chain) (Phase 48)** |
## 3. Defense Architecture

### Layer 1: Guardrails (Input Validation & Content Filtering)
- Detects prompt injections and command injections.
- Sanitizes control characters and enforces length limits.
- **Binary/CSAM Blocking (Phase 7.1 & 8.1)**: Strictly prohibits `data:image/`, `data:video/`, and `;base64,` content in the Biome P2P protocol. Enforces a **3-Layer Defense** for custom avatar uploads: 1) eKYC Age Verification, 2) Perceptual Image Hashing (against illegal CSAM blacklists), and 3) Skeletal Proportion Rules (5.5-head ratio to prevent child-like models). Non-compliant assets are actively quarantined and their metadata is persistently stored in the `QuarantineStore` (SQLite) to prevent bypasses and facilitate audits. **Phase 14** adds **EKYC Session Persistence** via SQLite, ensuring continuous verification state across server restarts.
- **Sync Throttling**: Limits CRDT state blobs to 1MB to structurally block steganographic binary embedding.
- **Global Payload Restriction (Phase 8.6)**: Enforces a system-wide 2MB limit on all request bodies to prevent OOM/DoS via oversized payloads. A strategic 50MB extension is granted exclusively to the `/upload` endpoint to support validated avatar assets.
- **Begging Supervisor (Phase 7.2)**: Implements an output-side guardrail (`shared/guardrails/BeggingSupervisor`) that detects and blocks AI-generated dark patterns (e.g., asking for money, tokens, or gifts) to ensure legal and ethical transparency in autonomous interactions.
- **Unified Response Purger (Phase 24)**: Implements `purge_entities` in `aiome-core` to provide robust, multi-step sanitization for all external inputs, including RSS feeds, Web Search results, and LLM outputs. It centralizes regex patterns, HTML decoding, and tag stripping to prevent XSS and script injection at the core layer.
- **Shadow Clone Output Sterilization (Phase 43)**: All outputs from Docker-based shadow workers are passed through `shared::guardrails::validate_input` (XSS/Malicious check) and `aiome_core::security_impl::purge_entities` (PII removal) before being returned to the parent agent or user.

### Layer 2: SecurityPolicy (Execution Control)
- **Whitelisting**: Only registered tools in the `ToolRegistry` can be executed.
- **Sandboxing**: Filesystem access is restricted via `PathSandbox`. WASM execution and external processes (like Python Forge) are explicitly isolated using **`SandboxProfile`** definitions running atop gVisor (`runsc`) or macOS native sandbox, preventing unrestrained host access.
- **Abyss Vault**: ALL LLM and remote API calls are routed through an isolated Key Proxy process utilizing `mlockall` and exact endpoint routing to prevent SSRF and memory leakage.
- **Boundary Tautology Verification (Phase 47)**: Implements `BoundaryVerifier` as a microsecond-latency O(1) filter. It enforces immutable security invariants (shell meta-chars, restricted system paths, size limits) before any command reaches the OS shell, independent of LLM reasoning.
- **OAuth 2.1 Foundation (Phase 8.2)**: Transitioned from hardcoded dummy IDs to a stateless **JWT AuthManager**. Standardized `AiomeCustomClaims` (sub, ekyc_verified, roles) are extracted and injected into handlers via Rust type-safe Extensions, strictly enforcing session-based resource ownership and access control.
- **Gift Policy Enforcement (Phase 7.2)**: The `GiftEngine` enforces a hard limit of $5.0 USD per autonomous gift and requires valid administrator (`MASTER_EMAIL`) credentials to prevent asset draining by malicious or hallucinating agents.
- **5-Layer Shadow Sandbox (Phase 43)**: `DockerConductor` enforces five progressive security layers for sub-agent delegation: 1) **Fork Bomb Protection** (Semaphore limit: 3), 2) **Economic Binding** (Validation via `CommerceEngine`), 3) **Absolute Sterilization** (Pre-execution environment isolation), 4) **BastionGuard Strict** (Read-only root, no network by default), and 5) **Technical Timeout** (300s hard kill).
- **Hybrid Context Isolation (Phase 5)**: `InteractionsGeminiProvider` isolates conversation state per session using `interaction_id`. This prevents cross-session context leakage and ensures that the agent's "chain of thought" (Reasoning Log) is tied to specific, authenticated job contexts within the `TrajectoryStore`.

### Layer 3: Audit Log & Hash Chains
- Every tool invocation and systemic decision is logged for post-hoc analysis.
- **Hash Chains**: All operational logs in SQLite are cryptographically linked using SHA-256 hash chains, enabling immediate detection of deletion or tampering efforts.
- **Diagnostics & Immunity Ledger (Phase 8.8)**: Exposes a formalized `Audit & Immunity Ledger` in the management console. This provides human-readable visibility into local `agent_diagnoses` (self-repair trails) and `audit_ledger_global` (hash-chained record mutations), satisfying NURTURE §12 auditability requirements.
- **Causal Hash Chains (Invariant-DAG) (Phase 48)**: All task execution graphs are secured using SHA-256 hash chains. The `TaskDispatcher` autonomously verifies the parent link integrity before dispatching sub-jobs, preventing "causal hijacking" where an agent might be tricked into executing a malicious step from a fake history.
- **Federated Metrics Persistence (Phase 24)**: Extends the `Samsara Hub` with a `federated_metrics` table to record node-level health, job completion rates, and karma growth. Enables global observability and anomaly detection across the autonomous federation.

### Layer 4: Build Isolation & Formal TDD Forge (S-Rank Defense)
- **OS-Native Sandbox**: Autonomous compilation (`cargo build`) executed by the agent is forcibly containerized using OS-native guardrails (`sandbox-exec` / `bwrap`) to prevent supply chain attacks during the Forge process.
- **Fail-Forward Training**: Instead of terminating agents when code fails to compile, the system employs TDD-based reinforcement loops without permanent Karma penalties, allowing self-healing code generation.
- **AI-Driven Skill Audit (Phase 24)**: Integrates LLM-based security auditing into the `Cleanroom` forge process. Before compiling any imported skill, the system performs an AI-driven review to detect malicious patterns, unauthorized network calls, or "Vampire Attacks" (credential exfiltration) that might bypass static WASM analysis.
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
- **SQLite Pool Exhaustion Prevention (Phase 25.5 / ADR-014)**: The `AutonomousDemo` orchestrator avoids `gig_engine` trait method calls (which internally use `pool.begin()` transactions) and instead executes individual SQL statements. This prevents connection pool exhaustion when multiple browser tabs maintain concurrent SSE connections. Audit triggers on gig tables are temporarily suspended during demo execution to eliminate cascading WRITE lock contention. See `docs/decisions/014-sqlite-pool-exhaustion-demo-strategy.md` for the full production migration plan (PostgreSQL, async audit logging, SSE connection sharing).
- **Database Backend Safety (Phase 31)**: Eliminated 10+ instances of direct `unwrap()` on database pools. All internal router handlers now use safe getters returning explicit Errors via `DatabasePool::get_sqlite_pool_or_err`. This prevents system-wide crashes when switching to alternative backends (e.g., PostgreSQL for high concurrency).
- **LLM Structured Output (Phase 31)**: Formally supports `format: "json"` in `LlmRequest` to enforce structured responses from LLMs (Ollama), reducing parsing errors and potential hallucination impacts.
- **Autonomous Memory Lifecycle (Phase 4)**: Mitigates "cognitive noise" and resource exhaustion by autonomously pruning low-importance memories. Integrates `SlmBridge` for Poincare-based importance scoring and enforces a 0.3 threshold for background archival via Watchtower.

## 5. Comparison with Traditional Systems

| Criteria | Existing Frameworks | Aiome |
|---|---|---|
| LLM Privileges | Full Access | Whitelisted Only |
| Plugin Loading | Dynamic/Remote | Compile-time / WASM Sandbox |
| Memory Safety | GC-based (Python/JS) | Ownership-based (Rust) |
| Validation | Middleware Dependent | Hardened Core Implementation |

---
*Last Mutated: 2026-03-23*
*Managed by: Aiome Sovereign Task Force (Ref: Phase 27 — Architecture Audit & Hardening)*

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

### 6.5 Key Hierarchy (Phase 11)
The Voice DRM and future encrypted assets rely on a strict key hierarchy:
1. **Master Key (`VAULT_MASTER_KEY`)**: A symmetric 256-bit AES key, provided via environment variable, representing the root of trust. Cached securely in memory via `OnceCell` to minimize environmental reads and securely zeroized.
2. **Asset Data Keys (DEK)**: A unique 256-bit symmetric key is randomly generated per uploaded asset (e.g. Voice Models) for AES-256-GCM.
3. **Encrypted Key Storage (KEK)**: The Asset Data Keys are encrypted by the Master Key and stored persistently in the `vault_keys` SQLite table, ensuring that a database compromise without the Master Key yields no usable assets.

---
*最終更新: 2026-03-28 (Phase 48 / Invariant-DAG Foundation)*
