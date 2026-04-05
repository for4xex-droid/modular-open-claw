# 📡 Aiome Deep Scan AST Matrix

> Generated at: 2026-04-06T03:35:56.701861

This file contains the AST-extracted structural matrix of the codebase. Use it to cross-reference against Project NURTURE requirements without hitting LLM context limits.

## 📦 APPS (Endpoints & Services)
### `management-console`
**React Components**
- AVATAR_ASSETS, AgentConsole, AiomeAvatar, ArtifactVault, AuthOverlay, AvatarCharacterContext, AvatarCharacterProvider, AvatarViewerModal, BiomeDialogueView, BiotopeView, CausalVisualizer, CharacterBillboard, CharacterPanel, DEMO_STEPS_META, DemoView, DiagnosticsHistory, DioramaView, ExpressionPipeline, FilterButton, FlowCard, GlbRenderer, GraphView, HomePage, ImmuneSystem, Inlets, InxRenderer, LanguageContext, LoraTrainingView, McpConfigManager, MiniTabBar, ModelSetupStep, OllamaModelSelector, OnboardingModal, OriginsManager, Outlet, PAGE_SIZE, Rat, SYNAPSES, SecretUpdater, SettingInput, SettingsPage, SkillCard, SkillVault, StoryFlow, SystemBirth, Timeline, TreasureBox, VaultProtectionItem, VoiceStore, VrmRenderer

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
- `/api/skills/mcp/config`
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
- `/api/v1/audit/quarantine/:id/release`
- `/api/v1/auth/authorize`
- `/api/v1/auth/token`
- `/api/v1/avatar/inochi2d/upload`
- `/api/v1/bootstrap/detect-ollama`
- `/api/v1/bootstrap/factory-reset`
- `/api/v1/bootstrap/status`
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
- `/api/v1/lora/market`
- `/api/v1/lora/market/:listing_id`
- `/api/v1/lora/market/complete/:purchase_id`
- `/api/v1/lora/market/my-listings`
- `/api/v1/lora/market/publish`
- `/api/v1/lora/market/purchase`
- `/api/v1/lora/status/:job_id`
- `/api/v1/lora/train`
- `/api/v1/metrics`
- `/api/v1/models/pull`
- `/api/v1/models/status`
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
- `/api/v1/voice/synthesize`
- `/api/v1/voice/upload`
- `/api/v1/watchtower/ws`
- `/api/wiki`
- `/api/wiki/content`
- `/documents`
- `/documents/:id`
- `/feedback`
- `/health`
- `/ingest`
- `/ingest/text`
- `/messages`
- `/query`
- `/sse`
- `/suggestions`
- `/synth`
- `/wiki`
- `/wiki/:id`
**Key Structs**
- AddMemberRequest, AgentChatRequest, AgentEngine, ApiDoc, AppError, AppState, AuditLedgerResponse, Authenticated, AuthenticatedUser, AuthorizeRequest, AutoToggle, AutonomousDemo, AvatarAssetRequest, AvatarVerificationResult, BootContext, BootstrapStatusResponse, CallToolResult, CancelSubscriptionRequest, ChatMessage, CommerceBalanceResponse, Component, CreateGuildRequest, CreateSubscriptionRequest, DbLoggerLayer, DefaultToolCallRouter, DemoApiDoc, DiagnosisResponse, EkycSessionResponse, FactoryResetResponse, GiftPolicyResponse, GiftResponse, GraphData, GraphEdge, GraphNode, IdentityResponse, ImportRequest, ImportSkillRequest, IngestResp, IngestTextReq, IngestUrlReq, Inochi2dUploadResponse, JobReviewPayload, JsonRpcError, JsonRpcRequest, JsonRpcResponse, KarmaBridge, KarmaFeedbackRequest, ListArtifactsParams, ListParams, ListToolsResult, ListVoiceAssetsQuery, ListingQueryParams, LogEntry, LogEntryResponse, LoraJobStatusResponse, LoraTrainRequest, LoraTrainResponse, McpClient, McpDiscoveryFile, McpHttpClient, McpProcessManager, McpServerConfig, McpSpawnRequest, McpTool, MessageQuery, ModelStatusResponse, OllamaDetectionResponse, PluginRegistry, PublishListingRequest, PullModelRequest, PurchaseRequest, PurchaseResponse, QueryReq, SendBiomeRequest, SkillSummary, SoulStatusResponse, StartAutonomousRequest, SubscriptionResponse, SynthReq, SynthesizeQuery, SynthesizeRequest, TestConnectionRequest, TestConnectionResponse, TokenRequest, TokenResponse, TrendsResponse, UpdateSettingsRequest, WikiArticleSummary

### `aiome-node`
**REST / Websocket Routes**
- `/agent.json`
**Key Structs**
- JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpServer

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
- AiomeConfig, AiomeCustomClaims, AppDataResolver, AuditEntry, BeggingSupervisor, BootstrapDetector, BootstrapDiagnosis, CleanupTarget, FactoryReset, FactoryResetReport, HealthMonitor, ImageHasher, JwtAuthManager, McpConfig, MockAuthManager, PathSandbox, ProportionsChecker, ResourceStatus, Secret, SecurityPolicy, StorageCleaner

### `avatar-engine`
**Traits (Interfaces)**
- LipSyncProvider
**Domain Structs**
- AssetManifest, AvatarDimensions, AvatarParameters, EmotionToParameterMapper, Inochi2dLoader, InxModel, LipSyncFrame, PcmResampler, PhysicsConfig, PhysicsSimulator, ProportionsChecker, SimpleLipSyncEngine

### `aiome-commerce`
**Traits (Interfaces)**
- X402Negotiator
**Domain Structs**
- CommerceEngineFactory, MockCommerceEngine, MockEkycEngine, MockEkycSessionStore, RevenueSplitter, SqliteSyndicateStore, StripeCommerceEngine, StripeEkycEngine, TremendousGiftEngine, UniversalEkycSessionStore, UniversalGigEngine, X402Client

### `aiome-core-contracts`
**Traits (Interfaces)**
- A2aClient, AgentAct, AgentEvolver, AiomeLogger, ArtifactStore, AuditLogger, AuditStore, BiomeRegistry, CapabilityProvider, ChatStore, ConstitutionalValidator, EkycEngine, EkycSessionStore, FederationRegistry, ForecastProvider, GenerativeEngine, GigEngine, HarnessRegistryOps, ImmuneSystemOps, JobQueue, KarmaRegistry, LiveSessionManager, LoraEngine, LoraMarketplace, MediaProcessor, NewsService, PromptExtractor, Publisher, SoulStore, StrategicPlanner, SyndicateOps, SystemStateOps, TaskRegistry, ToolDiscoveryEngine, TrajectoryStore, TranscriptionEngine, TrendSource, TtsProvider, VaultBackend, VoiceKeyVault
**Domain Structs**
- A2aTaskProgress, A2aTaskRequest, AgentCard, AgentDiagnosis, AgentStats, AnomalyResult, ArenaMatch, ArtifactEdge, ArtifactEdgeInput, ArtifactFile, ArtifactMeta, ArtifactResponse, BiomeDialogue, BiomeMessage, ConceptRequest, ConceptResponse, ConstraintViolation, ContextEntry, CreateArtifactRequest, CustomStyle, DelegationResult, DialogueDistillation, EkycSession, Endpoints, Expression, FederatedMetrics, FederationHandshake, FederationPushRequest, FederationPushResponse, FederationSyncRequest, FederationSyncResponse, ForecastConfig, ForecastResult, GenerativeRequest, GigBid, GigDeliverable, GigIntent, Guild, GuildMember, HarnessRecord, HypothesisManifest, ImmuneRule, Invariant, InvariantDagNode, Job, JobMetrics, KarmaClassification, KarmaDirectives, KarmaEntry, KarmaMetrics, KarmaSearchResult, ListingFilter, LiveFunctionCall, LiveFunctionResponse, LiveToolCall, LiveToolResponse, LlmJobResponse, LocalizedScript, LogEntry, LoraListing, LoraPurchase, MediaProcessingRequest, MediaProcessingResponse, Message, MessageMeta, MultiReviewResult, OracleVerdict, OutputArtifact, PricingConfig, QuarantinedAsset, RefundedEscrow, ResourceUsageLog, ReviewConfig, ReviewContext, ScoringCriterion, SecurityProfile, SlaConfig, SnsMetricsRecord, SoTConfig, SpentEscrow, SynthesisRequest, SynthesisResponse, SystemSetting, SystemStatus, TrajectoryStep, TranscriptionResult, TranscriptionSegment, TreasureFeedback, TreasureItem, TrendItem, TrendRequest, TrendResponse, UnspentEscrow, UpdateJobStatusRequest, VerificationResult, WorkflowRequest, WorkflowResponse, ZtasProfile

### `aiome-contracts`
**Traits (Interfaces)**
- AgentHook, AiomePlugin, CommerceEngine, EmbeddingProvider, GiftEngine, LlmProvider, RuntimeJail, X402Negotiator
**Domain Structs**
- BudgetExhaustedError, EconomicContext, GiftPolicyContext, GiftRequest, LlmMessage, LlmRequest, LlmResponse, NativeModelConfig, PaymentProof, PermissionManifest, TrellisGenerateRequest, TrellisGenerateResponse

### `soul`
**Traits (Interfaces)**
- SamsaraEngine, SoulDomainAdapter, SoulMiddleware, SoulMiddlewareNext
**Domain Structs**
- AgentSoul, AnamnesisProfile, AttachmentModel, BoundingGuard, Defense, DomainModel, Experience, Instinct, InstinctRule, PersonaBoundaries, PredictiveModel, SemanticRecaller, SemanticSummary, SomaticMarker, SoulContext, SoulPipeline

### `infrastructure`
**Traits (Interfaces)**
- ActionHarness, ChannelBridge, CoreOps, CrdtOps, DbInitializer, EvaluationOps, EvolutionOps, ExpressionOps, FederationOps, GuardrailOps, KarmaOps, QuarantineStore, SecurityOps, SettingsOps, SlmBackend, SoulStoreOps, SwarmOps, TaskConductor, ToolHook, TrajectoryOps, TrendAdapter, VectorOps, WatchtowerOps
**Domain Structs**
- A2aGrpcClient, AbyssVoiceVault, ActionsImporter, AdapterFamilyInfo, AdapterFileInfo, AdaptiveImmuneSystem, AffiliateAdapter, AgentRateLimiter, AgentRxDiagnostics, AiomeLogClient, ApiSurface, ArchivedLoraModel, AssetManifest, AsyncAuditLogger, AudioHasher, AuditEntry, AutoProfileEngine, BackgroundLlmProvider, BastionGuard, BehaviorMonitor, BeliefConsistencyGate, BeliefGateConfig, BoundaryVerifier, CapabilityRegistry, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStatus, Cleanroom, CliSlmBackend, CognitiveSentinel, CognitiveThresholds, ComfyUiGenerativeEngine, CompilationReport, ConceptCandidate, ConceptManager, ConstraintChecker, ContextBudget, ContextEngine, CoreDomainAdapter, CortexAnswer, CortexCompiler, CortexDocument, CortexIngester, CortexQueryEngine, CortexSynthesizer, CostBypassSwitch, CostCircuitBreaker, CostStatus, CriterionScore, CriticScoreResponse, CsamScanConductor, DatasetExtractor, DefaultConstitutionalValidator, DefaultSamsaraEngine, DefaultStrategicPlanner, DefaultToolDiscoveryEngine, DetectedSkill, DiscordBridge, DiskQuotaManager, DockerConductor, DreamState, DynamicLlmProvider, EnumInfo, Evidence, ExternalTaskRequest, ExternalTrendSonar, FalAiGenerativeEngine, FallbackRouter, FieldInfo, FunctionInfo, GenericLlmConductor, GlobalMockJobQueue, GlobalMockLlm, GraphEdge, GraphNode, GrpcClientConfig, HarnessCache, HarnessOps, HeartbeatWakeupService, HierarchicalRouter, HookChain, HookManager, HumanizerFilter, HumanizerRule, IntentFirewall, IntentGenerator, InvariantDag, KarmaTaxonomy, L1Metadata, L2Metadata, L3Metadata, LoraAutotuner, LoraTrainingConfig, LoraTrainingService, MemoryCrystallizer, MlockedVec, MockA2aClient, MockForecastProvider, MockGenerativeEngine, MockQuarantineStore, MockSoulStore, MockTtsProvider, MockXPublisher, ModelManager, NativeEmbeddingProvider, NativeModelInner, NativeSlmBackend, OpenAiTtsProvider, Oracle, OssAdapterCodeGen, OssAstAnalyzer, OssIntegrationOrchestrator, OssKnowledgeSession, OssRepositoryIndexer, OssTypeMatcher, PlateauReport, PolarQuantEncoder, PostgresInitializer, ProjectKnowledgeIndexer, ProxyLlmProvider, PublishPipeline, QueryOptions, RegistryManager, RepairCalculator, RouteResult, RssCollector, ScoreTracker, SecureGigGateway, SecurityConfig, SemanticCache, SkillArena, SkillForge, SkillImporter, SkillManifest, SkillMetadata, SkillPerformance, SlmBridge, SlmMemoryEntry, SlmRecallData, SlmRecallJsonResponse, SlmRecallResult, SlmTraceChannelScores, SlmTraceData, SlmTraceJsonResponse, SlmTraceResult, SloConfig, SloEngine, SoTEngine, SoulMutator, SoulSnapshot, SoulVersion, SqliteTrajectoryStore, StandardVectorOps, StructInfo, SynthDataset, SynthPair, SynthStats, TaskDispatcher, TelegramBridge, Test, TimesFmProvider, TrainingMetrics, TraitInfo, TrajectoryGraph, TreeNode, TunedHyperparams, TypeMismatch, UniversalArtifactStore, UniversalJobQueue, UniversalLoraMarketplace, UniversalQuarantineStore, UniversalSoulStore, UniversalVaultBackend, UnverifiedSkill, UserLearner, UserProfile, VerifiedSkill, VoiceCoreDrm, WasmHarness, WasmSkillManager, WebSearchAdapter, WhisperMiddleware, WhisperTranscriptionAdapter, WikiArticle, WikiIssue, WorkspaceManager, XttsProvider

