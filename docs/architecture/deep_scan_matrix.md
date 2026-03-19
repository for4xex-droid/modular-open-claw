# 📡 Aiome Deep Scan AST Matrix

> Generated at: 2026-03-20T01:24:23.848418

This file contains the AST-extracted structural matrix of the codebase. Use it to cross-reference against Project NURTURE requirements without hitting LLM context limits.

## 📦 APPS (Endpoints & Services)
### `management-console`
**React Components**
- AgentConsole, AiomeAvatar, AuthOverlay, AvatarCharacterProvider, BiomeDialogueView, BiotopeView, CharacterBillboard, DioramaView, ExpressionPipeline, FilterButton, GraphView, ImmuneSystem, InxRenderer, OllamaModelSelector, OnboardingModal, OriginsManager, SecretUpdater, SettingInput, SettingsPage, SkillCard, SkillVault, SystemBirth, Timeline, VaultProtectionItem, VrmRenderer

### `api-server`
**REST / Websocket Routes**
- `/api/biome/list`
- `/api/biome/status`
- `/api/health`
- `/api/skills`
- `/api/soul/status`
- `/api/synergy/karma`
- `/api/v1/logs`
- `/api/v1/watchtower/ws`
- `/api/wiki`
- `/ekyc-status`
- `/messages`
- `/sse`
**Key Structs**
- AgentChatRequest, ApiDoc, AppError, AppState, Authenticated, AuthenticatedUser, AutoToggle, AvatarAssetRequest, AvatarVerificationResult, CallToolResult, ChatMessage, CommerceBalanceResponse, DbLoggerLayer, GraphData, GraphEdge, GraphNode, ImportRequest, ImportSkillRequest, JsonRpcError, JsonRpcRequest, JsonRpcResponse, KarmaBridge, KarmaFeedbackRequest, ListArtifactsParams, ListParams, ListToolsResult, LogEntry, LogEntryResponse, McpClient, McpDiscoveryFile, McpProcessManager, McpServerConfig, McpSpawnRequest, McpTool, MessageQuery, PluginRegistry, PurchaseRequest, PurchaseResponse, SendBiomeRequest, SkillSummary, SoulStatusResponse, StartAutonomousRequest, TestConnectionRequest, TestConnectionResponse, UpdateSettingsRequest

### `samsara-hub`
**REST / Websocket Routes**
- `/api/v1/biome/relay`
- `/api/v1/biome/ws`
- `/api/v1/federation/push`
- `/api/v1/federation/sync`
- `/api/v1/federation/ws`
- `/api/v1/health`
- `/api/v1/relay/timeline/sync`
**Key Structs**
- BiomeWsQuery, HubState, TimelineSyncRequest

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
- AbyssVaultProvider, AutonomousBiomeEngine, AutonomousConfig, ClaudeProvider, DialogueManager, ExpressionEngine, GeminiProvider, JobBudget, LmStudioProvider, MockLlmProvider, OllamaProvider, OpenAiProvider, RuriProvider

### `napi-bridge`
**Domain Structs**
- SubagentSpawnResponse, ToolCheckResponse

### `shared`
**Domain Structs**
- AiomeConfig, AiomeCustomClaims, AuditEntry, BeggingSupervisor, CleanupTarget, HealthMonitor, ImageHasher, PathSandbox, ProportionsChecker, ResourceStatus, Secret, SecurityPolicy, StorageCleaner

### `avatar-engine`
**Domain Structs**
- AssetManifest, AvatarParameters, EmotionToParameterMapper, LipSyncFrame

### `aiome-contracts`
**Traits (Interfaces)**
- AgentAct, AiomeLogger, AiomePlugin, ArtifactStore, CommerceEngine, ConstitutionalValidator, EmbeddingProvider, GenerativeEngine, GiftEngine, JobQueue, LlmProvider, MediaProcessor, Publisher, RuntimeJail, TrajectoryStore, TrendSource
**Domain Structs**
- AgentDiagnosis, AgentStats, ArenaMatch, ArtifactEdge, ArtifactEdgeInput, ArtifactFile, ArtifactMeta, ArtifactResponse, BiomeDialogue, BiomeMessage, BudgetExhaustedError, ConceptRequest, ConceptResponse, ConstraintViolation, CreateArtifactRequest, CustomStyle, DelegationResult, DialogueDistillation, EconomicContext, Expression, FederatedKarma, FederationHandshake, FederationPushRequest, FederationPushResponse, FederationSyncRequest, FederationSyncResponse, GenerativeRequest, GiftRequest, ImmuneRule, Job, KarmaClassification, KarmaDirectives, KarmaEntry, KarmaSearchResult, LlmJobResponse, LlmResponse, LocalizedScript, LogEntry, MediaProcessingRequest, MediaProcessingResponse, Message, MessageMeta, OracleVerdict, OutputArtifact, PermissionManifest, ResourceUsageLog, SnsMetricsRecord, SynthesisRequest, SynthesisResponse, SystemSetting, SystemStatus, TrajectoryStep, TrendItem, TrendRequest, TrendResponse, WorkflowRequest, WorkflowResponse

### `soul`
**Traits (Interfaces)**
- SamsaraEngine, SoulDomainAdapter
**Domain Structs**
- AgentSoul, AnamnesisProfile, AttachmentModel, Defense, DomainModel, Experience, Instinct, InstinctRule, PredictiveModel, SomaticMarker, SoulPipeline

### `infrastructure`
**Traits (Interfaces)**
- AuthManager, ChannelBridge, CoreOps, CrdtOps, DbInitializer, EkycEngine, EvaluationOps, EvolutionOps, ExpressionOps, FederationOps, GuardrailOps, KarmaOps, QuarantineStore, SettingsOps, SwarmOps, TrajectoryOps, WatchtowerOps
**Domain Structs**
- ActionsImporter, AdaptiveImmuneSystem, AgentRxDiagnostics, AiomeLogClient, BackgroundLlmProvider, BastionGuard, CircuitBreaker, CircuitBreakerConfig, Cleanroom, ConceptManager, ConstraintChecker, ContextEngine, CoreDomainAdapter, DefaultConstitutionalValidator, DefaultSamsaraEngine, DiscordBridge, DreamState, DynamicLlmProvider, ExternalTrendSonar, HeartbeatWakeupService, KarmaTaxonomy, L1Metadata, L2Metadata, L3Metadata, MemoryCrystallizer, MockAuthManager, MockCommerceEngine, MockEkycEngine, MockJobQueue, MockQuarantineStore, MockXPublisher, Oracle, ProjectKnowledgeIndexer, ProxyLlmProvider, PublishPipeline, SecurityConfig, SkillArena, SkillForge, SkillImporter, SkillManifest, SkillMetadata, SkillPerformance, SloConfig, SloEngine, SoulMutator, SoulSnapshot, SqliteArtifactStore, SqliteJobQueue, SqliteQuarantineStore, SqliteSoulStore, StripeEkycEngine, TelegramBridge, TremendousGiftEngine, TrendArgs, TrendOutput, UnverifiedSkill, UserLearner, VerifiedSkill, WasmSkillManager, WorkspaceManager

