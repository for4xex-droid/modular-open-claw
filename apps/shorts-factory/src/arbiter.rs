//! # Resource Arbiter — 資源調停官
//! 
//! Mac mini M4 Pro の VRAM 資源を管理し、複数の重負荷アクター（LLM, TTS, ImageGen）
//! が同時に実行されるのを防ぐ「単一占有（Single-Tenant）」ポリシーを強制する。

use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::info;

/// 資源の占有者
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUser {
    #[allow(dead_code)]
    Scripting, // LLM (Ollama)
    Voicing,   // TTS (Style-Bert-VITS2)
    Generating, // Image/Video (ComfyUI)
}

impl std::fmt::Display for ResourceUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceUser::Scripting => write!(f, "Scripting (LLM)"),
            ResourceUser::Voicing => write!(f, "Voicing (TTS)"),
            ResourceUser::Generating => write!(f, "Generating (Video)"),
        }
    }
}

/// 資源調停官
#[derive(Clone)]
pub struct ResourceArbiter {
    lock: Arc<Mutex<Option<ResourceUser>>>,
}

impl ResourceArbiter {
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(None)),
        }
    }

    /// 資源を要求する。既に占有されている場合は待機する。
    pub async fn acquire(&self, user: ResourceUser) -> ArbiterGuard {
        info!("⏳ ResourceArbiter: Requesting access for {}", user);
        
        let mut guard = self.lock.clone().lock_owned().await;
        *guard = Some(user);
        
        info!("🔑 ResourceArbiter: Access GRANTED for {}", user);
        ArbiterGuard { guard, user }
    }
}

/// 資源の占有を解除するためのガード
pub struct ArbiterGuard {
    guard: OwnedMutexGuard<Option<ResourceUser>>,
    user: ResourceUser,
}

impl Drop for ArbiterGuard {
    fn drop(&mut self) {
        info!("🔓 ResourceArbiter: Access RELEASED for {}", self.user);
        *self.guard = None;
    }
}
