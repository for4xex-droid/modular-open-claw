/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use anyhow::{bail, Result};
use bastion::net_guard::ShieldClient;
use serde::{Deserialize, Serialize};

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

        let shield = builder.build().expect("Failed to build network shield");

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

/// 監査ログのエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: AuditAction,
    pub tool_name: String,
    pub detail: String,
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(policy.validate_url("http://127.0.0.1:8188").await.is_ok());
        assert!(policy.validate_url("http://localhost:11434").await.is_ok());
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
        assert!(policy.validate_url("http://1.2.3.4:9999").await.is_err());
        Ok(())
    }

    #[test]
    fn test_register_new_tool() {
        let mut policy = SecurityPolicy::default();
        assert!(policy.validate_tool("external_api_client").is_err());
        policy.register_tool("external_api_client");
        assert!(policy.validate_tool("external_api_client").is_ok());
    }
}
