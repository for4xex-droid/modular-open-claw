/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::state::SharedState;
use async_trait::async_trait;
use nurture_bridge::commerce::CommerceEngine;
use nurture_bridge::error::AiomeError;
use nurture_bridge::plugin::AgentHook;
use nurture_bridge::plugin::AiomePlugin;
use nurture_bridge::traits::JobQueue;
use nurture_infra::economy::bridge::NurtureCommerceBridge;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct NurturePlugin {
    state: SharedState,
    bridge: Arc<NurtureCommerceBridge>,
    _cancel_token: CancellationToken,
}

#[async_trait]
impl AiomePlugin for NurturePlugin {
    fn name(&self) -> &str {
        "nurture"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn routes(&self) -> Option<nurture_bridge::plugin::OpaqueRouter> {
        Some(Box::new(crate::routes::nurture_routes(self.state.clone())))
    }

    fn registered_tools(&self) -> Vec<String> {
        vec![
            "marketplace_search".to_string(),
            "marketplace_buy".to_string(),
            "marketplace_upload".to_string(), // Unlocked for Sprint-C (CSAM+DRM ready)
            "wallet_balance".to_string(),
            "sandbox_exec".to_string(),
        ]
    }

    fn required_env_vars(&self) -> Vec<String> {
        vec![
            "STRIPE_WEBHOOK_SECRET".to_string(),
            "AIOME_C2PA_SIGNING_KEY".to_string(),
        ]
    }

    fn commerce_engine(&self) -> Option<Arc<dyn CommerceEngine>> {
        Some(self.bridge.clone() as Arc<dyn CommerceEngine>)
    }

    fn agent_hooks(&self) -> Vec<Arc<dyn AgentHook>> {
        vec![Arc::new(NurtureAgentHook {
            karma_forge: self.state.karma_forge.clone(),
        })]
    }
}

pub struct NurtureAgentHook {
    pub karma_forge: Arc<nurture_infra::economy::karma_forge::KarmaForge>,
}

impl std::fmt::Debug for NurtureAgentHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NurtureAgentHook").finish_non_exhaustive()
    }
}

#[async_trait]
impl AgentHook for NurtureAgentHook {
    async fn on_pre_execute(
        &self,
        _request: &nurture_bridge::LlmRequest,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_post_execute(
        &self,
        _request: &nurture_bridge::LlmRequest,
        _response: &nurture_bridge::LlmResponse,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_job_completed(&self, job_id: &str, status: &str) -> Result<(), AiomeError> {
        tracing::info!("🧬 [Nurture] Triggering KarmaForge synthesis on job completion");

        // 経済活動の監査ログを KarmaForge と統合
        let content = serde_json::json!({
            "job_id": job_id,
            "status": status,
            "event": "job_execution_audit"
        });

        if let Err(e) = self
            .karma_forge
            .job_queue
            .store_karma(
                "economy_audit",
                "job_completed",
                &content.to_string(),
                "audit",
                "agent-synergy",
                Some("economy"),
                None,
                None,
                false,
            )
            .await
        {
            tracing::warn!("⚠️ [Nurture] Failed to store economy audit karma: {}", e);
        }

        // 未結合の Karma をフェッチして実際の合成パイプラインに渡す
        if let Err(e) = self
            .karma_forge
            .synthesize_unincorporated("agent-synergy")
            .await
        {
            tracing::warn!(
                "⚠️ [Nurture] KarmaForge synthesis failed (non-fatal): {}",
                e
            );
        }
        Ok(())
    }

    async fn on_proof_completed(&self, skill_name: &str, is_valid: bool) -> Result<(), AiomeError> {
        tracing::info!(
            "🛡️ [Nurture] Proof completed for skill '{}', valid: {}",
            skill_name,
            is_valid
        );
        if let Err(e) = self
            .karma_forge
            .inject_proof_seed(skill_name, is_valid)
            .await
        {
            tracing::warn!(
                "⚠️ [Nurture] Failed to inject proof seed into KarmaForge: {}",
                e
            );
        }
        Ok(())
    }

    async fn on_transaction_completed(
        &self,
        source: &str,
        amount_cents: i64,
        actor_id: &str,
        transaction_id: &str,
    ) -> Result<(), AiomeError> {
        tracing::info!(
            "🧬 [Nurture] Triggering KarmaForge synthesis on transaction completed: {}",
            transaction_id
        );

        let content = serde_json::json!({
            "source": source,
            "amount_cents": amount_cents,
            "actor_id": actor_id,
            "transaction_id": transaction_id,
            "event": "transaction_completed"
        });

        if let Err(e) = self
            .karma_forge
            .job_queue
            .store_karma(
                "economy_audit",
                "transaction_completed",
                &content.to_string(),
                "audit",
                "agent-synergy",
                Some("economy"),
                None,
                None,
                false,
            )
            .await
        {
            tracing::warn!(
                "⚠️ [Nurture] Failed to store economy transaction karma: {}",
                e
            );
        }

        if let Err(e) = self
            .karma_forge
            .synthesize_unincorporated("agent-synergy")
            .await
        {
            tracing::warn!(
                "⚠️ [Nurture] KarmaForge synthesis failed (non-fatal): {}",
                e
            );
        }
        Ok(())
    }
}

use nurture_bridge::db::DatabasePool;

#[allow(clippy::too_many_arguments)]
pub async fn create_plugin(
    pool: DatabasePool,
    system_id: uuid::Uuid,
    _event_sender: tokio::sync::broadcast::Sender<nurture_bridge::watchtower::CoreEvent>,
    job_queue: Arc<dyn JobQueue>,
    cancel_token: CancellationToken,
    nurture_secret: String,
    stripe_webhook_secret: Option<String>,
    polar_webhook_secret: Option<String>,
    auth_manager: Arc<dyn nurture_bridge::auth::AuthManager>,
    drm_master_key: String,
) -> Result<CreatedNurturePlugin, AiomeError> {
    let policy = crate::state::EconomyPolicy::default();
    let state = crate::state::AppState::init(
        pool.clone(),
        job_queue.clone(),
        policy,
        commerce_protocol::identity::ActorId(system_id),
        cancel_token.clone(),
        secrecy::SecretString::from(nurture_secret),
        stripe_webhook_secret.map(secrecy::SecretString::from),
        polar_webhook_secret.map(secrecy::SecretString::from),
        auth_manager,
        secrecy::SecretString::from(drm_master_key),
        {
            #[cfg(feature = "cloud-storage")]
            {
                let bucket = std::env::var("S3_BUCKET_NAME").map_err(|_| {
                    nurture_bridge::error::AiomeError::Validation {
                        reason: "S3_BUCKET_NAME must be set when cloud-storage feature is enabled"
                            .into(),
                    }
                })?;
                let aws_config =
                    aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
                let s3_client = aws_sdk_s3::Client::new(&aws_config);
                std::sync::Arc::new(nurture_infra::storage::S3AssetStorage::new(
                    s3_client, bucket,
                ))
            }
            #[cfg(not(feature = "cloud-storage"))]
            {
                tracing::info!("📦 [Desktop] cloud-storage feature disabled in Plugin. Using MockAssetStorage.");
                std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new())
            }
        },
        std::env::var("A2A_AUTH_TOKEN").ok().map(secrecy::SecretString::from),
        std::env::var("SHADOW_CLONE_GRPC_HOST").unwrap_or_else(|_| "localhost".to_string()),
        std::env::var("SHADOW_CLONE_GRPC_PORT").unwrap_or_else(|_| "50051".to_string()),
    )
    .await?;

    let bridge = state.commerce_engine.clone();
    let s2s_router = crate::routes::s2s_internal_service(state.clone());

    Ok(CreatedNurturePlugin {
        plugin: Arc::new(NurturePlugin {
            state,
            bridge,
            _cancel_token: cancel_token,
        }),
        s2s_router,
    })
}

/// InProcess 登録用: JWT 配下 Plugin + JWT 外 S2S ルータ（OP-088 P1）。
pub struct CreatedNurturePlugin {
    pub plugin: Arc<dyn AiomePlugin>,
    /// `nest_service("/internal", …)` 用（パス prefix なし）
    pub s2s_router: axum::Router,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurture_bridge::traits::JobQueue;
    use nurture_infra::economy::karma_forge::KarmaForge;

    #[tokio::test]
    async fn test_nurture_agent_hook_no_op_functions() {
        let job_queue = Arc::new(
            nurture_infra::mock_job_queue::RealJobQueue::new("sqlite::memory:")
                .await
                .expect("Failed to create in-memory SQLite DB for Mock Job Queue"),
        );
        let llm = Arc::new(nurture_bridge::llm::OllamaProvider::new(
            "http://dummy".to_string(),
            "dummy".to_string(),
        ));
        let executor = Arc::new(nurture_infra::sandbox::executor::PythonExecutor::new(
            nurture_infra::sandbox::executor::ResourceLimits::default(),
        ));
        let forge = Arc::new(KarmaForge::new(
            job_queue as Arc<dyn JobQueue>,
            llm,
            executor,
        ));
        let hook = NurtureAgentHook { karma_forge: forge };

        // Test Phase 2 stub behavior
        assert!(hook.on_job_completed("test_job", "success").await.is_ok());
        assert!(hook.on_proof_completed("test_skill", true).await.is_ok());
        assert!(hook
            .on_transaction_completed("polar", 1000, "actor", "tx_1")
            .await
            .is_ok());
    }
}
