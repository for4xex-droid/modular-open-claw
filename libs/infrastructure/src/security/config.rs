/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// `SecurityConfig` 構造体
pub struct SecurityConfig {
    /// allowed_binaries
    pub allowed_binaries: Vec<String>,
    /// workspace_root
    pub workspace_root: PathBuf,
    /// vault_path (Phase 3: DRM 隔離領域)
    pub vault_path: Option<PathBuf>,
    /// use_runsc_sandbox (F-01)
    #[serde(default = "default_true")]
    pub use_runsc_sandbox: bool,
    /// enable_syscall_audit (Phase 0)
    #[serde(default)]
    pub enable_syscall_audit: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_binaries: vec![
                "ls".to_string(),
                "cat".to_string(),
                "cargo".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "wc".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "git".to_string(),
                "rustc".to_string(),
                "node".to_string(),
                "npm".to_string(),
                "python3".to_string(),
                "mkdir".to_string(),
                "cp".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "diff".to_string(),
                "tree".to_string(),
                "which".to_string(),
                "sandbox-exec".to_string(),
                "ollama".to_string(),
                "docker".to_string(),
                "podman".to_string(),
                "slm".to_string(),
                "npx".to_string(),
                "uvx".to_string(),
                "obscura".to_string(),
            ],
            workspace_root: shared::app_data::AppDataResolver::new()
                .map(|r| r.root().to_path_buf())
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to initialize AppDataResolver in SecurityConfig default: {}. Falling back to '.'", e);
                    PathBuf::from(".")
                }),
            vault_path: None,
            use_runsc_sandbox: true,
            enable_syscall_audit: false,
        }
    }
}

impl SecurityConfig {
    /// `load_or_default` を実行する
    pub fn load_or_default() -> Self {
        let workspace = std::env::var("WORKSPACE_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                shared::app_data::AppDataResolver::new()
                    .map(|r| r.root().to_string_lossy().to_string())
                    .unwrap_or_else(|e| {
                        tracing::error!("⚠️ AppDataResolver failed in load_or_default: {}", e);
                        // allow-anti-pattern: WORKSPACE_DIR未設定時のフォールバック
                        ".".to_string()
                    })
            });
        let workspace_root = PathBuf::from(&workspace);
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let path = PathBuf::from(home).join(".aiome/config/security.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut config) = serde_json::from_str::<SecurityConfig>(&content) {
                    info!(
                        "🛡️ [SecurityConfig] Loaded whitelist from ~/.aiome/config/security.json."
                    );
                    config.workspace_root = workspace_root;
                    // AiomeConfig との整合性を取るため、環境変数からも取得を試みる
                    if config.vault_path.is_none() {
                        let vault = std::env::var("VAULT_PATH")
                            .unwrap_or_else(|_| "~/.aiome/vault".to_string());
                        let expanded = shared::os_utils::expand_home(&vault);
                        let path = expanded;
                        // SEC-CANONICAL: 絶対パスへの正規化とシンボリックリンク解消を強制 (G-21/G-26)
                        config.vault_path = path.canonicalize().ok().or_else(|| {
                            // canonicalize が失敗した場合は絶対パス化を試みる
                            std::env::current_dir().ok().map(|curr| curr.join(path))
                        });
                    }
                    return config;
                }
            }
        }
        let mut config = Self::default();
        config.workspace_root = workspace_root;
        let vault = std::env::var("VAULT_PATH").unwrap_or_else(|_| "~/.aiome/vault".to_string());
        let expanded = shared::os_utils::expand_home(&vault);
        let path = expanded;
        // SEC-CANONICAL: デフォルト設定時も正規化を強制
        config.vault_path = path.canonicalize().ok().or_else(|| {
            // canonicalize が失敗した（ディレクトリがない等）場合は、
            // 少なくとも絶対パス化して .. を解決する。
            std::env::current_dir().ok().map(|curr| curr.join(path))
        });
        config
    }
}

/// プロセス起動時に一度だけ初期化されるグローバルセキュリティ設定
pub static GLOBAL_SECURITY_CONFIG: once_cell::sync::Lazy<SecurityConfig> =
    once_cell::sync::Lazy::new(SecurityConfig::load_or_default);

/// Defense-in-depth: env_clear() 後に再注入する必須環境変数の SSOT ホワイトリスト。
pub use shared::security::PROCESS_SAFE_ENV_VARS;
