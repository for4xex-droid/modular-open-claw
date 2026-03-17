# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **AgentRx Framework Integration**: Implemented a comprehensive agentic diagnostic and recovery system including `TrajectoryStore` (SQLite), `ConstraintChecker` (rule-based validation), and `AgentRxDiagnostics` (LLM-based self-review). Added full integration into `skill_handler.rs` and the main chat loop for autonomous failure recovery.
- **WASM `fs_reader` Security Fix (P0)**: Corrected a path traversal vulnerability in the `fs_reader` WASM skill by replacing string-based prefix checks with component-aware `Path::starts_with`.
- **Enhanced BastionGuard Whitelisting & Flag Validation**: Strengthened `safe_exec` to recursively validate paths within command-line flags (e.g., `--file=path`) and explicitly blacklisted access to sensitive internal files like `.env`, `.git`, and `security.json` even within the allowed workspace.
- **AgentRx Schema Migration**: Added `trajectory_steps` and `agent_diagnoses` tables to the core database to enable persistent storage of agent execution paths and recovery hints.
- **Security Whitelist Optimization**: Removed `mv` from the default `BastionGuard` whitelist to prevent uncontrolled file movement and maintain consistency with other restricted destructive commands.
- **FTS5 Synchronization Triggers**: Implemented robust error handling for Search Index (FTS5) triggers in `migrations.rs`, replacing silent ignores with structured warnings and idempotent creation checks.
- **Duplicate Safety Directive Removal**: Cleaned up code across `infrastructure` and `core` crates by removing redundant `![forbid(unsafe_code)]` attributes.

### Fixed
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
