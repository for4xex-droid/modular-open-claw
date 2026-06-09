use serde::{Deserialize, Serialize};

/// ヒッグス粒子によって固定された形質のスナップショット
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrozenTraitSnapshot {
    /// 固定された形質のインデックス（CellGenome の 32 次元中のどれか）
    pub trait_index: u32,
    /// 固定された値
    pub frozen_value: f64,
    /// 対応する SomaticMarker の ID（レイヤー 1 との紐付け）
    pub somatic_marker_id: String,
    /// ヒッグス粒子が適用された世代
    pub frozen_at_generation: u32,
    /// 固定時のタイムスタンプ
    pub created_at: String,
}
