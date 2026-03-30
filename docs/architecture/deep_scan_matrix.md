# 📡 Aiome Deep Scan AST Matrix

> Generated at: 2026-03-31T01:17:35.466183

This file contains the AST-extracted structural matrix of the codebase. Use it to cross-reference against Project NURTURE requirements without hitting LLM context limits.

## 📦 APPS (Endpoints & Services)
### `management-console`
**React Components**
- AVATAR_ASSETS, AgentConsole, AiomeAvatar, ArtifactVault, AuthOverlay, AvatarCharacterContext, AvatarCharacterProvider, BiomeDialogueView, BiotopeView, CausalVisualizer, CharacterBillboard, DEMO_STEPS_META, DemoView, DiagnosticsHistory, DioramaView, ExpressionPipeline, FilterButton, GraphView, ImmuneSystem, Inlets, InxRenderer, OllamaModelSelector, OnboardingModal, OriginsManager, Outlet, PAGE_SIZE, Rat, SYNAPSES, SecretUpdater, SettingInput, SettingsPage, SkillCard, SkillVault, SystemBirth, Timeline, TreasureBox, VaultProtectionItem, VoiceStore, VrmRenderer

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
- `/api/v1/commerce/subscription/:agent_id`
- `/api/v1/commerce/subscription/cancel`
- `/api/v1/commerce/subscription/create`
- `/api/v1/commerce/webhook`
- `/api/v1/demo/start`
- `/api/v1/ekyc/session`
- `/api/v1/ekyc/status`
- `/api/v1/gift/policy/:agent_id`
- `/api/v1/gift/send/:agent_id`
- `/api/v1/gig/accept/:intent_id/:bid_id`
- `/api/v1/gig/bid`
- `/api/v1/gig/deliver`
- `/api/v1/gig/publish`
- `/api/v1/gig/verify/:order_id`
- `/api/v1/jobs/:id/cancel`
- `/api/v1/jobs/:id/logs`
- `/api/v1/jobs/:id/review`
- `/api/v1/logs`
- `/api/v1/metrics`
- `/api/v1/ollama/models`
- `/api/v1/settings`
- `/api/v1/settings/identity`
- `/api/v1/settings/test`
- `/api/v1/syndicate/guilds`
- `/api/v1/syndicate/guilds/:id`
- `/api/v1/syndicate/guilds/:id/members`
- `/api/v1/trajectory/:id`
- `/api/v1/trajectory/:id/diagnosis`
- `/api/v1/trends`
- `/api/v1/voice/list`
- `/api/v1/voice/upload`
- `/api/v1/watchtower/ws`
- `/api/wiki`
- `/api/wiki/content`
- `/feedback`
- `/health`
- `/messages`
- `/sse`
**Key Structs**
- AddMemberRequest, AgentChatRequest, ApiDoc, AppError, AppState, AuditLedgerResponse, Authenticated, AuthenticatedUser, AuthorizeRequest, AutoToggle, AutonomousDemo, AvatarAssetRequest, AvatarVerificationResult, CallToolResult, CancelSubscriptionRequest, ChatMessage, CommerceBalanceResponse, Component, CreateGuildRequest, CreateSubscriptionRequest, DbLoggerLayer, DemoApiDoc, DiagnosisResponse, EkycSessionResponse, GiftPolicyResponse, GiftResponse, GraphData, GraphEdge, GraphNode, IdentityResponse, ImportRequest, ImportSkillRequest, Inochi2dUploadResponse, JobReviewPayload, JsonRpcError, JsonRpcRequest, JsonRpcResponse, KarmaBridge, KarmaFeedbackRequest, ListArtifactsParams, ListParams, ListToolsResult, ListVoiceAssetsQuery, LogEntry, LogEntryResponse, McpClient, McpDiscoveryFile, McpHttpClient, McpProcessManager, McpServerConfig, McpSpawnRequest, McpTool, MessageQuery, PluginRegistry, PurchaseRequest, PurchaseResponse, SendBiomeRequest, SkillSummary, SoulStatusResponse, StartAutonomousRequest, SubscriptionResponse, TestConnectionRequest, TestConnectionResponse, TokenRequest, TokenResponse, TrendsResponse, UpdateSettingsRequest

### `aiome-node`
**REST / Websocket Routes**
- `/agent.json`

### `samsara-hub`
**REST / Websocket Routes**
- `/api/v1/biome/relay`
- `/api/v1/biome/topics`
- `/api/v1/biome/ws`
- `/api/v1/federation/push`
- `/api/v1/federation/sync`
- `/api/v1/federation/ws`
- `/api/v1/health`
- `/api/v1/registry/agents`
- `/api/v1/relay/timeline/sync`
**Key Structs**
- AgentInfo, AuthenticatedUser, BiomeWsQuery, HubState, TimelineSyncRequest

### `watchtower`

### `shadow-worker`
**Key Structs**
- ShadowWorkerService

### `timesfm-sidecar`

### `key-proxy`
**REST / Websocket Routes**
- `/api/v1/health`
- `/api/v1/llm/complete`
- `/api/v1/llm/embed`
- `/api/v1/llm/stream`

### `aiome-migrate`

## 📚 LIBS (Core Domain & Infrastructure)
### `core`
**Domain Structs**
- AbyssVaultProvider, AutonomousBiomeEngine, AutonomousConfig, ClaudeProvider, ConstitutionalValidator, DialogueManager, ExpressionEngine, GeminiProvider, InteractionsGeminiProvider, JobBudget, LiveSessionProvider, LmStudioProvider, LoraEngine, LoraModel, MockLlmProvider, OllamaProvider, OpenAiProvider, RuriProvider, TtsWorker

### `napi-bridge`
**Domain Structs**
- SubagentSpawnResponse, ToolCheckResponse

### `shared`
**Traits (Interfaces)**
- AuthManager
**Domain Structs**
- AiomeConfig, AiomeCustomClaims, AuditEntry, BeggingSupervisor, CleanupTarget, HealthMonitor, ImageHasher, JwtAuthManager, McpConfig, MockAuthManager, PathSandbox, ProportionsChecker, ResourceStatus, Secret, SecurityPolicy, StorageCleaner

### `avatar-engine`
**Traits (Interfaces)**
- LipSyncProvider
**Domain Structs**
- AssetManifest, AvatarDimensions, AvatarParameters, EmotionToParameterMapper, Inochi2dLoader, InxModel, LipSyncFrame, PcmResampler, PhysicsConfig, PhysicsSimulator, ProportionsChecker

### `aiome-commerce`
**Domain Structs**
- MockCommerceEngine, MockEkycEngine, MockEkycSessionStore, RevenueSplitter, SqliteSyndicateStore, StripeCommerceEngine, StripeEkycEngine, TremendousGiftEngine, UniversalEkycSessionStore, UniversalGigEngine

### `aiome-contracts`
**Traits (Interfaces)**
- A2aClient, AgentAct, AgentEvolver, AgentHook, AiomeLogger, AiomePlugin, ArtifactStore, AuditLogger, AuditStore, BiomeRegistry, CapabilityProvider, ChatStore, CommerceEngine, ConstitutionalValidator, EkycEngine, EkycSessionStore, EmbeddingProvider, FederationRegistry, ForecastProvider, GenerativeEngine, GiftEngine, GigEngine, ImmuneSystemOps, JobQueue, KarmaRegistry, LiveSessionManager, LlmProvider, LoraEngine, MediaProcessor, NewsService, PromptExtractor, Publisher, RuntimeJail, SoulStore, StrategicPlanner, SyndicateOps, SystemStateOps, TaskRegistry, ToolDiscoveryEngine, TrajectoryStore, TranscriptionEngine, TrendSource, TtsProvider, VaultBackend, VoiceKeyVault
**Domain Structs**
- A2aTaskProgress, A2aTaskRequest, AgentCard, AgentDiagnosis, AgentStats, AnomalyResult, ArenaMatch, ArtifactEdge, ArtifactEdgeInput, ArtifactFile, ArtifactMeta, ArtifactResponse, BiomeDialogue, BiomeMessage, BudgetExhaustedError, ConceptRequest, ConceptResponse, ConstraintViolation, ContextEntry, CreateArtifactRequest, CustomStyle, DelegationResult, DialogueDistillation, EconomicContext, EkycSession, Endpoints, Expression, FederatedMetrics, FederationHandshake, FederationPushRequest, FederationPushResponse, FederationSyncRequest, FederationSyncResponse, ForecastConfig, ForecastResult, GenerativeRequest, GiftPolicyContext, GiftRequest, GigBid, GigDeliverable, GigIntent, Guild, GuildMember, HypothesisManifest, ImmuneRule, Invariant, InvariantDagNode, Job, JobMetrics, KarmaClassification, KarmaDirectives, KarmaEntry, KarmaMetrics, KarmaSearchResult, LiveFunctionCall, LiveFunctionResponse, LiveToolCall, LiveToolResponse, LlmJobResponse, LlmMessage, LlmRequest, LlmResponse, LocalizedScript, LogEntry, MediaProcessingRequest, MediaProcessingResponse, Message, MessageMeta, MultiReviewResult, NativeModelConfig, OracleVerdict, OutputArtifact, PermissionManifest, PricingConfig, QuarantinedAsset, RefundedEscrow, ResourceUsageLog, ReviewConfig, ReviewContext, SecurityProfile, SlaConfig, SnsMetricsRecord, SpentEscrow, SynthesisRequest, SynthesisResponse, SystemSetting, SystemStatus, TrajectoryStep, TranscriptionResult, TranscriptionSegment, TreasureFeedback, TreasureItem, TrendItem, TrendRequest, TrendResponse, UnspentEscrow, UpdateJobStatusRequest, VerificationResult, WorkflowRequest, WorkflowResponse, ZtasProfile

### `soul`
**Traits (Interfaces)**
- SamsaraEngine, SoulDomainAdapter, SoulMiddleware, SoulMiddlewareNext
**Domain Structs**
- AgentSoul, AnamnesisProfile, AttachmentModel, BoundingGuard, Defense, DomainModel, Experience, Instinct, InstinctRule, PersonaBoundaries, PredictiveModel, SemanticRecaller, SemanticSummary, SomaticMarker, SoulContext, SoulPipeline

### `infrastructure`
**Traits (Interfaces)**
- ChannelBridge, CoreOps, CrdtOps, DbInitializer, EvaluationOps, EvolutionOps, ExpressionOps, FederationOps, GuardrailOps, KarmaOps, QuarantineStore, SecurityOps, SettingsOps, SlmBackend, SoulStoreOps, SwarmOps, TaskConductor, TrajectoryOps, TrendAdapter, VectorOps, WatchtowerOps
**Domain Structs**
- A2aGrpcClient, AbyssVoiceVault, ActionsImporter, AdaptiveImmuneSystem, AffiliateAdapter, AgentRateLimiter, AgentRxDiagnostics, AiomeLogClient, ApiSurface, AssetManifest, AsyncAuditLogger, AudioHasher, AuditEntry, BackgroundLlmProvider, BastionGuard, BehaviorMonitor, BeliefConsistencyGate, BeliefGateConfig, BoundaryVerifier, CapabilityRegistry, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStatus, Cleanroom, CliSlmBackend, CognitiveSentinel, CognitiveThresholds, ConceptManager, ConstraintChecker, ContextBudget, ContextEngine, CoreDomainAdapter, CostBypassSwitch, CostCircuitBreaker, CostStatus, DefaultConstitutionalValidator, DefaultSamsaraEngine, DefaultStrategicPlanner, DefaultToolDiscoveryEngine, DiscordBridge, DockerConductor, DreamState, DynamicLlmProvider, EnumInfo, Evidence, ExternalTrendSonar, FallbackRouter, FieldInfo, FunctionInfo, GlobalMockJobQueue, GlobalMockLlm, GraphEdge, GraphNode, GrpcClientConfig, HeartbeatWakeupService, HierarchicalRouter, HookManager, HumanizerFilter, HumanizerRule, IntentFirewall, IntentGenerator, InvariantDag, KarmaTaxonomy, L1Metadata, L2Metadata, L3Metadata, MemoryCrystallizer, MlockedVec, MockA2aClient, MockForecastProvider, MockQuarantineStore, MockSoulStore, MockTtsProvider, MockXPublisher, ModelManager, NativeEmbeddingProvider, NativeModelInner, NativeSlmBackend, OpenAiTtsProvider, Oracle, OssAdapterCodeGen, OssAstAnalyzer, OssIntegrationOrchestrator, OssKnowledgeSession, OssRepositoryIndexer, OssTypeMatcher, PlateauReport, PolarQuantEncoder, PostgresInitializer, ProjectKnowledgeIndexer, ProxyLlmProvider, PublishPipeline, RegistryManager, RouteResult, RssCollector, ScoreTracker, SecurityConfig, SemanticCache, SkillArena, SkillForge, SkillImporter, SkillManifest, SkillMetadata, SkillPerformance, SlmBridge, SlmMemoryEntry, SlmRecallData, SlmRecallJsonResponse, SlmRecallResult, SlmTraceChannelScores, SlmTraceData, SlmTraceJsonResponse, SlmTraceResult, SloConfig, SloEngine, SoulMutator, SoulSnapshot, SoulVersion, SqliteTrajectoryStore, StandardVectorOps, StructInfo, TaskDispatcher, TelegramBridge, Test, TimesFmProvider, TraitInfo, TrajectoryGraph, TreeNode, TypeMismatch, UniversalArtifactStore, UniversalJobQueue, UniversalQuarantineStore, UniversalSoulStore, UniversalVaultBackend, UnverifiedSkill, UserLearner, UserProfile, VerifiedSkill, VoiceCoreDrm, WasmSkillManager, WebSearchAdapter, WhisperMiddleware, WhisperTranscriptionAdapter, WorkspaceManager, XttsProvider

