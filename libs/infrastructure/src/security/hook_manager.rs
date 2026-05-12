/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmRequest, LlmResponse};
use aiome_core_contracts::security::AgentHook;
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct HookManager {
    hooks: RwLock<Vec<Arc<dyn AgentHook>>>,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    pub fn add_hook(&self, hook: Arc<dyn AgentHook>) {
        match self.hooks.write() {
            Ok(mut hooks) => hooks.push(hook),
            Err(e) => {
                tracing::error!(
                    "⛔ [HookManager] Failed to acquire write lock for add_hook: {}",
                    e
                );
            }
        }
    }

    pub async fn trigger_pre_execute(&self, request: &LlmRequest) -> Result<(), AiomeError> {
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!(
                    "⛔ [HookManager] RwLock poisoned in trigger_pre_execute: {}",
                    e
                );
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            hook.on_pre_execute(request).await?;
        }
        Ok(())
    }

    pub async fn trigger_post_execute(
        &self,
        request: &LlmRequest,
        response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!(
                    "⛔ [HookManager] RwLock poisoned in trigger_post_execute: {}",
                    e
                );
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            hook.on_post_execute(request, response).await?;
        }
        Ok(())
    }

    /// ジョブ完了時のフック通知（ベストエフォート）。
    /// `on_pre_execute` と異なり、1つのフックの失敗が残りのフックをブロックしない。
    pub async fn trigger_job_completed(
        &self,
        job_id: &str,
        status: &str,
    ) -> Result<(), AiomeError> {
        let mut last_error: Option<AiomeError> = None;
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!(
                    "⛔ [HookManager] RwLock poisoned in trigger_job_completed: {}",
                    e
                );
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            if let Err(e) = hook.on_job_completed(job_id, status).await {
                tracing::warn!(
                    "⚠️ [HookManager] Hook {:?} failed on job_completed({}): {}",
                    hook,
                    job_id,
                    e
                );
                last_error = Some(e);
            }
        }
        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// OxiLean等の形式検証完了時のフック通知（ベストエフォート）。
    /// Nurture側のKarmaForge等へ証明力を伝搬するために使用される。
    pub async fn trigger_proof_completed(
        &self,
        skill_name: &str,
        is_valid: bool,
    ) -> Result<(), AiomeError> {
        let mut last_error: Option<AiomeError> = None;
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!(
                    "⛔ [HookManager] RwLock poisoned in trigger_proof_completed: {}",
                    e
                );
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            if let Err(e) = hook.on_proof_completed(skill_name, is_valid).await {
                tracing::warn!(
                    "⚠️ [HookManager] Hook {:?} failed on proof_completed({}): {}",
                    hook,
                    skill_name,
                    e
                );
                last_error = Some(e);
            }
        }
        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// ユーザーへのツール実行許可要求を全フックに伝搬する。
    /// **注意**: 1つのフックが `false` を返しても他のフックへの問い合わせを続行し、
    /// 最終的に「全員が許可した場合のみ `true`」を返す（保守的なAND結合）。
    pub async fn trigger_permission_request(
        &self,
        tool: &str,
        reason: &str,
    ) -> Result<bool, AiomeError> {
        let mut allowed = true;
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!(
                    "⛔ [HookManager] RwLock poisoned in trigger_permission_request: {}",
                    e
                );
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            if !hook.on_permission_request(tool, reason).await? {
                allowed = false;
            }
        }
        Ok(allowed)
    }

    /// セッション開始時のフック通知。
    pub async fn trigger_session_start(&self) -> Result<(), AiomeError> {
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!(
                    "⛔ [HookManager] RwLock poisoned in trigger_session_start: {}",
                    e
                );
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            hook.on_session_start().await?;
        }
        Ok(())
    }

    /// 停止時のフック通知。
    pub async fn trigger_stop(&self, reason: &str) -> Result<(), AiomeError> {
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!("⛔ [HookManager] RwLock poisoned in trigger_stop: {}", e);
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            hook.on_stop(reason).await?;
        }
        Ok(())
    }

    /// 決済イベント完了時のフック通知（ベストエフォート）。
    /// Nurture側のKarmaForge等へ経済的貢献を伝搬するために使用される。
    pub async fn trigger_transaction_completed(
        &self,
        source: &str,
        amount_cents: i64,
        actor_id: &str,
        transaction_id: &str,
    ) -> Result<(), AiomeError> {
        let mut last_error: Option<AiomeError> = None;
        let hooks = self
            .hooks
            .read()
            .map_err(|e| {
                tracing::error!(
                    "⛔ [HookManager] RwLock poisoned in trigger_transaction_completed: {}",
                    e
                );
                AiomeError::Infrastructure {
                    reason: "HookManager RwLock poisoned".to_string(),
                }
            })?
            .clone();
        for hook in hooks {
            if let Err(e) = hook
                .on_transaction_completed(source, amount_cents, actor_id, transaction_id)
                .await
            {
                tracing::warn!(
                    "⚠️ [HookManager] Hook {:?} failed on transaction_completed({}): {}",
                    hook,
                    transaction_id,
                    e
                );
                last_error = Some(e);
            }
        }
        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::llm::LlmMessage;

    #[derive(Debug)]
    struct MockHook {
        pre_called: std::sync::atomic::AtomicBool,
        job_completed_called: std::sync::atomic::AtomicBool,
        proof_completed_called: std::sync::atomic::AtomicBool,
        transaction_completed_called: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl AgentHook for MockHook {
        async fn on_pre_execute(&self, _request: &LlmRequest) -> Result<(), AiomeError> {
            self.pre_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        async fn on_post_execute(
            &self,
            _request: &LlmRequest,
            _response: &LlmResponse,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn on_job_completed(&self, _job_id: &str, _status: &str) -> Result<(), AiomeError> {
            self.job_completed_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        async fn on_proof_completed(
            &self,
            _skill_name: &str,
            _is_valid: bool,
        ) -> Result<(), AiomeError> {
            self.proof_completed_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        async fn on_transaction_completed(
            &self,
            _source: &str,
            _amount_cents: i64,
            _actor_id: &str,
            _transaction_id: &str,
        ) -> Result<(), AiomeError> {
            self.transaction_completed_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_hook_manager_executes_hooks() {
        let manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
            proof_completed_called: std::sync::atomic::AtomicBool::new(false),
            transaction_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        let request = LlmRequest {
            messages: vec![LlmMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                cache: false,
            }],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };

        manager
            .trigger_pre_execute(&request)
            .await
            .expect("Hook should pass");
        assert!(hook.pre_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_hook_manager_executes_job_completed_hooks() {
        let manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
            proof_completed_called: std::sync::atomic::AtomicBool::new(false),
            transaction_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        manager
            .trigger_job_completed("job-42", "completed")
            .await
            .expect("Hook should pass");
        assert!(hook
            .job_completed_called
            .load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_hook_manager_executes_proof_completed_hooks() {
        let manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
            proof_completed_called: std::sync::atomic::AtomicBool::new(false),
            transaction_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        manager
            .trigger_proof_completed("test_skill", true)
            .await
            .expect("Hook should pass");
        assert!(hook
            .proof_completed_called
            .load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_hook_manager_executes_transaction_completed_hooks() {
        let manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
            proof_completed_called: std::sync::atomic::AtomicBool::new(false),
            transaction_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        manager
            .trigger_transaction_completed("polar", 1000, "actor-1", "tx-42")
            .await
            .expect("Hook should pass");
        assert!(hook
            .transaction_completed_called
            .load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_hook_manager_permission_request_default_allows() {
        let manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
            proof_completed_called: std::sync::atomic::AtomicBool::new(false),
            transaction_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        let result = manager
            .trigger_permission_request("run_command", "needs approval")
            .await
            .expect("Hook should pass");
        assert!(result, "Default implementation should allow");
    }

    #[tokio::test]
    async fn test_hook_manager_session_start() {
        let manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
            proof_completed_called: std::sync::atomic::AtomicBool::new(false),
            transaction_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        manager
            .trigger_session_start()
            .await
            .expect("Session start hook should pass");
    }

    #[tokio::test]
    async fn test_hook_manager_stop() {
        let manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
            proof_completed_called: std::sync::atomic::AtomicBool::new(false),
            transaction_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        manager
            .trigger_stop("shutdown")
            .await
            .expect("Stop hook should pass");
    }
}
