/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::types::{AgentStats, LogEntry, SystemStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEvent {
    Log(LogEntry),
    Heartbeat(SystemStatus),
    ApprovalRequest {
        transition_id: Uuid,
        description: String,
    },
    TaskCompleted {
        job_id: String,
        result: String,
        topic: String,
        style: String,
        preview_url: Option<String>,
    },
    /// コアからの対話応答 (音声付き)
    ChatResponse {
        response: String,
        channel_id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource_path: Option<String>,
    },
    /// 自律的な話しかけ（プッシュ通知）
    ProactiveTalk {
        message: String,
        channel_id: u64,
    },
    /// 育成ステータスの応答
    AgentStatsResponse(AgentStats),

    /// Phase A-0: プラグイン固有イベント
    PluginEvent {
        plugin_name: String,
        event_type: String,
        payload: serde_json::Value,
    },
    /// Phase 43: cmux Task Dispatcher 連携イベント
    TaskProgress {
        job_id: String,
        conductor_id: String,
        message: String,
        percent: Option<u8>,
    },
    TaskEvaluating {
        job_id: String,
    },
    TaskFailed {
        job_id: String,
        error: String,
    },
    /// Phase 3B: Task cancellation support
    TaskCancelled {
        job_id: String,
    },
    /// Phase 1-2: セキュリティ違反によるユーザー介入待機
    TaskAwaitingInput {
        job_id: String,
        reason: String,
    },
    /// Phase 51: Autonomous GIG Publishing
    GigPublished {
        job_id: String,
        intent_id: String,
        description: String,
        budget: u64,
    },
    /// Phase 6: Live API からの音声チャンク
    LiveAudioChunk {
        session_id: String,
        data: Vec<u8>,
    },
    /// Phase 6: Live API からのリアルタイム文字起こし
    LiveTranscript {
        session_id: String,
        text: String,
        is_final: bool,
    },
    /// Phase 6: Live API ターン完了
    LiveTurnEnd {
        session_id: String,
    },
    /// Phase 15+: SoT 審議プログレス
    SoTProgress {
        event: crate::contracts::SoTEvent,
    },
    /// GEO Optimizer Quality Gate result
    QualityGate {
        job_id: String,
        score: u32,
        passed: bool,
        conductor: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    GetStatus,
    /// 育成ステータス取得
    GetAgentStats,
    /// Aiomeとの対話 (一般チャット)
    Chat {
        message: String,
        channel_id: u64,
    },
    /// システム操作用の対話 (コマンドチャネル)
    CommandChat {
        message: String,
        channel_id: u64,
    },
    Generate {
        category: String,
        topic: String,
        style: Option<String>,
    },
    StopGracefully,
    /// Hybrid Nuke Protocol: 即時強制終了要求
    EmergencyShutdown,
    ApprovalResponse {
        transition_id: Uuid,
        approved: bool,
    },
    /// Samsara Phase 4: 人間からのクリエイティブ評価
    SetCreativeRating {
        job_id: String,
        rating: i32,
    },
    /// Phase 11: The Anchor Link (SNS動画IDの紐付け)
    LinkSns {
        job_id: String,
        platform: String,
        content_id: String,
    },
}
