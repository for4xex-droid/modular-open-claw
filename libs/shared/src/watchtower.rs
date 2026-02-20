use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub memory_used_mb: u64,
    pub vram_used_mb: u64,
    pub active_job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub target: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEvent {
    Log(LogEntry),
    Heartbeat(SystemStatus),
    ApprovalRequest { transition_id: Uuid, description: String },
    TaskCompleted { job_id: String, result: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    GetStatus,
    Generate {
        category: String,
        topic: String,
        style: Option<String>,
    },
    StopGracefully,
    /// Hybrid Nuke Protocol: 即時強制終了要求
    EmergencyShutdown,
    ApprovalResponse { transition_id: Uuid, approved: bool },
    /// Samsara Phase 4: 人間からのクリエイティブ評価
    SetCreativeRating { job_id: String, rating: i32 },
}
