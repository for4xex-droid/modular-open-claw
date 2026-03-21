/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::registry::RegistryManager;
use aiome_core::error::AiomeError;
pub use aiome_core::security::{PermissionManifest, RuntimeJail};
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
            ],
            workspace_root: std::path::PathBuf::from("workspace"),
        }
    }
}

impl SecurityConfig {
    /// `load_or_default` を実行する
    pub fn load_or_default() -> Self {
        let workspace = std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| "workspace".to_string());
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
                    return config;
                }
            }
        }
        let mut config = Self::default();
        config.workspace_root = workspace_root;
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
}

impl RuntimeJail for BastionGuard {
    /// シェルコマンドの実行を検証し、許可されていれば実行する
    fn safe_exec(&self, cmd_str: &str) -> Result<String, AiomeError> {
        info!("🛡️ [BastionGuard] 検証中: {}", cmd_str);

        // 1. マニフェスト・チェック
        // G-26: もし is_system_internal が true ならマニフェストをバイパスする
        if !self.is_system_internal && !self.manifest.allow_shell_execution {
            error!("🚨 [SECURITY VIOLATION] Shell execution is disabled.");
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Forbidden.".to_string(),
            });
        }

        // 2. インジェクション・フィルタ
        let dangerous_parts = [
            ";", "&&", "||", ">", "<", "|", "`", "$(", "${", "\n", "\r", "%0a", "%0d",
        ];
        for part in dangerous_parts {
            if cmd_str.contains(part) {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Security Violation: '{}' prohibited.", part),
                });
            }
        }

        // 3. (REMOVED) Blacklist-based sensitive path check.
        // Replaced by canonicalized whitelist below in step 6.

        // 4. Safer Execution: Parse command with quote-aware splitting for paths with spaces
        let parts = Self::shell_split(cmd_str);
        if parts.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Empty command.".into(),
            });
        }
        let binary = parts[0].as_str();
        let args: Vec<&str> = parts[1..].iter().map(|s| s.as_str()).collect();

        // Strict Whitelist check against SecurityConfig (Global Singleton)
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

        // 5. Script Engine and Command Constraints (Red Team fix)
        // DS-19 FIX: Allow system internal calls to bypass these restrictions for LoRA/System tasks
        if !self.is_system_internal {
            if (binary == "python3" || binary == "python")
                && (args.contains(&"-c") || args.contains(&"-m"))
            {
                return Err(AiomeError::Infrastructure {
                    reason: "Security Violation: python -c/-m is forbidden.".into(),
                });
            }
            if binary == "node" && (args.contains(&"-e") || args.contains(&"--eval")) {
                return Err(AiomeError::Infrastructure {
                    reason: "Security Violation: node -e is forbidden.".into(),
                });
            }
            if (binary == "find" || binary == "xargs")
                && (args.contains(&"-exec") || args.contains(&"-I"))
            {
                return Err(AiomeError::Infrastructure {
                    reason: "Security Violation: find -exec or xargs -I is forbidden.".into(),
                });
            }
        }

        // 6. Path Canonicalization & Strict traversal check (SEC-Whitelist)
        let sandbox = shared::sandbox::PathSandbox::new(&GLOBAL_SECURITY_CONFIG.workspace_root)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Sandbox initialization failed: {}", e),
            })?;

        for arg in &args {
            // SEC: Validate all arguments. Check both standalone paths and paths hidden in flags (e.g. --file=path).
            let potential_paths: Vec<&str> = if arg.starts_with("--") && arg.contains('=') {
                arg.splitn(2, '=').skip(1).collect()
            } else if !arg.starts_with('-') && !arg.is_empty() {
                vec![arg]
            } else {
                vec![]
            };

            for p in potential_paths {
                // 1. Jail traversal check
                if let Err(e) = sandbox.validate_path(p) {
                    error!(
                        "🛡️ [BastionGuard] Blocked access to unauthorized path: {} (Reason: {})",
                        p, e
                    );
                    return Err(AiomeError::Infrastructure {
                        reason: format!(
                            "Security Violation: Path '{}' is outside sandbox jail.",
                            p
                        ),
                    });
                }

                // 2. Sensitive file blacklist (even within valid workspace)
                let p_lower = p.to_lowercase();
                if p_lower.contains(".env")
                    || p_lower.contains(".git")
                    || p_lower.contains("config/security.json")
                {
                    error!(
                        "🛡️ [BastionGuard] Blocked access to sensitive internal file: {}",
                        p
                    );
                    return Err(AiomeError::Infrastructure {
                        reason: format!(
                            "Security Violation: Access to sensitive file '{}' is forbidden.",
                            p
                        ),
                    });
                }
            }
        }

        info!(
            "✅ [BastionGuard] All checks passed. Executing: {} {}",
            binary,
            args.join(" ")
        );

        use std::process::Command;

        // Phase 9: Dynamic Sandbox Wrapping (gVisor / sandbox-exec)
        let mut cmd = if cfg!(target_os = "macos") && !self.is_system_internal {
            // macOS: sandbox-exec (if available) - Fallback to normal for dev
            if std::path::Path::new("/usr/bin/sandbox-exec").exists() {
                let mut c = Command::new("sandbox-exec");
                c.arg("-p").arg("(version 1) (allow default)"); // Placeholder for Phase 9.1 profile
                c.arg(binary);
                c
            } else {
                Command::new(binary)
            }
        } else if cfg!(target_os = "linux") && !self.is_system_internal {
            // Linux: prioritize gVisor (runsc)
            if std::path::Path::new("/usr/bin/runsc").exists() {
                let mut c = Command::new("runsc");
                c.arg("do");
                c.arg(binary);
                c
            } else {
                warn!("⚠️ [Security] runsc (gVisor) not found. Running with standard host kernel.");
                Command::new(binary)
            }
        } else {
            Command::new(binary)
        };

        let output = cmd
            .args(args)
            .output()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Execution failed: {}", e),
            })?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(AiomeError::Infrastructure {
                reason: format!("Command error: {}", err_msg),
            });
        }

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
        }
    }

    /// システム内部用インスタンスを生成する (G-26)
    pub fn new_internal(manifest: PermissionManifest) -> Self {
        Self {
            manifest,
            is_system_internal: true,
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
                ' ' | '\t' if !in_double_quote && !in_single_quote => {
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
/// 暗号化・復号処理のユーティリティ群
pub mod crypto;
use abyss_voice_vault::AbyssVoiceVault;
use aiome_contracts::voice_vault::VoiceKeyVault;
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
        pool: sqlx::SqlitePool,
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

    #[test]
    fn test_bastion_guard_internal_bypass() {
        let manifest = PermissionManifest {
            allow_shell_execution: false,
            ..Default::default()
        };
        // 通常のガードは拒否
        let guard = BastionGuard::new(manifest.clone());
        assert!(guard.safe_exec("ls").is_err());

        // システム内部用ガードは許可 (Manifestをバイパス)
        let guard_internal = BastionGuard::new_internal(manifest);
        assert!(guard_internal.safe_exec("ls").is_ok());
    }

    #[test]
    fn test_bastion_guard_whitelist_ollama_docker() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // 新しく追加されたバイナリが許可されることを確認 (実行は環境に依存するが、whitelistチェックまでは通るはず)
        // ここでは、バイナリが存在しなくても "execution failed" なら whitelist はパスしている。
        // "Binary 'xxx' is not in the whitelist" が出ないことを確認する。
        let res = guard.safe_exec("ollama --version");
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(!reason.contains("not in the whitelist"));
        }

        let res = guard.safe_exec("docker ps");
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(!reason.contains("not in the whitelist"));
        }
    }

    #[test]
    fn test_bastion_guard_disallow_shell() {
        let manifest = PermissionManifest {
            allow_shell_execution: false,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);
        let res = guard.safe_exec("ls");
        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(reason.contains("Security Violation: Forbidden"));
        } else {
            panic!("Expected security violation error");
        }
    }

    #[test]
    fn test_bastion_guard_injection_prevention() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // Test various injection characters
        assert!(guard.safe_exec("ls; rm -rf /").is_err());
        assert!(guard.safe_exec("ls && whoami").is_err());
        assert!(guard.safe_exec("ls | grep foo").is_err());
        assert!(guard.safe_exec("ls > out.txt").is_err());
        assert!(guard.safe_exec("echo `whoami`").is_err());
        assert!(guard.safe_exec("echo $(whoami)").is_err());
    }

    #[test]
    fn test_bastion_guard_sensitive_paths() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        assert!(guard.safe_exec("cat /etc/passwd").is_err());
        assert!(guard.safe_exec("ls ~/.ssh").is_err());
        assert!(guard.safe_exec("grep API_KEY .env").is_err());
    }

    #[test]
    fn test_bastion_guard_whitelist_enforcement() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // "ls" is in the default whitelist
        let res = guard.safe_exec("ls");
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(!reason.contains("not in the whitelist"));
        }

        // "rm" is not in the default whitelist
        let res = guard.safe_exec("rm -rf /tmp/foo");
        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(reason.contains("Binary 'rm' is not in the whitelist"));
        }
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_bastion_guard_avoids_forbidden_chars(s in ".*[;&|><`\\$\\n\\r].*") {
            let manifest = PermissionManifest {
                allow_shell_execution: true,
                ..Default::default()
            };
            let guard = BastionGuard::new(manifest);
            let res = guard.safe_exec(&s);

            // If the string contains any of the dangerous parts, it MUST be blocked
            let dangerous_parts = [";", "&&", "||", ">", "<", "|", "`", "$(", "${", "\n", "\r"];
            let mut should_be_blocked = false;
            for part in dangerous_parts {
                if s.contains(part) {
                    should_be_blocked = true;
                    break;
                }
            }

            if should_be_blocked {
                prop_assert!(res.is_err(), "Input '{}' should have been blocked", s);
            }
        }
    }

    #[test]
    fn test_bastion_guard_sandbox_selection() {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            ..Default::default()
        };
        let guard = BastionGuard::new(manifest);

        // This test is mostly for coverage and log inspection.
        // It ensures the logic doesn't panic.
        let _ = guard.safe_exec("ls -la");
    }

    #[test]
    fn test_bastion_guard_system_internal_bypass() {
        let manifest = PermissionManifest {
            allow_shell_execution: false, // Normal users restricted
            ..Default::default()
        };

        // System internal guard should bypass even if shell is disallowed in manifest
        let guard = BastionGuard::new_internal(manifest);
        let cmd = "ls";

        // We don't check for success/failure of the actual command (as it depends on env),
        // but we check if it was BLOCKED by the guard logic.
        let res = guard.safe_exec(cmd);

        if let Err(AiomeError::Infrastructure { reason }) = &res {
            assert!(
                !reason.contains("Forbidden"),
                "Internal guard should not be forbidden. Error: {}",
                reason
            );
        }
    }

    #[test]
    fn test_bastion_guard_macos_sandbox_regex() {
        // Test internal helper if it was exposed, or just verify the wrapping logic
        if cfg!(target_os = "macos") && std::path::Path::new("/usr/bin/sandbox-exec").exists() {
            let manifest = PermissionManifest {
                allow_shell_execution: true,
                ..Default::default()
            };
            let guard = BastionGuard::new(manifest);

            // This is hard to test tanpa mock Command, but we can verify it doesn't fail.
            let res = guard.safe_exec("ls");
            assert!(res.is_ok() || !res.unwrap_err().to_string().contains("Forbidden"));
        }
    }

    #[test]
    fn test_bastion_guard_internal_bypasses_whitelist() {
        let manifest = PermissionManifest::default();
        let guard = BastionGuard::new_internal(manifest);

        // "rm" is NOT in the default whitelist
        let res = guard.safe_exec("rm --version");

        // If it fails with "is not in the whitelist", then the bypass is NOT working (RED)
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(
                !reason.contains("is not in the whitelist"),
                "Internal guard should bypass whitelist. Error: {}",
                reason
            );
        }
    }
}
