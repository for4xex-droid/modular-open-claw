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
    #[allow(dead_code)]
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
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tempfile::tempdir;

    struct MockActor {
        fail_count: std::sync::atomic::AtomicUsize,
        security_violation: bool,
    }

    #[async_trait]
    impl AgentAct for MockActor {
        type Input = ();
        type Output = String;

        async fn execute(&self, _input: Self::Input, _jail: &Jail) -> Result<Self::Output, FactoryError> {
            if self.security_violation {
                return Err(FactoryError::SecurityViolation { reason: "test violation".into() });
            }

            let count = self.fail_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < 2 {
                Err(FactoryError::Infrastructure { reason: "temporary failure".into() })
            } else {
                Ok("success".into())
            }
        }
    }

    #[tokio::test]
    async fn test_supervisor_retry_policy() {
        let dir = tempdir().unwrap();
        let jail = Arc::new(Jail::init(dir.path()).unwrap());
        let supervisor = Supervisor::new(jail, SupervisorPolicy::Retry { max_retries: 3 });
        
        let actor = MockActor {
            fail_count: std::sync::atomic::AtomicUsize::new(0),
            security_violation: false,
        };

        let result = supervisor.enforce_act(&actor, ()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(actor.fail_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_supervisor_security_escalation() {
        let dir = tempdir().unwrap();
        let jail = Arc::new(Jail::init(dir.path()).unwrap());
        let supervisor = Supervisor::new(jail, SupervisorPolicy::Retry { max_retries: 3 });
        
        let actor = MockActor {
            fail_count: std::sync::atomic::AtomicUsize::new(0),
            security_violation: true,
        };

        let result = supervisor.enforce_act(&actor, ()).await;
        assert!(matches!(result, Err(FactoryError::SecurityViolation { .. })));
    }
}
