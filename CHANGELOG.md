## [Unreleased] - 2026-03-24

### Added
- **Phase 35: PostgreSQL 移行 & 最終検証 [完了]**
    - **Dual DB Testing Infrastructure**: Ensured all 86 integration tests and CI scripts run equivalently on both SQLite and PostgreSQL backends via `TEST_POSTGRES_URL` configuration (`docker-compose.test.yml`).
    - **PostgreSQL Audit Trigger (Phase 35)**: Replaced application-layer ledger tracking with robust PL/pgSQL database triggers for automated `audit_ledger_global` lineage and hashing.
- **Phase 32: DeerFlow Architectural Pattern Integration [完了]**
    - **Middleware Chain**: `SoulPipeline` を Reactive, Deliberative, Meta-cognitive の 3 層ミドルウェア構造に刷新。`async-trait` による拡張性とスレッド安全性を両立。
    - **Progressive Skill Loading**: `WasmSkillManager` に `mtime` ベースのキャッシュ無効化ロジックを導入。WASM ファイルの更新を自動検知し、実行時に最新化。
    - **Virtual Path System**: `PathSandbox` に論理パスマッピング機能を統合。`/mnt/workspace` などの仮想パスを物理ディレクトリに安全にバインド。
    - **Fact Extraction**: `MemoryCrystallizer` に `FactCategory` (Preference, Knowledge, Context, Behavior, Goal) による事実抽出・分類機能を実装。
    - **Test Utility**: `VerifiedSkill::new_for_test` を追加し、統合テストにおける WASM スキルのモック・検証を容易化。

### Changed
- **Code Quality & Refactoring**:
    - `WritingContext` に `#[derive(Default)]` と `#[default]` 属性を追加し、ボイラープレートを削減。
    - `MlockedVec` の `Drop` 実装における安全な `munlock` 呼び出しの条件判定を最適化。
    - `SqliteVaultBackend` のマスターキー取得ロジックにおける関数ポインタの直接渡しによる簡略化。
    - `UniversalJobQueue` 内の SQLite 数値キャスト（`i32`）の整合性を修正。

- **Phase 31: 信頼性向上 & LLM TDD 実装 [完了]**

## [Unreleased] - 2026-03-23

### Added
- **Phase 28: 基盤強化 (ADR-019 Phase B / L1 強化) [完了]**
    - `SqliteVaultBackend` への LRU キャッシュ (1000 keys) 統合。`MlockedVec` によるメモリ保護を維持。
    - `lru` クレイトの導入（ワークスペース依存関係）。
- `VaultBackend` trait (ADR-019 Phase A)
- `SqliteVaultBackend` based on `MlockedVec`

### Changed
- **L1 強化 (Code Quality)**:
    - `api-server` の `commerce_engine` 初期化における `.unwrap()` を `.expect()` に置換し、デバッグ性を向上。
    - `TcpListener::bind` 失敗時のエラーメッセージを詳細化。
    - `cfg!(debug_assertions)` を `#[cfg(debug_assertions)]` に統一し、コンパイル時判定を最適化 (guardrails, gift_engine)。
    - `infrastructure` クレイト内のドキュメント警告 (FederationOps, MockJobQueue等) をすべて解消し、警告ゼロを達成。
- **Phase 28.5: `std::env::set_var` 脱却 [完了]**:
    - `SqliteVaultBackend::new_with_master_key()` コンストラクタを追加。テスト環境で環境変数操作なしに Master Key を注入可能に。
    - `AbyssVoiceVault` テストから `std::env::set_var` を完全排除。スレッドセーフかつ並列テスト安全な設計に移行。
- Refactored `AbyssVoiceVault` to use `SqliteVaultBackend` internally
- Updated `SECURITY_DESIGN.md` §6.5 with vault abstraction specs.

---

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased] - 2026-03-22

### Added
- **Phase 26: AI Writing Enhancement**:
    - Added `HumanizerFilter` middleware to detect and remove common AI-isms and robotic phrasing (e.g., excessive hedging, chatbot artifacts).
    - Added `WritingContext` to dynamically apply different writing rules based on the output destination (Chat, Manifesto, TechLog, etc.).
- **Unified Response Purger (G-21)**:
    - Implemented `purge_entities` in `aiome-core` for robust, multi-step sanitization of external inputs.
    - Centralized regex patterns and HTML/entity decoding logic to prevent XSS and script injection.
    - Applied to `TrendSonar` and RSS collectors for unified safety.
- **AI-Driven Code Review (G-22)**:
    - Integrated LLM-based security auditing into `Cleanroom` for skill imports.
    - Performs pre-forge analysis of skill source code to detect "Vampire Attacks" or malicious network calls.
- **Periodic Federated Metrics (G-23)**:
    - Added a background task to the `api-server` to push node metrics to the Samsara Hub every hour.
    - Implemented `do_push_federated_metrics` in `FederationOps` and fixed its `SqliteJobQueue` implementation.
- **AgentSense MVP (AS-1)**:
    - Implemented `GET /api/v1/treasure` to generate and fetch personalized AI "Sense" (recommendations).
    - Implemented `POST /api/v1/treasure/feedback` to record agent interactions and reward resonance.
    - Added `AffiliateAdapter` for modular bid/recommendation fetching (currently mocked).
    - **Soul-Linked Intent Generation**: Enhanced `IntentGenerator` to derive agent "wishes" from their `AgentSoul` attachment style (Anxious/Avoidant/Secure), stored in the `SoulStore` (Gap G-26).
    - **TreasureBox UI**: Developed a premium glassmorphism React component and `useTreasure` hook in the Management Console to display and interact with recommendations (Gap G-25).
    - Added comprehensive integration test `test_treasure_get_recommendations` covering the full feedback-reward loop.
- **Audit API: Quarantine Ledger**:
    - Implemented `GET /api/v1/audit/quarantine` to allow the system agent to list and audit quarantined assets.
    - Extended `QuarantineStore` trait and `SqliteQuarantineStore` with `list_assets` capability.

### Fixed
- **SSE Connection Stability**: Implemented exponential backoff and maximum retry limits (5) in frontend `useSystemVitality` hook to prevent browser connection slot exhaustion and infinite re-auth loops.
- **Audit Logging**: Fixed `NEW.id` reference error in SQLite triggers for `system_state` and other tables with non-standard primary keys. Added `DROP TRIGGER IF EXISTS` to migration logic for reliable updates.
- **Path Sandbox**: Fixed `validate_path` in `PathSandbox` by using canonical paths for comparison, preventing incorrect "Out of sandbox" errors on valid relative paths.
- **Autonomous Demo**: Added automatic database cleanup of gig-related tables before starting a new demo cycle to ensure state consistency.
- **Autonomous Demo — SQLite Lock (ADR-014)**: Resolved `database is locked (SQLITE_BUSY 517)` errors that halted the demo at Steps 5–7. Root cause: `gig_engine` transactions held exclusive SQLite WRITE locks while audit triggers cascaded additional writes, compounded by SSE multi-tab connection pool exhaustion (`max_connections=10`). Fix: rewrote `autonomous_demo.rs` to use individual SQL queries (no transactions), temporarily disable audit triggers during demo execution, and yield connections between writes. See `docs/decisions/014-sqlite-pool-exhaustion-demo-strategy.md`.
- **Authentication**: Updated `MockAuthManager` to allow `mock_token` for seamless local testing and E2E verification.
- **OAuth 2.1 Mock Endpoints**:
    - Added stub handlers for `/api/v1/auth/authorize` and `/api/v1/auth/token` in `api-server`.
    - Integrated with `utoipa` for OpenAPI documentation of the authentication flow.
- **Autonomous AI Economy Demo (Phase 25)**:
    - Implemented `AutonomousDemo` orchestrator to simulate a 60-second autonomous agent lifecycle.
    - Added `POST /api/v1/demo/start` endpoint to trigger the autonomous cycle in the background.
    - Integrated `IntentGenerator`, `GigEngine`, `SwarmOps`, and `Karma` for a complete "Earn & Evolve" demonstration.
    - Implemented real-time event broadcasting via `PluginEvent` for frontend visualization.
    - Added TDD-based integration tests to verify the demo API and lifecycle.
- **Autonomous Demo UI (Phase 25.5)**:
    - Added `DemoView.tsx` to the Management Console for real-time visualization of the autonomous cycle.
    - Implemented an animated execution timeline tracking the 8-step process.
    - Integrated with `useSystemVitality` SSE hook to display live agent stats and karma progression.
    - Added TDD-based E2E Playwright tests to verify rendering and UI interactions.
- **Enhanced Mock Authentication**:
    - Updated `MockAuthManager` to support custom `agent_id` via mock tokens (`mock_valid_token_<sub>:<agent_id>`).
    - Standardized separator to `:` to prevent collisions with usernames containing underscores.

### Fixed
- **BastionGuard gVisor (runsc) Integration**:
    - Implemented dynamic detection of `runsc` bin in Linux environments.
    - Prioritizes gVisor for secure user-space kernel execution while maintaining a gracefull host fallback for development.
- **Integration Test Stability**:
    - Resolved `test_gig_lifecycle` failure by ensuring compatible `artifact_path` for `PathSandbox` validation.
    - Restored functionality to 32/33 integration tests after auth manager refactoring.
- **Cleanroom Security Auditor**:
    - Resolved `LlmProvider::complete` argument mismatch and response parsing logic to ensure robust AI-driven code auditing.

### Fixed
- **Trend Sonar Refactoring (Multi-Source Support)**:
    - Introduced `TrendAdapter` trait to decouple trend fetching from the core `ExternalTrendSonar` logic.
    - Implemented `WebSearchAdapter` for real-time trend gathering via external Search APIs.
    - Integrated `RssCollector` as a `TrendAdapter`, enabling it to serve as both a `TrendSource` and a flexible adapter.
    - Updated `ExternalTrendSonar` to manage a collection of adapters, aggregating results from multiple sources (Web Search, RSS, etc.).
    - Added `sanitize_snippet` utility to clean raw HTML/URL data from external search results.
    - Restructured `main.rs` to pre-initialize a shared `TrendSonar` instance with multiple adapters, improving performance and consistency across the Background Worker and Dream State.
    - Added comprehensive unit tests for multi-source aggregation and adapter logic.
- **RSS Sanitization (G-Security)**:
    - Implemented `sanitize_snippet` to clean RSS feed content and prevent HTML/script injection.
    - Applied sanitization to all incoming RSS titles in `RssCollector`.
- **Gig Engine Security (G-22)**:
    - Integrated `PathSandbox` into `SqliteGigEngine::deliver` to enforce strict path validation.
    - Prevents path traversal attacks by ensuring delivery artifacts remain within the designated `ARTIFACT_ROOT`.
- **API Security Hardening (G-Log)**:
    - Restricted access to sensitive endpoints (`/api/v1/logs`, `/api/v1/audit/ledger`, `/api/v1/audit/diagnostics`) to the system agent only.
    - Verified security with automated access control checks in the routing layer.

### Added
- **Phase 20: AI Gig Engine (The Immutable Gateway)**:
    - **SqliteGigEngine Implementation**: Developed a robust, TDD-driven `GigEngine` implementation using SQLite.
    - **`publish_intent`**: Enabled AI agents to broadcast work requests with automated `AcceptanceCriteria` and JSON serialization.
    - **`submit_bid`**: Implemented bidding logic for AI agents to compete for intents, including price and duration estimations.
    - **`accept_bid`**: Implemented atomic transaction-based bid acceptance. Automatically creates and locks escrows in the `CommerceEngine` to secure payments.
    - **`deliver`**: Enabled secure delivery of artifacts with metadata and artifact path recording. Enforces state transitions from 'Accepted' to 'Delivered'. Added `PathSandbox` validation to prevent path traversal attacks (G-22).
    - **`verify_and_settle`**: Implemented the core settlement logic. Performs automated verification against `AcceptanceCriteria`, updates order status to 'Completed' or 'Rejected', and executes escrow release or refund accordingly.
    - **Persistence & Logging**: Added full database support with tables for `gig_intents`, `gig_bids`, `escrows`, `gig_deliveries`, and `verification_logs`.
    - **TDD Test Suite**: Added 5 comprehensive integration tests covering the entire gig lifecycle, ensuring 100% path coverage for core engine operations.
- **Phase 20 Enhancement: Federated Metrics (G-23)**:
    - Added `metrics` field to `FederationPushRequest` to support transmission of node-level statistics to Samsara Hub.
    - Implemented `fetch_federated_metrics` in `SqliteJobQueue` to aggregate level, XP, job completion rates, and karma counts.
    - Updated Samsara Hub to persist received metrics in the `federated_metrics` table for global analytics.
- **Trend Oracle (LLM Evaluation)**:
    - Integrated an optional `LlmProvider` into `ExternalTrendSonar` for intelligent trend assessment.
    - Implemented LLM-based scoring and filtering to prioritize high-value trends for autonomous agents.

- **Phase 17 Enhancement: Gaps G-1 & G-2 Remediation**:
    - **Gap G-1: Circuit Breaker Observability**: Added `llm_circuit_breaker` status to the `/api/health` and `/health` endpoints. Modified the `CircuitBreaker` struct to include a `get_status` method, allowing proactive monitoring of LLM failover states.
    - **Gap G-2: Per-Agent Rate Limiting**: Implemented a per-agent rate limiter using the `governor` crate. Integrated this into the authentication middleware to protect core API endpoints from individual agent abuse, with a default limit of 60 requests per minute.
- **Phase 17: ArrowCanaria Fallback & Resilience**:
    - **FallbackRouter Implementation**: Implemented a robust `FallbackRouter` in `libs/infrastructure` that wraps a primary LLM and automatically switches to a fallback (e.g., Gemini) if the primary fails.
    - **Circuit Breaker Integration**: Integrated the Circuit Breaker pattern into the LLM routing logic, enabling automatic failover and preventing cascading failures when primary providers go offline.
    - **AppState Failover**: Updated the `api-server` `AppState` to use the `FallbackRouter` for all core LLM operations, providing a seamless transition between local and cloud models.
    - **Failover Integration Tests**: Added comprehensive integration tests in `api_integration_tests.rs` to verify that the system correctly routes requests to the fallback provider during simulated primary failures.
- **Phase 16: EKYC Protection & Revenue Splitter**:
    - **EKYC Enforcement**: Added hard integration of eKYC verification to the `send_gift` and `execute_purchase` endpoints. Unverified users will be blocked with a `403 Forbidden` response to enforce strict economic compliance.
    - **Commerce Revenue Splitter**: Implemented the `RevenueSplitter` module triggered by Stripe `checkout.session.completed` webhooks. It automatically calculates an 80/20 split between creators and the platform, inserting the split logic securely within the license grant database transaction.
    - **Zeroize Security Hardening**: Addressed in-memory secret persistence vulnerabilities by immediately zeroizing and removing `STRIPE_API_KEY`, `JWT_PRIVATE_KEY_B64`, and `SEARCH_API_KEY` from the environment immediately after load during application startup.
    - **Audit Ledger Visibility**: Ensured `revenue_splits` has been added to the automated database `audit_ledger_global` triggers to maintain a comprehensive changelog.
- **Phase 14: eKYC Persistence & Inochi2D Physics Sync**:
    - **EKYC Session Store**: Implemented `EkycSessionStore` using SQLite to persist Stripe verification session IDs, ensuring continuity across application restarts.
    - **Stripe API Hardening**: Updated `EkycEngine` to use `client_reference_id` for accurate filtering and implemented technical timeouts (30s) for better resilience.
    - **Inochi2D Physics Sync**: Added `physics_override` to the `avatar_expression` SSE stream. Implemented a 1.5x "Resonance Boost" logic that amplifies physics animations when resonance level exceeds 80.
    - **Secure Mascot Upload**: Enforced the `jwt_auth_middleware` and `PathSandbox` jail on the Inochi2D upload route, preventing unauthorized access and path traversal attacks (Expert Review v3).
    - **Stripe Fail-safe**: Introduced a mandatory check for `STRIPE_API_KEY` in release builds, causing the server to exit if missing, preventing insecure mock-state deployments.
- **Phase 13a: Stripe EKYC Session API Implementation**:
    - **StripeEkycEngine**: Upgraded `StripeEkycEngine` from mock behavior to real `reqwest`-based implementation, calling the Stripe Identity Verification Sessions API directly.
    - **Session API**: Implemented `POST /api/v1/ekyc/session` to initiate verification sessions and updated `GET /api/v1/ekyc/status` to handle real status tracking for user-agent verification.
    - **Refactor**: Modified `main.rs` to inject the shared `reqwest::Client` into `StripeEkycEngine`, ensuring consistent connection pool management.
- **Phase 13b: Inochi2D (2D Avatar) Pipeline Integration**:
    - **Inochi2dLoader**: Developed `Inochi2dLoader` in `libs/avatar-engine` providing magic byte validation (`INX\x02`) and metadata extraction for versioned Inochi2D models.
    - **Physics Simulator**: Implemented `PhysicsSimulator` using spring-damping algorithms to calculate real-time secondary animations for 2D avatars.
    - **Mascot Upload pipeline**: Exposed `POST /api/v1/avatar/inochi2d/upload` with a dedicated 50MB allowance in `router.rs`, enabling registration of `.inx` assets.
- **Phase 10.2 Security Hardening (Expert Review Integrated)**:
        - **Persistent Key Vault**: Implemented `vault_keys` table for persistent storage of voice asset keys, protected by a 256-bit Master Key (`VAULT_MASTER_KEY`).
        - **AES-256-GCM Nonce Management**: Enforced random 12-byte nonce generation for every encryption operation in `crypto.rs` to prevent ciphertext reuse attacks.
        - **Authorization Bypass Fix**: Replaced the vulnerable `LIKE` operator with `json_extract` in `RegistryManager::check_ownership` for precise atomic matching of `agent_id` and `asset_id`.
        - **Memory Protection**: Reduced the voice upload body limit from 500MB to 100MB to mitigate OOM-based DoS/OOMKill attack chains.
        - **Mutex Poison Recovery**: Added `unwrap_or_else` recovery for `AbyssVoiceVault` mutexes to prevent permanent vault locks on thread panic.
        - **Audit & Compliance**: Added audit logging for all decryption key access attempts and removed redundant sync-I/O `exists()` checks in async upload handlers.
- **Phase 11.0 (Voice DRM Refinement)**:
    - **Dual-Read Ownership & Migration**: Implemented `licenses` table priority reading with legacy `stripe_webhook_events` fallback in `RegistryManager::check_ownership`. Developed a standalone seamless data migration script (`migrate_licenses.rs`).
    - **Audio CSAM Detection**: Developed multi-threaded, robust CSAM detection for audio uploads via `AudioHasher` employing `tokio::task::spawn_blocking` and timeout defenses to prevent CPU exhaustion.
    - **LipSync Responsibility Separation**: Extracted `get_lipsync_frames` from `VoiceKeyVault` into a standalone `LipSyncProvider` trait moved to `avatar-engine`, drastically improving Interface Segregation.
    - **Vault Key Caching**: Refactored `AbyssVoiceVault` internal logic to feature lazy-initialized caching for `VAULT_MASTER_KEY` via `OnceCell`, shielding E2E test suites from environment variable absence panics.
    - **Voice E2E Roundtrip**: Introduced end-to-end integration tests encapsulating the complete lifecycle of a voice upload, spanning CSAM triage, registry ownership logic, AES-GCM encrypted persistence, and authorized Vault decryption.
- **Phase 10.1a (XTTS Core Integration)**:
    - **XTTS Synthesis**: Implemented `ExpressionEngine::synthesize_audio_xtts` in `aiome-core`, enabling integration with local XTTS v2 servers for high-quality, personalized voice synthesis.
    - **API Provider Selection**: Updated the `/api/expression/generate` endpoint to support switching between OpenAI (tts-1) and localized XTTS providers via system settings.
    - **Legal Guardrails**: Established `voice_upload_terms.md` and a TTS provider comparison matrix to manage copyright and licensing risks associated with creator-first audio assets.
- **Phase 10.1b (LoRA Metadata & Soul Persistence)**:
    - **LoraEngine**: Introduced `LoraEngine` in `libs/core` to manage LoRA model metadata (hashes, base models, file paths) for consistent model identification.
    - **Soul Identification**: Extended `AgentSoul` with a `lora_hash` field, ensuring that the AI's internal identity and hash-chains are tied to the specific fine-tuned model version.
    - **Persistence Layer**: Updated `SqliteSoulStore` and database migrations to support the `lora_hash` column, enabling seamless recovery of model settings across AI rebirth cycles.
- **Phase 8.8 (Audit & Immunity Ledger UI)**:
    - **Audit UI**: Implemented `DiagnosticsHistory` component in the management console, providing a transparent view of agent self-repair logs and system-wide hash-chained change logs.
    - **User Experience**: Integrated the Audit tab into the main navigation with lazy loading and optimized "Load More" pagination to prevent UI bloat from large log sets.
    - **AST Visibility**: Fixed a regex bug in the `nurture_auditor.py` script that was skipping 10+ React components (e.g., `ArtifactVault`), restoring full visibility to the AST structure matrix.
- **Phase 8.7 (Post-Scan Remediation & Synergy)**:
    - **Type Synchronization**: Added `DiagnosisResponse`, `TrendsResponse`, and `AuditLedgerResponse` to the OpenAPI schema in `api.rs`, enabling full TypeScript type safety across the stack.
    - **Artifact Lineage Visualizer**: Expanded `GraphView.tsx` to integrate Artifact data (`/api/artifacts`) alongside Karma nodes, visualizing the creative lineage of the AI through purple diamond nodes and "materialized" edges.
    - **Load Test Readiness**: Added a dedicated load testing task to the Phase 9.0 roadmap to verify the impact of the new 2MB request body limit on high-traffic routes.
- **Phase 8.6 (Deep Scan Remediation & Security Hardening)**:
    - **DDoS Protection**: Implemented a global request body limit of 2MB in `api-server` using `RequestBodyLimitLayer` to mitigate resource exhaustion attacks.
    - **Route Specific Bypass**: Added a 50MB body limit bypass specifically for the `/upload` (avatar) route to maintain essential high-payload functionality while securing the rest of the API.
    - **API Registry Expansion**: Formally exposed the Diagnostics API (`/api/v1/audit/diagnostics`) and implemented a Trends API skeleton (`/api/v1/trends`) to fulfill Project NURTURE requirements.
    - **AST Scripting**: Created `scripts/nurture_auditor.py` for automated AST-based structural analysis, enabling deep codebase audits without hitting LLM context limits.
- **Phase 8.2 (OAuth 2.1 & JWT Authentication)**:
    - **JWT AuthManager**: Implemented `JwtAuthManager` and `AuthManager` trait to replace legacy shared secrets with stateless, secure Ed25519 JWT tokens.
    - **Hybrid Middleware**: Added `auth_middleware` in `samsara-hub` and `key-proxy` supporting both Bearer JWT and legacy secrets for backward compatibility.
    - **PII Protection**: Implemented SHA-256 hashing for user identifiers (`sub`) in JWT validation logs to prevent PII exposure.
    - **Production Security**: Hardened `key-proxy` to require `JWT_PRIVATE_KEY_B64` in production, eliminating the risk of accidental MockAuthManager usage.
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
- **Integration Test Stability**:
    - **test_gig_lifecycle**: Resolved a series of issues in the Gig Engine integration test, including JSON response parsing (mapping UUID from response object), incorrect status code assertions, and missing database tables.
    - **Database Migrations**: Added Gig Engine tables (`gig_intents`, `gig_bids`, etc.) to the core migrations to ensure schema consistency.
    - **Mock LLM Enhancements**: Updated `DummyLlm` to return valid JSON for `OracleJudge` requests, enabling verification tests to pass.
    - **Prometheus Conflict Resolution**: Consolidated Prometheus recorder initialization using a single global `Lazy` cell, fixing the `test_fallback_router_failover` panic.
    - **Ownership & Type Safety**: Fixed several Rust compilation errors (`E0382`, `E0308`) related to `AppState` component cloning and type mismatches in test utilities.

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

## [0.1.0] - 2026-03-20

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
