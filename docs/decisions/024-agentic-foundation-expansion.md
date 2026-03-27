# ADR-024: Agentic Foundation Expansion — エージェンティック基盤の6軸拡張

> **Status**: Partially Implemented (Phase 1 & 2 Completed)  
> **Date**: 2026-03-26  
> **Origin**: ディープスキャンによるギャップ分析  
> **Impact**: infrastructure / soul / aiome-contracts / api-server

## Context

Aiome のエージェンティック基盤は、認知アーキテクチャ（SoulPipeline L1-L3）、自律行動（TaskDispatcher/DreamState）、経済基盤（CommerceEngine）、セキュリティ（BastionGuard/RuntimeJail）、A2A通信（Biome/Federation）において高い成熟度を持つ。

しかし、ディープスキャンにより以下の6つの不足要素が検出された。これらを段階的かつ安全に組み込むことで、AIの自律性と品質を飛躍的に向上させる。

## Decision

### 概要：6軸拡張マップ

```
Phase 0 (前提)                   Phase 1 (P0)                    Phase 2 (P1)                    Phase 3 (P2-P3)
┌─────────────────┐            ┌─────────────────┐            ┌─────────────────┐            ┌─────────────────┐
│ ① Registry     │            │ ② MCP Tool      │            │ ⑤ 統合コンテキスト │            │ ⑦ Trajectory    │
│    LIKE修正    │───前提──▶│    Discovery     │───依存──▶│    検索          │            │    拡張       │
│                  │            │                  │            │                  │            │                  │
│                  │            │ ③ 長期計画       │            │ ⑥ 認知健全性      │            │ ⑧ ユーザー学習   │
│                  │            │    エンジン      │            │    モニター      │            │    モデル深化    │
└─────────────────┘            └─────────────────┘            └─────────────────┘            └─────────────────┘
```

> [!IMPORTANT]
> **Phase 0 (前提条件)**: `RegistryManager::check_ownership` の LIKE 演算子による UUID 部分一致脆弱性（Deep Scan で検出済み）を完全一致検索に修正すること。これが未修正の場合、② `CapabilityIndex` の拡張にセキュリティリスクが波及する。

---

### ① MCP Tool Discovery の自律化（P0）

#### 課題
`McpClient` / `McpProcessManager` は存在するが、AI が「この課題にはこのツールが必要だ」と自ら判断して MCP サーバーを選択・接続するパスがない。

#### 設計

```rust
/// MCP ツールの自律的発見・選択エンジン
pub struct ToolDiscoveryEngine {
    registry: Arc<RegistryManager>,        // 既存アセットレジストリ
    capability_index: CapabilityIndex,      // ツール能力のベクトルインデックス
    mcp_manager: Arc<McpProcessManager>,   // 既存 MCP プロセス管理
}

impl ToolDiscoveryEngine {
    /// タスク記述から最適なツールを推薦する
    pub async fn discover_tools(
        &self,
        task_description: &str,
        budget_limit: u64,
    ) -> Result<Vec<ToolRecommendation>, AiomeError>;

    /// 推薦されたツールを自動的にスポーンする
    /// ただし、SecurityPolicy の approve_tool_spawn ゲートを通過する必要がある
    pub async fn auto_spawn(
        &self,
        recommendation: &ToolRecommendation,
    ) -> Result<McpSessionId, AiomeError>;
}
```

> [!CAUTION]
> **安全装置**: `auto_spawn` は `SecurityPolicy::approve_tool_spawn()` による事前承認を必須とする。未認証の MCP サーバーの自動起動は禁止。`CapabilityRegistry` に登録済みのツールのみが候補対象。

#### 統合ポイント
- `TaskConductor::conduct()` 内で、Job の `category` に適合するツールが不足している場合に `ToolDiscoveryEngine` を呼び出す
- `CapabilityRegistry`（既存）にベクトルインデックスを追加
- **Optional 注入**: `TaskDispatcher` へは `Option<Arc<ToolDiscoveryEngine>>` として渡し、None の場合はフォールバックを行わない（既存動作を破壊しない）

---

### ② 長期計画エンジン（P0）

#### 課題
`DreamState` はキュー空時のリアクティブなトリガー。「3日後のイベントに向けたコンテンツ準備」のようなプロアクティブな計画が立てられない。

#### 設計

```rust
/// 長期目標と日次タスクのブリッジ
pub struct StrategicPlanner {
    llm: Arc<dyn LlmProvider>,
    job_queue: Arc<dyn JobQueue>,
}

/// 目標の構造化表現
struct Goal {
    id: String,
    description: String,
    deadline: Option<DateTime<Utc>>,
    milestones: Vec<Milestone>,
    status: GoalStatus,
    parent_goal_id: Option<String>,  // 階層化対応
}

struct Milestone {
    description: String,
    target_date: DateTime<Utc>,
    jobs: Vec<String>,  // 関連 Job ID
    completed: bool,
}

enum GoalStatus {
    Active,
    Completed,
    Abandoned { reason: String },
}
```

#### 実行タイミング
- `HeartbeatWakeupService` の定期実行（既存）に組み込む
- 毎日 1 回、`StrategicPlanner::daily_review()` で目標の進捗確認 + 新規 Job 生成

> [!IMPORTANT]
> **暴走防止**: 1日あたりの自動生成 Job 数に上限を設ける（`max_auto_jobs_per_day: 10`）。これにより、計画エンジンが無限にタスクを生成し続けることを防ぐ。

> [!CAUTION]
> **コンプライアンス**: `daily_review()` が自動生成する全ての Job は、既存の `DefaultConstitutionalValidator` によるコンプライアンスチェックを必ず通過すること。また `BeggingSupervisor`（ダークパターン監視）にも接続し、スパム的コンテンツの生成を防止する。

---

### ③ 統合コンテキスト検索（P1）

#### 課題
Karma, SomaticMarker, Experience, Anamnesis が個別に存在し、横断検索ができない。

#### 設計

```rust
/// 全記憶ストアを横断する統合検索
pub struct UnifiedMemorySearch {
    vector_store: Arc<dyn VectorOps>,  // 既存
    job_queue: Arc<dyn JobQueue>,      // Karma 検索
    soul_store: Arc<dyn SoulStore>,    // SomaticMarker 検索
}

impl UnifiedMemorySearch {
    /// セマンティック検索 + キーワード検索のハイブリッド
    pub async fn recall(
        &self,
        query: &str,
        context: &RecallContext,
    ) -> Result<Vec<MemoryFragment>, AiomeError>;
}

/// 異なるストアからの記憶断片を統一的に扱う
struct MemoryFragment {
    source: MemorySource,  // Karma / Somatic / Experience / Anamnesis
    content: String,
    relevance_score: f64,
    emotional_valence: Option<f64>,  // SomaticMarker 由来
    timestamp: DateTime<Utc>,
}

enum MemorySource {
    Karma,
    SomaticMarker,
    Experience,
    Anamnesis,
}
```

#### 統合ポイント
- `SoulPipeline` の L2（Deliberative）層で、意思決定前に `UnifiedMemorySearch::recall()` を呼び出す
- 既存の `StandardVectorOps` + `fetch_relevant_karma` を内部で組み合わせる

---

### ④ 認知健全性モニター（P1）

#### 課題
`HealthMonitor` はシステムリソース監視であり、認知レベル（感情の偏り、行動のルーチン化）の健全性を監視していない。

#### 設計

```rust
/// AI の認知状態の健全性を監視する
pub struct CognitiveSentinel {
    thresholds: CognitiveThresholds,
}

struct CognitiveThresholds {
    /// SomaticMarker の valence 分散が低すぎる場合（感情硬直化）
    min_valence_variance: f64,      // デフォルト: 0.1
    /// Defense の総数が多すぎる場合（過防衛状態）
    max_active_defenses: usize,     // デフォルト: 50
    /// 同一カテゴリの Job が連続する場合（行動ルーチン化）
    max_category_streak: usize,     // デフォルト: 10
    /// SomaticMarker の総数が少なすぎる場合（経験不足）
    min_somatic_count: usize,       // デフォルト: 5
}

enum CognitiveAnomaly {
    EmotionalRigidity { variance: f64 },
    OverDefensive { count: usize },
    BehavioralRoutine { category: String, streak: usize },
    ExperienceDeficit { count: usize },
}
```

#### 実行タイミング
- `DreamState::reflective_dream()` 内で `CognitiveSentinel::diagnose()` を呼び出す
- 異常検知時は `AgentRxDiagnostics`（既存）にレポートを記録

---

### ⑦ Trajectory の拡張（P2）

#### 課題
`TrajectoryStore` のインターフェースと SQLite 永続化は既に `aiome-contracts/src/trajectory.rs` + `infrastructure/src/job_queue/trajectory_store.rs` に実装済み。ただし、因果連鎖（「なぜその行動を選んだか」）のグラフ化と API エンドポイントが欠けている。

#### 設計

既存の `TrajectoryStep` を拡張し、因果関係を表現するフィールドを追加:

```rust
/// 既存 TrajectoryStep への拡張フィールド
pub struct TrajectoryStep {
    // ... 既存フィールド ...
    pub reasoning: Option<String>,       // 「なぜこの行動を選んだか」
    pub parent_step_id: Option<String>,  // 因果の親
    pub step_category: StepCategory,     // カテゴリ分類
}

enum StepCategory {
    Hypothesis,    // 仮説生成（ADR-023 連携）
    ToolSelection, // ツール選択（②連携）
    Planning,      // 計画策定（③連携）
    Execution,     // 実行
    Review,        // 自己レビュー（ADR-023 連携）
    Decision,      // 最終判断
}
```

#### API エンドポイント
- `GET /api/v1/trajectory/:job_id` — 特定 Job の因果ツリーを JSON で返す
- Watchtower UI で Mermaid 図として可視化

---

### ⑧ ユーザー学習モデルの深化（P2）

#### 課題
`UserLearner` は `infrastructure/src/user_learner.rs` に既存だが、構造化されたユーザーモデルを持っていない。

#### 設計

既存の `UserLearner` に `UserProfile` を内部フィールドとして追加:

```rust
/// ユーザーの行動パターンの構造化モデル
struct UserProfile {
    /// 好みのスタイル傾向（Karma 分析から自動構築）
    style_preferences: HashMap<String, f32>,
    /// アクティブな時間帯の分布
    activity_pattern: [f32; 24],  // 24時間の正規化分布
    /// フィードバック傾向（厳しい/寛容）
    feedback_tendency: f32,  // -1.0 (厳格) 〜 1.0 (寛容)
    /// 関心ドメインの重み
    domain_interests: HashMap<String, f32>,
    last_updated: DateTime<Utc>,
}
```

> [!WARNING]
> **プライバシー**: `UserProfile` はローカルストレージにのみ保存し、Hub 同期・Federation には**一切含めない**。`AssetOrigin::LocalCustom` と同等の扱い。

---

## 安全装置の総括

| 要素 | ガードレール | 既存機構の活用 |
|---|---|---|
| ① MCP Discovery | `SecurityPolicy::approve_tool_spawn` + 認証済みツールのみ | `BastionGuard`, `CapabilityRegistry` |
| ② 長期計画 | `max_auto_jobs_per_day: 10` | `HeartbeatWakeupService`, `JobQueue` |
| ③ 統合検索 | 検索結果の上限（`max_recall: 20`）+ タイムアウト | `StandardVectorOps`, `fetch_relevant_karma` |
| ④ 認知監視 | 閾値のユーザーカスタマイズ可能 | `AgentRxDiagnostics`, `HealthMonitor` |
| ⑤ Trajectory | ログサイズ制限（直近 1000 ノード + 古いノードの自動圧縮） | `TrajectoryStore`, `storage_gc` |
| ⑥ ユーザー学習 | ローカルオンリー + Hub 同期禁止 + ユーザーによる削除可能 | `UserLearner`, `PathSandbox` |

## Proposed Changes

### Phase 1 (P0) — 基盤拡張

#### [NEW] `infrastructure/src/tool_discovery.rs`
- `ToolDiscoveryEngine`, `ToolRecommendation`, `CapabilityIndex`

#### [NEW] `infrastructure/src/strategic_planner.rs`
- `StrategicPlanner`, `Goal`, `Milestone`, `GoalStatus`

#### [MODIFY] [task_orchestrator.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/task_orchestrator.rs)
- `TaskDispatcher::run_dispatch_loop()` 内で `ToolDiscoveryEngine` を呼び出すフォールバック追加

---

### Phase 2 (P1) — 記憶と監視

#### [NEW] `infrastructure/src/unified_memory.rs`
- `UnifiedMemorySearch`, `MemoryFragment`, `MemorySource`, `RecallContext`

#### [NEW] `infrastructure/src/cognitive_sentinel.rs`
- `CognitiveSentinel`, `CognitiveThresholds`, `CognitiveAnomaly`

#### [MODIFY] [dream_state.rs](file:///Users/motista/Desktop/antigravity/aiome/libs/infrastructure/src/dream_state.rs)
- `reflective_dream()` 内に `CognitiveSentinel::diagnose()` 呼び出しを追加

---

### Phase 3 (P2) — 可視化と学習

#### [NEW] `infrastructure/src/trajectory_graph.rs`
- `TrajectoryNode`, `TrajectoryStepType`, `TrajectoryGraph`

#### [MODIFY] `UserLearner` (既存)
- `UserProfile` 構造体の追加と `build_profile()` メソッドの実装

#### [NEW] `api-server/src/routes/trajectory.rs`
- `GET /api/v1/trajectory/:job_id` エンドポイント

## Verification Plan

### Automated Tests

```bash
# Phase 1
cargo test --package infrastructure -- tool_discovery::tests
cargo test --package infrastructure -- strategic_planner::tests

# Phase 2
cargo test --package infrastructure -- unified_memory::tests
cargo test --package infrastructure -- cognitive_sentinel::tests

# Phase 3
cargo test --package infrastructure -- trajectory_graph::tests

# 全体の回帰テスト
cargo check --workspace --tests && cargo test --workspace
```

### Manual Verification

- Autonomous Demo を実行し、`ToolDiscoveryEngine` がツール推薦ログを出力することを確認
- `GET /api/v1/trajectory/:job_id` の JSON レスポンスが因果ツリーとして正しい構造であることを確認

## Consequences

- **既存 API への影響**: 全て新規追加。既存コードの変更は `DreamState` と `TaskDispatcher` への呼び出し追加のみで、後方互換
- **パフォーマンス**: `UnifiedMemorySearch` は既存の `StandardVectorOps` + `fetch_relevant_karma` の組み合わせ。新たな外部依存なし
- **プライバシー**: `UserProfile` はローカルオンリーで Hub 同期禁止
- **段階的導入**: Phase 1 → 2 → 3 の順で独立してデプロイ可能。各 Phase は前の Phase が完了していなくても部分的に動作する
