/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
pub use aiome_core::security::{PermissionManifest, RuntimeJail};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub allowed_binaries: Vec<String>,
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
                "mv".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "diff".to_string(),
                "tree".to_string(),
                "which".to_string(),
                "env".to_string(),
            ],
        }
    }
}

impl SecurityConfig {
    pub fn load_or_default() -> Self {
        let workspace = std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| "workspace".to_string());
        let path = std::path::Path::new(&workspace).join("config/security.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str::<SecurityConfig>(&content) {
                    info!(
                        "🛡️ [SecurityConfig] Loaded whitelist from workspace/config/security.json."
                    );
                    return config;
                }
            }
        }
        Self::default()
    }
}

pub static GLOBAL_SECURITY_CONFIG: once_cell::sync::Lazy<SecurityConfig> =
    once_cell::sync::Lazy::new(SecurityConfig::load_or_default);


/// Phase 2: Runtime Enforcement (The Bastion Guard)
///
/// エージェントが実行しようとする「アクション」を監視し、
/// 権限マニフェストおよびOSレベルの制限（seccomp等）と照合する。
pub struct BastionGuard {
    manifest: PermissionManifest,
}

impl RuntimeJail for BastionGuard {
    /// シェルコマンドの実行を検証し、許可されていれば実行する
    fn safe_exec(&self, cmd_str: &str) -> Result<String, AiomeError> {
        info!("🛡️ [BastionGuard] 検証中: {}", cmd_str);

        // 1. マニフェスト・チェック
        if !self.manifest.allow_shell_execution {
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

        // 3. センシティブなパス
        if cmd_str.contains("/etc/") || cmd_str.contains(".ssh") || cmd_str.contains(".env") || cmd_str.contains("../") {
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Sensitive access.".to_string(),
            });
        }

        info!("✅ [BastionGuard] 検証完了。コマンドを実行します...");

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
        if !GLOBAL_SECURITY_CONFIG
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
        if binary == "python3" || binary == "python" {
            if args.contains(&"-c") || args.contains(&"-m") {
                return Err(AiomeError::Infrastructure {
                    reason: "Security Violation: python -c/-m is forbidden.".into(),
                });
            }
        }
        if binary == "node" {
            if args.contains(&"-e") || args.contains(&"--eval") {
                return Err(AiomeError::Infrastructure {
                    reason: "Security Violation: node -e is forbidden.".into(),
                });
            }
        }
        if binary == "find" || binary == "xargs" {
            if args.contains(&"-exec") || args.contains(&"-I") {
                return Err(AiomeError::Infrastructure {
                    reason: "Security Violation: find -exec or xargs -I is forbidden.".into(),
                });
            }
        }

        // 6. Path Canonicalization & Strict traversal check
        for arg in &args {
            let path = std::path::Path::new(arg);
            if let Ok(canon) = std::fs::canonicalize(path) {
                let canon_str = canon.to_string_lossy();
                if canon_str.contains("/etc/") || canon_str.contains("/.ssh") || canon_str.contains(".env") {
                    return Err(AiomeError::Infrastructure {
                        reason: "Security Violation: Directory traversal resolving to sensitive path detected.".into(),
                    });
                }
            }
        }

        use std::process::Command;

        let output =
            Command::new(binary)
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
        if !self.manifest.allow_filesystem_write {
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Filesystem write disabled.".into(),
            });
        }
        Ok(())
    }

    /// ネットワーク接続をチェック
    fn check_network(&self, url: &str) -> Result<(), AiomeError> {
        if !self.manifest.allow_network {
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
    pub fn new(manifest: PermissionManifest) -> Self {
        Self { manifest }
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
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
