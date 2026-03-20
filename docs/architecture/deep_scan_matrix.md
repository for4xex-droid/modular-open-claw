# 📡 Aiome Deep Scan AST Matrix

> Generated at: 2026-03-21T01:48:41.430250

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
- `/api/v1/commerce/balance/:agent_id`
- `/api/v1/commerce/purchase/:agent_id`
- `/api/v1/commerce/webhook`
- `/api/v1/logs`
- `/api/v1/metrics`
- `/api/v1/ollama/models`
- `/api/v1/settings`
- `/api/v1/settings/identity`
- `/api/v1/settings/test`
- `/api/v1/trends`
- `/api/v1/voice/upload`
- `/api/wiki`
- `/api/wiki/content`
- `/health`
- `/messages`
- `/sse`
**Key Structs**
- AgentChatRequest, ApiDoc, AppError, AppState, AuditLedgerResponse, Authenticated, AuthenticatedUser, AutoToggle, AvatarAssetRequest, AvatarVerificationResult, CallToolResult, ChatMessage, CommerceBalanceResponse, DbLoggerLayer, DiagnosisResponse, GraphData, GraphEdge, GraphNode, IdentityResponse, ImportRequest, ImportSkillRequest, JsonRpcError, JsonRpcRequest, JsonRpcResponse, KarmaBridge, KarmaFeedbackRequest, ListArtifactsParams, ListParams, ListToolsResult, LogEntry, LogEntryResponse, McpClient, McpDiscoveryFile, McpProcessManager, McpServerConfig, McpSpawnRequest, McpTool, MessageQuery, PluginRegistry, PurchaseRequest, PurchaseResponse, SendBiomeRequest, SkillSummary, SoulStatusResponse, StartAutonomousRequest, TestConnectionRequest, TestConnectionResponse, TrendsResponse, UpdateSettingsRequest

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
- AbyssVaultProvider, AutonomousBiomeEngine, AutonomousConfig, ClaudeProvider, DialogueManager, ExpressionEngine, GeminiProvider, JobBudget, LmStudioProvider, LoraEngine, LoraModel, MockLlmProvider, OllamaProvider, OpenAiProvider, RuriProvider

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
- AgentAct, AiomeLogger, AiomePlugin, ArtifactStore, CommerceEngine, ConstitutionalValidator, EmbeddingProvider, GenerativeEngine, GiftEngine, JobQueue, LlmProvider, MediaProcessor, Publisher, RuntimeJail, TrajectoryStore, TrendSource, VoiceKeyVault
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
- AbyssVoiceVault, ActionsImporter, AdaptiveImmuneSystem, AgentRxDiagnostics, AiomeLogClient, AssetManifest, BackgroundLlmProvider, BastionGuard, CircuitBreaker, CircuitBreakerConfig, Cleanroom, ConceptManager, ConstraintChecker, ContextEngine, CoreDomainAdapter, DefaultConstitutionalValidator, DefaultSamsaraEngine, DiscordBridge, DreamState, DynamicLlmProvider, ExternalTrendSonar, HeartbeatWakeupService, JwtAuthManager, KarmaTaxonomy, L1Metadata, L2Metadata, L3Metadata, MemoryCrystallizer, MockAuthManager, MockCommerceEngine, MockEkycEngine, MockJobQueue, MockQuarantineStore, MockXPublisher, Oracle, ProjectKnowledgeIndexer, ProxyLlmProvider, PublishPipeline, RegistryManager, SecurityConfig, SkillArena, SkillForge, SkillImporter, SkillManifest, SkillMetadata, SkillPerformance, SloConfig, SloEngine, SoulMutator, SoulSnapshot, SqliteArtifactStore, SqliteJobQueue, SqliteQuarantineStore, SqliteSoulStore, StripeCommerceEngine, StripeEkycEngine, TelegramBridge, TremendousGiftEngine, TrendArgs, TrendOutput, UnverifiedSkill, UserLearner, VerifiedSkill, VoiceCoreDrm, WasmSkillManager, WorkspaceManager

