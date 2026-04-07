/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::boundary_verifier::BoundaryVerifier;
use crate::registry::RegistryManager;
use aiome_core::error::AiomeError;
pub use aiome_core::security::{PermissionManifest, RuntimeJail, SandboxProfile};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// `SecurityConfig` 構造体
pub struct SecurityConfig {
    /// allowed_binaries
    pub allowed_binaries: Vec<String>,
    /// workspace_root
    pub workspace_root: std::path::PathBuf,
    /// vault_path (Phase 3: DRM 隔離領域)
    pub vault_path: Option<std::path::PathBuf>,
    /// use_runsc_sandbox (F-01)
    #[serde(default = "default_true")]
    pub use_runsc_sandbox: bool,
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
                "slm".to_string(),
            ],
            workspace_root: shared::app_data::AppDataResolver::new()
                .root()
                .to_path_buf(),
            vault_path: None,
            use_runsc_sandbox: true,
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
                    .root()
                    .to_string_lossy()
                    .to_string()
            });
        let workspace_root = std::path::PathBuf::from(&workspace);
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let path = std::path::PathBuf::from(home).join(".aiome/config/security.json");
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

/// Phase 2: Runtime Enforcement (The Bastion Guard)
///
/// エージェントが実行しようとする「アクション」を監視し、
/// 権限マニフェストおよびOSレベルの制限（seccomp等）と照合する。
pub struct BastionGuard {
    manifest: PermissionManifest,
    /// システム内部のバイパスフラグ (G-26)
    pub is_system_internal: bool,
    boundary_verifier: BoundaryVerifier,
}

#[async_trait]
impl RuntimeJail for BastionGuard {
    /// シェルコマンドの実行を検証し、許可されていれば実行する
    async fn safe_exec(&self, cmd_str: &str) -> Result<String, AiomeError> {
        self.safe_exec_with_profile(cmd_str, SandboxProfile::Default)
            .await
    }

    /// プロファイルを指定してシェルコマンドを実行する
    async fn safe_exec_with_profile(
        &self,
        cmd_str: &str,
        profile: SandboxProfile,
    ) -> Result<String, AiomeError> {
        // Phase 47: Boundary Tautology Check (O(1), no LLM)
        let _verified = self
            .boundary_verifier
            .verify_command(cmd_str, self.is_system_internal)?;

        info!(
            "🛡️ [BastionGuard] 検証中 (Profile: {:?}): {}",
            profile, cmd_str
        );

        // 1. マニフェスト・チェック
        if !self.is_system_internal && !self.manifest.allow_shell_execution {
            error!("🚨 [SECURITY VIOLATION] Shell execution is disabled.");
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Forbidden.".to_string(),
            });
        }

        // 2. インジェクション・フィルタ (強化版 (§7.1))
        // メタ文字そのものだけでなく、エスケープ試行 (\) も含めてより広範に検知する。
        // 単純な contains ではなく、エスケープを考慮した正規化後のチェックが望ましいが、
        // 現状はエスケープ文字 '\' 自体もメタ文字として扱い、コマンドラインでの直接使用を制限する。
        let dangerous_parts = [
            ";", "&&", "||", ">", "<", "|", "`", "$(", "${", "\n", "\r", "\x0b", "\x0c", "\0",
            "%0a", "%0d", "\\$(", "\\`", "\\{", // エスケープによる回避試行
            "{", "}", "[", "]", "*", "?", // ブレース展開・グロブ攻撃 (SEC-BRACE)
        ];
        for part in dangerous_parts {
            if cmd_str.contains(part) {
                error!("🛡️ [Security Violation] Dangerous meta-character or escape sequence detected: '{}'", part);
                return Err(AiomeError::Infrastructure {
                    reason: format!("Security Violation: '{}' prohibited.", part),
                });
            }
        }

        // 4. Safer Execution: Parse command
        let parts = Self::shell_split(cmd_str);
        if parts.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Empty command.".into(),
            });
        }
        let binary = parts[0].as_str();
        let args: Vec<&str> = parts[1..].iter().map(|s| s.as_str()).collect();

        // Strict Whitelist check
        if !self.is_system_internal
            && !GLOBAL_SECURITY_CONFIG
                .allowed_binaries
                .contains(&binary.to_string())
        {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Security Violation: Binary '{}' is not in the whitelist.",
                    binary
                ),
            });
        }

        // 5. Script Engine constraints
        if !self.is_system_internal {
            if binary == "python3" || binary == "python" {
                for arg in &args {
                    // Check for `-c` or `-m`, including combined flags like `-Sc`
                    if (arg.starts_with('-') && !arg.starts_with("--") && arg.contains('c'))
                        || *arg == "-m"
                    {
                        return Err(AiomeError::Infrastructure {
                            reason: "Security Violation: python -c/-m is forbidden.".into(),
                        });
                    }
                }
            }
            if binary == "node" && (args.contains(&"-e") || args.contains(&"--eval")) {
                return Err(AiomeError::Infrastructure {
                    reason: "Security Violation: node -e is forbidden.".into(),
                });
            }
        }

        // 6. Path Canonicalization
        let sandbox = shared::sandbox::PathSandbox::new(&GLOBAL_SECURITY_CONFIG.workspace_root)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Sandbox initialization failed: {}", e),
            })?;

        for arg in &args {
            let potential_paths: Vec<&str> = if arg.starts_with("--") && arg.contains('=') {
                arg.splitn(2, '=').skip(1).collect()
            } else if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2 {
                // Extracts the potential path part from a short flag like -f/etc/foo or -f..
                vec![&arg[2..]]
            } else if !arg.starts_with('-') && !arg.is_empty() {
                vec![arg]
            } else {
                vec![]
            };

            for p in potential_paths {
                let mut path_allowed = false;

                if self.is_system_internal {
                    path_allowed = true;
                } else {
                    // Check against Workspace Sandbox
                    if sandbox.validate_path(p).is_ok() {
                        path_allowed = true;
                    }

                    // Check against Vault Sandbox (if configured)
                    if !path_allowed {
                        if let Some(vault_root) = &GLOBAL_SECURITY_CONFIG.vault_path {
                            if let Ok(vault_sandbox) = shared::sandbox::PathSandbox::new(vault_root)
                            {
                                if vault_sandbox.validate_path(p).is_ok() {
                                    // G-21 Hardening: Vault access is STRICTLY for internal processes only.
                                    // (is_system_internal is already checked above, but keep logic symmetric)
                                    warn!("🛡️ [BastionGuard] Access to vault path '{}' blocked: unauthorized skill context.", p);
                                }
                            }
                        }
                    }
                }

                if !path_allowed {
                    error!(
                        "🛡️ [BastionGuard] Blocked access to unauthorized path: {}",
                        p
                    );
                    let reason = if GLOBAL_SECURITY_CONFIG
                        .vault_path
                        .as_ref()
                        .is_some_and(|v| p.contains(&*v.to_string_lossy()))
                    {
                        format!("Security Violation: Path '{}' is in the Vault and requires system internal context.", p)
                    } else {
                        format!("Security Violation: Path '{}' is outside sandbox jail.", p)
                    };
                    return Err(AiomeError::Infrastructure { reason });
                }
            }
        }

        // Phase 36.5: Strict Profile Enforcement
        if profile == SandboxProfile::Strict
            && (self.manifest.allow_network || self.manifest.allow_filesystem_write)
        {
            return Err(AiomeError::Infrastructure {
                reason:
                    "Security Violation: Strict profile requires zero-privilege (no network/write)."
                        .into(),
            });
        }

        // Dynamic Sandbox Wrapping
        let (program, mut sandbox_args) = if cfg!(target_os = "macos") && !self.is_system_internal {
            if std::path::Path::new("/usr/bin/sandbox-exec").exists() {
                let profile_str = match profile {
                    SandboxProfile::Strict => {
                        "(version 1)
                         (allow default)
                         (deny network*)
                         (deny file-write*)"
                    }
                    SandboxProfile::PythonForge | SandboxProfile::WasmRun => {
                        "(version 1)
                         (allow default)
                         (deny network*)"
                    }
                    SandboxProfile::WasmBuild | SandboxProfile::ForgeBuild => {
                        "(version 1)
                         (allow default)"
                    }
                    _ => "(version 1) (allow default)",
                };
                (
                    "sandbox-exec",
                    vec![
                        "-p".to_string(),
                        profile_str.to_string(),
                        binary.to_string(),
                    ],
                )
            } else {
                (binary, vec![])
            }
        } else if cfg!(target_os = "linux") && !self.is_system_internal {
            let runsc_exists = std::process::Command::new("which")
                .arg("runsc")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if runsc_exists {
                let mut args = Vec::new();
                if profile == SandboxProfile::Strict || profile == SandboxProfile::WasmRun {
                    args.push("--network=none".to_string());
                }
                args.push("do".to_string());
                args.push(binary.to_string());
                ("runsc", args)
            } else {
                (binary, vec![])
            }
        } else {
            (binary, vec![])
        };

        for a in args {
            sandbox_args.push(a.to_string());
        }

        let output = crate::security_zombie::run_with_timeout_vec(
            program,
            sandbox_args,
            std::time::Duration::from_secs(60),
        )
        .await?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// ファイルパスへの書き込み権限をチェック
    fn check_fs_write(&self, _path: &std::path::Path) -> Result<(), AiomeError> {
        if !self.is_system_internal && !self.manifest.allow_filesystem_write {
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Filesystem write disabled.".into(),
            });
        }
        Ok(())
    }

    /// ネットワーク接続をチェック
    fn check_network(&self, url: &str) -> Result<(), AiomeError> {
        if !self.is_system_internal && !self.manifest.allow_network {
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Network access disabled.".into(),
            });
        }

        // ドメイン・フィルタ
        if !self.manifest.allowed_domains.is_empty() {
            let mut allowed = false;
            for domain in &self.manifest.allowed_domains {
                if url.contains(domain) {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Security Violation: Domain '{}' not in allowed list.", url),
                });
            }
        }

        Ok(())
    }
}

impl BastionGuard {
    /// 新しいインスタンスを生成する
    pub fn new(manifest: PermissionManifest) -> Self {
        Self {
            manifest,
            is_system_internal: false,
            boundary_verifier: BoundaryVerifier::from_global_config(),
        }
    }

    /// [Internal] 🛡️ 指定されたバイナリをサンドボックスでラップする
    fn wrap_binary(&self, binary: &str, profile: SandboxProfile) -> (String, Vec<String>) {
        if cfg!(target_os = "macos") && !self.is_system_internal {
            let sandbox_exists = std::process::Command::new("which")
                .arg("sandbox-exec")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if sandbox_exists {
                let profile_str = match profile {
                    SandboxProfile::LoraTraining => {
                        "(version 1)\n                         (allow default)\n                         (allow network-outbound (remote tcp \"*:443\"))\n                         (allow network-outbound (remote tcp \"*:80\"))\n                         (deny file-write* (regex #\"^/etc\"))\n                         (deny file-write* (regex #\"^/var\"))"
                    }
                    _ => "(version 1) (allow default)",
                };
                return (
                    "sandbox-exec".to_string(),
                    vec![
                        "-p".to_string(),
                        profile_str.to_string(),
                        binary.to_string(),
                    ],
                );
            }
        } else if cfg!(target_os = "linux") && !self.is_system_internal {
            let runsc_exists = std::process::Command::new("which")
                .arg("runsc")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if runsc_exists && GLOBAL_SECURITY_CONFIG.use_runsc_sandbox {
                let mut runsc_args = Vec::new();
                if profile == SandboxProfile::Strict || profile == SandboxProfile::WasmRun {
                    runsc_args.push("--network=none".to_string());
                }
                runsc_args.push("do".to_string());
                runsc_args.push(binary.to_string());
                return ("runsc".to_string(), runsc_args);
            }
        }
        (binary.to_string(), vec![])
    }

    /// 🛡️ [F-01] 構造化された引数を用いて Command を構築する。
    pub fn build_safe_command_args(
        &self,
        program_name: &str,
        args: Vec<String>,
        profile: SandboxProfile,
    ) -> Result<tokio::process::Command, AiomeError> {
        if !self.is_system_internal
            && !GLOBAL_SECURITY_CONFIG
                .allowed_binaries
                .contains(&program_name.to_string())
        {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Security Violation: Binary '{}' is not whitelisted.",
                    program_name
                ),
            });
        }

        let (actual_prog, mut actual_args) = self.wrap_binary(program_name, profile);

        actual_args.extend(args);

        let mut cmd = tokio::process::Command::new(actual_prog);
        cmd.args(actual_args);
        Ok(cmd)
    }

    /// システム内部用インスタンスを生成する (G-26)
    pub fn new_internal(manifest: PermissionManifest) -> Self {
        Self {
            manifest,
            is_system_internal: true,
            boundary_verifier: BoundaryVerifier::from_global_config(),
        }
    }

    /// SEC-5: Quote-aware command splitting for paths with spaces
    /// Supports double and single quotes. Does NOT support escape sequences for security.
    fn shell_split(input: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_double_quote = false;
        let mut in_single_quote = false;

        for ch in input.chars() {
            match ch {
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }
                ' ' | '\t' | '\r' | '\n' | '\x0b' | '\x0c'
                    if !in_double_quote && !in_single_quote =>
                {
                    if !current.is_empty() {
                        parts.push(std::mem::take(&mut current));
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        if !current.is_empty() {
            parts.push(current);
        }
        // NOTE: If in_double_quote or in_single_quote is true here, the quote was never closed.
        // We currently just keep the content, which is safer than dropping it or panicking.
        parts
    }
}

/// Abyss Voice Vault (暗号化ボイスアセットの復号管理)
pub mod abyss_voice_vault;
/// 🛡️ 行動監視（Trojan's Whisper §7.3）
pub mod behavior_monitor;
/// 暗号化・復号処理のユーティリティ群
pub mod crypto;
/// 🪝 フック管理基盤 (Phase 36)
pub mod hook_manager;
/// メモリ上に固定(mlock)されたバイトベクタの実装
pub mod mlock;
/// SQLite を使用した Vault Backend の実装。
pub mod sqlite_vault_backend;
use crate::db::DatabasePool;
pub use abyss_voice_vault::AbyssVoiceVault;
use aiome_core_contracts::voice_vault::VoiceKeyVault;
use zeroize::Zeroizing;

/// Phase 9: Voice Core DRM (Digital Rights Management)
///
/// ボイスアセット（TTSモデル、LoRA等）の正当な所有権を管理し、
/// Abyss Vault 経由でのキー取得と復号を行う基盤。
pub struct VoiceCoreDrm {
    /// Abyss Vault のベースURL
    pub vault_url: String,
    vault: AbyssVoiceVault,
    #[allow(dead_code)]
    registry: Arc<RegistryManager>,
}

impl VoiceCoreDrm {
    /// 新しい DRM インスタンスを生成する
    pub async fn new(
        vault_url: String,
        registry: Arc<RegistryManager>,
        pool: DatabasePool,
    ) -> Self {
        let vault = AbyssVoiceVault::new(registry.clone(), pool);
        // 起動時に永続化された鍵をリストア (§CISO-1)
        match vault.restore_keys_from_db().await {
            Ok(n) => tracing::info!("🔐 [DRM] {} vault keys restored on startup", n),
            Err(e) => tracing::error!("🚨 [DRM] Failed to restore vault keys: {:?}", e),
        }
        Self {
            vault_url,
            vault,
            registry,
        }
    }

    /// アセットの復号キーを取得する (Abyss Vault 連携)
    pub async fn fetch_decryption_key(
        &self,
        agent_id: Uuid,
        asset_id: Uuid,
    ) -> Result<Zeroizing<Vec<u8>>, AiomeError> {
        self.vault.fetch_decryption_key(agent_id, asset_id).await
    }

    /// ライセンスを検証する
    pub async fn verify_license(&self, agent_id: Uuid, asset_id: Uuid) -> Result<bool, AiomeError> {
        self.vault.verify_license(agent_id, asset_id).await
    }

    /// アセットキーを登録する
    pub async fn register_asset_key(
        &self,
        asset_id: Uuid,
        key: Zeroizing<Vec<u8>>,
    ) -> Result<(), AiomeError> {
        self.vault.register_asset_key(asset_id, key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::security::PermissionManifest;

    #[tokio::test]
    async fn test_bastion_guard_internal_bypass() {
        let manifest = PermissionManifest {
            allow_shell_execution: false,
            ..Default::default()
        };
        // 通常のガードは拒否
        let guard = BastionGuard::new(manifest.clone());
        assert!(guard.safe_exec("ls").await.is_err());

        // システム内部用ガードは許可 (Manifestをバイパス)
        let guard_internal = BastionGuard::new_internal(manifest);
        assert!(guard_internal.safe_exec("ls").await.is_ok());
    }

    #[tokio::test]
    async fn test_bastion_guard_whitelist_ollama_docker() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        let res = guard.safe_exec("ollama --version").await;
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(!reason.contains("not in the whitelist"));
        }

        let res = guard.safe_exec("docker ps").await;
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(!reason.contains("not in the whitelist"));
        }
    }

    #[tokio::test]
    async fn test_bastion_guard_disallow_shell() {
        let manifest = PermissionManifest {
            allow_shell_execution: false,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);
        let res = guard.safe_exec("ls").await;
        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(reason.contains("Security Violation: Forbidden"));
        } else {
            panic!("Expected security violation error");
        }
    }

    #[tokio::test]
    async fn test_bastion_guard_injection_prevention() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // Test various injection characters
        assert!(guard.safe_exec("ls; rm -rf /").await.is_err());
        assert!(guard.safe_exec("ls && whoami").await.is_err());
        assert!(guard.safe_exec("ls | grep foo").await.is_err());
        assert!(guard.safe_exec("ls > out.txt").await.is_err());
        assert!(guard.safe_exec("echo `whoami`").await.is_err());
        assert!(guard.safe_exec("echo $(whoami)").await.is_err());
        assert!(guard.safe_exec("python3 -Sc import os").await.is_err()); // Red Team combined flag test
    }

    #[tokio::test]
    async fn test_bastion_guard_sensitive_paths() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        assert!(guard.safe_exec("cat /etc/passwd").await.is_err());
        assert!(guard.safe_exec("grep -f/etc/passwd foo.txt").await.is_err()); // Red Team short flag attached path test
        assert!(guard.safe_exec("ls ~/.ssh").await.is_err());
        assert!(guard.safe_exec("grep API_KEY .env").await.is_err());
    }

    #[tokio::test]
    async fn test_bastion_guard_whitelist_enforcement() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // "ls" is in the default whitelist
        let res = guard.safe_exec("ls").await;
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(!reason.contains("not in the whitelist"));
        }

        // "rm" is not in the default whitelist
        let res = guard.safe_exec("rm -rf /tmp/foo").await;
        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(reason.contains("is not in the allowed whitelist"));
        }
    }

    use proptest::prelude::*;

    // Note: proptest with async is tricky, skipping for now or wrapping in block_on if needed.
    // However, I will keep the existing synchronous-style tests as tokio::test.

    #[tokio::test]
    async fn test_bastion_guard_sandbox_selection() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        let _ = guard.safe_exec("ls -la").await;
    }

    #[tokio::test]
    async fn test_bastion_guard_system_internal_bypass() {
        let manifest = PermissionManifest {
            allow_shell_execution: false, // Normal users restricted
            ..Default::default()
        };

        let guard = BastionGuard::new_internal(manifest);
        let cmd = "ls";
        let res = guard.safe_exec(cmd).await;

        if let Err(AiomeError::Infrastructure { reason }) = &res {
            assert!(
                !reason.contains("Forbidden"),
                "Internal guard should not be forbidden. Error: {}",
                reason
            );
        }
    }

    #[tokio::test]
    async fn test_bastion_guard_macos_sandbox_regex() {
        if cfg!(target_os = "macos") && std::path::Path::new("/usr/bin/sandbox-exec").exists() {
            let manifest = PermissionManifest {
                allow_shell_execution: true,
                ..Default::default()
            };
            let guard = BastionGuard::new(manifest);
            let res = guard.safe_exec("ls").await;
            assert!(res.is_ok() || !res.unwrap_err().to_string().contains("Forbidden"));
        }
    }

    #[tokio::test]
    async fn test_bastion_guard_internal_bypasses_whitelist() {
        let manifest = PermissionManifest::default();
        let guard = BastionGuard::new_internal(manifest);
        let res = guard.safe_exec("rm --version").await;

        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(
                !reason.contains("is not in the whitelist"),
                "Internal guard should bypass whitelist. Error: {}",
                reason
            );
        }
    }

    #[tokio::test]
    async fn test_bastion_guard_profile_selection() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // 1. Default profile should work for 'ls'
        let res = guard
            .safe_exec_with_profile("ls", SandboxProfile::Default)
            .await;
        assert!(res.is_ok(), "Default profile failed: {:?}", res.err());

        // 2. Strict profile should work for 'ls' if manifest allows shell (but network/write are restricted by logic)
        let res = guard
            .safe_exec_with_profile("ls", SandboxProfile::Strict)
            .await;
        assert!(res.is_ok(), "Strict profile failed: {:?}", res.err());

        // 3. Strict profile should REJECT if manifest has network/write enabled (Logic check in Phase 36.5)
        let manifest_unsafe = PermissionManifest {
            allow_shell_execution: true,
            allow_network: true,
            ..Default::default()
        };
        let guard_unsafe = BastionGuard::new(manifest_unsafe);
        let res = guard_unsafe
            .safe_exec_with_profile("ls", SandboxProfile::Strict)
            .await;
        assert!(
            res.is_err(),
            "Strict profile should have rejected unsafe manifest"
        );
    }

    #[tokio::test]
    async fn test_bastion_guard_vault_isolation_regressions() {
        // G-21: 通常スキルが Vault にアクセスできないことを保証する回帰テスト
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            allow_filesystem_write: true, // 書き込み権限があっても Vault は拒否されるべき
            ..Default::default()
        };

        if let Some(vault_path) = &GLOBAL_SECURITY_CONFIG.vault_path {
            let vault_file = vault_path.join("secret_key.txt");
            let _ = std::fs::create_dir_all(vault_path);
            let _ = std::fs::write(&vault_file, "SENSITIVE_DATA");

            // 1. 通常のガードは拒否されること
            let guard = BastionGuard::new(manifest.clone());
            let cmd = format!("cat {}", vault_file.to_string_lossy());
            let res = guard.safe_exec(&cmd).await;
            assert!(
                res.is_err(),
                "Regular skills must NOT access Vault even with write perm."
            );
            if let Err(aiome_core::error::AiomeError::Infrastructure { reason }) = res {
                assert!(reason.contains("requires system internal context"));
            }

            // 2. システム内部用ガードは許可されること
            let guard_internal = BastionGuard::new_internal(manifest);
            let res_internal = guard_internal.safe_exec(&cmd).await;
            assert!(
                res_internal.is_ok(),
                "Internal system processes must have Vault access."
            );
            assert_eq!(res_internal.unwrap().trim(), "SENSITIVE_DATA"); // allow-anti-pattern

            // Cleanup
            let _ = std::fs::remove_file(&vault_file);
        }
    }

    #[tokio::test]
    async fn test_bastion_guard_escaped_injection() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // エスケープされた $ を使ったインジェクションの試行
        let malicious_cmd = "ls \\$(whoami)";
        let res = guard.safe_exec(malicious_cmd).await;

        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(reason.contains("forbidden"));
        } else {
            panic!("Expected Security Violation for escaped injection");
        }
    }
    #[tokio::test]
    async fn test_bastion_guard_brace_expansion_bypass() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // ブレース展開を用いたフィルタ回避の試行 (例: .env を .e{n,v}v で表現)
        // 現状のフィルタでは '{' や '}' を検知していないため、パスバリデーションまで到達してしまう可能性がある
        let malicious_cmd = "cat .e{n,v}v";
        let res = guard.safe_exec(malicious_cmd).await;

        assert!(res.is_err(), "Should have blocked brace expansion attempt");
    }

    #[test]
    fn test_security_config_canonicalization_vulnerability() {
        // 環境変数に相対パスやシンボリックリンクを含むパスを設定した場合の挙動確認
        // 実際に存在するディレクトリを指定することで canonicalize() を成功させる
        let temp = std::env::current_dir().unwrap().join("test_vault_tmp"); // allow-anti-pattern
        let _ = std::fs::create_dir_all(&temp);

        std::env::set_var("VAULT_PATH", "./test_vault_tmp");
        let config = SecurityConfig::load_or_default();

        if let Some(vault) = config.vault_path {
            assert!(
                vault.is_absolute(),
                "Vault path must be absolute after canonicalization."
            );
        }

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[tokio::test]
    async fn test_bastion_guard_boundary_verifier_integration() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // 1. 巨大なペイロード（BoundaryVerifier で拒否されるはず）
        let large_cmd = format!("echo {}", "a".repeat(70000));
        let res = guard.safe_exec(&large_cmd).await;
        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            // 統合前は別の理由でエラーになるか、あるいはパスバリデーション等でエラーになる。
            // BoundaryVerifier のメッセージが含まれていることを確認するテスト。
            assert!(
                reason.contains("exceeds limit (64KB)"),
                "Expected BoundaryVerifier rejection, got: {}",
                reason
            );
        }
    }
}
