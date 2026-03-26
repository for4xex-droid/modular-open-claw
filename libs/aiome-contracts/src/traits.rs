/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

//! # ドメイントレイト定義
//!
//! Framework の4つのツールモジュールのインターフェースを定義する。
//! 具体実装は `libs/infrastructure` に配置する（依存性逆転の原則）。

use crate::contracts::{ConceptRequest, ConceptResponse};
use crate::error::AiomeError;
pub use crate::expression::Expression;
use crate::AgentStats;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;

/// 過去の教訓 (Karma / Fact) の基本構造
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KarmaEntry {
    pub id: String,
    pub job_id: Option<String>,
    pub karma_type: String,
    pub related_skill: String,
    pub lesson: String,
    pub weight: i32,
    pub created_at: String,
    pub soul_version_hash: Option<String>,
    pub last_applied_at: Option<String>,
    #[serde(default)]
    pub score: f64,

    // --- Phase 10-B: Swarm Sync & CRDT ---
    #[serde(default)]
    pub lamport_clock: u64,
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub signature: Option<String>,

    // --- Phase SC-MB: Clone Tracking ---
    #[serde(default)]
    pub clone_origin_id: Option<String>,

    // --- Phase Soul Engine v3 ---
    #[serde(default)]
    pub generation: Option<u32>,
    #[serde(default)]
    pub somatic_valence: Option<f64>,
}

/// コンポーネントの能力（Capability）を提供するトレイト (ADR-020)
#[async_trait]
pub trait CapabilityProvider: Send + Sync {
    /// コンポーネントの識別名 (例: "GenerativeEngine")
    fn capability_name(&self) -> &str;

    /// コンポーネントの能力の要約 (LLM が段階的発見に使用する)
    fn capability_description(&self) -> &str;

    /// 詳細な能力仕様 (JSON Schema 等、任意実装)
    fn capability_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}
use std::path::PathBuf;

/// トレンド調査ツール (TrendSonar)
///
/// X, Google Trends, 5ch 等から今バズっているテーマを取得する。
#[async_trait]
pub trait TrendSource: Send + Sync {
    /// 指定カテゴリのトレンドキーワードを取得
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError>;
}

/// トレンド情報の1件分
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TrendItem {
    /// キーワード
    pub keyword: String,
    /// ソース (例: "X", "GoogleTrends", "5ch")
    pub source: String,
    /// スコア (高いほど注目度が高い)
    pub score: f64,
}

/// 生成エンジン (旧 GenerativeEngine)
#[async_trait]
pub trait GenerativeEngine: Send + Sync {
    /// ワークフローを実行し、生成されたファイルのパスを返す
    async fn generate_artifact(
        &self,
        prompt: &str,
        workflow_id: &str,
        input_artifact: Option<&std::path::Path>,
    ) -> Result<crate::contracts::ArtifactResponse, AiomeError>;

    /// 接続状態を確認
    async fn health_check(&self) -> Result<bool, AiomeError>;
}

/// メディアプロセッサー (旧 MediaForge)
#[allow(clippy::ptr_arg)]
#[async_trait]
pub trait MediaProcessor: Send + Sync {
    /// 複数のアセットを合成して最終出力を生成
    async fn combine_assets(
        &self,
        input: &PathBuf,
        context: &PathBuf,
        metadata: Option<&PathBuf>,
        force_style: Option<String>,
    ) -> Result<PathBuf, AiomeError>;

    /// メディアを標準化 (旧 standardize_format)
    async fn standardize_media(&self, input: &PathBuf) -> Result<PathBuf, AiomeError>;

    /// 複数のメディアブロックを 1つのファイルに結合
    async fn concatenate_media(
        &self,
        blocks: Vec<String>,
        output_name: String,
    ) -> Result<String, AiomeError>;

    /// メディアファイルの尺長（秒）を取得する
    async fn get_duration(&self, path: &std::path::Path) -> Result<f32, AiomeError>;
}

/// 音声文字起こし結果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionResult {
    /// 全体のテキスト
    pub text: String,
    /// 検出された言語 (ISO 639-1)
    pub language: String,
    /// 単語レベル/セグメントレベルのタイムスタンプ情報
    pub segments: Vec<TranscriptionSegment>,
}

/// 文字起こしのセグメント情報
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionSegment {
    /// テキスト内容
    pub text: String,
    /// 開始時間 (秒)
    pub start: f32,
    /// 終了時間 (秒)
    pub end: f32,
    /// 信頼度 (0.0 - 1.0)
    pub confidence: f32,
}

/// 音声文字起こしエンジン (STT)
#[async_trait]
pub trait TranscriptionEngine: Send + Sync {
    /// 音声ファイルを文字起こしする
    async fn transcribe(
        &self,
        audio_path: &std::path::Path,
    ) -> Result<TranscriptionResult, AiomeError>;

    /// エンジンの健全性（ランタイム/GPU）を確認
    async fn health_check(&self) -> Result<bool, AiomeError>;
}

// --- Phase 10: The Automaton ---

/// ジョブステータス
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum JobStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Quarantined,
    Archived,
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            JobStatus::Pending => "Pending",
            JobStatus::InProgress => "InProgress",
            JobStatus::Completed => "Completed",
            JobStatus::Failed => "Failed",
            JobStatus::Cancelled => "Cancelled",
            JobStatus::Quarantined => "Quarantined",
            JobStatus::Archived => "Archived",
        }
    }

    pub fn from_string(s: impl AsRef<str>) -> Self {
        match s.as_ref() {
            "InProgress" | "Processing" => JobStatus::InProgress,
            "Completed" => JobStatus::Completed,
            "Failed" => JobStatus::Failed,
            "Cancelled" => JobStatus::Cancelled,
            "Quarantined" => JobStatus::Quarantined,
            "Archived" => JobStatus::Archived,
            _ => JobStatus::Pending,
        }
    }
}

///背景データ
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextEntry {
    pub role: String,
    pub content: String,
}

/// ジョブの基本構造
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Job {
    pub id: String,
    pub category: String,
    pub topic: String,
    pub style: String,
    pub status: JobStatus,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub last_heartbeat: Option<String>,
    pub tech_karma_extracted: bool,
    pub creative_rating: Option<i32>,
    pub execution_log: Option<String>,
    pub error_message: Option<String>,
    pub sns_platform: Option<String>,
    pub sns_content_id: Option<String>,
    pub published_at: Option<String>,
    pub output_artifacts: Option<String>,
    pub karma_directives: Option<String>,
    pub permission_manifest: Option<crate::security::PermissionManifest>,
    pub agent_id: Option<uuid::Uuid>,
}

/// ジョブのステータス更新リクエスト
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateJobStatusRequest {
    pub status: JobStatus,
    pub error_message: Option<String>,
    pub execution_log: Option<String>,
}

/// カルマ検索結果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KarmaSearchResult {
    pub entries: Vec<KarmaEntry>,
    pub is_ood: bool,
    pub max_score: f64,
}

impl KarmaSearchResult {
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            is_ood: false,
            max_score: 0.0,
        }
    }
}

/// SNSメトリクスの記録
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnsMetricsRecord {
    pub id: i64,
    pub job_id: String,
    pub milestone_days: i64,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
    pub raw_comments_json: Option<String>,
    pub hard_metric_score: Option<f64>,
    pub engagement_rate: Option<f64>,
}

// --- Domain-Specific Traits (Refactored from JobQueue God Trait) ---

/// 1. ジョブ管理 (TaskRegistry)
#[async_trait]
pub trait TaskRegistry: Send + Sync + std::fmt::Debug {
    async fn enqueue(
        &self,
        category: &str,
        topic: &str,
        style: &str,
        karma_directives: Option<&str>,
        permission_manifest: Option<crate::security::PermissionManifest>,
        agent_id: Option<uuid::Uuid>,
        priority: i32,
    ) -> Result<String, AiomeError>;

    async fn dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError>;
    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError>;
    async fn complete_job(&self, job_id: &str, output_artifacts: Option<&str>) -> Result<(), AiomeError>;
    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError>;
    async fn cancel_job(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError>;
    async fn get_pending_job_count(&self) -> Result<i64, AiomeError>;
    async fn get_job_count_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<i64, AiomeError>;
    async fn fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
    async fn fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
    async fn fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError>;
    async fn reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError>;
    async fn purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError>;
    async fn heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError>;
}

/// 2. 監査・実行軌跡 (AuditStore)
#[async_trait]
pub trait AuditStore: Send + Sync + std::fmt::Debug {
    async fn store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError>;
    async fn store_trajectory_step(&self, step: crate::trajectory::TrajectoryStep) -> Result<(), AiomeError>;
    async fn fetch_trajectory_steps(&self, job_id: &str) -> Result<Vec<crate::trajectory::TrajectoryStep>, AiomeError>;
    async fn get_security_request_count(&self, agent_id: Option<uuid::Uuid>) -> Result<u32, AiomeError>;
    async fn increment_security_request_count(&self, agent_id: Option<uuid::Uuid>) -> Result<u32, AiomeError>;
}

/// 3. 対話履歴・短期記憶 (ChatStore)
#[async_trait]
pub trait ChatStore: Send + Sync + std::fmt::Debug {
    async fn fetch_chat_history(&self, channel_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn store_chat_message(&self, channel_id: &str, role: &str, content: &str) -> Result<(), AiomeError>;
    async fn get_chat_memory_summary(&self, channel_id: &str) -> Result<Option<(String, Option<String>)>, AiomeError>;
    async fn update_chat_memory_summary(&self, channel_id: &str, summary: &str, last_interaction_id: Option<&str>) -> Result<(), AiomeError>;
    async fn mark_chats_as_distilled(&self, channel_id: &str, last_id: i64) -> Result<(), AiomeError>;
}

/// 4. 教育・教訓・長期記憶 (KarmaRegistry)
#[async_trait]
pub trait KarmaRegistry: Send + Sync + std::fmt::Debug {
    async fn fetch_relevant_karma(&self, topic: &str, skill_id: &str, limit: i64, current_soul_hash: &str) -> Result<KarmaSearchResult, AiomeError>;
    #[allow(clippy::too_many_arguments)]
    async fn store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, karma_type: &str, soul_hash: &str, domain: Option<&str>, subtopic: Option<&str>, clone_origin_id: Option<&str>) -> Result<(), AiomeError>;
    async fn adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError>;
    async fn karma_decay_sweep(&self) -> Result<u64, AiomeError>;
    async fn fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
    async fn mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn fetch_unincorporated_karma(&self, limit: i64, current_soul_hash: &str) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn mark_karma_as_incorporated(&self, karma_ids: Vec<String>, new_soul_hash: &str) -> Result<(), AiomeError>;
    async fn fetch_relevant_karma_by_category(&self, topic: &str, category: &str, limit: i64) -> Result<KarmaSearchResult, AiomeError>;
}

/// 5. エージェント進化・統計 (AgentEvolver)
#[async_trait]
pub trait AgentEvolver: Send + Sync + std::fmt::Debug {
    async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError>;
    async fn add_resonance(&self, amount: i32) -> Result<(), AiomeError>;
    async fn add_tech_exp(&self, amount: i32) -> Result<(), AiomeError>;
    async fn add_creativity(&self, amount: i32) -> Result<(), AiomeError>;
    async fn sync_samsara_level(&self) -> Result<Option<crate::contracts::SamsaraEvent>, AiomeError>;
    async fn record_evolution_event(&self, level: i32, event_type: &str, description: &str, inspiration: Option<&str>, karma_json: Option<&str>) -> Result<(), AiomeError>;
    async fn fetch_evolution_history(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn record_soul_mutation(&self, old_hash: &str, new_hash: &str, reason: &str) -> Result<(), AiomeError>;
}

/// 6. 適応型免疫システム (ImmuneSystemOps)
#[async_trait]
pub trait ImmuneSystemOps: Send + Sync + std::fmt::Debug {
    async fn store_immune_rule(&self, rule: &crate::contracts::ImmuneRule) -> Result<(), AiomeError>;
    async fn delete_immune_rule(&self, rule_id: &str) -> Result<(), AiomeError>;
    async fn fetch_active_immune_rules(&self) -> Result<Vec<crate::contracts::ImmuneRule>, AiomeError>;
    async fn record_arena_match(&self, match_data: &crate::contracts::ArenaMatch) -> Result<(), AiomeError>;
    async fn get_immune_rules(&self) -> Result<Vec<crate::contracts::ImmuneRule>, AiomeError>;
}

/// 7. 教訓連携 (FederationOps)
#[async_trait]
pub trait FederationRegistry: Send + Sync + std::fmt::Debug {
    async fn export_federated_data(&self, since: Option<&str>) -> Result<(Vec<KarmaEntry>, Vec<crate::contracts::ImmuneRule>, Vec<crate::contracts::ArenaMatch>), AiomeError>;
    async fn import_federated_data(&self, karmas: Vec<KarmaEntry>, rules: Vec<crate::contracts::ImmuneRule>, matches: Vec<crate::contracts::ArenaMatch>) -> Result<(), AiomeError>;
    async fn get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError>;
    async fn update_peer_sync_time(&self, peer_url: &str, sync_time: &str) -> Result<(), AiomeError>;
    async fn get_node_id(&self) -> Result<String, AiomeError>;
    async fn fetch_unfederated_data(&self) -> Result<(Vec<KarmaEntry>, Vec<crate::contracts::ImmuneRule>), AiomeError>;
    async fn mark_as_federated(&self, karma_ids: Vec<String>, rule_ids: Vec<String>) -> Result<(), AiomeError>;
    async fn fetch_federated_metrics(&self) -> Result<crate::contracts::FederatedMetrics, AiomeError>;
}

/// 8. バイオーム (BiomeRegistry)
#[async_trait]
pub trait BiomeRegistry: Send + Sync + std::fmt::Debug {
    async fn get_biome_topic_status(&self, topic_id: &str) -> Result<Option<(i32, Option<String>)>, AiomeError>;
    async fn advance_biome_turn(&self, topic_id: &str, cooldown_minutes: i64) -> Result<i32, AiomeError>;
    async fn fetch_biome_messages(&self, topic_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn store_biome_message(&self, message: &crate::biome::BiomeMessage) -> Result<(), AiomeError>;
    async fn update_biome_reputation(&self, pubkey: &str, delta: f64) -> Result<f64, AiomeError>;
    async fn archive_biome_topic(&self, topic_id: &str) -> Result<(), AiomeError>;
}

/// 統合ジョブキュー (Composite Trait)
#[async_trait]
pub trait JobQueue: 
    TaskRegistry + 
    AuditStore + 
    ChatStore + 
    KarmaRegistry + 
    AgentEvolver + 
    ImmuneSystemOps + 
    FederationRegistry + 
    BiomeRegistry + 
    SoulStore
{
    // --- Phase 10-B: Swarm Sync & CRDT ---
    async fn sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError>;
    async fn sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError>;
    async fn tick_local_clock(&self) -> Result<u64, AiomeError>;

    // --- GC & Storage ---
    async fn storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError>;

    // --- Project NURTURE compliance ---
    async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError>;

    // --- Expression ---
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError>;
    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError>;
}

/// Soul Storage Trait
#[async_trait]
pub trait SoulStore: Send + Sync {
    async fn load_soul(&self, id: &str) -> Result<Option<serde_json::Value>, AiomeError>;
    async fn store_soul_fragment(
        &self,
        fragment_yaml: &str,
        version_hash: &str,
    ) -> Result<(), AiomeError>;
    async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError>;
}

// --- AI Artifacts Storage System ---

/// AI生成物のカテゴリ
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactCategory {
    Report,
    Code,
    Image,
    Audio,
    Expression,
    Data,
    Knowledge,
}

/// 個別ファイルのメタデータ
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactFile {
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub hash: String,
}

/// アーティファクトの繋がり（Provenance DAG/血統）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub source_type: String, // "Artifact" or "Karma"
    pub relation: String,    // "DerivedFrom", "AssociatedWith"
    pub metadata: serde_json::Value,
    pub created_at: String,
}

/// アーティファクトの繋がり入力
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactEdgeInput {
    pub target_id: String,
    pub source_type: String,
    pub relation: String,
    pub metadata: Option<serde_json::Value>,
}

/// アーティファクトのメタデータ
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: String,
    pub title: String,
    pub category: ArtifactCategory,
    pub tags: Vec<String>,
    pub created_by: String,
    #[serde(skip_serializing)]
    pub dir_path: String,
    pub files: Vec<ArtifactFile>,
    pub karma_refs: Vec<String>,
    pub job_ref: Option<String>,
    pub soul_version_hash: Option<String>,
    pub signature: Option<String>,
    pub text_content: Option<String>,
    pub edges: Vec<ArtifactEdge>,
    pub created_at: String,
}

/// アーティファクト保存リクエスト
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateArtifactRequest {
    pub title: String,
    pub category: ArtifactCategory,
    pub tags: Vec<String>,
    pub created_by: String,
    pub files: Vec<(String, Vec<u8>, String)>, // (filename, content, mime_type)
    pub karma_refs: Vec<String>,
    pub text_content: Option<String>,
    pub job_ref: Option<String>,
    pub parent_refs: Vec<ArtifactEdgeInput>,
    #[serde(default)]
    pub is_protected: bool, // Phase 3: DRM 隔離フラグ
}

/// アーティファクト・ストレージ・トレイト
#[async_trait]
pub trait ArtifactStore: Send + Sync {
    /// 新しい成果物を保存する
    async fn save_artifact(
        &self,
        req: CreateArtifactRequest,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<String, AiomeError>;

    /// 成果物の一覧を取得する
    async fn list_artifacts(
        &self,
        category: Option<ArtifactCategory>,
        limit: i64,
    ) -> Result<Vec<ArtifactMeta>, AiomeError>;

    /// 特定の成果物の詳細（メタデータ）を取得する
    async fn fetch_artifact(&self, id: &str) -> Result<Option<ArtifactMeta>, AiomeError>;

    /// ファイル実体を読み込む
    async fn read_artifact_file(
        &self,
        id: &str,
        filename: &str,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<Vec<u8>, AiomeError>;

    /// 成果物を削除する（物理削除）
    async fn delete_artifact(
        &self,
        id: &str,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<(), AiomeError>;

    // --- Phase 1/2: Memory Crystal (Evolution) ---

    /// 指定した成果物に関連するコネクション（血統）を取得する
    async fn get_artifact_edges(&self, id: &str) -> Result<Vec<ArtifactEdge>, AiomeError>;

    /// 成果物の間に新しいコネクションを追加する
    async fn add_artifact_edge(&self, edge: ArtifactEdge) -> Result<(), AiomeError>;

    /// セマンティック検索（自然言語検索）を行う
    async fn search_artifacts_semantic(
        &self,
        query: &str,
        category: Option<ArtifactCategory>,
        limit: i64,
    ) -> Result<Vec<ArtifactMeta>, AiomeError>;
}

/// プロンプト抽出ツール (MetaCognitive)
#[async_trait]
pub trait PromptExtractor: Send + Sync {
    /// 実行ログから教訓を抽出する
    async fn extract_verdict(
        &self,
        log: &str,
    ) -> Result<crate::contracts::OracleVerdict, AiomeError>;

    /// 業（Karma）を分類する
    async fn classify_karma(
        &self,
        lesson: &str,
    ) -> Result<crate::contracts::KarmaClassification, AiomeError>;
}

/// パブリッシャー・トレイト (Publisher)
#[async_trait]
pub trait Publisher: Send + Sync {
    /// コンテンツを特定のプラットフォームに配信する
    async fn publish(
        &self,
        content: &str,
        media_paths: &[std::path::PathBuf],
        metadata: &serde_json::Value,
    ) -> Result<String, AiomeError>;

    /// プラットフォーム名を取得
    fn platform_name(&self) -> &str;
}

// LlmProvider is now defined in aiome_contracts::llm (ADR-021)

/// 憲法バリデーター (ConstitutionalValidator)
#[async_trait]
pub trait ConstitutionalValidator: Send + Sync {
    async fn verify_constitutional(&self, output: &str, soul_md: &str) -> Result<(), AiomeError>;
}

/// ログ出力インターフェース (AiomeLogger)
#[async_trait]
pub trait AiomeLogger: Send + Sync {
    async fn log_success(
        &self,
        artifact_id: &str,
        output_path: &std::path::Path,
    ) -> Result<(), AiomeError>;
    async fn log_error(&self, reason: &str) -> Result<(), AiomeError>;
    async fn daily_summary(&self, jail: &bastion::fs_guard::Jail) -> Result<String, AiomeError>;
}

/// エージェントのアクションインターフェース (AgentAct)
#[async_trait]
pub trait AgentAct: Send + Sync {
    type Input;
    type Output;

    async fn execute(
        &self,
        input: Self::Input,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, AiomeError>;
}

/// 自律的ツール発見エンジン (Phase 13: Autonomous Tool Discovery)
///
/// 実行時に利用可能なスキルや API を動的に探索し、タスクの実行に最適なツールを提案する。
#[async_trait]
pub trait ToolDiscoveryEngine: Send + Sync {
    /// 利用可能なツールのメタデータを取得する
    async fn discover_tools(&self) -> Result<Vec<serde_json::Value>, AiomeError>;
    /// 指示に基づいて最適なツールのセットを提案する
    async fn suggest_tools(&self, instruction: &str) -> Result<Vec<String>, AiomeError>;
}

/// 戦略的計画エンジン (Phase 13: Strategic Planning)
///
/// 抽象的な Goal を具体的な TrajectoryStep のリスト（ツリー）に分解し、
/// エージェントの長期的な行動計画を策定する。
#[async_trait]
pub trait StrategicPlanner: Send + Sync {
    /// Goal を実行可能なステップに分解する
    async fn plan_goal(
        &self,
        goal: &str,
        context: serde_json::Value,
    ) -> Result<Vec<crate::trajectory::TrajectoryStep>, AiomeError>;
}

/// 9. LoRA 生成エンジン
#[async_trait]
pub trait LoraEngine: Send + Sync + std::fmt::Debug {
    /// 特定の LoRA アセットを使用してテキストを生成（モックまたは特定実装）
    async fn complete_with_lora(
        &self,
        prompt: &str,
        lora_id: &str,
    ) -> Result<crate::llm::LlmResponse, AiomeError>;
    async fn health_check(&self) -> Result<bool, AiomeError>;
}

/// 10. TTS (Text-to-Speech) プロバイダー
#[async_trait]
pub trait TtsProvider: Send + Sync + std::fmt::Debug {
    /// テキストを音声データに変換
    async fn synthesize(&self, text: &str, voice_id: &str) -> Result<Vec<u8>, AiomeError>;
    async fn health_check(&self) -> Result<bool, AiomeError>;
}

/// 11. Live セッションマネージャー (Gemini 3.1 Flash Live)
#[async_trait]
pub trait LiveSessionManager: Send + Sync + std::fmt::Debug {
    /// 新しいストリーミングセッションを開始
    async fn create_session(
        &self,
        config: crate::live_types::ThinkingLevel,
    ) -> Result<String, AiomeError>;
    /// セッションを終了
    async fn close_session(&self, session_id: &str) -> Result<(), AiomeError>;
    /// 音声データを送信 (GAP-4)
    async fn send_audio(&self, session_id: &str, pcm_data: &[u8]) -> Result<(), AiomeError>;
    /// テキストを送信 (GAP-4)
    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), AiomeError>;
    /// イベントを受信 (GAP-4)
    async fn receive_events(&self, session_id: &str) -> Result<Vec<crate::live_types::LiveEvent>, AiomeError>;
}

/// 12. ニュース・情報サービス
#[async_trait]
pub trait NewsService: Send + Sync + std::fmt::Debug {
    /// 最新のトレンドやニュースを収集
    async fn fetch_latest(&self, query: &str) -> Result<Vec<serde_json::Value>, AiomeError>;
}
