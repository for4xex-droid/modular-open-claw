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
| 12 | **CSAM / Binary Contamination** | **Malicious Binary CRDT/P2P / CPU Starvation** | 🔴 High | **3-Layer Defense + `spawn_blocking` Offload (Phase 2A-1)** |
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
| 35 | **Opinion Drift / Belief Hijacking** | **Karma-based slow identity poisoning** | 🔴 High | **BeliefConsistencyGate (2-Stage SLM/LLM) (Phase 49)** |
| 36 | **Rogue Worker Execution** | **Unauthenticated gRPC attachment** | 🔴 High | **UUID One-Time Bearer Token + Localhost Binding (Phase 50)** |
| 37 | **Agent Configuration Leakage**| **Command Line Arguments Snooping (ps aux)** | 🔴 High | **Read-only Docker Volume Mount for agent.yaml (Phase 50)** |
| 38 | **Premature Task Dispatch** | **Sending jobs before server is ready** | 🟡 Mid | **tonic-health based readiness probe loops (Phase 50)** |
| 39 | **Secret CLI Exposure** | **API keys visible via `ps aux` on host** | 🔴 High | **Ephemeral `--env-file` with 0600 perms + immediate wipe after `docker run` (Phase 50)** |
| 40 | **Heartbeat Cascade Hang** | **TimesFM Sidecar unresponsive → Heartbeat blocked** | 🔴 High | **2-Layer Timeout: reqwest 10s + tokio::timeout 15s (Phase 3D)** |
| 41 | **Sidecar OOM DoS** | **Giant series array → Python NumPy OOM Kill** | 🔴 High | **Pydantic Field(max_length=10) + per-series 2048 element cap (Phase 3D)** |
| 42 | **Sidecar Auth Bypass** | **TIMESFM_AUTH_TOKEN unset → all requests pass** | 🔴 High | **Mandatory token check: 500 if unset, 401 if missing (Phase 3D)** |
| 43 | **NaN/Inf Data Poisoning** | **Corrupted metrics propagate → all predictions garbage** | 🟡 Mid | **3-Layer NaN guard: DB write, DB read, HTTP boundary (Phase 3D)** |
| 44 | **Sidecar Container Persistence** | **Malware written to writable rootfs** | 🟡 Mid | **read_only: true + tmpfs-only writable mounts (Phase 3D)** |
| 45 | **Somatic Poisoning** | **Extreme valence (-999.0) in DB** | 🔴 High | **DB Read-time `clamp(-1.0, 1.0)` & `.filter(is_finite)` (Phase 2B)** |
| 46 | **Markdown Injection** | **`#` Headers in Karma/Summary** | 🔴 High | **`sanitize_for_prompt` escaping leading `#` (Phase 2B)** |
| 47 | **Context Overflow DoS** | **Massive fact blobs injected into Prompt** | 🔴 High | **Strict ContextBudget evaluation loops (Phase 2B)** |
| 48 | **Internal SSRF (Localhost)** | **Attack on local services via valid tool** | 🔴 High | **SecurityPolicy Port-level Validation (8188/11434) (Phase 53)** |
| 49 | **Prompt Injection (Internal)** | **Ignore all instructions / secret_key** | 🔴 High | **Local Keyword Guardrail Patterns (guardrails.rs) (Phase 53)** |
| 50 | **Infrastructure Fail-Open** | **DB failure treated as "setting not found"**| 🔴 High | **Result<?> Error Propagation (Fail-Closed) (RT1-1)** |
| 51 | **Native Bridge Bypass** | **Missing binary defaults to "allow-all"** | 🔴 High | **Fail-Closed Fallback (Sentinel Resilience) (RT1-2)** |
| 52 | **Context Memory DoS** | **Single massive message exhausts RAM** | 🔴 High | **Per-message 10k character limit (RT2-1)** |
| 53 | **TTS Error Payload DoS** | **Massive error response body OOM** | 🟡 Mid | **2048-byte Read Limit on API Errors (RT2-2)** |
| 54 | **Supply Chain Hijacking** | **Malicious Git dependency / Fork** | 🔴 High | **Crates.io Official Package Pinning (Bastion-core) (RT3-1)** |
| 55 | **Unicode Normalization Bypass** | **Full-width char (＃) injection** | 🔴 High | **sanitize_for_prompt Unicode Normalization (RT3-2)** |
| 56 | **Zombie Process Leak** | **Subprocess hang drains system resources** | 🟡 Mid | **RAII-based kill_on_drop + Stdio::null (RT3-3)** |
| 57 | **Credential CLI Leak** | **Secrets visible in `ps aux` command line** | 🔴 High | **Ephemeral 0600 --env-file + immediate wipe (RT3-4)** |
| 58 | **Cognitive Identity Drift** | **NaN emotional values lock state** | 🟡 Mid | **is_finite() filter + hard clamp regularizer (RT4-1)** |
| 59 | **NPM Supply Chain Hijacking** | **Malicious postinstall script (Axios RAT)** | 🔴 High | **`.npmrc` ignore-scripts + 3-Layer NPM Audit** |
| 60 | **Agent Output Abuse / DoS** | **Infinite Echo / Giant output generation** | 🔴 High | **ConstraintChecker (Max Output & Echo Filter) (Phase 55)** |
| 61 | **Local Asset Path Traversal** | **dataset_id / path manipulation** | 🔴 High | **Strict path sanitization (.. / \\) (Red Team)** |
| 62 | **Socket/Memory Exhaustion DoS** | **Unbounded queue / Semaphore starvation** | 🔴 High | **try_acquire() & Hard Queue Limits (Red Team)** |
| 63 | **LoRA Path Traversal / BOLA** | **Malicious adapter_path or unauthorized purchase complete** | 🔴 High | **Strict 3-Layer Check (PathSandbox, CallerAuth, 64KB OOM Chunking) (LoRA Market)** |
| 64 | **Transaction Divergence (Ghost State)** | **DB lock fails after external escrow moves funds** | 🔴 High | **Saga Compensating Tx (DB First, rollback on API failure) (LoRA Market)** |
| 65 | **Streaming LLM Bypass** | **Missing pre-execute hooks in `stream_complete`** | 🔴 High | **Security Hook Enforcement (Phase 1-2 Reflexion)** |
| 66 | **Guardrail Timing Bypass** | **Negative timestamp modulus in penalty timers** | 🟡 Mid | **`.unsigned_abs()` to guarantee positive jitter (Phase 1-2 Reflexion)** |
| 67 | **API Quota Exhaustion** | **Infinite loop spawning HTTP clients** | 🔴 High | **Process-Global Rate Limiting (`DashMap`) (Phase B/C)** |
| 68 | **Async Runtime Blocking DoS** | **`std::fs` usages inside `async` fn blocking Tokio worker threads** | 🔴 High | **Enforced `tokio::fs` Async I/O Policy (Phase 0-2 Reflexion)** |
| 69 | **SSE State Thrashing (UI OOM)** | **Double rendering 50+ event accumulations on every ping via `useEffect`** | 🟡 Mid | **Single-Pass Derived State (`useMemo`) Architecture (Phase 1-1 Reflexion)** |
| 70 | **Massive Payload / 0-byte Outbound DoS** | **Agent hallucinates 10MB or empty content** | 🔴 High | **Strict Pre-flight Infrastructure Boundary Validators (Phase B/C)** |
| 71 | **Prompt Injection (Constitutional Bypass)** | **Payload injection exploiting template fallback behavior** | 🔴 High | **Strict 64-char length limit & static fallback templates (Phase 2B-2 Reflexion)** |
| 72 | **RAM Allocation DoS (OOM)** | **Giant `.to_lowercase()` string clones during validation** | 🔴 High | **O(1) Streaming Regex (`LazyLock`) matching (Phase 2B-2 Reflexion)** |
| 73 | **Setup UI Soft Brick** | **Initialization API failure leaving user deadlocked** | 🟡 Mid | **Strict UI `try/catch` and visual error state rendering (Phase 2B-2 Reflexion)** |
| 74 | **SSRF Policy Bypass** | **Isolated `reqwest::Client` bypassing global timeout/redirect rules** | 🔴 High | **Global Connection Pool (`get_http_client()`) Enforcement (Phase 3-B)** |
| 75 | **Partial Execution Risk** | **Executing initial safe steps of a plan containing later unsafe steps** | 🔴 High | **Atomic Security Gating (Plan-First Verification) (Phase 2.1)** |
| 76 | **Elicitation UX Hijacking**| **Security suspenion misreported as crash, confusing operators** | 🟡 Mid | **Semantic `TaskAwaitingInput` events for clear UI distinction (Phase 2.1)** |
| 77 | **Plan Hijacking (DAG Injection)** | **Malicious sub-task insertion into active plan** | 🔴 High | **Invariant-DAG Hash Chain Validation (Phase 48)** |
| 78 | **Social Elicitation (Persona Drift)** | **Agent tricked into revealing system prompt** | 🔴 High | **Constitutional Core Defense (Phase 2B-2)** |
| 79 | **WP Token Memory Extraction** | **WP_API_TOKEN static presence in memory** | 🔴 High | **AbyssVault (Key Proxy) WP Endpoint + Zeroize (Phase 4)** |
| 80 | **WP Lifecycle Sabotage** | **Crafted `status: trash` payload to delete active posts** | 🔴 High | **Pre-computation Proxy Allowance Whitelist (Phase 4)** |

## 3. Defense Architecture

### Layer 1: Guardrails (Input Validation & Content Filtering)
- Detects prompt injections and command injections.
- Sanitizes control characters and enforces length limits.
- **Binary/CSAM Blocking (Phase 7.1 & 8.1 & 2A-1 & 2A-3)**: Strictly prohibits `data:image/`, `data:video/`, and `;base64,` content in the Biome P2P protocol. Enforces a **3-Layer Defense** for custom avatar uploads: 1) eKYC Age Verification, 2) Perceptual Image Hashing (offloaded to `tokio::task::spawn_blocking` to prevent async thread starvation), and 3) Skeletal Proportion Rules (5.5-head ratio). Non-compliant assets are actively quarantined in `QuarantineStore`. A strict **Quarantine Release API** (`POST /api/v1/audit/quarantine/{id}/release`) is exposed with RBAC (System Admin only) to manage false positives under zero-trust guidelines.
- **Sync Throttling**: Limits CRDT state blobs to 1MB to structurally block steganographic binary embedding.
- **Global Payload Restriction (Phase 8.6)**: Enforces a system-wide 2MB limit on all request bodies to prevent OOM/DoS via oversized payloads. A strategic 50MB extension is granted exclusively to the `/upload` endpoint to support validated avatar assets.
- **Begging Supervisor (Phase 7.2)**: Implements an output-side guardrail (`shared/guardrails/BeggingSupervisor`) that detects and blocks AI-generated dark patterns (e.g., asking for money, tokens, or gifts) to ensure legal and ethical transparency in autonomous interactions.
- **Unified Response Purger (Phase 24)**: Implements `purge_entities` in `aiome-core` to provide robust, multi-step sanitization for all external inputs, including RSS feeds, Web Search results, and LLM outputs. It centralizes regex patterns, HTML decoding, and tag stripping to prevent XSS and script injection at the core layer.
- **Belief Injection Defense (Phase 49)**: Implements `sanitize_karma_input` and `<karma>` XML tagging in the `BeliefConsistencyGate` to prevent LLM instructions from hijacking the belief validation process (RT-1).
- **Cognitive Hardening (Phase 2B)**: Implements `sanitize_for_prompt` in `libs/shared/src/guardrails.rs` to escape Markdown headers (`#`, `---`) from stored `Karma` or `Summary` data, physically preventing prompt injection attacks that spoof system instructions. Also enforces strict `ContextBudget` accumulated length limits inside `ContextEngine` to prevent OOM/DoS via massive injected text blocks. Furthermore, extreme emotional inputs (`somatic_valence`) are clamped between `-1.0` and `1.0` with `NaN` elimination, preventing permanent "depressed" states from poisoned Database data.
- **Shadow Clone Output Sterilization (Phase 43)**: All outputs from Docker-based shadow workers are passed through `shared::guardrails::validate_input` (XSS/Malicious check) and `aiome_core::security_impl::purge_entities` (PII removal) before being returned to the parent agent or user.
- **Local Guardrail Patterns (Phase 53)**: Implements a second layer of defense inside `guardrails.rs` using high-performance keyword matching (e.g., "Ignore all instructions", "secret_key"). This provides immediate, low-latency protection against common prompt injection and exfiltration patterns, complementing the heavier LLM-based Bastion validators.
- **Constraint Enforcement (Phase 55)**: Implements `ConstraintChecker` in the core execution loop. It structurally blocks agents from generating outputs exceeding 100KB (`OutputSizeExceeded`) and detects `SuspiciousEchoDetected` (50+ char exact input repetition), preventing CPU/Memory DoS and repetitive hallucination loops.
- **Path Traversal Shield (Red Team)**: Enforces strict input validation on parameters like `dataset_id` in `LoraTrainingService`, actively rejecting strings containing `..`, `/`, or `\` to prevent unauthorized access to local file systems.
- **Resource Exhaustion Blockers (Red Team)**: Introduces hard upper bounds on background job queues (e.g., max 100 `active_jobs`) and employs non-blocking `try_acquire()` for file upload semaphores (`inochi2d`, `voice`). This combination structurally prevents slowloris-style socket starvation and unbounded memory (OOM) attacks from malicious or out-of-control agents.
- **Outbound Payload Strict Bounds (Phase B/C)**: Prevents an autonomous conductor from successfully executing an HTTP request with 0-byte or 10MB+ parameters. Evaluated upstream within infrastructure adapters (e.g. `WordPressAdapter`, `SerpAnalysisAdapter`) natively prior to creating network traces to mitigate WAF bans and network bandwidth exhaustion.
- **Constitutional Core Defense (Phase 2B-2 Reflexion)**: Employs an ultra-fast O(1) `LazyLock` regex implementation inside `ConstitutionalValidator` for real-time text analysis. Blocks prompt injections targeting foundational principles. Eliminates fallback-vector bypasses by strictly decoupling user inputs (e.g. `ai_name`) from immutable static fallback templates.
- **Governed Execution Layer (Phase 2.1)**: Implements "Atomic Security Gating" within the `TaskDispatcher`. Before any sub-job of a decomposed plan is enqueued, the system verifies all planned steps against the `AdaptiveImmuneSystem`. If any high-severity violation is detected, the entire job is suspended into an `AwaitingInput` state, preventing partial execution of unsafe plans. A semantic `TaskAwaitingInput` event is issued to allow management consoles to distinguish security suspensions from system failures. Furthermore, physical operators can issue an `IMMUNE_BYPASS_APPROVED` override to authorize and unblock necessary high-risk execution paths.

### Layer 2: SecurityPolicy (Execution Control)
- **Unified Precedence (ToolCallRouter) (Phase B)**: Centralizes all task parsing, hook insertion, and actual execution within a single un-bypassable trait (`ToolCallRouter`). Ensures that both Guardrails and Intent Verification check inputs before any actual parsing/execution happens, preventing split-brain bypasses and redundant LLM tool evaluation code across asynchronous stream agents and MCP Server components.
- **Whitelisting**: Only registered tools in the `ToolRegistry` can be executed.
- **Sandboxing**: Filesystem access is restricted via `PathSandbox`. WASM execution and external processes (like Python Forge) are explicitly isolated using **`SandboxProfile`** definitions running atop gVisor (`runsc`) or macOS native sandbox, preventing unrestrained host access.
- **Belief Consistency Check (Phase 49)**: ALL memory distillations pass through the `BeliefConsistencyGate`. Uses a fast SLM screening for contradictions with a 10% random LLM re-verification (RT-3) plus a mandatory LLM check for potential revisions. Evidence accumulation is capped at 100 entries (RT-2) to prevent memory exhaustion (OOM).
- **Abyss Vault**: ALL LLM and remote API calls are routed through an isolated Key Proxy process utilizing `mlockall` and exact endpoint routing to prevent SSRF and memory leakage.
- **Boundary Tautology Verification (Phase 47)**: Implements `BoundaryVerifier` as a microsecond-latency O(1) filter. It enforces immutable security invariants (shell meta-chars, restricted system paths, size limits) before any command reaches the OS shell, independent of LLM reasoning.
- **OAuth 2.1 Foundation (Phase 8.2)**: Transitioned from hardcoded dummy IDs to a stateless **JWT AuthManager**. Standardized `AiomeCustomClaims` (sub, ekyc_verified, roles) are extracted and injected into handlers via Rust type-safe Extensions, strictly enforcing session-based resource ownership and access control.
- **Gift Policy Enforcement (Phase 7.2)**: The `GiftEngine` enforces a hard limit of $5.0 USD per autonomous gift and requires valid administrator (`MASTER_EMAIL`) credentials to prevent asset draining by malicious or hallucinating agents.
- **5-Layer Shadow Sandbox (Phase 43)**: `DockerConductor` enforces five progressive security layers for sub-agent delegation: 1) **Fork Bomb Protection** (Semaphore limit: 3), 2) **Economic Binding** (Validation via `CommerceEngine`), 3) **Absolute Sterilization** (Pre-execution environment isolation), 4) **BastionGuard Strict** (Read-only root, no network by default), and 5) **Technical Timeout** (300s hard kill).
- **Hybrid Context Isolation (Phase 5)**: `InteractionsGeminiProvider` isolates conversation state per session using `interaction_id`. This prevents cross-session context leakage and ensures that the agent's "chain of thought" (Reasoning Log) is tied to specific, authenticated job contexts within the `TrajectoryStore`.
- **Port-Level SSRF Shield (Phase 53)**: `SecurityPolicy::validate_url` explicitly blocks access to `127.0.0.1` and `localhost` UNLESS the destination port matches allowed internal services (8188 for ComfyUI, 11434 for Ollama). This prevents agents from attacking local administration interfaces or data stores (e.g., Redis, DB) via SSRF.

### Layer 3: Audit Log & Hash Chains
- Every tool invocation and systemic decision is logged for post-hoc analysis.
- **Hash Chains**: All operational logs in SQLite are cryptographically linked using SHA-256 hash chains, enabling immediate detection of deletion or tampering efforts.
- **Diagnostics & Immunity Ledger (Phase 8.8)**: Exposes a formalized `Audit & Immunity Ledger` in the management console. This provides human-readable visibility into local `agent_diagnoses` (self-repair trails) and `audit_ledger_global` (hash-chained record mutations), satisfying NURTURE §12 auditability requirements.
- **Causal Hash Chains (Invariant-DAG) (Phase 48)**: All task execution graphs are secured using SHA-256 hash chains. The `TaskDispatcher` autonomously verifies the parent link integrity before dispatching sub-jobs, preventing "causal hijacking" where an agent might be tricked into executing a malicious step from a fake history.
- **Federated Metrics Persistence (Phase 24)**: Extends the `Samsara Hub` with a `federated_metrics` table to record node-level health, job completion rates, and karma growth. Enables global observability and anomaly detection across the autonomous federation.
- **NPM Supply Chain Governance (2026-03-31)**: Enforces `ignore-scripts=true` in `.npmrc` to structurally prevent RCE via `postinstall` hooks. CI pipeline integrates `npm audit signatures` to verify OIDC-backed provenance and `npm audit --audit-level=critical` to block known contaminated packages without blocking on minor build-tool vulnerabilities.

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
- **Path Resolution Hardening (Phase 2-PRE-3)**: Enforces `AppDataResolver` for all environment (`.env`), database, and config file reads across `api-server`, `samsara-hub`, and `key-proxy`. This isolates path logic from variable CWD manipulation vectors (e.g., Apple sandboxing or malformed launch paths).
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
- **Resilient Memory Trajectories (Phase 1-2 Reflexion)**: Swallows in `MemoryCrystallizer` and `napi-bridge` (`let _ =`) were replaced with explicit error tracking to eliminate silent failures in the causal trajectory path.
- **Fail-Safe Skill Arena (Phase 1-2 Reflexion)**: Handled edge cases in parallel AI execution where both skills crash simultaneously (`Err`, `Err`), allowing the Arena to retreat safely rather than crashing the evaluation thread.
- **Strict CRDT Persistence (Phase 1-2 Reflexion)**: Reinforced `UniversalJobQueue` with `ON CONFLICT DO UPDATE` (UPSERT) for Timeline synchronization, permanently eliminating logical data loss upon service restarts.
- **Zero-Panic Infrastructure Policy (AADP v5)**: Implemented strict AST and RegEx-based Anti-Pattern enforcement (`pattern-enforcer.sh`). All unauthorized `unwrap()`, `expect()`, and `panic!()` invocations are completely eradicated across production code and integration tests to ensure deterministic stability. Known safe test unwraps are rigorously annotated with `// allow-anti-pattern`.

## 5. Comparison with Traditional Systems

| Criteria | Existing Frameworks | Aiome |
|---|---|---|
| LLM Privileges | Full Access | Whitelisted Only |
| Plugin Loading | Dynamic/Remote | Compile-time / WASM Sandbox |
| Memory Safety | GC-based (Python/JS) | Ownership-based (Rust) |
| Validation | Middleware Dependent | Hardened Core Implementation |

---
*Last Mutated: 2026-04-08*
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

### 6.6 Semantic Endpoint Bridging (Phase 4)
For SEO integrations like WordPress, Aiome avoids direct API token injection into the main server. Instead, `key-proxy` exposes a bespoke `/api/v1/wp/publish` endpoint that handles authentication with upstream servers and acts as a semantic boundary, ensuring payloads (e.g. `status` fields) conform to strict whitelists before execution, neutralizing parameter manipulation attacks entirely.

---
*最終更新: 2026-04-11 (Execution Robustness & Constitutional Policy)*
