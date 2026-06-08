/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

// サブモジュールの宣言
pub mod bastion_guard;
pub mod config;
pub mod exec_policy;
pub mod voice_core_drm;

pub mod abyss_voice_vault;
pub mod behavior_monitor;
pub mod crypto;
pub mod hook_manager;
pub mod loop_detector;
pub mod mlock;
pub mod secret_redactor;
pub mod sqlite_vault_backend;
pub mod tool_call_reviewer;

// 再エクスポート (後方互換性のため)
pub use bastion_guard::{BastionGuard, SafeCommandBuilder};
pub use config::{SecurityConfig, GLOBAL_SECURITY_CONFIG, PROCESS_SAFE_ENV_VARS};
pub use exec_policy::ExecPolicy;
pub use voice_core_drm::VoiceCoreDrm;

pub use abyss_voice_vault::AbyssVoiceVault;
pub use loop_detector::LoopDetectorHook;
pub use tool_call_reviewer::ToolCallReviewerHook;

// aiome_core からの再エクスポート
pub use aiome_core::security::{PermissionManifest, RuntimeJail, SandboxProfile};

#[cfg(test)]
mod tests;
