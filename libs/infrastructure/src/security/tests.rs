/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::bastion_guard::build_runsc_args;
use super::*;
use aiome_core::error::AiomeError;
use aiome_core::security::{PermissionManifest, RuntimeJail};

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
async fn test_build_safe_command_args_env_clear() {
    std::env::set_var("BASTION_SECRET_TEST", "super-secret-bastion-123");

    let manifest = PermissionManifest::default();
    let guard = BastionGuard::new_internal(manifest);

    let mut cmd = guard
        .build_safe_command_args(
            "python3",
            vec!["-c".into(), "import os; print(dict(os.environ))".into()],
            SandboxProfile::Default,
        )
        .expect("Failed to build command args");

    let output = cmd.output().await.expect("Failed to run command");

    std::env::remove_var("BASTION_SECRET_TEST");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stdout.contains("super-secret-bastion-123"),
        "Secret leaked to sandboxed child process! Stdout: {}",
        stdout
    );

    assert!(
        stdout.contains("'PATH'"),
        "PATH must be re-injected for processes to function correctly. Stdout: {}",
        stdout
    );
}

#[tokio::test]
async fn test_safe_command_builder_env_passthrough() {
    let secret_key = "BASTION_TEST_PASSTHROUGH_SECRET";
    std::env::set_var(secret_key, "passthrough-success-123"); // gitleaks:allow

    let mut cmd = SafeCommandBuilder::new("python3")
        .arg("-c")
        .arg("import os; print(os.environ.get('BASTION_TEST_PASSTHROUGH_SECRET', 'not_found'))")
        .env_passthrough(secret_key)
        .build_internal()
        .expect("Failed to build command args");

    let output = cmd.output().await.expect("Failed to run command");

    std::env::remove_var(secret_key);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("passthrough-success-123"),
        "env_passthrough failed to inject the requested variable. Stdout: {}",
        stdout
    );
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
async fn test_sandbox_profile_mcp_server() {
    std::env::set_var("CELL_ID", "test_cell");
    // Test that SandboxProfile::McpServer properly limits file access but allows network
    let manifest = PermissionManifest {
        allow_shell_execution: true,
        allow_network: true,
        allow_filesystem_write: false,
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);

    // This should use the new McpServer profile logic
    let _cmd = guard
        .build_safe_command_args("ls", vec![], SandboxProfile::McpServer)
        .expect("Failed to build command args");

    // We just ensure it builds successfully.
    // The actual runsc profile validation is complex to test without runsc,
    // but we verify the profile is accepted and creates a valid builder.
}

#[tokio::test]
async fn test_bastion_guard_disallow_shell() {
    let manifest = PermissionManifest {
        allow_shell_execution: false,
        ..Default::default()
    };
    let _guard = BastionGuard::new(manifest);
}

#[test]
fn test_syscall_audit_config_parsing() {
    let json = r#"{
        "allowed_binaries": [],
        "workspace_root": "/tmp",
        "enable_syscall_audit": true
    }"#;
    let config: SecurityConfig = serde_json::from_str(json).unwrap();
    assert!(config.enable_syscall_audit);

    let json_default = r#"{
        "allowed_binaries": [],
        "workspace_root": "/tmp"
    }"#;
    let config_default: SecurityConfig = serde_json::from_str(json_default).unwrap();
    assert!(!config_default.enable_syscall_audit);
}

#[test]
fn test_build_runsc_args() {
    let (cmd, args) = build_runsc_args(SandboxProfile::Default, "ls", true);
    assert_eq!(cmd, "runsc");
    assert_eq!(args, vec!["--strace", "do", "ls"]);

    let (cmd2, args2) = build_runsc_args(SandboxProfile::Strict, "ls", false);
    assert_eq!(cmd2, "runsc");
    assert_eq!(args2, vec!["--network=none", "do", "ls"]);

    let (cmd3, args3) = build_runsc_args(SandboxProfile::Strict, "ls", true);
    assert_eq!(cmd3, "runsc");
    assert_eq!(args3, vec!["--network=none", "--strace", "do", "ls"]);
}

#[tokio::test]
async fn test_bastion_guard_disallow_shell_real() {
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
        assert_eq!(res_internal.unwrap().trim(), "SENSITIVE_DATA");

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
    let temp = std::env::current_dir().unwrap().join("test_vault_tmp");
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
        // BoundaryVerifier のメッセージが含まれていることを確認するテスト。
        assert!(
            reason.contains("exceeds limit (64KB)"),
            "Expected BoundaryVerifier rejection, got: {}",
            reason
        );
    }
}

#[test]
fn test_bastion_check_network_empty_domains_deny() {
    let manifest = PermissionManifest {
        allow_network: true,
        allowed_domains: vec![],
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);
    assert!(guard.check_network("example.com").is_err());
    assert!(guard.check_network("https://example.com/path").is_err());
}

#[test]
fn test_bastion_check_network_bare_host_and_url_ok() {
    let manifest = PermissionManifest {
        allow_network: true,
        allowed_domains: vec!["example.com".into()],
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);
    assert!(guard.check_network("example.com").is_ok());
    assert!(guard.check_network("https://api.example.com/v1").is_ok());
    assert!(guard.check_network("https://evil.com").is_err());
}

#[test]
fn test_bastion_check_network_substring_no_longer_bypasses() {
    // Old `url.contains(domain)` would allow "notexample.com" when domain is "example.com".
    let manifest = PermissionManifest {
        allow_network: true,
        allowed_domains: vec!["example.com".into()],
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);
    assert!(guard.check_network("notexample.com").is_err());
}

#[test]
fn test_bastion_check_network_wildcard_and_wasm_star_skip_contract() {
    let manifest = PermissionManifest {
        allow_network: true,
        allowed_domains: vec!["*".into()],
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);
    // Bastion honors `*` (Code Mode parity). WASM registration still skips `*` (skills/mod.rs).
    assert!(guard.check_network("anywhere.test").is_ok());
}

#[test]
fn test_bastion_check_network_target_resolution_edges() {
    let manifest = PermissionManifest {
        allow_network: true,
        allowed_domains: vec!["example.com".into()],
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);
    assert!(guard.check_network("  example.com  ").is_ok());
    assert!(guard.check_network("http://example.com/path").is_ok());
    assert!(guard.check_network("https://api.example.com").is_ok());
    // Non-http(s) schemes are Fail-Closed → deny
    assert!(guard.check_network("ftp://example.com").is_err());
    assert!(guard.check_network("http://").is_err());
    assert!(guard.check_network("").is_err());
}

#[test]
fn test_bastion_check_network_fail_closed_hostile_targets() {
    let manifest = PermissionManifest {
        allow_network: true,
        allowed_domains: vec!["example.com".into()],
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);

    assert!(guard.check_network("ftp://evil.com/.example.com").is_err());
    assert!(guard.check_network("allowed.com@evil.com").is_err());
    assert!(guard.check_network("example.com@evil.com").is_err());
    assert!(guard.check_network("example.com:443").is_err());
    assert!(guard.check_network("example.com/path").is_err());
    assert!(guard.check_network("example.com?x=1").is_err());
    assert!(guard.check_network("example.com#frag").is_err());
}
