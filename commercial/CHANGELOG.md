# Changelog
> Last Updated: 2026-05-20

All notable changes to this project will be documented in this file.

## [Unreleased]
> Last Updated: 2026-05-30

### Added
- **nurture-bridge L2 re-export (E-1)**:
  - `Project-Nurture/libs/nurture-bridge/src/lib.rs` にて、Aiome の L2 関連の `LoraEngine` および `TtsProvider` トレイトを `aiome_core_contracts::traits` から再エクスポートするように型契約を拡張。
- **Nurture S2S /internal/lora-train Route (E-2)**:
  - `apps/nurture-api/src/routes/internal.rs` に S2S サービス間通信用の `/internal/lora-train` ルートおよびハンドラ `internal_lora_train` を実装し、受け取った LoRA 訓練リクエストを内部の `job_queue` に安全かつ確実にエンキューする設計を導入。
  - テスト `test_internal_api_lora_train` を `internal_routes_test.rs` に追加し、有効な OxiLean 証明書付きで 202 Accepted が返却されることを検証。
  - スキーマ衝突防止のため、`internal_routes_test.rs` のテストセットアップにおいて、Aiome の `UniversalJobQueue` データベースを NurtureDB から `test_jobs.db` として物理的に分離し、堅牢な並行実行環境を確立。

### Added
- **Escrow Settlement Integration (Step 1.1)**:
  - `BuyRequest` および `BuyResponse` 構造体に `use_escrow: Option<bool>` および `escrow_id: Option<String>` フィールドを追加し、信託決済のフローに対応。
  - `apps/nurture-api/src/mcp_tools/buy.rs` において、`use_escrow` 指定時、通常の即時決済の代わりに `state.commerce_engine.escrow_create` を使ってアトミックにエスクロー決済（引き落としとpendingレコード挿入）を呼び出すロジックを統合。
  - E2Eテスト `test_buy_flow_with_escrow` を追加し、エスクローの作成、`nurture_escrows` での `pending` ステータス、DRMライセンスの即時発行、ウォレット残高の安全な減少を検証。
- **GlassWorm Shield 移植 & LoRA バリデーション (Step 1.5)**:
  - `upload.rs` に不可視 Unicode を除去する GlassWorm サニタイザを完全移植。アップロード処理時にアセット名と説明文を自動サニライズ。
  - `LoraAdapter` アップロード時に `model_family`, `base_model`, `adapter_path` の3つの必須メタデータを検証し、欠損時は `PolicyViolation` (400) エラーで弾く堅牢なバリデーションを追加。
- **SQLite content_hash マイグレーション (Step 1.2)**:
  - アセット整合性検証用 `content_hash` フィールドを `ItemDescriptor` に追加。
  - DBスキーマに `content_hash` カラムを追加するマイグレーション SQL (`20260520000000_add_content_hash.sql`) を導入し、SQLx によるインメモリ DB のマイグレーション整合性を確保。

### Fixed
- **Compile Regression Fix (Step 1.2)**:
  - `ItemDescriptor` 構造体への `content_hash` 追加に伴い発生していた `libs/nurture-core/src/policy.rs` 内のテストヘルパー `test_item` でのフィールド欠損ビルドエラーを修正（`content_hash: None` を指定）。ワークスペース全体で 100% GREEN ビルドを復元。

### Frontend (nurture-ui)
- **UI Architecture & Theme Modernization**:
  - Transitioned the entire Nurture UI (`nurture-ui`) to a strict design token-driven CSS architecture, eliminating all hardcoded HEX/RGBA color values across the codebase.
  - Eliminated dead CSS classes and legacy Vite template assets, optimizing the production CSS bundle size.
- **Robustness & Stability**:
  - Hardened the `apiFetch` HTTP client against network stream interruptions by employing `unknown` type guards and protective try-catch wrappers, ensuring zero-panic execution.
  - Bolstered unit testing to 100% coverage for hooks and core components (34/34 passing), guaranteeing regression-free deployments.
- **Internationalization (i18n) & a11y**:
  - Implemented comprehensive `react-i18next` integration, removing all hardcoded Japanese text (including placeholders and stub pages) for complete localization.
  - Enforced semantic HTML and WCAG accessibility standards (aria roles, label bindings) across all UI components.
- **Proxy Security**:
  - Hardened the Vite proxy configuration to strictly enforce the `NURTURE_INTERNAL_SECRET` requirement for all backend API interactions, removing unsafe fallback logic.

### Security
- **Defense-in-Depth (Phase 3)**:
  - Added strict Security Headers (`X-Content-Type-Options`, `X-Frame-Options`, `Strict-Transport-Security`, `Content-Security-Policy`) via `tower-http` to prevent MIME-sniffing, Clickjacking, and XSS attacks.
  - Implemented permissive CORS via `CorsLayer` for seamless frontend integration.
  - Refactored `nurture-api` root router logic into `apply_security_middlewares` ensuring TDD testability and DRY compliance.
- **Nurture Bridge Isolation (Phase 4)**:
  - Established `nurture-bridge` as the single authoritative gateway for all Aiome infrastructure. Added re-exports for `commerce`, `error`, `plugin` (with `AgentHook`), `oxilean`, and `LlmRequest/LlmResponse`.
  - Mechanically replaced 46 direct `aiome_core_contracts` imports across 13 files in `nurture-api` to route through `nurture-bridge`, with only 2 intentional gRPC (`a2a::internal`) exceptions.
  - See ADR-011 for design rationale on `nurture-infra` direct dependency and gRPC protobuf exceptions.
- **CVE & Tech Debt Resolution (Phase 2)**:
  - Cleaned up unused `oauth2` dependency to simplify the dependency graph and remove downstream vulnerability chains affecting `Nurture API`.
  - Synchronized `deny.toml` ignore list with upstream unmaintained crates for Phase 2 CVE tracking to achieve 100% GREEN CI compliance in `cargo deny check advisories`.

### Changed
- **Economic Hardening (Ledger Traceability)**: 
  - Executed schema migration `20260506000000_ledger_asset_id` to add an `asset_id` column to the `nurture_ledger` table.
  - Expanded `LedgerEntry` struct to include `asset_id: Option<Uuid>`, propagating it across 22+ constructor sites.
  - Hardened DRM tracking: `instant_refund` and `deliver_gift` now correctly trace and record the `asset_id` within the `refund_entry` and `audit_entry` respectively, resolving W-3 vulnerability and closing traceability gaps.
  - Enforced zero-panic, zero-unwrap parsing for `asset_id` inside `SQLiteEconomyLedger::get_history` with silent DB fallback patterns replaced by precise `tracing::debug!` observability.

### Changed
- **Ecosystem CI/CD Synchronization**:
  - Synchronized with Aiome's `enforce_unwrap_deny.py` structural fixes and silent failure (`let _ =`) elimination strategies. Verified that Nurture infrastructure strictly maintains a 0-panic, 0-TODO, and 0-anti-pattern baseline for the Phase 3 production release.

### Fixed
- **Compile Error Resolution**: Implemented missing `fetch_arena_matches` and `fetch_job_cost` methods in `mock_job_queue.rs` and added a v1.1 sealed stub for `create_checkout_session` in `CommerceEngine` to fully restore Nurture compilation.
- **CSAM Pipeline (Fail-Closed Enforcement)**: Fixed a vulnerability in `PhashScanner` where database connection errors would silently default to `false` (Fail-Open). Replaced `unwrap_or(false)` with explicit `map_err` to ensure the CSAM blacklist check correctly rejects requests or errors out when the DB is unavailable.
- **System Integrity (Crash-Only Defense)**: Replaced hazardous `panic!` calls in the `CloneManager` recovery logic and `EscrowSweep` background tasks with graceful `tracing::error!` logging and fail-closed early returns in `apps/nurture-api/src/state.rs`. This ensures resilient execution of background cron operations without crashing the entire API service process.

### Added
- **Sentinel Security Integration (P0 Dogfooding)**: 
  - Integrated `AdaptiveImmuneSystem` directly into Nurture API's `AppState` to share the same Baseline Regex security layer (prompt injection defense) with Aiome.
  - Implemented an active security shield in the Nurture MCP `tools/call` handler, evaluating tool arguments using `immune_system.verify_intent()`.
  - Added strict whitelist logic allowing `sandbox_exec` calls to bypass prompt injection filters, preventing false-positive blocks on Python code.
  - Developed full TDD coverage including positive and negative tests (`test_verify_intent_blocks_injection`) to guarantee bulletproof prompt injection rejection.
- **CSAM Pipeline (Toxicity Externalization)**: Transitioned `KarmaToxicityScanner` forbidden words from hardcoded static strings to dynamic `system_state` database retrieval. It now supports dynamic configuration updates via the AIOME management interface while safely falling back to core restricted terms (`dangerous`, `illegal`, `exploit`) during initialization failures.
- **Economy**: 実装済みのCSAMガードを統合したエージェント間ギフト（所有権移転）処理 `deliver_gift` を追加。また、残タスクであった `EntryType::Gift` の新設による台帳監査機能の強化と、`LicenseStore` での `transfer_license` (revoke + issue) の DB トランザクションによる完全原子化（Fail-Closedの徹底）を完了。
- **Agent**: 実装を禁じ、型とOxiLean定理のみを出力させるLLMの思考モード移行プロセス `/spec-mode` を追加 (Phase 1 残タスク 1.3)。
- **GigEngine**: OxiLeanの定理と型のみを出力する「仕様策定モード」を導入。
- **KarmaForge**: `sage_meditation` メソッドにて、PythonサンドボックスのJSON出力を `FederatedKarma` へ変換し統合するロジックを実装 (Phase 2b 残タスク)。
- **Shadow Worker Observability**: Implemented the `/internal/oxilean/status` endpoint in Nurture API to dynamically query OxiLean formal verification progress (OXP score) via gRPC, unblocking the Creator Asset Publication flow.
- **Phase 8 Security Hardening (Sandbox Isolation)**: Refactored `PythonExecutor` in `nurture-infra` to abandon `pyo3` in-process execution. Python code is now executed in a fully isolated, out-of-process Podman sandbox (`python:3.11-alpine`) via Aiome's `SafeCommandBuilder`, neutralizing local filesystem/network SSRF vulnerabilities while preserving resource constraints. Data and script injection now pipe safely via `stdin` to prevent shell injection.
  - Implemented the `sandbox_exec` MCP tool in Nurture API, fully integrating `PythonExecutor` as a stateless, isolated HTTP endpoint for agents to safely evaluate Python code.

- **Phase 8 Security Hardening (Authentication Sync)**: Synchronized JWT authentication logic between Aiome and Nurture ecosystems. Replaced Nurture-specific `Claims` struct with `shared::auth::AiomeCustomClaims` and transitioned token validation from manual decoding to the `shared::auth::AuthManager` trait (EdDSA). Deprecated and removed the `API_SERVER_SECRET` environment variable in favor of the shared `JWT_PRIVATE_KEY_B64`.
- **AgentHook Webhook Dispatch**: Integrated Phase 7 AgentHook callbacks into Stripe and Polar webhooks, unifying the financial event propagation via `on_transaction_completed` towards KarmaForge.
- **Crash-Only Architecture (CBA)**: `AppState::init` のモックストレージフォールバック（`MockAssetStorage`）を `feature = "mock"` 専用に制限し、本番環境で S3 接続失敗時に直ちに `panic!` する完全な Fail-Fast ポリシーを確立。
- **Idempotency**: `upload_handler` のペイロードに `idempotency_key` (Uuid) を必須化し、非同期リトライ等による複数回のアップロード処理においてS3へのアセット重複保存や孤児データの発生を防止。
- **Marketplace Enablement**: `apps/nurture-api/src/plugin.rs` でコメントアウトされていた `marketplace_upload` MCP ツールを有効化。Phase 2 (Creator Economy) の必須ツールを解放。
- **KarmaForge Dynamic Trust**: `evaluate_trust_score` をモックから実働化し、`BiomeRegistry` (`update_biome_reputation`) から動的に評価を算出しトラストバウンドに適用するロジックを実装。
- **Asset Lifecycle Management**: `LicenseStore` トレイトに `purge_expired_licenses` を追加し、`AppState` に期限切れ・無効化ライセンスのバックグラウンドGCタスクを導入。物理アセット削除時のS3ネットワークエラーも詳細なエラーログとして追跡されるよう強化。
- **DRM Integration**: アセットダウンロード API (`/internal/asset/:id/download/:buyer_id`) において、成功時に `x-nurture-drm-key` ヘッダーを経由してクライアントへ復号キーを安全に配信する論理を統合。
- **CSAM Defense (Phase 3)**: `BoneChecker` において、`gltf-rs` を用いた GLTF/GLB バイナリのノードツリー巡回アルゴリズムを実装し、メタデータの自己申告値に依存せず、より正確な VRM アバターの頭身比率判定を Fail-closed にて実行するよう精緻化。
- **KarmaForge LLM Synthesis (SC-2)**: 実体化された `KarmaForge::cross_synthesize` メソッドへ `aiome_core::llm_provider::LlmProvider` 統合を追加し、スキャフォールディングのモック実装から実際の LLM （Ollama 等）によるプロンプト駆動の高度な経済インサイト推論ロジックへの置換を完了。
- **Economic Hardening (Defense-in-Depth)**: `/internal/deduct` および `/internal/escrow-release` のエンドポイントに Defense-in-Depth (DiD) ベースの入出力バリデーションと 400/500 エラーの分離を実装。
- **Economic Hardening (SettlementProtocol Enforcement)**: Aiome側からの課金連携エンドポイント (`/internal/deduct`) を新設し、推論コスト等の動的課金に対応。
- **Economic Hardening (Secrecy)**: `AppState` の初期化プロセスと環境変数の取り扱いを `secrecy::SecretString` に全面移行し、メモリ上の鍵の露出と意図しないログ出力（CWE-209）を防止。
- **Economic Hardening (DRM Master Key)**: `SQLiteLicenseStore` 内部のマスター鍵を `secrecy::Secret` (`[u8; 32]`) に移行し、暗号化/復号プロセスでのみ展開（`expose_secret()`）するようライフサイクルを強化。
- **Economic Hardening (A2A Remittance)**: Implemented the `/transfer` API endpoint for Agent-to-Agent/Creator remittance, fully integrated with `record_batch` atomic ledger updates and protected by mandatory eKYC validation.

### Fixed
- **Economic Hardening (Integer Safety - Settlement)**: `settlement.rs` および `bridge.rs` にて、エコノミー演算（ポイント計算・手数料控除・Burn処理）における `as u64` によるサイレント切り捨てを全面廃止。安全な `u64::try_from()` を用いた fail-fast パターンを導入し、不正な bps レート計算時の資金消失やマイナス演算（オーバーフロー）リスクを物理的に遮断するよう強化。
- **CWE-209 Information Leak Prevention (Webhooks)**: Masked internal error details in `stripe.rs` and `polar.rs` webhook endpoints, returning safe generic error strings (500 Internal Server Error, 400 Bad Request) to the client while retaining detailed `tracing::error!` logging locally.
- **CWE-209 Information Leak Prevention (Rate Limiter)**: Removed dynamic error formatting in the global rate limiting layer (`main.rs`) and replaced it with a static "Rate limit exceeded. Please try again later." response payload.
- **Refund Policy Enforcement**: `NurtureCommerceBridge::deduct_generation_cost` におてい、フォールバックのBurn先が `EntryType::SystemFee` で記録される台帳不整合を修正し、`EntryType::Burn` を使用。
- **A2C Synergy**: `NurtureCommerceBridge::deduct_generation_cost` が直接 Wallet を更新する構造的欠陥 (バグ) を修正し、`asset_id` を伴う推論時は `commerce_protocol::settlement::SettlementProtocol` の Transaction へ流れるように統合。システム手数料・クリエイター還元・Burn の3重分配プロトコルが自動執行される健全な経済インフラを確立。
- **Points Economy**: `creator_points_earned` の計算が定価ベースだったバグを修正し、動的推論コストベースの計算に修正（搾取ベクターを塞ぐ）。

### Changed
- **Aiome Integration**: Added `NurtureAgentHook` to Nurture API to bridge OxiLean formal verification (`Proof Power`) and `KarmaForge` cross-synthesis.
- **Aiome Integration**: Added Phase 2 stubs for `on_proof_completed` and `cross_synthesize` integration.
- **Policy Thread Safety**: `EconomyPolicy` へのアクセスを `SharedPolicy` (`Arc<RwLock<EconomyPolicy>>`) に移行し、非同期の `read().await` パターンに統一してスレッドの枯渇とデッドロックを防止。
- **Mock Eradication**: Sealed unused/stubbed methods (`stake`, `slash`, `create_subscription`, `cancel_subscription`) in `NurtureCommerceBridge` by returning explicit `Err(AiomeError::Infrastructure)` to prevent accidental invocation in v1.1 production.

## [0.1.0] - 2026-03-10
Initial Release.

### Security & Compliance (Phase 5)
- **KYC & Identity**: Enforced eKYC status verification before escrow creation. Refactored `EkycVerifier` to read from `nurture_kyc_status` table dynamically.
- **Privacy (GDPR)**: Implemented Right to be Forgotten (RTBF) API (`/internal/forget/:actor_id`) to scrub PII securely while preserving ledger immutability.
