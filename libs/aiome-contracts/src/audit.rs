use async_trait::async_trait;

/// 共通監査ロガーインターフェース
#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// ログを安全に記録し、必要に応じて永続化する
    async fn log_event(
        &self,
        event_type: &str,
        actor: &str,
        details: &serde_json::Value,
    ) -> anyhow::Result<()>;

    /// セキュリティ違反や不正アクセスの記録
    async fn log_violation(
        &self,
        violation_type: &str,
        description: &str,
        context: &serde_json::Value,
    ) -> anyhow::Result<()>;
}
