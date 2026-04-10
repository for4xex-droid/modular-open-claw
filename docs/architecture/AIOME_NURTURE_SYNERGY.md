# Aiome × Project NURTURE 統合仕様書

> **自動生成元**: `/docs-gen` ワークフロー  
> **最終更新**: 2026-04-09  
> **対象リポジトリ**: `aiome/` (OSS) + `ProjectーNurture/` (商用拡張)

---

## 1. 概要

**Aiome** は自律型 AI エージェントのための「文明の OS」（OSS、BSL-1.1）。
**Project NURTURE** は Aiome に経済的自我を注入する商用拡張モジュール（BSL-1.1）。

2つは物理的に別リポジトリだが、設計上は「1つのシステム」として動作する。
Aiome は NURTURE を知らなくても機能し、NURTURE は Aiome に依存する。

```
NURTURE ──依存──▶ Aiome OSS
Aiome OSS ──✕依存しない✕──▶ NURTURE
```

---

## 2. 統合アーキテクチャ全体図

```mermaid
graph TB
    subgraph "ユーザー / 外部"
        USER["👤 ユーザー"]
        CREATOR["🎨 クリエイター"]
        MERCHANT["🏪 外部事業者"]
        GAME_SRV["🎮 ゲームサーバー"]
    end

    subgraph "Aiome OSS — 文明の OS"
        direction TB
        subgraph "L0: Safety Foundation"
            TLA["TLA+ 形式検証"]
            BASTION["BastionGuard"]
            CONST["ConstitutionalValidator"]
        end

        subgraph "L1: Soul Engine"
            SOUL["AgentSoul"]
            KARMA["KarmaRegistry"]
            TRAJECTORY["TrajectoryStore"]
            SOT["SoTEngine"]
        end

        subgraph "L2: Capabilities"
            LLM["LlmProvider (6実装)"]
            TTS["TtsProvider"]
            LORA["LoraEngine"]
            AVATAR["AvatarEngine"]
            VISION["VisionProvider (計画)"]
            SKILL["SkillForge (WASM)"]
            MCP_REG["RegistryManager (MCP)"]
            TREND["TrendSonar (X/Serp)"]
        end

        subgraph "L3: Social Infrastructure"
            SAMSARA["SamsaraHub (Federation)"]
            SYNDICATE["SmartSyndicate"]
            IMMUNE["AdaptiveImmuneSystem"]
        end

        subgraph "Apps"
            API["api-server (108 endpoints)"]
            MGMT["Management Console (57 screens)"]
            PROXY["key-proxy (AbyssVault/WP)"]
            TAURI["Tauri Desktop (計画)"]
        end
    end

    subgraph "Project NURTURE — 経済拡張"
        direction TB
        subgraph "Commerce Protocol"
            TX["Transaction&lt;S: TxState&gt;"]
            COMMODITY["CommodityKind"]
            OFFER["Offer + SaleMode"]
            ACTOR["EconomicActor"]
            REPUTATION["ReputationScore"]
        end

        subgraph "Nurture Core"
            COIN["AiomeCoin (前払い非換金)"]
            POINTS["CreatorPoints (報酬)"]
            LEDGER["EconomyLedger (二重記帳)"]
            POLICY["EconomyPolicy"]
            LICENSE["AssetLicense"]
            SURPRISE["SurpriseEngine (A2C)"]
        end

        subgraph "Nurture Infra"
            BRIDGE["NurtureCommerceBridge"]
            INTERCEPTOR["EconomyInterceptor"]
            MARKET_DB["SQLiteMarketplace"]
            DRM["DrmEngine"]
            CSAM_N["CsamPipeline (3層防壁)"]
            STRIPE["StripeWebhookHandler"]
            SANDBOX["PythonExecutor"]
            SIDECAR["SidecarLauncher"]
            VRAM["VramArbiter"]
            CLONE["CloneManager"]
            P2P_SAFE["P2PSanitizer"]
            RESIDENCY["ResidencyManager"]
        end

        subgraph "Nurture API"
            NAPI["nurture-api (JSON-RPC)"]
            MCP_AUTH["McpAuth (OAuth 2.1)"]
            MCP_TOOLS["MCP Tools (search/buy/gift)"]
            PLUGIN["NurturePlugin"]
        end
    end

    subgraph "将来統合 (OSS側)"
        PET["🐾 デスクトップペット (2C-7)"]
        PWA["📱 PWA (2D-6)"]
        MC_MCP["🎮 Minecraft MCP (6-4)"]
        VISION_MOD["👁️ 視覚認識 (7-6)"]
    end

    %% ユーザー接続
    USER -->|チャット / 育成| MGMT
    USER -->|デスクトップ| TAURI
    CREATOR -->|アセット出品| NAPI
    MERCHANT -->|SDK| NAPI
    GAME_SRV -->|RCON| MC_MCP

    %% Aiome 内部フロー
    API --> LLM
    API --> SOUL
    API --> MCP_REG
    MGMT --> API
    TAURI --> API
    SOUL --> KARMA
    SOUL --> TRAJECTORY
    SOUL --> SOT
    LLM --> BASTION
    LLM --> CONST
    SKILL --> BASTION
    AVATAR --> TTS
    MCP_REG -.->|将来| MC_MCP
    MCP_REG -.->|将来| VISION_MOD

    %% NURTURE → Aiome 依存
    BRIDGE -->|use aiome_core::CommerceEngine| API
    BRIDGE -->|use aiome_core::JobQueue| API
    INTERCEPTOR --> POLICY
    NAPI --> API
    PLUGIN -->|AiomePlugin trait| API
    MCP_TOOLS --> MCP_REG
    CLONE -->|use aiome_core::FederatedKarma| KARMA

    %% NURTURE 内部フロー
    TX --> LEDGER
    OFFER --> COMMODITY
    COIN --> LEDGER
    POINTS --> LEDGER
    NAPI --> MCP_AUTH
    NAPI --> MCP_TOOLS
    NAPI --> STRIPE
    DRM --> LICENSE
    CSAM_N --> DRM
    SIDECAR --> VRAM
    CLONE --> SIDECAR
    P2P_SAFE --> SAMSARA

    %% 将来統合
    TAURI -.-> PET
    MGMT -.-> PWA
```

---

## 3. レイヤード・アーキテクチャ

### Aiome OSS — 4層構造

| レイヤー | 責務 | 主要クレート |
|---------|------|------------|
| **L0: Safety** | TLA+ 形式検証、BastionGuard、Constitutional Validator | `shared`, `aiome-contracts` |
| **L1: Soul** | 記憶、感情、人格、自己修復 | `soul`, `aiome-core-contracts` |
| **L2: Capabilities** | LLM推論、TTS、LoRA、3Dアバター、MCP | `core`, `avatar-engine`, `infrastructure` |
| **L3: Social** | Federation、CRDT同期、ギルド | `samsara-hub`, `infrastructure` |

### Project NURTURE — 3層構造

| レイヤー | 責務 | クレート |
|---------|------|--------|
| **Protocol** | 経済モデルの型定義、不変条件 | `commerce-protocol` |
| **Core** | 取引承認、台帳管理、暴走防壁 | `nurture-core` |
| **Infra** | DB実装、DRM、CSAM、Stripe、サンドボックス | `nurture-infra` |
| **API** | JSON-RPC/MCP エンドポイント | `nurture-api` |

---

## 4. クロスリポジトリ依存マップ

```mermaid
graph LR
    subgraph "Aiome OSS (aiome/)"
        AC[aiome-core]
        SH[shared]
        INF[infrastructure]
        CONTRACTS[aiome-contracts]
        COMMERCE[aiome-commerce]
    end

    subgraph "Project NURTURE (ProjectーNurture/)"
        CP[commerce-protocol]
        NC[nurture-core]
        NI[nurture-infra]
        NA[nurture-api]
    end

    NA --> NI
    NI --> NC
    NC --> CP
    NI --> CP

    NI -->|"path = ../aiome/libs/core"| AC
    NI -->|"path = ../aiome/libs/shared"| SH
    NI -->|"path = ../aiome/libs/infrastructure"| INF
    NA -->|"path = ../aiome/libs/core"| AC

    style AC fill:#4CAF50,color:white
    style SH fill:#4CAF50,color:white
    style INF fill:#4CAF50,color:white
    style CONTRACTS fill:#4CAF50,color:white
    style CP fill:#FF9800,color:white
    style NC fill:#FF9800,color:white
    style NI fill:#FF9800,color:white
    style NA fill:#FF9800,color:white
```

### 具体的な依存ポイント（Rust `use` 文レベル）

| NURTURE 側ファイル | 使用する Aiome シンボル |
|-------------------|---------------------|
| `economy/bridge.rs` | `aiome_core::commerce::CommerceEngine`, `aiome_core::error::AiomeError`, `aiome_core::traits::JobQueue` |
| `economy/karma_forge.rs` | `aiome_core::contracts::FederatedKarma`, `aiome_core::traits::JobQueue` |
| `economy/karma_immune_filter.rs` | `aiome_core::contracts::{FederatedKarma, ImmuneRule}` |
| `sidecar/clone_manager.rs` | `aiome_core::contracts::FederatedKarma`, `aiome_core::traits::JobQueue` |
| `mock_job_queue.rs` | `infrastructure::job_queue::SqliteJobQueue`, `aiome_core::contracts::*` |

---

## 5. 主要データフロー

### 5.1 ユーザーチャット → 自律購入フロー

```mermaid
sequenceDiagram
    actor User as 👤 ユーザー
    participant MC as Management Console
    participant API as api-server (Aiome)
    participant Soul as AgentSoul
    participant LLM as LlmProvider
    participant MCP as RegistryManager
    participant NAPI as nurture-api
    participant Ledger as EconomyLedger
    participant Market as SQLiteMarketplace
    participant Interceptor as EconomyInterceptor

    User->>MC: 「新しい服が欲しいな」
    MC->>API: POST /api/stream/chat
    API->>Soul: process_input()
    Soul->>LLM: generate(system_prompt + context)
    LLM-->>Soul: "マーケットプレイスで探しましょう"
    Soul->>MCP: call_tool("marketplace_search")
    MCP->>NAPI: JSON-RPC: marketplace_search
    NAPI->>Market: search(query)
    Market-->>NAPI: [Offer { item: "花柄ワンピース", price: 500 coin }]
    NAPI-->>MCP: search_results
    MCP-->>Soul: tool_result
    Soul->>LLM: "この花柄ワンピース、似合うと思います！"
    LLM-->>Soul: "購入しましょうか？"
    Soul-->>API: SSE response
    API-->>MC: SSE stream
    MC-->>User: 「花柄ワンピース見つけました！買いますか？」

    User->>MC: 「買って！」
    MC->>API: POST /api/stream/chat
    API->>Soul: process_input()
    Soul->>MCP: call_tool("buy")
    MCP->>NAPI: JSON-RPC: buy { offer_id, amount: 500 }

    NAPI->>Interceptor: check_budget(500)
    alt 予算超過
        Interceptor-->>NAPI: REJECTED (暴走ストッパー発動)
        NAPI-->>MCP: Error: BudgetExhausted
    else 予算内
        Interceptor-->>NAPI: APPROVED
        NAPI->>Ledger: debit(agent_wallet, 500)
        Ledger->>Ledger: atomic double-entry
        NAPI-->>MCP: BuyResponse { success: true }
    end

    MCP-->>Soul: tool_result
    Soul-->>API: SSE "お買い物完了！"
    API-->>MC: SSE stream
    MC-->>User: 「花柄ワンピース買えました！🎉」
```

### 5.2 クリエイターアセット出品 → CSAM検査 → DRM保護

```mermaid
sequenceDiagram
    actor Creator as 🎨 クリエイター
    participant NAPI as nurture-api
    participant Auth as McpAuth (OAuth 2.1)
    participant CSAM as CsamPipeline
    participant eKYC as EkycVerifier
    participant PHash as PhashScanner
    participant Bone as BoneChecker
    participant DRM as DrmEngine
    participant Market as SQLiteMarketplace

    Creator->>NAPI: POST /marketplace/upload(vrm_file)
    NAPI->>Auth: verify_token(bearer)
    Auth-->>NAPI: Claims { role: Creator }

    NAPI->>CSAM: scan(vrm_file)
    CSAM->>eKYC: verify_identity()
    eKYC-->>CSAM: ✅ 本人確認済み
    CSAM->>PHash: check_hash(file)
    PHash-->>CSAM: ✅ 既知ブラックリスト不一致
    CSAM->>Bone: check_proportions(vrm)
    Bone-->>CSAM: ✅ 頭身比 OK

    CSAM-->>NAPI: ScanVerdict::Safe

    NAPI->>DRM: encrypt_asset(vrm_file)
    DRM-->>NAPI: DrmPackage { encrypted_blob, license_key }
    NAPI->>Market: list(Offer { sale_mode: Parts, price: 800 })
    Market-->>NAPI: offer_id

    NAPI-->>Creator: { status: "listed", offer_id }
```

### 5.3 A2C ギフト（恩返し）フロー

```mermaid
sequenceDiagram
    participant Soul as AgentSoul (Aiome)
    participant Karma as KarmaRegistry (Aiome)
    participant MCP as RegistryManager
    participant NAPI as nurture-api
    participant Surprise as SurpriseEngine
    participant Gift as GiftEngine (Aiome)
    participant Tremendous as Tremendous API

    Soul->>Karma: get_metrics(user_id)
    Karma-->>Soul: { total_care: 9500, streak: 30 days }
    Soul->>Soul: 内部判定: "感謝の閾値を超えた"

    Soul->>MCP: call_tool("gift_delivery")
    MCP->>NAPI: JSON-RPC: gift_delivery { user_id }
    NAPI->>Surprise: evaluate(karma_metrics)
    Surprise-->>NAPI: { tier: "イースターエッグ", gift_type: "coffee_coupon" }

    NAPI->>Gift: send(gift_request)
    Gift->>Tremendous: POST /orders (coffee_coupon, $5)
    Tremendous-->>Gift: { order_id, redemption_link }
    Gift-->>NAPI: ✅

    NAPI-->>MCP: gift_response
    MCP-->>Soul: tool_result
    Soul-->>Soul: "いつもありがとう！☕️"
```

---

## 6. 主要データ構造（クラス図）

### Commerce Protocol（経済モデル型）

```mermaid
classDiagram
    class TxState {
        <<trait>>
    }
    class Initiated
    class Authorized
    class Settled
    class Failed
    class Refunded
    class Cancelled

    TxState <|-- Initiated
    TxState <|-- Authorized
    TxState <|-- Settled
    TxState <|-- Failed
    TxState <|-- Refunded
    TxState <|-- Cancelled

    class Transaction~S: TxState~ {
        +Uuid id
        +ActorId buyer
        +ActorId seller
        +PriceTag price
        +initiate() Transaction~Initiated~
        +authorize() Transaction~Authorized~
        +settle() Transaction~Settled~
    }

    class CommodityKind {
        <<enum>>
        AvatarFull
        AvatarPart
        VoiceCore
        LoraAdapter
        Skill
        EasterEgg
    }

    class ItemDescriptor {
        +CommodityKind kind
        +String name
        +String description
    }

    class PriceTag {
        <<enum>>
        Coin(u64)
        Points(u64)
        Free
    }

    class Offer {
        +Uuid id
        +ActorId seller
        +ItemDescriptor item
        +PriceTag price
        +SaleMode mode
        +OfferStatus status
    }

    class SaleMode {
        <<enum>>
        FullAvatar
        Parts
    }

    class EconomicActor {
        +ActorId id
        +ActorKind kind
        +ReputationScore reputation
    }

    class ActorKind {
        <<enum>>
        Agent
        Creator
        Merchant
    }

    class ReputationScore {
        +f64 score
        +TrustLevel level
        +u64 total_transactions
    }

    Transaction --> PriceTag
    Transaction --> EconomicActor
    Offer --> ItemDescriptor
    Offer --> SaleMode
    ItemDescriptor --> CommodityKind
    EconomicActor --> ActorKind
    EconomicActor --> ReputationScore
```

### Aiome OSS — 経済インターフェース（接合点）

```mermaid
classDiagram
    class CommerceEngine {
        <<trait / aiome-contracts>>
        +get_balance(agent_id) Result~EconomicContext~
        +process_purchase(agent_id, request) Result
        +create_subscription(request) Result
        +cancel_subscription(request) Result
    }

    class GiftEngine {
        <<trait / aiome-contracts>>
        +evaluate_policy(context) Result
        +send_gift(request) Result
    }

    class AiomePlugin {
        <<trait / aiome-contracts>>
        +name() String
        +version() String
        +register_routes(router) Router
    }

    class LlmProvider {
        <<trait / aiome-contracts>>
        +generate(request) Result~LlmResponse~
        +stream(request) Result~Stream~
    }

    class NurtureCommerceBridge {
        CommerceEngine を実装
        +aiome 台帳と nurture 台帳を橋渡し
    }

    class NurturePlugin {
        AiomePlugin を実装
        +nurture-api のルートを登録
    }

    CommerceEngine <|.. NurtureCommerceBridge : implements
    AiomePlugin <|.. NurturePlugin : implements

    note for CommerceEngine "Aiome OSS 側で定義\nNURTURE側で実装"
    note for LlmProvider "Aiome OSS 側で定義・実装\nNURTUREは触れない"
```

---

## 7. セキュリティ境界

```mermaid
graph TB
    subgraph "Trust Zone A (Aiome OSS)"
        direction TB
        A1["BastionGuard — 認証/認可"]
        A2["ConstitutionalValidator — 倫理フィルター"]
        A3["PathSandbox — ファイルシステム隔離"]
        A4["IntentFirewall — MCP インテント検証"]
    end

    subgraph "Trust Zone B (NURTURE)"
        direction TB
        B1["McpAuth — OAuth 2.1 + RBAC"]
        B2["EconomyInterceptor — 予算超過防止"]
        B3["CsamPipeline — 3層コンテンツ安全"]
        B4["DrmEngine — オンメモリ暗号化"]
        B5["P2PSanitizer — Federation浄化"]
        B6["PythonExecutor — サンドボックス実行"]
    end

    subgraph "Trust Zone C (外部)"
        direction TB
        C1["Stripe API"]
        C2["Tremendous API"]
        C3["LLM Providers"]
    end

    A1 -->|認証トークン| B1
    A4 -->|MCP ツール呼び出し検証| B2
    B3 -->|アセット検査結果| A3
    B5 -->|浄化済みデータ| A1

    B2 -->|決済| C1
    B2 -->|ギフト| C2
    A1 -->|推論| C3
```

---

## 8. 将来統合（OSS インスピレーション統合）

### 統合タイムライン

```mermaid
gantt
    title Aiome OSS + NURTURE 統合タイムライン
    dateFormat YYYY-MM
    axisFormat %Y-%m

    section Aiome OSS MVP
    Phase 2-PRE Path Standard.  :a1, 2026-04, 1w
    Phase 2C Tauri              :a2, after a1, 2w
    Phase 2D E2E                :a3, after a2, 2w
    Phase 1 TTS + LoRA          :a4, after a3, 4w
    Phase 4 Inochi2D            :a5, 2026-07, 4w
    Phase 6 MCP Ecosystem       :a6, 2026-09, 4w
    Phase 7 Advanced AI         :a7, 2026-11, 6w

    section OSS 統合タスク
    2C-7 Desktop Pet            :crit, o1, after a1, 2d
    2D-6 PWA Manifest           :o2, after a2, 1d
    6-4 Minecraft MCP           :o3, after a6, 5d
    7-6 Vision (libs/vision/)   :o4, after a7, 4d

    section NURTURE
    N-Phase 1 Core Economy      :n1, 2026-04, 12w
    N-Phase 2 Creator           :n2, 2026-07, 12w
    N-Phase 3 Symbiotic         :n3, 2026-10, 8w
    N-Phase 4 A2A + Merchant    :n4, 2027-01, 12w
```

### 統合による新しいシナジー

| OSS 統合要素 | NURTURE との相乗効果 |
|-------------|-------------------|
| **デスクトップペット (2C-7)** | NURTURE の着せ替え機能と組み合わせ → ペットモードのアバターにマーケットプレイスの衣装を着用 |
| **PWA (2D-6)** | モバイルから NURTURE マーケットプレイスへのアクセスが可能に → 移動中のアセット購入 |
| **Minecraft MCP (6-4)** | ゲーム内の成果が Karma に反映 → NURTURE の ReputationScore に影響 |
| **視覚認識 (7-6)** | 画面の内容を理解した上でのコンテキスト購入推薦 → NURTURE の購買体験が飛躍的に向上 |

---

## 9. コンプライアンス・ライセンス

| 項目 | Aiome OSS | Project NURTURE |
|------|----------|----------------|
| **ライセンス** | BSL-1.1 → Apache 2.0 (2030) | BSL-1.1 → Apache 2.0 (2030) |
| **著作権** | motivationstudio, LLC | motivationstudio, LLC |
| **依存方向** | 単独で動作（NURTURE 不要） | Aiome に依存（Aiome 必須） |
| **外部 OSS 参考** | Open-LLM-VTuber (MIT), AIRI (MIT): 設計参考のみ、コードコピーなし | — |
| **CSAM 対策** | OSS 側: `ImageHasher`, `ProportionsChecker` | NURTURE 側: `CsamPipeline` (3層: eKYC + PHash + BoneCheck) |
| **決済法** | 非該当 | BSL 下で資金決済法の監視対象（未使用残高 1,000万円超時） |

---

## 10. ファイル一覧（全構造体・トレイト・列挙型）

### Aiome OSS — 主要シンボル (抜粋)

| クレート | シンボル数 | 代表的なシンボル |
|---------|----------|---------------|
| `aiome-contracts` | 16 | `LlmProvider`, `CommerceEngine`, `GiftEngine`, `AiomePlugin`, `RuntimeJail` |
| `aiome-core-contracts` | 70+ | `JobQueue`, `KarmaRegistry`, `ArtifactStore`, `Publisher` |
| `infrastructure` | 150+ | `RegistryManager`, `WordPressAdapter`, `ContextEngine`, `SoTEngine` |
| `soul` | 20+ | `AgentSoul`, `SoulPipeline`, `SomaticMarker`, `SemanticRecaller` |
| `core` | 20+ | `OllamaProvider`, `GeminiProvider`, `ClaudeProvider`, `OpenAiProvider` |
| `avatar-engine` | 15+ | `Inochi2dLoader`, `SimpleLipSyncEngine`, `PhysicsSimulator` |

### Project NURTURE — 全シンボル

| クレート | シンボル数 | 代表的なシンボル |
|---------|----------|---------------|
| `commerce-protocol` | 26 | `Transaction<S>`, `CommodityKind`, `Offer`, `SaleMode`, `EconomicActor`, `ReputationScore` |
| `nurture-core` | 15 | `AiomeCoin`, `CreatorPoints`, `EconomyLedger`, `EconomyPolicy`, `SurpriseEngine` |
| `nurture-infra` | 40+ | `NurtureCommerceBridge`, `EconomyInterceptor`, `DrmEngine`, `CsamPipeline`, `VramArbiter`, `CloneManager` |
| `nurture-api` | 15+ | `NurturePlugin`, `McpAuth`, MCP ツール群 (`search`, `buy`, `gift`, `wallet`, `sandbox_exec`) |

---

*本ドキュメントは `/docs-gen` ワークフローにより Aiome OSS および Project NURTURE の Rust ソースコードから自動生成されました。*
