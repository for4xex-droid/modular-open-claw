use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// ツール呼び出しのセキュリティポリシー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// 許可されたツール名のホワイトリスト
    pub allowed_tools: HashSet<String>,
    /// 許可されたネットワーク通信先のホワイトリスト
    pub allowed_hosts: HashSet<String>,
    /// 外部Skillのインストールを禁止するフラグ
    pub block_external_skills: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        let mut allowed_tools = HashSet::new();
        allowed_tools.insert("trend_sonar".to_string());
        allowed_tools.insert("comfy_bridge".to_string());
        allowed_tools.insert("media_forge".to_string());
        allowed_tools.insert("factory_log".to_string());

        let mut allowed_hosts = HashSet::new();
        // Ollama (LLM)
        allowed_hosts.insert("127.0.0.1:11434".to_string());
        allowed_hosts.insert("localhost:11434".to_string());
        // ComfyUI (画像/動画生成)
        allowed_hosts.insert("127.0.0.1:8188".to_string());
        allowed_hosts.insert("localhost:8188".to_string());

        Self {
            allowed_tools,
            allowed_hosts,
            block_external_skills: true, // デフォルトで外部Skill禁止
        }
    }
}

impl SecurityPolicy {
    /// ツール名がホワイトリストに含まれているか検証
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.allowed_tools.contains(tool_name)
    }

    /// 通信先がホワイトリストに含まれているか検証
    pub fn is_host_allowed(&self, host: &str) -> bool {
        self.allowed_hosts.contains(host)
    }

    /// ホワイトリストにツールを追加（開発者が明示的に行う）
    pub fn register_tool(&mut self, tool_name: impl Into<String>) {
        self.allowed_tools.insert(tool_name.into());
    }

    /// ホワイトリストに通信先を追加（開発者が明示的に行う）
    pub fn allow_host(&mut self, host: impl Into<String>) {
        self.allowed_hosts.insert(host.into());
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
        assert!(policy.is_tool_allowed("trend_sonar"));
        assert!(policy.is_tool_allowed("comfy_bridge"));
        assert!(policy.is_tool_allowed("media_forge"));
        assert!(policy.is_tool_allowed("factory_log"));
    }

    #[test]
    fn test_default_policy_blocks_unknown_tools() {
        let policy = SecurityPolicy::default();
        assert!(!policy.is_tool_allowed("malicious_skill"));
        assert!(!policy.is_tool_allowed("shell_exec"));
        assert!(!policy.is_tool_allowed("file_delete"));
    }

    #[test]
    fn test_default_policy_allows_local_hosts() {
        let policy = SecurityPolicy::default();
        assert!(policy.is_host_allowed("127.0.0.1:11434"));
        assert!(policy.is_host_allowed("127.0.0.1:8188"));
    }

    #[test]
    fn test_default_policy_blocks_external_hosts() {
        let policy = SecurityPolicy::default();
        assert!(!policy.is_host_allowed("evil-server.com:443"));
        assert!(!policy.is_host_allowed("1.2.3.4:9999"));
    }

    #[test]
    fn test_external_skills_blocked_by_default() {
        let policy = SecurityPolicy::default();
        assert!(policy.block_external_skills);
    }

    #[test]
    fn test_register_new_tool() {
        let mut policy = SecurityPolicy::default();
        assert!(!policy.is_tool_allowed("tiktok_uploader"));
        policy.register_tool("tiktok_uploader");
        assert!(policy.is_tool_allowed("tiktok_uploader"));
    }
}
