# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Phase 8.3 (Type-Driven Security)**:
    - **Type-Level Enforcement**: Added `_auth: Authenticated` extractor to all (45) public asynchronous route handlers in `api-server` (agent, biome, karma, artifacts, skill, streams, expression, general), ensuring that unauthenticated routes cannot compile.
    - **Performance Optimization**: Cached `system_agent_id` into `AppState` during application startup, eliminating redundant database queries inside the `Authenticated` extractor for hot-path APIs.
    - **CI Defense-in-Depth**: Introduced the `missing-auth-extractor` rule to `.github/anti-patterns.yml` and integrated `CC-6` (Auth Extractor Enforcement Ratio) into `scripts/deep-scan.sh` to prevent regressions in security guardrails.
- **Phase 8.2 (OAuth 2.1 Foundation)**:
    - **JWT Custom Claims**: Added `AiomeCustomClaims` in `libs/shared/auth` to define the standardized token structure (including `sub`, `ekyc_verified`, `roles`).
    - **AuthManager Interface**: Introduced `AuthManager` trait and `MockAuthManager` in `libs/infrastructure/auth` to handle stateless token validation.
    - **API Middlewares**: Implemented `jwt_auth_middleware` in `api-server` to extract and validate Bearer tokens, injecting `AuthenticatedUser` into request extensions.
    - **Route Refactoring**: Updated `/api/avatar/upload` and `/api/avatar/ekyc-status` to use dynamic user IDs and eKYC statuses from the JWT claims, eliminating `session_dummy` hardcodes.
- **Phase 8.1.5 (Asset Quarantine Persistence)**:
    - **Quarantine DB**: Added `QuarantineStore` and `SqliteQuarantineStore` to `libs/infrastructure` to provide a persistent "waiting room" for non-compliant assets (CSAM, eKYC fails, proportion violations).
    - **API Integration**: Integrated quarantine storage into the `/api/avatar/upload` flow, ensuring illegal or unvetted assets are recorded for audit and prevented from re-uploading via hash-based detection.
    - **Ripple Synchronization**: Fully synchronized `AppState` and `api_integration_tests.rs` to maintain high-quality CI through structural changes.
    - **Performance Optimization (Gap-5)**: Added SQLite index to `quarantined_assets.image_hash` for faster duplicate lookups.
    - **Bug Fix (E-101)**: Resolved a "Future is not Send" compilation error in `upload_avatar_handler` by ensuring `ImageHasher` is dropped before await points.
- **Phase 8.1 (CSAM 3-Layer Defense & Compliance)**:
    - **eKYC Integration (Stripe)**: Added `StripeEkycEngine` and `MockEkycEngine` in `libs/infrastructure/compliance` for strict 18+ real-name and age verification prior to custom asset publishing.
    - **Perceptual Image Hashing (CSAM Defense)**: Added `ImageHasher` in `libs/shared/csam` using DCT hashing (`img_hash`) to resiliently detect illegal or malicious textures against a known blacklist.
    - **NURTURE Compliance (5.5 Head-to-Body Checker)**: Implemented `ProportionsChecker` in `libs/shared/csam` to mathematically restrict avatar skeletal proportions that mimic underage anatomies (`LegalStatus::Restricted`).
    - **Asset Quarantine Flow**: Built `/api/avatar/upload` endpoint integrating the 3-layer defense to actively quarantine non-compliant assets from the Samsara Federation.
- **Phase 7.2 (Legal Guardrails & A2C Gratitude)**:
    - **Begging Supervisor**: Added `BeggingSupervisor` to `shared/guardrails` to detect and block AI-driven dark patterns (e.g., asking for money or gifts).
    - **Gift Engine (Tremendous API)**: Implemented `TremendousGiftEngine` in `libs/infrastructure` to enable autonomous real-world gift generation for users with high Karma.
    - **Autonomous Gratitude (A2C 恩返し)**: Integrated gift-triggering logic into `AutonomousBiomeEngine`, rewarding helpful human peers with $1-5 gift codes.
- **CSAM Defense & Inochi2D Foundation (Phase 7.1)**:
    - **Asset Separation Layer**: Implemented a mandatory asset/origin separation in `avatar-engine` to isolate local unvetted assets from Hub-syncable official assets.
    - **Protocol-Level CSAM Filter**: Integrated binary content detection (`data:image/`, `data:video/`, `;base64,`) into `samsara-hub` relay and `api-server` biome endpoints to strictly prohibit binary data embedding in the P2P network.
    - **Oversized CRDT Protection**: Added 1MB hard limit to `samsara-hub` timeline sync blobs to prevent covert binary data exfiltration through CRDT documents.
    - **Avatar Expression Mapper**: Created `libs/avatar-engine` to map AI emotions to VRM/Inochi2D blendshape parameters.
    - **SSE Avatar Parameters**: Extended the Server-Sent Events (SSE) stream to push `avatar_params` updates in real-time via the `avatar_expression` event.
    - **InxRenderer (WASM Foundation)**: Integrated `InxRenderer` into the management console's `DioramaView`, supporting the future `.inx` (Inochi2D) WASM runtime.
- **Context Management System (4-Layer Guardrails)**: Implemented an autonomous guardrail system to prevent AI agent contextual collapse and cascade errors. Includes `.context/RIPPLE_MAP.md` for deterministic dependency tracking, `preflight` workflow commands, Architecture Decision Records (ADRs `001` through `007`), and rigorous documentation synchronization rules.
- **Comprehensive Documentation Update**: Replaced 254 instances of "自動補完" (placeholder) documentation with context-aware, inferred descriptions across 47 Rust files. Resolved all `missing_documentation` warnings workspace-wide.
- **Preserve Intent Policy**: Established a workspace-wide policy (ADR 007) to suppress unused code/import warnings via `#[allow(...)]` instead of deletion, preserving developer context and intent for future features. Updated CI and all crate roots (`shared`, `infrastructure`, `soul`, `core`, `api-server`, `watchtower`) to enforce this policy.
- **Soul Engine (Phase 4 - Autonomic Deepening)**:
    - **Step 0 (Soul Memory)**: Added `SoulSnapshot` cache to `SqliteSoulStore` mapping memory to the chat LLM via `build_system_instructions`, eliminating DB latency on hot paths.
    - **Step 1 (Attachment Transitions)**: Implemented dynamics for `AttachmentStyle` shifting (Secure/Anxious/Avoidant/Disorganized) based on positive/negative interaction valences.
    - **Step 2 (Compound Triggers)**: Added true evaluation logic for `DefenseTrigger::Compound` in the internal `SoulPipeline`.
    - **Step 3 & 4 (Somatic Resonance & Reflex)**: Integrated Somatic Bias into predictions (incorporating intuitive emotion) and implemented fallback reflex defense generation for heavily negative experiences.
    - **Step 5 & 6 (Physical Execution & Temporal Decay)**: Mapped `RequireEscrow`, `Deflect`, and `Custom` defense actions to `Evolution Chronicle` audit trails, and added temporal intensity decay (death threshold 0.2) to prevent memory saturation over infinite rebirth cycles.
    - **Step 7 (Observability API)**: Added `GET /api/soul/status` to expose real-time internal state metrics (attachment style, active defenses count, soul resonance avg).
- **Soul Engine (Phase 3 - Reactive Hardening)**: 
    - **Step 1 (R-2)**: Improved `SoulError` granularity with context-aware prefixes (`[SoulDistill]`, `[SoulAdapter]`, etc.) for better debugging.
    - **Step 2 (Immune Feedback)**: Integrated `ImmuneSystem` threat detection into the `SoulPipeline` as negative experiences (`security.immune_response`), allowing the AI to learn from neutralized attacks.
    - **Step 3 (Somatic Marking)**: Enabled automatic generation of `SomaticMarker` (emotional grounding) during experience processing for all experiences with valence > 0.3 or < -0.3.
    - **Step 4 (Physical Defense)**: Connected `execute_defense` to physical actions: `DefenseAction::Reject` now dynamically injects `ImmuneRule`s, `Warn` records to the `Evolution Chronicle`, and `Hesitate` injects adaptive latency.
    - **Step 5 (Semantic Defense)**: Upgraded `SoulPipeline` to pre-fetch text embeddings asynchronously, enabling `DefenseTrigger::Semantic` cosine similarity matching while preventing redundant API calls (DS-1/RTT=0 optimization).
    - **Step 6 (Anamnesis Profile)**: Integrated `AnamnesisProfile` into `AgentSoul` and persistent storage, allowing narrative identity and maladaptive schemas to persist across generations via the `SamsaraEngine` rebirth cycle.
- **Soul Engine (Phase 1)**: Initial implementation of the Three-Layer Soul Architecture. Added `libs/soul` crate containing `AgentSoul` (Core), `PredictiveModel` (L2/Plasticity), `SomaticMarker` (L1/Reactive), and `Instinct` (L3/Meta-cognitive).
- **Soul Persistence**: Implemented `SqliteSoulStore` in `libs/infrastructure` for persisting and loading `AgentSoul` state, including a 10,000-entry experience buffer limit to prevent storage bloat.
- **Federated Soul Metadata**: Extended `FederatedKarma` and `approved_karma` schema to include `generation` and `somatic_valence`, enabling cross-node learning of personality traits.
- **AgentRx Framework Integration**: Implemented a comprehensive agentic diagnostic and recovery system including `TrajectoryStore` (SQLite), `ConstraintChecker` (rule-based validation), and `AgentRxDiagnostics` (LLM-based self-review). Added full integration into `skill_handler.rs` and the main chat loop for autonomous failure recovery.
- **WASM `fs_reader` Security Fix (P0)**: Corrected a path traversal vulnerability in the `fs_reader` WASM skill by replacing string-based prefix checks with component-aware `Path::starts_with`.
- **Enhanced BastionGuard Whitelisting & Flag Validation**: Strengthened `safe_exec` to recursively validate paths within command-line flags (e.g., `--file=path`) and explicitly blacklisted access to sensitive internal files like `.env`, `.git`, and `security.json` even within the allowed workspace.
- **AgentRx Schema Migration**: Added `trajectory_steps` and `agent_diagnoses` tables to the core database to enable persistent storage of agent execution paths and recovery hints.
- **Security Whitelist Optimization**: Removed `mv` from the default `BastionGuard` whitelist to prevent uncontrolled file movement and maintain consistency with other restricted destructive commands.
- **FTS5 Synchronization Triggers**: Implemented robust error handling for Search Index (FTS5) triggers in `migrations.rs`, replacing silent ignores with structured warnings and idempotent creation checks.
- **Duplicate Safety Directive Removal**: Cleaned up code across `infrastructure` and `core` crates by removing redundant `![forbid(unsafe_code)]` attributes.
- **Soul Engine Pipeline Integration (Phase 1.5)**: Implemented `DefaultSamsaraEngine` and `CoreDomainAdapter`, and integrated `SoulPipeline` and `SqliteSoulStore` directly into the `api-server` background worker loop for cyclic experience processing.
- **Experience Buffer Bounding**: Implemented `push_experience` to enforce a 1000-item memory limit on `experience_buffer` to prevent Out-Of-Memory issues during active sessions.
- **Soul Engine LLM Distillation (Phase 2)**: Core Samsara Engine now strictly limits distillation token context (`R-5`) and natively uses the LLM via `provider.complete` to generate real `InstinctRules` from recent experiences, replacing the static Phase 1 mock.
- **Soul Architecture Integration (Phase 2b)**: Unified `DreamState` (L0 Contemplation) and `SamsaraEvent::LevelUp` (L0 Progression) direct into `SoulPipeline` (L2/L3) converting environmental and introspective triggers into dynamic `Experience` objects safely decoupled from the heartbeat pattern (`R-6`).
- **Predictive Grounding**: Enabled actual `predict_outcome` capabilities inside the `CoreDomainAdapter`, activating `PredictiveModel` prediction accuracy calculations across arbitrary domains and enabling true surprise mechanics (`R-1`).
- **LoRA Configuration Persistence**: Added `lora_adapter_path` and `lora_base_model` columns to the `agent_souls` table in SQLite. Updated `SqliteSoulStore` to correctly save and load these values, ensuring LoRA-based fine-tuning settings persist across sessions.
- **LoRA Settings UI**: Integrated LoRA configuration inputs into the Management Console Settings page under the "AI Training & Voice" section, allowing users to specify paths for adapters and base models through the web interface.
- **Infrastructure Test Utilities Hardening**: Extracted and standardized `test_utils.rs` in the `infrastructure` crate. Fixed `MockJobQueue` function signatures and import paths to align with the core `JobQueue` trait, resolving compilation errors and improving test reliability.

### Fixed
- **Biome Encryption (NG-27, 28)**: Implemented automatic encryption in `send_message` and decryption in `list_messages` for Biome P2P dialogue using a shared secret derived from `FEDERATION_SECRET`.
- **Biome DB Stability (NG-29)**: Resolved silent database failures by adding proper error logging to all message/topic insertion paths in `api-server`.
- **Hub URL Unification (NG-30)**: Centralized all `SAMSARA_HUB_URL` resolution across the workspace via `AiomeConfig` and `AppState`, removing redundant `env::var` calls.
- **Ollama LoRA Dynamic Builder**: Implemented `build_lora_model()` inside `OllamaProvider`. When users select a LoRA adapter via the settings UI, the system autonomously builds a custom `Modelfile` and re-deploys a tailored model natively into the local Ollama backend, fixing the API parameter limitations (NG-21).
- **TTS Expression Engine**: Integrated actual Text-to-Speech synthesis (OpenAI's `tts-1`) into `ExpressionEngine`. Automatically converts AI expressions into local `.mp3` audio files and links them to the agent's internal state machine, replacing the previous placeholder behavior (NG-22).
- **Settings Sync Resilience**: Implemented automated state-hydration for `AgentSoul` in the `update_setting` API endpoint to fix UI-to-Backend synchronization gaps for LoRA parameter tuning.
- **Missing Whitelists**: Fixed missing settings keys (TTS configuration, LoRA paths, Voice selections) in API whitelist preventing database persistence.
- **Biome Transport Security**: Secured `AutonomousBiomeEngine` by enforcing symmetric encryption (ChaCha20-Poly1305) derived from `FEDERATION_SECRET` as a pseudo-HKDF, solving plaintext transport/storage vulnerabilities across the Samsara Hub.
- **LLM Economy Binding**: Integrated `CommerceEngine` directly into the `trigger_agent_chat` execution path, ensuring LLM invocations enforce autonomous token limits and simulate computational purchases correctly.
- **Infrastructure Auditing**: Resolved missing Apache License Headers in core API endpoints and testing utilities.
- **MockLlmProvider Stream Implementation**: Fixed a panic-prone placeholder in `MockLlmProvider::stream_complete` by providing a default empty stream implementation. This ensures stability when streaming is requested from mock providers during development or testing.
- **Soul Engine Robustness**: Resolved 16 issues discovered during deep scans, including `AgentSoul` ID validation, `compute_hash` logic for state integrity, safety attributes, and error conversion between `SoulError` and `AiomeError`.
- **Samsara Hub Data Integrity**: Fixed missing columns in `samsara-hub` SELECT/INSERT operations to ensure `generation` and `somatic_valence` are properly synchronized across the federation.
- **SQLite Deadlock in Swarm Ops (Critical)**: Resolved a deadlock in `do_sign_swarm_payload` where recursive calls to `do_get_node_id` created nested SQLite transactions, hitting the single-writer constraint and causing 8 karma tests to hang indefinitely. Refactored to a linear flow: ensure keys exist first, then sign without recursion.
- **Stack Overflow in JobQueue Trait Methods**: Applied `Box::pin` to all 55 delegation methods in `impl JobQueue for SqliteJobQueue` to heap-allocate async futures, preventing the 60+ method async state machine from overflowing the thread stack.
- **SwarmOps Direct Call Pattern**: Replaced `JobQueue` trait method calls (`get_node_id`, `tick_local_clock`, `sign_swarm_payload`) with direct `SwarmOps::do_*` calls in `guardrails.rs` and `karma.rs` to avoid pulling in the entire trait's massive async future.
- **Test Thread Stack Size**: Added `.cargo/config.toml` with `RUST_MIN_STACK=64MB` to ensure sufficient stack space for debug-build async futures.
- **Repository Hygiene**: Removed `check_output.txt` from the repository and added it to `.gitignore`.
- **Dream State JSON Injection (A-9)**: Replaced `format!`-based JSON construction with `serde_json::json!` macro in `dream_state.rs` to prevent JSON injection through unsanitized strings.

### Changed
- **Panic-Free Startup**: Replaced all `expect()` / `unwrap()` calls in `api-server/main.rs` startup path with `unwrap_or_else` + `error!` + `std::process::exit(1)` for graceful error reporting.
- **Guardrail Test Safety (B6)**: Removed unsafe and redundant `std::env::set_var` calls from `guardrails.rs` unit tests, relying on the secure default of `true` for `ENFORCE_GUARDRAIL`.
- **API Server Secret in Tests**: Updated `api_integration_tests.rs` to pass secrets via type-safe `AppState` instead of environment variables, aligning with the new centralized secret management.
- **CORS Configuration**: Migrated from hardcoded origin strings to `AiomeConfig.allowed_origins`, loaded dynamically from `ALLOWED_ORIGINS` env var.
- **Hub URL Resolution**: `SAMSARA_HUB_REST` and `SAMSARA_HUB_WS` now resolved from `AiomeConfig` instead of scattered `env::var()` calls.
- **Federation Secret**: `FEDERATION_SECRET` no longer panics when unset; instead logs a warning and defaults to empty string.
- **DB Migration Logging**: Replaced silent `.ok()` error suppression in `migrations.rs` with `info!` logging for index creation and ALTER TABLE operations.
- **Server Bind Error**: TCP listener bind failure now shows the actual address and exits gracefully instead of panicking.
- **API Server Modularization**: Extracted massive monolithic routing into `routes/` (karma, agent, biome, expression, general) to prepare for Biome integration.
- **Samsara Engine (Evolution)**: AI self-leveling based on cumulative Technical Karma weights (`do_sync_samsara_level`).
- **Meta-Control Security**: Introduced `ConstitutionalValidator` trait for Heterogeneous Dual-LLM validation. The `SoulMutator` now securely verifies `SOUL.md` mutations using a prosecutor LLM.
- **Management Console (Dashboard v2)**: Launched a Tauri React-based desktop shell (`apps/management-console`) featuring Quantum Glass UI, live Karma stream, and Synapse Resonance Graph.
- **LLM Hybrid Architecture (Pattern B)**: Front-end uses Gemini Cloud (`gemini-2.5-flash`), background tasks use Ollama Local (`qwen3.5:9b`).
- **AI Name Customization**: Users can set a custom AI name during onboarding and change it later via Settings.
- **Onboarding Wizard v2**: 4-step onboarding (Welcome → Name → Avatar → Security) with avatar selection (gender + style).
- **Background LLM Settings UI**: Added Background LLM configuration section to Settings page.
- **IME Input Fix**: Fixed Japanese IME input clearing bug in Agent Console and Settings.

### Changed
- Background worker interval increased from 60s to 300s for Ollama stability.
- System prompt now dynamically injects AI name from DB settings.
- `build_system_instructions()` prioritizes `SOUL.md` content over hardcoded identity.

## [0.1.0] - 2026-03-05

### Added
- **Full OSS Strategy**: Pivoted from Open-Core to a Full Open Source foundation under the Elastic License 2.0 (ELv2).
- **Aiome Branding**: Applied new visual identity including "Abstract Eye" logo and "Lobster Pilot" mascot.
- **Bilingual Documentation**: Established bilingual (EN/JP) versions for CLA, Code of Conduct, and Security Policy.
- **Governance Setup**: Implemented License Grant style CLA to encourage community contributions while protecting commercial rights.
- **Samsara Hub**: Central validator/quarantine node for federated learning and collective immunity.
- **Immune System**: Adaptive defense mechanism against malicious prompts and system anomalies.
- **Dream State**: Background generation of creative concepts and visual experiments.
- **Skill Arena**: Automated A/B testing framework for evaluating LLM prompts and styles.
- **Oracle**: Multi-model consensus system for scoring and validating generated media.
- **Resilience**: Jitter, Circuit Breaker, and HITL (Human-in-the-Loop) for federation sync and API calls.
- **Watchtower (Discord)**: Persona-driven interaction with rich stats (Resonance, Tech Lv) and evolution tracking.
- **Safety**: Structured JSON logging, `cargo audit` integration, and `cargo deny` license auditing.
- **Self-Healing**: Automated memory distillation, DB scavenging, and karma pruning.

### Changed
- Migrated federation endpoints to versioned API (`/api/v1/`).
- Enhanced `api-server` structured logging for observability.

---
[0.1.0]: https://github.com/motivationstudio-llc/aiome/releases/tag/v0.1.0

*Initial Release*
