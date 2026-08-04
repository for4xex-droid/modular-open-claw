/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::config::GLOBAL_SECURITY_CONFIG;
use crate::boundary_verifier::BoundaryVerifier;
use aiome_core::error::AiomeError;
pub use aiome_core::security::{PermissionManifest, RuntimeJail, SandboxProfile};
use async_trait::async_trait;
use tracing::{error, info, warn};

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
                error!(
                    "🛡️ [Security Violation] Dangerous meta-character or escape sequence detected: '{}'",
                    part
                );
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
                                    warn!(
                                        "🛡️ [BastionGuard] Access to vault path '{}' blocked: unauthorized skill context.",
                                        p
                                    );
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
                        format!(
                            "Security Violation: Path '{}' is in the Vault and requires system internal context.",
                            p
                        )
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
                let mut s_profile = shared::sandbox::seatbelt::SeatbeltProfile::default();
                match profile {
                    SandboxProfile::Strict => {}
                    SandboxProfile::PythonForge | SandboxProfile::WasmRun => {
                        // ネットワークのみ拒否、ファイル書き込みは許可
                        s_profile.allow_fs_write = true;
                    }
                    SandboxProfile::WasmBuild | SandboxProfile::ForgeBuild => {
                        s_profile.allow_network = true;
                        s_profile.allow_fs_write = true;
                    }
                    SandboxProfile::McpServer => {
                        s_profile.allow_network = self.manifest.allow_network;
                        s_profile.allow_fs_write = self.manifest.allow_filesystem_write;
                    }
                    _ => {
                        s_profile.allow_network = true;
                        s_profile.allow_fs_write = true;
                    }
                }
                shared::sandbox::seatbelt::create_seatbelt_command_args(binary, &s_profile)
            } else {
                (binary.to_string(), vec![])
            }
        } else if cfg!(target_os = "linux") && !self.is_system_internal {
            let runsc_exists = BastionGuard::binary_exists_on_path("runsc");

            if runsc_exists && GLOBAL_SECURITY_CONFIG.use_runsc_sandbox {
                build_runsc_args(profile, binary, GLOBAL_SECURITY_CONFIG.enable_syscall_audit)
            } else {
                (binary.to_string(), vec![])
            }
        } else {
            (binary.to_string(), vec![])
        };

        for a in args {
            sandbox_args.push(a.to_string());
        }

        let output = crate::security_zombie::run_with_timeout_vec(
            &program,
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
    ///
    /// `target` は URL（scheme 付き）または bare host（WASM `allowed_domains` 登録時）。
    /// 空 `allowed_domains` は Deny（OP-096 / ADR-057）。`url.contains` は使わない。
    fn check_network(&self, target: &str) -> Result<(), AiomeError> {
        if !self.is_system_internal && !self.manifest.allow_network {
            return Err(AiomeError::Infrastructure {
                reason: "Security Violation: Network access disabled.".into(),
            });
        }

        if self.is_system_internal {
            return Ok(());
        }

        let host = network_target_host(target).ok_or_else(|| AiomeError::Infrastructure {
            reason: format!(
                "Security Violation: Invalid network target '{}' (empty host).",
                target
            ),
        })?;

        if !aiome_core::security::host_permitted(&host, &self.manifest.allowed_domains) {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Security Violation: Domain '{}' not in allowed list.",
                    target
                ),
            });
        }

        Ok(())
    }
}

/// Resolve a `check_network` target to a host string (bare domain or URL host).
/// Fail-Closed: 非 http(s)/ws(s) scheme、および host として不正な文字を含む
/// bare 文字列は None（deny）を返す。
fn network_target_host(target: &str) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = url::Url::parse(trimmed) {
        return match parsed.scheme() {
            "http" | "https" | "ws" | "wss" => parsed
                .host_str()
                .filter(|h| !h.is_empty())
                .map(|h| h.to_string()),
            _ => None, // ftp / file / data 等は Fail-Closed
        };
    }
    // bare host: URL 構成文字・制御文字を含む場合は deny
    if trimmed
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '/' | '@' | ':' | '#' | '?'))
    {
        return None;
    }
    Some(trimmed.to_lowercase())
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

    /// [Internal] 指定されたバイナリが PATH 上に存在するかを安全にチェックする。
    ///
    /// `which` コマンドを `harden_command` で環境隔離した上で実行する。
    fn binary_exists_on_path(name: &str) -> bool {
        let mut cmd = std::process::Command::new("which");
        cmd.arg(name);
        shared::security::harden_command(&mut cmd);
        cmd.output().map(|o| o.status.success()).unwrap_or(false)
    }

    /// [Internal] 🛡️ 指定されたバイナリをサンドボックスでラップする
    fn wrap_binary(&self, binary: &str, profile: SandboxProfile) -> (String, Vec<String>) {
        if cfg!(target_os = "macos") && !self.is_system_internal {
            let sandbox_exists = Self::binary_exists_on_path("sandbox-exec");

            if sandbox_exists {
                let mut s_profile = shared::sandbox::seatbelt::SeatbeltProfile::default();
                match profile {
                    SandboxProfile::LoraTraining => {
                        s_profile.is_lora_training = true;
                    }
                    SandboxProfile::McpServer => {
                        s_profile.allow_network = self.manifest.allow_network;
                        s_profile.allow_fs_write = self.manifest.allow_filesystem_write;
                    }
                    SandboxProfile::BrowserAgent => {
                        s_profile.allow_network = true;
                        s_profile.allow_fs_write = false;
                    }
                    _ => {
                        s_profile.allow_network = true;
                        s_profile.allow_fs_write = true;
                    }
                }
                return shared::sandbox::seatbelt::create_seatbelt_command_args(binary, &s_profile);
            }
        } else if cfg!(target_os = "linux") && !self.is_system_internal {
            let runsc_exists = Self::binary_exists_on_path("runsc");

            if runsc_exists && GLOBAL_SECURITY_CONFIG.use_runsc_sandbox {
                return build_runsc_args(
                    profile,
                    binary,
                    GLOBAL_SECURITY_CONFIG.enable_syscall_audit,
                );
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

        // Step 4-B: Defense-in-depth — SSOT harden_command_async
        shared::security::harden_command_async(&mut cmd);

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

/// A builder for creating safe tokio::process::Command instances using BastionGuard.
pub struct SafeCommandBuilder {
    program: String,
    args: Vec<String>,
    profile: SandboxProfile,
    envs: Vec<(String, String)>,
    env_passthroughs: Vec<String>,
}

impl SafeCommandBuilder {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            profile: SandboxProfile::Default,
            envs: Vec::new(),
            env_passthroughs: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self.args.push(arg.into());
        }
        self
    }

    pub fn profile(mut self, profile: SandboxProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn env<K, V>(mut self, key: K, val: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.envs.push((key.into(), val.into()));
        self
    }

    pub fn env_passthrough<K>(mut self, key: K) -> Self
    where
        K: Into<String>,
    {
        self.env_passthroughs.push(key.into());
        self
    }

    fn apply_envs(
        mut cmd: tokio::process::Command,
        envs: Vec<(String, String)>,
        passthroughs: Vec<String>,
    ) -> tokio::process::Command {
        for (k, v) in envs {
            cmd.env(k, v);
        }
        for k in passthroughs {
            if let Ok(val) = std::env::var(&k) {
                cmd.env(k, val);
            }
        }
        cmd
    }

    pub fn build_internal(self) -> Result<tokio::process::Command, AiomeError> {
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            allow_filesystem_write: true,
            allow_network: true,
            ..Default::default()
        };
        let guard = BastionGuard::new_internal(manifest);
        let cmd = guard.build_safe_command_args(&self.program, self.args, self.profile)?;
        Ok(Self::apply_envs(cmd, self.envs, self.env_passthroughs))
    }

    pub fn build(
        self,
        manifest: PermissionManifest,
    ) -> Result<tokio::process::Command, AiomeError> {
        let guard = BastionGuard::new(manifest);
        let cmd = guard.build_safe_command_args(&self.program, self.args, self.profile)?;
        Ok(Self::apply_envs(cmd, self.envs, self.env_passthroughs))
    }
}

pub(crate) fn build_runsc_args(
    profile: SandboxProfile,
    binary: &str,
    enable_syscall_audit: bool,
) -> (String, Vec<String>) {
    let mut args = Vec::new();
    if profile == SandboxProfile::Strict || profile == SandboxProfile::WasmRun {
        args.push("--network=none".to_string());
    }
    if enable_syscall_audit {
        args.push("--strace".to_string());
    }
    args.push("do".to_string());
    args.push(binary.to_string());
    ("runsc".to_string(), args)
}
