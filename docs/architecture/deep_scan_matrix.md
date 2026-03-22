# 📡 Aiome Deep Scan AST Matrix

> Generated at: 2026-03-22T21:49:58.846812

This file contains the AST-extracted structural matrix of the codebase. Use it to cross-reference against Project NURTURE requirements without hitting LLM context limits.

## 📦 APPS (Endpoints & Services)
### `management-console`
**React Components**
- AVATAR_ASSETS, AgentConsole, AiomeAvatar, ArtifactVault, AuthOverlay, AvatarCharacterContext, AvatarCharacterProvider, BiomeDialogueView, BiotopeView, CharacterBillboard, DiagnosticsHistory, DioramaView, ExpressionPipeline, FilterButton, GraphView, ImmuneSystem, Inlets, InxRenderer, OllamaModelSelector, OnboardingModal, OriginsManager, Outlet, PAGE_SIZE, Rat, SYNAPSES, SecretUpdater, SettingInput, SettingsPage, SkillCard, SkillVault, SystemBirth, Timeline, VaultProtectionItem, VoiceStore, VrmRenderer

### `api-server`
**REST / Websocket Routes**
- `/`
- `/api/agent/feedback`
- `/api/artifacts`
- `/api/artifacts/:id`
- `/api/artifacts/:id/edges`
- `/api/artifacts/:id/files/:filename`
- `/api/avatar/ekyc-status`
- `/api/avatar/upload`
- `/api/biome/autonomous/start`
- `/api/biome/autonomous/status`
- `/api/biome/autonomous/stop`
- `/api/biome/list`
- `/api/biome/send`
- `/api/biome/status`
- `/api/biome/topics`
- `/api/expression/auto`
- `/api/expression/generate`
- `/api/expression/list`
- `/api/expression/status`
- `/api/health`
- `/api/skills`
- `/api/skills/import`
- `/api/skills/mcp/spawn`
- `/api/stream/chat`
- `/api/stream/vitality`
- `/api/synergy/graph`
- `/api/synergy/karma`
- `/api/synergy/rules`
- `/api/synergy/rules/:id`
- `/api/synergy/test/failure`
- `/api/synergy/test/federation`
- `/api/synergy/test/security`
- `/api/system/evolution`
- `/api/v1/audit/diagnostics`
- `/api/v1/audit/ledger`
- `/api/v1/audit/quarantine`
- `/api/v1/auth/authorize`
- `/api/v1/auth/token`
- `/api/v1/avatar/inochi2d/upload`
- `/api/v1/commerce/balance/:agent_id`
- `/api/v1/commerce/purchase/:agent_id`
- `/api/v1/commerce/webhook`
- `/api/v1/ekyc/session`
- `/api/v1/ekyc/status`
- `/api/v1/gift/policy/:agent_id`
- `/api/v1/gift/send/:agent_id`
- `/api/v1/gig/accept/:intent_id/:bid_id`
- `/api/v1/gig/bid`
- `/api/v1/gig/deliver`
- `/api/v1/gig/publish`
- `/api/v1/gig/verify/:order_id`
- `/api/v1/logs`
- `/api/v1/metrics`
- `/api/v1/ollama/models`
- `/api/v1/settings`
- `/api/v1/settings/identity`
- `/api/v1/settings/test`
- `/api/v1/trends`
- `/api/v1/voice/list`
- `/api/v1/voice/upload`
- `/api/wiki`
- `/api/wiki/content`
- `/health`
- `/messages`
- `/sse`
**Key Structs**
- AgentChatRequest, ApiDoc, AppError, AppState, AuditLedgerResponse, Authenticated, AuthenticatedUser, AuthorizeRequest, AutoToggle, AvatarAssetRequest, AvatarVerificationResult, CallToolResult, ChatMessage, CommerceBalanceResponse, Component, DbLoggerLayer, DemoApiDoc, DiagnosisResponse, EkycSessionResponse, GiftPolicyResponse, GiftResponse, GraphData, GraphEdge, GraphNode, IdentityResponse, ImportRequest, ImportSkillRequest, Inochi2dUploadResponse, JsonRpcError, JsonRpcRequest, JsonRpcResponse, KarmaBridge, KarmaFeedbackRequest, ListArtifactsParams, ListParams, ListToolsResult, ListVoiceAssetsQuery, LogEntry, LogEntryResponse, McpClient, McpDiscoveryFile, McpProcessManager, McpServerConfig, McpSpawnRequest, McpTool, MessageQuery, PluginRegistry, PurchaseRequest, PurchaseResponse, SendBiomeRequest, SkillSummary, SoulStatusResponse, StartAutonomousRequest, TestConnectionRequest, TestConnectionResponse, TokenRequest, TokenResponse, TrendsResponse, UpdateSettingsRequest

### `samsara-hub`
**REST / Websocket Routes**
- `/api/v1/biome/relay`
- `/api/v1/biome/topics`
- `/api/v1/biome/ws`
- `/api/v1/federation/push`
- `/api/v1/federation/sync`
- `/api/v1/federation/ws`
- `/api/v1/health`
- `/api/v1/relay/timeline/sync`
**Key Structs**
- AuthenticatedUser, BiomeWsQuery, HubState, TimelineSyncRequest

### `watchtower`

### `key-proxy`
**REST / Websocket Routes**
- `/api/v1/health`
- `/api/v1/llm/complete`
- `/api/v1/llm/embed`
- `/api/v1/llm/stream`

## 📚 LIBS (Core Domain & Infrastructure)
### `core`
**Domain Structs**
- AbyssVaultProvider, AutonomousBiomeEngine, AutonomousConfig, ClaudeProvider, DialogueManager, ExpressionEngine, GeminiProvider, JobBudget, LmStudioProvider, LoraEngine, LoraModel, MockLlmProvider, OllamaProvider, OpenAiProvider, RuriProvider, TtsWorker

### `napi-bridge`
**Domain Structs**
- SubagentSpawnResponse, ToolCheckResponse

### `shared`
**Domain Structs**
- AiomeConfig, AiomeCustomClaims, AuditEntry, BeggingSupervisor, CleanupTarget, HealthMonitor, ImageHasher, PathSandbox, ProportionsChecker, ResourceStatus, Secret, SecurityPolicy, StorageCleaner

### `avatar-engine`
**Traits (Interfaces)**
- LipSyncProvider
**Domain Structs**
- AssetManifest, AvatarParameters, EmotionToParameterMapper, Inochi2dLoader, InxModel, LipSyncFrame, PhysicsConfig, PhysicsSimulator

### `aiome-contracts`
**Traits (Interfaces)**
- AgentAct, AiomeLogger, AiomePlugin, ArtifactStore, CommerceEngine, ConstitutionalValidator, EmbeddingProvider, GenerativeEngine, GiftEngine, GigEngine, JobQueue, LlmProvider, MediaProcessor, Publisher, RuntimeJail, TrajectoryStore, TrendSource, VoiceKeyVault
**Domain Structs**
- AgentDiagnosis, AgentStats, ArenaMatch, ArtifactEdge, ArtifactEdgeInput, ArtifactFile, ArtifactMeta, ArtifactResponse, BiomeDialogue, BiomeMessage, BudgetExhaustedError, ConceptRequest, ConceptResponse, ConstraintViolation, CreateArtifactRequest, CustomStyle, DelegationResult, DialogueDistillation, EconomicContext, Expression, FederatedKarma, FederatedMetrics, FederationHandshake, FederationPushRequest, FederationPushResponse, FederationSyncRequest, FederationSyncResponse, GenerativeRequest, GiftPolicyContext, GiftRequest, GigBid, GigDeliverable, GigIntent, ImmuneRule, Job, JobMetrics, KarmaClassification, KarmaDirectives, KarmaEntry, KarmaMetrics, KarmaSearchResult, LlmJobResponse, LlmResponse, LocalizedScript, LogEntry, MediaProcessingRequest, MediaProcessingResponse, Message, MessageMeta, OracleVerdict, OutputArtifact, PermissionManifest, QuarantinedAsset, RefundedEscrow, ResourceUsageLog, SnsMetricsRecord, SpentEscrow, SynthesisRequest, SynthesisResponse, SystemSetting, SystemStatus, TrajectoryStep, TrendItem, TrendRequest, TrendResponse, UnspentEscrow, VerificationResult, WorkflowRequest, WorkflowResponse

### `soul`
**Traits (Interfaces)**
- SamsaraEngine, SoulDomainAdapter
**Domain Structs**
- AgentSoul, AnamnesisProfile, AttachmentModel, Defense, DomainModel, Experience, Instinct, InstinctRule, PredictiveModel, SomaticMarker, SoulPipeline

### `infrastructure`
**Traits (Interfaces)**
- AuthManager, ChannelBridge, CoreOps, CrdtOps, DbInitializer, EkycEngine, EkycSessionStore, EvaluationOps, EvolutionOps, ExpressionOps, FederationOps, GuardrailOps, KarmaOps, QuarantineStore, SettingsOps, SwarmOps, TrajectoryOps, TrendAdapter, WatchtowerOps
**Domain Structs**
- AbyssVoiceVault, ActionsImporter, AdaptiveImmuneSystem, AgentRateLimiter, AgentRxDiagnostics, AiomeLogClient, AssetManifest, AudioHasher, BackgroundLlmProvider, BastionGuard, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStatus, Cleanroom, ConceptManager, ConstraintChecker, ContextEngine, CoreDomainAdapter, CostBypassSwitch, CostCircuitBreaker, CostStatus, DefaultConstitutionalValidator, DefaultSamsaraEngine, DiscordBridge, DreamState, DynamicLlmProvider, EkycSession, ExternalTrendSonar, FallbackRouter, HeartbeatWakeupService, JwtAuthManager, KarmaTaxonomy, L1Metadata, L2Metadata, L3Metadata, MemoryCrystallizer, MockAuthManager, MockCommerceEngine, MockEkycEngine, MockEkycSessionStore, MockJobQueue, MockQuarantineStore, MockXPublisher, Oracle, ProjectKnowledgeIndexer, ProxyLlmProvider, PublishPipeline, RegistryManager, RevenueSplitter, RssCollector, SecurityConfig, SemanticCache, SkillArena, SkillForge, SkillImporter, SkillManifest, SkillMetadata, SkillPerformance, SloConfig, SloEngine, SoulMutator, SoulSnapshot, SqliteArtifactStore, SqliteEkycSessionStore, SqliteGigEngine, SqliteJobQueue, SqliteQuarantineStore, SqliteSoulStore, StripeCommerceEngine, StripeEkycEngine, TelegramBridge, TremendousGiftEngine, UnverifiedSkill, UserLearner, VerifiedSkill, VoiceCoreDrm, WasmSkillManager, WebSearchAdapter, WorkspaceManager

