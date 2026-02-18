//! # The Governance — 統治機構 (Supervisor)
//!
//! 憲法第3条に基づき、アクターの実行を監視し、失敗や法規違反を制御する。

use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use bastion::fs_guard::Jail;
use std::sync::Arc;

/// 監視ポリシー
#[derive(Debug, Clone)]
pub enum SupervisorPolicy {
    /// 失敗時に即座に停止 (Deny)
    Strict,
    /// 失敗をログに記録して継続試行 (Retry)
    Retry { max_retries: usize },
}

/// 統治機構（スーパーバイザー）
pub struct Supervisor {
    jail: Arc<Jail>,
    policy: SupervisorPolicy,
}

impl Supervisor {
    pub fn new(jail: Arc<Jail>, policy: SupervisorPolicy) -> Self {
        Self { jail, policy }
    }

    /// アクターを「法」の下で実行する
    pub async fn enforce_act<A>(&self, actor: &A, input: A::Input) -> Result<A::Output, FactoryError>
    where
        A: AgentAct,
    {
        tracing::info!("⚖️  Enforcing act for actor: {}", std::any::type_name::<A>());

        let mut retries = 0;
        loop {
            match actor.execute(input.clone(), &self.jail).await {
                Ok(output) => {
                    tracing::info!("✅ Act completed successfully");
                    return Ok(output);
                }
                Err(e) => {
                    tracing::error!("🚨 Act failed: {}", e);

                    // セキュリティ違反はポリシーに関わらず即座にエスカレーション
                    if matches!(e, FactoryError::SecurityViolation { .. }) {
                        tracing::error!("⛔ SECURITY VIOLATION detected. Escalating...");
                        return Err(e);
                    }

                    match &self.policy {
                        SupervisorPolicy::Strict => return Err(e),
                        SupervisorPolicy::Retry { max_retries } => {
                            if retries < *max_retries {
                                retries += 1;
                                tracing::warn!("🔄 Retrying act ({}/{})", retries, max_retries);
                                continue;
                            } else {
                                tracing::error!("❌ Max retries reached. Failing act.");
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }
    }
}
