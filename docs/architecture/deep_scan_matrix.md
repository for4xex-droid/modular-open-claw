# 📡 Aiome Deep Scan AST Matrix

> Generated at: 2026-04-27T16:39:05.903488

This file contains the AST-extracted structural matrix of the codebase. Use it to cross-reference against Project NURTURE requirements without hitting LLM context limits.

## 📦 APPS (Endpoints & Services)
### `management-console`
**React Components**
- A2uiRenderer, AVATAR_ASSETS, AgentConsole, AiomeAvatar, ArtifactVault, AuthOverlay, AvatarCharacterContext, AvatarCharacterProvider, AvatarViewerModal, BiomeDialogueView, BiotopeView, Bomb, CausalVisualizer, CharacterBillboard, CharacterPanel, ComponentRenderer, CortexView, DEMO_STEPS_META, DemoView, DiagnosticsHistory, DioramaView, EkycStatusBadge, EscrowManagementView, ExpressionPipeline, FeatureToggle, FilterButton, FlowCard, ForecastView, GlbRenderer, GraphView, HomePage, ImmuneSystem, InxRenderer, LanguageContext, LoraTrainingView, MAX_RETRIES, McpConfigManager, MiniTabBar, ModelSetupStep, MotionComponent, NurtureDashboard, OllamaModelSelector, OnboardingModal, OriginManager, PAGE_SIZE, PromptStatsView, ProofPowerIndicator, React, SYNAPSES, SecretUpdater, SeoPulseView, SettingInput, SettingsPage, SkillCard, SkillVault, SoTProgressBar, SoulStatusBadge, StoryFlow, SurfaceRenderer, SystemBirth, SystemVitalityContext, SystemVitalityProvider, TaskApprovalOverlay, Timeline, TokenSavingsIndicator, TreasureBox, TrendView, VoiceStore, VrmRenderer

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
- `/api/v1/a2ui/action`
- `/api/v1/auth/authorize`
- `/api/v1/auth/delete`
- `/api/v1/auth/token`
- `/api/v1/avatar/inochi2d/:filename`
- `/api/v1/avatar/inochi2d/upload`
- `/api/v1/bootstrap/detect-ollama`
- `/api/v1/bootstrap/factory-reset`
- `/api/v1/bootstrap/status`
- `/api/v1/commerce/balance/:agent_id`
- `/api/v1/commerce/escrow/:escrow_id/release`
- `/api/v1/commerce/escrow/history/:agent_id`
- `/api/v1/commerce/history/:agent_id`
- `/api/v1/commerce/points/:agent_id`
- `/api/v1/commerce/purchase/:agent_id`
- `/api/v1/commerce/subscription/:agent_id`
- `/api/v1/commerce/subscription/cancel`
- `/api/v1/commerce/subscription/create`
- `/api/v1/commerce/transfer`
- `/api/v1/commerce/webhook`
- `/api/v1/commerce/webhook/polar`
- `/api/v1/commerce/withdraw`
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
- `/api/v1/jobs/awaiting-input`
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
- `/api/v1/quality-gate/history`
- `/api/v1/settings`
- `/api/v1/settings/identity`
- `/api/v1/settings/test`
- `/api/v1/soul/init`
- `/api/v1/soul/status`
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
- `/api/v1/whisper/monologue`
- `/api/wiki`
- `/api/wiki/content`
- `/diagnostics`
- `/documents`
- `/documents/:id`
- `/feedback`
- `/health`
- `/ingest`
- `/ingest/text`
- `/ledger`
- `/messages`
- `/oxilean/power`
- `/predict`
- `/prompt-stats`
- `/quarantine`
- `/quarantine/:id/release`
- `/query`
- `/sse`
- `/suggestions`
- `/synth`
- `/wiki`
- `/wiki/:id`
**Key Structs**
- A2uiActionRequest, A2uiActionResponse, AddMemberRequest, AgentChatRequest, AgentEngine, ApiDoc, AppError, AppState, AuditLedgerResponse, Authenticated, AuthenticatedUser, AuthorizeRequest, AutoToggle, AutonomousDemo, AvatarAssetRequest, AvatarVerificationResult, BootContext, BootstrapStatusResponse, CallToolResult, CancelSubscriptionRequest, ChatMessage, CommerceBalanceResponse, Component, CreateGuildRequest, CreateSubscriptionRequest, DbLoggerLayer, DefaultToolCallRouter, DemoApiDoc, DiagnosisResponse, EkycSessionResponse, FactoryResetResponse, ForecastQuery, ForecastResponse, GiftPolicyResponse, GiftResponse, GraphData, GraphEdge, GraphNode, HistoryParams, IdentityResponse, ImportRequest, ImportSkillRequest, IngestResp, IngestTextReq, IngestUrlReq, InitSoulRequest, InitSoulResponse, Inochi2dUploadResponse, JobReviewPayload, JsonRpcError, JsonRpcRequest, JsonRpcResponse, KarmaBridge, KarmaFeedbackRequest, ListArtifactsParams, ListParams, ListToolsResult, ListVoiceAssetsQuery, ListingQueryParams, LogEntry, LogEntryResponse, LoraJobStatusResponse, LoraTrainRequest, LoraTrainResponse, McpClient, McpDiscoveryFile, McpHttpClient, McpProcessManager, McpServerConfig, McpSpawnRequest, McpTool, MessageQuery, ModelStatusResponse, MonologueEntry, MonologueQuery, MonologueResponse, OllamaDetectionResponse, OxiLeanPowerResponse, PluginRegistry, PromptStatsResponse, PublishListingRequest, PullModelRequest, PurchaseRequest, PurchaseResponse, QueryReq, ReleaseEscrowRequest, SendBiomeRequest, SidecarHealth, SkillSummary, SoulStatusResponse, StartAutonomousRequest, SubscriptionResponse, SynthReq, SynthesizeQuery, SynthesizeRequest, TestConnectionRequest, TestConnectionResponse, TokenRequest, TokenResponse, TransferRequest, TrendsResponse, UpdateSettingsRequest, WikiArticleSummary, WithdrawRequest

### `aiome-node`
**REST / Websocket Routes**
- `/agent.json`
- `/handshake`
- `/sync`
**Key Structs**
- FederationHandshake, HandshakeResponse, JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpServer

### `samsara-hub`
**REST / Websocket Routes**
- `/`
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

### `geo-optimizer`
**REST / Websocket Routes**
- `/audit`
- `/health`
**Key Structs**
- AuditRequest
**Python Functions**
- audit, health_check, test_health_endpoint

### `shadow-worker`
**Key Structs**
- OxiLeanProofService, ShadowWorkerService

### `timesfm-sidecar`
**REST / Websocket Routes**
- `/forecast`
- `/health`
**Key Structs**
- ForecastRequest, ForecastResponse
**Python Functions**
- __init__, forecast, get_api_key, health, lifespan, validate_series

### `key-proxy`
**REST / Websocket Routes**
- `/api/v1/health`
- `/api/v1/llm/complete`
- `/api/v1/llm/embed`
- `/api/v1/llm/stream`
- `/api/v1/wp/publish`

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
- CommerceConfig, CommerceEngineFactory, MockCommerceEngine, MockEkycEngine, MockEkycSessionStore, PolarCommerceEngine, RevenueSplitter, StripeCommerceEngine, StripeEkycEngine, TremendousGiftEngine, UniversalEkycSessionStore, UniversalGigEngine, UniversalSyndicateStore, X402Client

### `aiome-core-contracts`
**Traits (Interfaces)**
- A2aClient, AffiliateAdapter, AgentAct, AgentEvolver, AiomeLogger, ArtifactStore, AuditLogger, AuditStore, BiomeRegistry, CapabilityProvider, ChatStore, ConstitutionalValidator, EkycEngine, EkycSessionStore, FederationRegistry, ForecastProvider, GenerativeEngine, GigEngine, HarnessRegistryOps, ImmuneSystemOps, JobQueue, KarmaRegistry, LiveSessionManager, LoraEngine, LoraMarketplace, MediaProcessor, NewsService, PromptExtractor, Publisher, SoulStore, StrategicPlanner, SyndicateOps, SystemStateOps, TaskRegistry, ToolDiscoveryEngine, TrajectoryStore, TranscriptionEngine, TrendSource, TtsProvider, VaultBackend, VoiceKeyVault
**Domain Structs**
- A2aTaskProgress, A2aTaskRequest, AgentCard, AgentDiagnosis, AgentStats, AnomalyResult, ArenaMatch, ArtifactEdge, ArtifactEdgeInput, ArtifactFile, ArtifactMeta, ArtifactResponse, BiomeDialogue, BiomeMessage, ConceptRequest, ConceptResponse, ConstraintViolation, ContextEntry, CreateArtifactRequest, CustomStyle, DelegationResult, DialogueDistillation, EkycSession, ElicitationField, ElicitationRequest, Endpoints, Expression, FederatedMetrics, FederationHandshake, FederationPushRequest, FederationPushResponse, FederationSyncRequest, FederationSyncResponse, ForecastConfig, ForecastResult, GenerativeRequest, GigBid, GigDeliverable, GigIntent, Guild, GuildMember, HarnessRecord, HypothesisManifest, ImmuneRule, Invariant, InvariantDagNode, Job, JobMetrics, KarmaClassification, KarmaDirectives, KarmaEntry, KarmaMetrics, KarmaSearchResult, ListingFilter, LiveFunctionCall, LiveFunctionResponse, LiveToolCall, LiveToolResponse, LlmJobResponse, LocalizedScript, LogEntry, LoraListing, LoraPurchase, MediaProcessingRequest, MediaProcessingResponse, Message, MessageMeta, MultiReviewResult, OracleVerdict, OutputArtifact, OxiLeanProofCertificate, PricingConfig, QuarantinedAsset, RefundedEscrow, ResourceUsageLog, ReviewConfig, ReviewContext, ScoringCriterion, SecurityProfile, SlaConfig, SnsMetricsRecord, SoTConfig, SpentEscrow, SynthesisRequest, SynthesisResponse, SystemSetting, SystemStatus, TrajectoryStep, TranscriptionResult, TranscriptionSegment, TreasureFeedback, TreasureItem, TrendItem, TrendRequest, TrendResponse, UnspentEscrow, UpdateJobStatusRequest, VerificationResult, WorkflowRequest, WorkflowResponse, ZtasProfile

### `aiome-contracts`
**Traits (Interfaces)**
- AgentHook, AiomePlugin, CommerceEngine, EmbeddingProvider, FormalProofGate, GiftEngine, LlmProvider, RlmProvider, RuntimeJail, X402Negotiator
**Domain Structs**
- BudgetExhaustedError, EconomicContext, EscrowRecord, GiftPolicyContext, GiftRequest, LlmMessage, LlmRequest, LlmResponse, NativeModelConfig, PaymentProof, PermissionManifest, PointsBalance, RlmConfig, RlmResponse, TransactionRecord, TrellisGenerateRequest, TrellisGenerateResponse

### `soul`
**Traits (Interfaces)**
- SamsaraEngine, SoulDomainAdapter, SoulMiddleware, SoulMiddlewareNext
**Domain Structs**
- AgentSoul, AnamnesisProfile, AttachmentModel, BoundingGuard, Defense, DomainModel, Experience, Instinct, InstinctRule, PersonaBoundaries, PredictiveModel, SemanticRecaller, SemanticSummary, SomaticMarker, SoulContext, SoulPipeline

### `infrastructure`
**Traits (Interfaces)**
- ActionHarness, BlobStorageOps, ChannelBridge, CoreOps, CrdtOps, DbInitializer, EvaluationOps, EvolutionOps, ExpressionOps, FederationOps, GuardrailOps, KarmaOps, QualityGateStore, QuarantineStore, SecurityOps, SettingsOps, SlmBackend, SoulStoreOps, SupervisedTask, SwarmOps, TaskConductor, ToolHook, TrajectoryOps, TrendAdapter, VectorOps, WatchtowerOps
**Domain Structs**
- A2aGrpcClient, A2uiValidator, AbyssVoiceVault, ActionsImporter, AdapterFamilyInfo, AdapterFileInfo, AdaptiveImmuneSystem, AgentRateLimiter, AgentRxDiagnostics, AiomeCatalog, AiomeLogClient, ApiSurface, ArchivedLoraModel, ArenaBattle, AssetManifest, AsyncAuditLogger, AudioHasher, AuditEntry, AutoProfileEngine, BackgroundLlmProvider, BastionGuard, BehaviorMonitor, BeliefConsistencyGate, BeliefGateConfig, BlobStorageAdapter, BoundaryVerifier, CapabilityRegistry, ChaosLlmProvider, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStatus, Cleanroom, CliSlmBackend, CognitiveSentinel, CognitiveThresholds, ComfyUiGenerativeEngine, CompilationReport, Component, ConceptCandidate, ConstraintChecker, ContextBudget, ContextEngine, CoreDomainAdapter, CortexAnswer, CortexCompiler, CortexDocument, CortexFileProjector, CortexIngester, CortexQueryEngine, CortexSynthesizer, CostBypassSwitch, CostCircuitBreaker, CostStatus, CriterionScore, CriticScoreResponse, CsamScanConductor, DatasetExtractor, DefaultConstitutionalValidator, DefaultSamsaraEngine, DefaultStrategicPlanner, DefaultToolDiscoveryEngine, DetectedSkill, DisabledAffiliateAdapter, DisabledTtsProvider, DiscordBridge, DiskQuotaManager, DockerConductor, DreamState, DynamicLlmProvider, EnumInfo, EvaluationLogEntry, EvaluationLogger, Evidence, ExternalTaskRequest, ExternalTrendSonar, FalAiGenerativeEngine, FallbackRouter, FieldInfo, FilterResult, FunctionInfo, GenericLlmConductor, GeoAuditConductor, GlobalMockJobQueue, GlobalMockLlm, GraphEdge, GraphNode, GrpcClientConfig, HarnessCache, HarnessOps, HeartbeatWakeupService, HierarchicalRouter, HookChain, HookManager, HumanizerFilter, HumanizerRule, IntentFirewall, IntentGenerator, InvariantDag, KarmaTaxonomy, L1Metadata, L2Metadata, L3Metadata, LoraAutotuner, LoraTrainingConfig, LoraTrainingService, MemoryCrystallizer, MlockedVec, MockA2aClient, MockAffiliateAdapter, MockForecastProvider, MockGenerativeEngine, MockLlm, MockQuarantineStore, MockSoulStore, MockTtsProvider, MockXPublisher, ModelManager, NativeEmbeddingProvider, NativeModelInner, NativeSlmBackend, OpenAiTtsProvider, Oracle, OssAdapterCodeGen, OssAstAnalyzer, OssIntegrationOrchestrator, OssKnowledgeSession, OssRepositoryIndexer, OssTypeMatcher, OutputFilter, PlateauReport, PolarQuantEncoder, PostgresInitializer, ProjectKnowledgeIndexer, ProjectionReport, ProviderEvalStat, ProxyLlmProvider, PublishPipeline, QualityGateEntry, QueryOptions, RegistryManager, RepairCalculator, RlmClient, RouteResult, RssCollector, SafeCommandBuilder, ScoreTracker, SecureGigGateway, SecurityConfig, SemanticCache, SeoContentConductor, SerpAnalysisAdapter, SkillArena, SkillForge, SkillImporter, SkillManifest, SkillMetadata, SkillPerformance, SlmBridge, SlmMemoryEntry, SlmRecallData, SlmRecallJsonResponse, SlmRecallResult, SlmTraceChannelScores, SlmTraceData, SlmTraceJsonResponse, SlmTraceResult, SloConfig, SloEngine, SoTEngine, SoulMutator, SoulSnapshot, SoulVersion, SqliteQualityGateStore, SqliteTrajectoryStore, StandardVectorOps, StructInfo, Surface, SynthDataset, SynthPair, SynthStats, TaskDispatcher, TaskSupervisor, TelegramBridge, Test, TimesFmProvider, TrainingMetrics, TraitInfo, TrajectoryGraph, TreeNode, TunedHyperparams, TypeMismatch, UniversalArtifactStore, UniversalJobQueue, UniversalLoraMarketplace, UniversalQuarantineStore, UniversalSoulStore, UniversalVaultBackend, UnverifiedSkill, UserLearner, UserProfile, VerifiedSkill, VoiceCoreDrm, WasmHarness, WasmSkillManager, WebSearchAdapter, WhisperMiddleware, WhisperTranscriptionAdapter, WikiArticle, WikiIssue, WordPressAdapter, WorkspaceManager, XSignalProbe, XttsProvider

