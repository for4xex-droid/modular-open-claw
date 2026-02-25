//! # Resource Arbiter — 資源調停官
//! 
//! Mac mini M4 Pro の VRAM 資源を管理し、複数の重負荷アクター（LLM, TTS, ImageGen）
//! が同時に実行されるのを防ぐ「単一占有（Single-Tenant）」ポリシーを強制する。
//! 加えて、FFmpeg による動画合成（Forge）の並列実行も制御する。

use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};
use tracing::info;

/// 資源のカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceCategory {
    GPU,   // TTS, ComfyUI (排他、同時1)
    Forge, // FFmpeg (並列、同時2-3)
}

/// 資源の占有者
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUser {
    Voicing,    // TTS
    Generating, // ComfyUI
    Forging,    // FFmpeg
}

impl std::fmt::Display for ResourceUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceUser::Voicing => write!(f, "Voicing (TTS)"),
            ResourceUser::Generating => write!(f, "Generating (Video)"),
            ResourceUser::Forging => write!(f, "Forging (FFmpeg)"),
        }
    }
}

/// 資源調停官
#[derive(Clone)]
pub struct ResourceArbiter {
    gpu_sem: Arc<Semaphore>,
    forge_sem: Arc<Semaphore>,
}

impl ResourceArbiter {
    pub fn new() -> Self {
        Self {
            // GPUは完全に並列不可 (Apple Silicon MPS競合回避)
            gpu_sem: Arc::new(Semaphore::new(1)),
            // Forge (FFmpeg) はCPU/メモリに余裕があれば並列可能
            forge_sem: Arc::new(Semaphore::new(2)),
        }
    }

    /// GPU資源を要求する。既に占有されている場合は待機する。
    pub async fn acquire_gpu(&self, user: ResourceUser) -> Result<ArbiterGuard<'_>, tokio::sync::AcquireError> {
        info!("⏳ ResourceArbiter: Requesting GPU access for {}...", user);
        let permit = self.gpu_sem.acquire().await?;
        info!("🔑 ResourceArbiter: GPU access GRANTED for {}", user);
        Ok(ArbiterGuard { _permit: permit, category: ResourceCategory::GPU, user })
    }

    /// Forge (FFmpeg) 資源を要求する。
    pub async fn acquire_forge(&self, user: ResourceUser) -> Result<ArbiterGuard<'_>, tokio::sync::AcquireError> {
        info!("⏳ ResourceArbiter: Requesting Forge slot for {}...", user);
        let permit = self.forge_sem.acquire().await?;
        info!("🔑 ResourceArbiter: Forge slot GRANTED for {}", user);
        Ok(ArbiterGuard { _permit: permit, category: ResourceCategory::Forge, user })
    }
}

/// 資源の占有を解除するためのガード
pub struct ArbiterGuard<'a> {
    _permit: SemaphorePermit<'a>,
    category: ResourceCategory,
    user: ResourceUser,
}

impl<'a> Drop for ArbiterGuard<'a> {
    fn drop(&mut self) {
        info!("🔓 ResourceArbiter: {:?} Access RELEASED for {}", self.category, self.user);
    }
}
