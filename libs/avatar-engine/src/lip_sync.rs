use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Viseme {
    Closed,
    AA,
    IH,
    OU,
    EE,
    OH,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LipSyncFrame {
    pub timestamp_ms: u64,
    pub mouth_open: f32,
    pub viseme: Viseme,
}
