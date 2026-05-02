/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use anyhow::{bail, Result};
use bastion::net_guard::ShieldClient;
use serde::{Deserialize, Serialize};

/// Defense-in-depth: `env_clear()` 後に再注入する必須環境変数の SSOT ホワイトリスト。
///
/// **この定数が唯一の正規定義 (Single Source of Truth)** です。
/// `infrastructure::security::PROCESS_SAFE_ENV_VARS` はこの定数を re-export します。
/// `shared` クレート内の全 `Command::new` 呼び出しは [`harden_command`] ヘルパーを使用し、
/// この定数を直接参照します。
///
/// # 含まれる変数
/// - `PATH` / `HOME` / `LANG` / `TMPDIR`: OS 基本動作に必須
/// - `PYTHONPATH` / `VIRTUAL_ENV`: Python venv 互換性 (mlx-lm / pip 等)
pub const PROCESS_SAFE_ENV_VARS: &[&str] = &[
    "PATH",
    "HOME",
    "LANG",
    "TMPDIR",
    "USER",
    "LOGNAME",
    // Python venv 互換性: mlx-lm / pip 等がパッケージ解決に使用
    "PYTHONPATH",
    "VIRTUAL_ENV",
];

/// `std::process::Command` に対して環境変数の安全な隔離処理を適用する。
///
/// 1. `env_clear()` で親プロセスの全環境変数を除去
/// 2. [`PROCESS_SAFE_ENV_VARS`] に定義された変数のみを再注入
///
/// # Usage
/// ```ignore
/// let mut cmd = std::process::Command::new("git");
/// cmd.arg("status");
/// shared::security::harden_command(&mut cmd);
/// ```
pub fn harden_command(cmd: &mut std::process::Command) {
    cmd.env_clear();
    for var_name in PROCESS_SAFE_ENV_VARS {
        if let Ok(val) = std::env::var(var_name) {
            cmd.env(var_name, val);
        }
    }
}

/// `tokio::process::Command` に対して環境変数の安全な隔離処理を適用する。
///
/// [`harden_command`] の非同期版。`security_zombie::run_with_timeout` や
/// `BastionGuard::build_safe_command_args` 等、`tokio::process::Command` を
/// 使用する箇所で呼び出す。
///
/// # Usage
/// ```ignore
/// let mut cmd = tokio::process::Command::new("git");
/// cmd.arg("status");
/// shared::security::harden_command_async(&mut cmd);
/// ```
pub fn harden_command_async(cmd: &mut tokio::process::Command) {
    cmd.env_clear();
    for var_name in PROCESS_SAFE_ENV_VARS {
        if let Ok(val) = std::env::var(var_name) {
            cmd.env(var_name, val);
        }
    }
}

/// 工場のセキュリティポリシー
///
/// 許可されたホスト、ツール、リソースへのアクセスを制御する。
/// Bastion ShieldClient を使用して SSRF や DNS Rebinding を防止する。
#[derive(Clone, Debug)]
pub struct SecurityPolicy {
    network_shield: ShieldClient,
    allowed_tools: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::default_production()
    }
}

impl SecurityPolicy {
    /// デフォルトのポリシーを作成
    ///
    /// デフォルトでは以下を許可：
    /// - Localhost (127.0.0.1)
    /// - Test Node (8188)
    /// - Ollama (11434)
    pub fn default_production() -> Self {
        let mut builder = ShieldClient::builder()
            .allow_endpoint("127.0.0.1")
            .allow_endpoint("localhost")
            .allow_endpoint("::1")
            .allow_endpoint("[::1]")
            .allow_endpoint("trends.google.co.jp")
            .block_private_ips(true); // プライベートIPへのSSRFを防止（Allowlist以外）

        // Biome SSRF Defense: ホワイトリストを環境変数から自動登録
        if let Ok(whitelist) = std::env::var("BIOME_HUB_WHITELIST") {
            for endpoint in whitelist.split(',') {
                let trimmed = endpoint.trim();
                if !trimmed.is_empty() {
                    builder = builder.allow_endpoint(trimmed);
                }
            }
        }

        let shield = builder.build().expect("Failed to build network shield"); // allow-anti-pattern

        Self {
            network_shield: shield,
            allowed_tools: vec![
                "test_skill".to_string(),
                "task_processor".to_string(),
                "trend_sonar".to_string(),
                "aiome_log".to_string(),
                "fs_reader".to_string(),
                "fs_writer".to_string(),
                "terminal_exec".to_string(),
                "skill_tester".to_string(),
                "mcp_bridge".to_string(),
                "lora_trainer".to_string(),
                "lora_inspector".to_string(),
                "tts_generator".to_string(),
                "voice_profile_manager".to_string(),
                "voice_commercial_escrow".to_string(),
                "cci_journalist".to_string(),
                "cci_editor_ai".to_string(),
            ],
        }
    }

    /// 新しいエンドポイントを動的に許可する
    pub fn allow_endpoint(&mut self, endpoint: &str) {
        // ShieldClient は immutable なので、動的追加は builder の再構築が必要だが、
        // 現状の bastion::net_guard::ShieldClient の仕様に合わせて、
        // 必要なら新しいインスタンスを作成して入れ替える。
        // ※ bastion-oss の実装を確認する。
        // ここでは一旦、構築済みの shield を返す設計にするか、
        // 構築時に一括指定する形を推奨する。
    }

    /// ShieldClient への参照を取得 (内部利用用)
    pub fn shield(&self) -> &ShieldClient {
        &self.network_shield
    }

    /// URLの安全性を検証する
    pub async fn validate_url(&self, url: &str) -> Result<()> {
        let url_obj =
            reqwest::Url::parse(url).map_err(|e| anyhow::anyhow!("Invalid URL: {}", e))?;

        // ローカルホストの場合、ポート制限を独自に適用 (8188, 11434 のみ許可)
        if let Some(host) = url_obj.host_str() {
            let is_loopback = host == "127.0.0.1"
                || host == "localhost"
                || host == "::1"
                || host == "0:0:0:0:0:0:0:1"
                || host == "[::1]";

            if is_loopback {
                let port = url_obj.port().unwrap_or(80);
                if port != 8188 && port != 11434 {
                    bail!("Security Violation: Unauthorized local port {}", port);
                }
            }
        }

        self.network_shield
            .validate_url(url)
            .await
            .map_err(|e| anyhow::anyhow!("Security Violation: {}", e))
    }

    /// ツールの実行が許可されているか検証する
    pub fn validate_tool(&self, tool_name: &str) -> Result<()> {
        if self.allowed_tools.contains(&tool_name.to_string()) {
            Ok(())
        } else {
            bail!(
                "Access Denied: Tool '{}' is not in the allowed list",
                tool_name
            )
        }
    }

    /// 新しいツールを動的に許可する (管理者用)
    pub fn register_tool(&mut self, tool_name: &str) {
        if !self.allowed_tools.contains(&tool_name.to_string()) {
            self.allowed_tools.push(tool_name.to_string());
        }
    }
}

/// 定数時間での suffix 比較（タイミング攻撃対策）
///
/// トークンが期待するサフィックスで終わっているかを、`subtle::ConstantTimeEq` で検証する。
/// サフィックス部分のバイト比較は定数時間で実行される。
///
/// # Timing residual
///
/// 長さチェック (`token.len() < expected.len()`) は **非定数時間** であり、
/// トークンが期待値より短いことを攻撃者に漏洩する可能性がある。
/// ただし gRPC の `Bearer <token>` ヘッダ構造から token 長は推測可能であるため、
/// 追加のリスクは実質ゼロと判断する。
///
/// # Edge cases
///
/// - `expected_suffix` が空の場合: セキュリティ上 **常に `false`** を返す（空トークン認可の防止）。
/// - `token` が空で `expected_suffix` も空の場合: 同様に `false`。
pub fn constant_time_ends_with(token: &str, expected_suffix: &str) -> bool {
    let token_bytes = token.as_bytes();
    let expected_bytes = expected_suffix.as_bytes();

    // 空の expected_suffix は認可バイパスになるため無条件で拒否。
    if expected_bytes.is_empty() {
        return false;
    }

    if token_bytes.len() < expected_bytes.len() {
        return false;
    }

    let suffix_bytes = &token_bytes[token_bytes.len() - expected_bytes.len()..];
    use subtle::ConstantTimeEq;
    suffix_bytes.ct_eq(expected_bytes).into()
}

/// 監査ログのエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// action
    pub action: AuditAction,
    /// tool_name
    pub tool_name: String,
    /// detail
    pub detail: String,
    /// allowed
    pub allowed: bool,
}

/// 監査対象のアクション種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    /// ツール呼び出し
    ToolInvocation,
    /// ネットワーク通信
    NetworkRequest,
    /// 外部Skillインストール試行（常にブロック）
    ExternalSkillBlocked,
}

/// 環境変数を安全に消去する (§CISO-1)
///
/// `forbid(unsafe_code)` な crate からも呼び出し可能。
/// 起動時の初期化フェーズ (tokio ランタイム起動前) でのみ使用すること。
///
/// # Why unsafe?
///
/// `std::env::remove_var` は Rust 2024 Edition で `unsafe fn` に昇格。
/// マルチスレッド環境での環境変数操作がスレッドセーフでないため。
/// 本関数は起動直後の単一スレッドフェーズで呼ばれることを前提とする。
///
/// # Note
///
/// 本クレート唯一の `#[allow(unsafe_code)]` 例外。
/// 新規追加にはコードレビューでの承認を必須とすること。
#[allow(unsafe_code)]
pub fn scrub_env(key: &str) {
    #[allow(deprecated)]
    // SAFETY: 起動時の単一スレッドフェーズでのみ呼び出される。
    // tokio ランタイム起動前に全シークレット変数を消去する設計。
    unsafe {
        std::env::remove_var(key);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ───── harden_command tests (Verification Protocol: Positive + Negative) ─────

    #[test]
    fn test_harden_command_injects_path() {
        // Positive: PATH が再注入されていることを確認
        let mut cmd = std::process::Command::new("env");
        harden_command(&mut cmd);
        let output = cmd.output().expect("failed to run env");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("PATH="),
            "harden_command must re-inject PATH. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_harden_command_blocks_secret_leakage() {
        // Negative Injection: 任意の秘密変数が子プロセスに漏洩しないことを確認
        let secret_key = "HARDEN_CMD_TEST_SECRET_XYZ_987";
        #[allow(unsafe_code, deprecated)]
        unsafe {
            std::env::set_var(secret_key, "leak-canary-value");
        }

        let mut cmd = std::process::Command::new("env");
        harden_command(&mut cmd);
        let output = cmd.output().expect("failed to run env");
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Revert
        #[allow(unsafe_code, deprecated)]
        unsafe {
            std::env::remove_var(secret_key);
        }

        assert!(
            !stdout.contains("leak-canary-value"),
            "Secret '{}' leaked through harden_command! Stdout: {}",
            secret_key,
            stdout
        );
    }

    #[test]
    fn test_harden_command_only_injects_ssot_vars() {
        // PROCESS_SAFE_ENV_VARS 以外の変数が一切注入されないことを確認
        let mut cmd = std::process::Command::new("env");
        harden_command(&mut cmd);
        let output = cmd.output().expect("failed to run env");
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if let Some(key) = line.split('=').next() {
                assert!(
                    PROCESS_SAFE_ENV_VARS.contains(&key),
                    "Unexpected env var '{}' leaked through harden_command. Only SSOT vars should be present.",
                    key
                );
            }
        }
    }

    #[tokio::test]
    async fn test_harden_command_async_injects_path() {
        let mut cmd = tokio::process::Command::new("env");
        harden_command_async(&mut cmd);
        let output = cmd.output().await.expect("failed to run env");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("PATH="),
            "harden_command_async must re-inject PATH. Got: {}",
            stdout
        );
    }

    #[tokio::test]
    async fn test_harden_command_async_blocks_secret_leakage() {
        let secret_key = "HARDEN_ASYNC_TEST_SECRET_ABC_456";
        #[allow(unsafe_code, deprecated)]
        unsafe {
            std::env::set_var(secret_key, "async-leak-canary");
        }

        let mut cmd = tokio::process::Command::new("env");
        harden_command_async(&mut cmd);
        let output = cmd.output().await.expect("failed to run env");
        let stdout = String::from_utf8_lossy(&output.stdout);

        #[allow(unsafe_code, deprecated)]
        unsafe {
            std::env::remove_var(secret_key);
        }

        assert!(
            !stdout.contains("async-leak-canary"),
            "Secret '{}' leaked through harden_command_async! Stdout: {}",
            secret_key,
            stdout
        );
    }

    #[test]
    fn test_default_policy_allows_registered_tools() {
        let policy = SecurityPolicy::default();
        assert!(policy.validate_tool("trend_sonar").is_ok());
        assert!(policy.validate_tool("test_skill").is_ok());
        assert!(policy.validate_tool("task_processor").is_ok());
        assert!(policy.validate_tool("aiome_log").is_ok());
        assert!(policy.validate_tool("fs_reader").is_ok());
        assert!(policy.validate_tool("fs_writer").is_ok());
        assert!(policy.validate_tool("terminal_exec").is_ok());
        assert!(policy.validate_tool("skill_tester").is_ok());
        assert!(policy.validate_tool("mcp_bridge").is_ok());
    }

    #[test]
    fn test_default_policy_blocks_unknown_tools() {
        let policy = SecurityPolicy::default();
        assert!(policy.validate_tool("malicious_skill").is_err());
        assert!(policy.validate_tool("shell_exec").is_err());
    }

    #[tokio::test]
    async fn test_default_policy_allows_local_hosts() -> Result<()> {
        let policy = SecurityPolicy::default();
        assert!(policy.validate_url("http://127.0.0.1:8188").await.is_ok()); // allow-anti-pattern
        assert!(policy.validate_url("http://localhost:11434").await.is_ok()); // allow-anti-pattern
        assert!(policy.validate_url("http://[::1]:8188").await.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_default_policy_blocks_ipv6_loopback_unauthorized_port() -> Result<()> {
        let policy = SecurityPolicy::default();
        assert!(policy.validate_url("http://[::1]:22").await.is_err());
        assert!(policy
            .validate_url("http://[0:0:0:0:0:0:0:1]:22")
            .await
            .is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_default_policy_blocks_external_hosts() -> Result<()> {
        let policy = SecurityPolicy::default();
        // Bastion ShieldClient はデフォルトで private IP 以外をブロック (Allowlistにない場合)
        assert!(policy
            .validate_url("http://evil-server.com:443")
            .await
            .is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_default_policy_blocks_metadata_srv() -> Result<()> {
        let policy = SecurityPolicy::default();
        // AWS Metadata Service for SSRF
        assert!(policy
            .validate_url("http://169.254.169.254/latest/meta-data/")
            .await
            .is_err());
        assert!(policy.validate_url("http://10.0.0.1/admin").await.is_err());
        assert!(policy
            .validate_url("http://192.168.1.1/setup")
            .await
            .is_err());
        Ok(())
    }

    #[test]
    fn test_register_new_tool() {
        let mut policy = SecurityPolicy::default();
        assert!(policy.validate_tool("external_api_client").is_err());
        policy.register_tool("external_api_client");
        assert!(policy.validate_tool("external_api_client").is_ok());
    }

    #[test]
    fn test_scrub_env_removes_variable() {
        // Arrange: テスト用の一意なキーを設定
        let test_key = "TEST_SCRUB_ENV_KEY_12345";
        #[allow(unsafe_code, deprecated)]
        unsafe {
            std::env::set_var(test_key, "super_secret_value");
        }
        assert_eq!(std::env::var(test_key).unwrap(), "super_secret_value");

        // Act: scrub_env で消去
        crate::security::scrub_env(test_key);

        // Assert: 環境変数が消えていること
        assert!(
            std::env::var(test_key).is_err(),
            "Environment variable should be removed after scrub_env"
        );
    }

    #[test]
    fn test_scrub_env_nonexistent_key_does_not_panic() {
        // 存在しないキーに対して呼んでもパニックしないこと
        crate::security::scrub_env("ABSOLUTELY_NONEXISTENT_KEY_999");
    }

    #[test]
    fn test_constant_time_ends_with() {
        assert!(crate::security::constant_time_ends_with(
            "Bearer mysecrettoken123",
            "mysecrettoken123"
        ));
        assert!(crate::security::constant_time_ends_with(
            "mysecrettoken123",
            "mysecrettoken123"
        ));
        assert!(!crate::security::constant_time_ends_with(
            "Bearer wrongtoken",
            "mysecrettoken123"
        ));
        assert!(!crate::security::constant_time_ends_with(
            "short",
            "mysecrettoken123"
        ));
        assert!(!crate::security::constant_time_ends_with(
            "Bearer mysecrettoken12",
            "mysecrettoken123"
        ));
    }

    #[test]
    fn test_constant_time_ends_with_edge_cases() {
        // 空の expected_suffix は認可バイパスになるため常に false
        assert!(!crate::security::constant_time_ends_with("anything", ""));
        assert!(!crate::security::constant_time_ends_with("", ""));
        // token 空 + 非空 suffix
        assert!(!crate::security::constant_time_ends_with("", "a"));
        // 完全一致
        assert!(crate::security::constant_time_ends_with("abc", "abc"));
        // 1文字
        assert!(crate::security::constant_time_ends_with("abc", "c"));
        assert!(!crate::security::constant_time_ends_with("abc", "b"));
    }
}
