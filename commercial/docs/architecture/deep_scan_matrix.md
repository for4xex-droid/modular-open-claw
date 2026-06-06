# 📡 Aiome Deep Scan AST Matrix

> Generated at: 2026-05-06T06:19:02.748201

This file contains the AST-extracted structural matrix of the codebase. Use it to cross-reference against Project NURTURE requirements without hitting LLM context limits.

## 📦 APPS (Endpoints & Services)
### `nurture-api`
**REST / Websocket Routes**
- `/:id/download/:buyer_id`
- `/balance`
- `/balance/:actor_id`
- `/buy`
- `/buy/:tx_id/refund`
- `/coin-charge`
- `/daily-stats/:actor_id`
- `/deduct`
- `/escrow-create`
- `/escrow-list/:actor_id`
- `/escrow-refund`
- `/escrow-release`
- `/exec`
- `/forget/:actor_id`
- `/fork`
- `/health`
- `/history`
- `/instant-refund`
- `/list`
- `/message`
- `/oxilean/status`
- `/points`
- `/points/:actor_id`
- `/points/withdraw`
- `/search`
- `/sse`
- `/terminate/:id`
- `/transaction-history/:actor_id`
- `/transfer`
- `/upload`
- `/webhook`
- `/withdraw-points`
**Key Structs**
- AppState, BalanceResponse, CallToolResult, CoinChargeRequest, DailyStatsResponse, DeductCostRequest, EscrowCreateRequest, EscrowCreateResponse, EscrowRefundRequest, EscrowReleaseRequest, ForkRequest, ForkResponse, HistoryQuery, InstantRefundRequest, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ListToolsResult, McpAuth, McpTool, MessageQuery, NurtureAgentHook, NurturePlugin, TransferRequest, TransferResponse, UploadRequest, UploadResponse, WithdrawPointsRequest

### `nurture-ui`
**React Components**
- App

## 📚 LIBS (Core Domain & Infrastructure)
### `nurture-infra`
**Traits (Interfaces)**
- AssetStorage, ContentSafetyChecker, EkycStore, IdempotencyStore, NcmecReporter
**Domain Structs**
- BoneChecker, CloneInstance, CloneManager, CloneSpec, CsamPipeline, DrmEngine, DrmPackage, EconomyInterceptor, EkycVerifier, FilterResult, ForgeResult, IdempotencyResponse, KarmaForge, KarmaImmuneFilter, KarmaToxicityScanner, MerkleAudit, MockAssetStorage, MockEkycStore, MockNcmecReporter, NurtureCommerceBridge, PhashScanner, PolarWebhookHandler, PromotedClone, PromotionCriteria, PythonExecutor, RealJobQueue, ResidencyManager, ResourceBudget, ResourceLimits, S3AssetStorage, SQLiteCustomerStore, SQLiteEconomyLedger, SQLiteEkycStore, SQLiteIdempotencyStore, SQLiteLicenseStore, SQLiteMarketplace, SQLiteNcmecReporter, SQLiteSettlementProvider, SidecarInstance, SidecarLauncher, SqliteCommerceUow, SqliteUowManager, StripeWebhookHandler, VramArbiter, VramReservation

### `nurture-core`
**Traits (Interfaces)**
- CommerceUow, CustomerStore, EconomyLedger, LicenseStore, UowManager
**Domain Structs**
- AiomeCoin, AssetLicense, CoinWallet, CreatorPoints, EconomyPolicy, KarmaPackage, LedgerEntry, Merchant, PointsAccount, ProductCatalog, SurpriseEngine

### `commerce-protocol`
**Traits (Interfaces)**
- SettlementProtocol, TxState
**Domain Structs**
- ActorId, Authorized, BuyRequest, BuyResponse, Cancelled, EconomicActor, Failed, Initiated, ItemDescriptor, MarketSearchRequest, MarketSearchResponse, Offer, Refunded, ReputationScore, SandboxExecRequest, SandboxExecResponse, Settled, SettlementReceipt, Transaction

