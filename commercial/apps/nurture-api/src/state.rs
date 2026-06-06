/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use commerce_protocol::identity::ActorId;
use commerce_protocol::settlement::SettlementProtocol;
use nurture_bridge::error::AiomeError;
use nurture_bridge::immune_system::AdaptiveImmuneSystem;
use nurture_bridge::traits::JobQueue;
use nurture_core::ledger::EconomyLedger;
pub use nurture_core::policy::EconomyPolicy;
use nurture_core::policy::SharedPolicy;
use nurture_infra::csam::CsamPipeline;
use nurture_infra::economy::interceptor::EconomyInterceptor;
use nurture_infra::economy::karma_forge::KarmaForge;
use nurture_infra::economy::karma_immune_filter::KarmaImmuneFilter;
use nurture_infra::marketplace::sqlite::SQLiteMarketplace;
use nurture_infra::sidecar::clone_manager::CloneManager;
use nurture_infra::sidecar::vram_arbiter::VramArbiter;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

pub struct AppState {
    pub ledger: Arc<dyn EconomyLedger>,
    pub settlement: Arc<dyn SettlementProtocol>,
    pub marketplace: Arc<SQLiteMarketplace>,
    pub interceptor: Arc<EconomyInterceptor>,
    pub csam_pipeline: Arc<CsamPipeline>,
    pub job_queue: Arc<dyn JobQueue>,
    pub idempotency: Arc<dyn nurture_infra::economy::idempotency::IdempotencyStore>,
    pub customer_store: Arc<dyn nurture_core::customer::CustomerStore>,
    pub stripe_handler: Option<Arc<nurture_infra::stripe::webhook::StripeWebhookHandler>>,
    pub polar_handler: Option<Arc<nurture_infra::polar::webhook::PolarWebhookHandler>>,
    pub ekyc_store: Arc<dyn nurture_infra::identity::ekyc::EkycStore>,
    pub pool: sqlx::SqlitePool,
    pub policy: SharedPolicy,
    pub system_actor_id: ActorId,
    pub license_store: Arc<dyn nurture_core::license::LicenseStore>,
    pub clone_manager: Arc<CloneManager>,
    pub karma_forge: Arc<KarmaForge>,
    pub karma_immune_filter: Arc<KarmaImmuneFilter>,
    pub immune_system: Arc<AdaptiveImmuneSystem>,
    pub commerce_engine: Arc<nurture_infra::economy::bridge::NurtureCommerceBridge>,
    pub internal_secret: SecretString,
    pub auth_manager: std::sync::Arc<dyn nurture_bridge::auth::AuthManager>,
    pub asset_storage: Arc<dyn nurture_infra::storage::AssetStorage>,
    pub python_executor: Arc<nurture_infra::sandbox::executor::PythonExecutor>,
    pub a2a_auth_token: Option<SecretString>,
    pub shadow_clone_grpc_host: String,
    pub shadow_clone_grpc_port: String,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        pool: sqlx::SqlitePool,
        job_queue: Arc<dyn JobQueue>,
        policy: EconomyPolicy,
        system_id: ActorId,
        cancel_token: tokio_util::sync::CancellationToken,
        internal_secret: SecretString,
        stripe_webhook_secret: Option<SecretString>,
        polar_webhook_secret: Option<SecretString>,
        auth_manager: std::sync::Arc<dyn nurture_bridge::auth::AuthManager>,
        drm_master_key: SecretString,
        asset_storage: Arc<dyn nurture_infra::storage::AssetStorage>,
        a2a_auth_token: Option<SecretString>,
        shadow_clone_grpc_host: String,
        shadow_clone_grpc_port: String,
    ) -> Result<SharedState, AiomeError> {
        // Enable SQLite WAL mode and busy_timeout for concurrency
        sqlx::query("PRAGMA journal_mode=WAL;")
            .execute(&pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to set WAL mode: {}", e),
            })?;
        sqlx::query("PRAGMA busy_timeout=5000;")
            .execute(&pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to set busy_timeout: {}", e),
            })?;

        let shared_policy = Arc::new(tokio::sync::RwLock::new(policy));
        let ledger = Arc::new(nurture_infra::economy::ledger::SQLiteEconomyLedger::new(
            pool.clone(),
        ));
        let settlement = Arc::new(
            nurture_infra::economy::settlement::SQLiteSettlementProvider::new(
                pool.clone(),
                ledger.clone(),
                shared_policy.clone(),
                system_id,
            ),
        );
        let marketplace = Arc::new(SQLiteMarketplace::new(pool.clone()));
        let interceptor = Arc::new(EconomyInterceptor::new(shared_policy.clone()));
        let ncmec_reporter = Arc::new(nurture_infra::csam::ncmec::SQLiteNcmecReporter::new(
            pool.clone(),
        ));
        let ekyc_store = Arc::new(nurture_infra::identity::ekyc::SQLiteEkycStore::new(
            pool.clone(),
        ));
        let db_pool_shared = nurture_bridge::db::DatabasePool::Sqlite(pool.clone());
        let csam_pipeline = Arc::new(
            nurture_infra::csam::default_pipeline(ekyc_store.clone(), db_pool_shared)
                .with_reporter(ncmec_reporter),
        );

        let idempotency = Arc::new(
            nurture_infra::economy::idempotency::SQLiteIdempotencyStore::new(pool.clone()),
        );
        let customer_store = Arc::new(nurture_infra::economy::customer::SQLiteCustomerStore::new(
            pool.clone(),
        ));
        let license_store = Arc::new(nurture_infra::drm::license::SQLiteLicenseStore::new(
            pool.clone(),
            &drm_master_key,
        ));

        let python_executor = Arc::new(nurture_infra::sandbox::executor::PythonExecutor::new(
            nurture_infra::sandbox::executor::ResourceLimits::default(),
        ));
        let llm_provider = Arc::new(nurture_bridge::llm::OllamaProvider::new(
            std::env::var("LLM_PROVIDER_HOST")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
            std::env::var("LLM_PROVIDER_MODEL").unwrap_or_else(|_| "llama3".to_string()),
        ));
        let karma_forge = Arc::new(KarmaForge::new(
            job_queue.clone(),
            llm_provider.clone(),
            python_executor.clone(),
        ));
        let karma_immune_filter = Arc::new(KarmaImmuneFilter::new(job_queue.clone()));
        let immune_system = Arc::new(AdaptiveImmuneSystem::new(llm_provider.clone()));

        let nurture_agent_hook: Arc<dyn nurture_bridge::plugin::AgentHook> =
            Arc::new(crate::plugin::NurtureAgentHook {
                karma_forge: karma_forge.clone(),
            });
        let agent_hooks = vec![nurture_agent_hook];

        let stripe_handler = stripe_webhook_secret.map(|secret| {
            Arc::new(nurture_infra::stripe::webhook::StripeWebhookHandler::new(
                secret.expose_secret().clone(),
                ledger.clone(),
                customer_store.clone(),
                system_id,
                idempotency.clone(),
                agent_hooks.clone(),
            ))
        });

        let polar_handler = polar_webhook_secret.map(|secret| {
            Arc::new(nurture_infra::polar::webhook::PolarWebhookHandler::new(
                secret.expose_secret().clone(),
                ledger.clone(),
                system_id,
                idempotency.clone(),
                agent_hooks.clone(),
            ))
        });

        let vram_arbiter = Arc::new(VramArbiter::new(24_000)); // デフォルト 24GB VRAM
        let clone_manager = Arc::new(CloneManager::new(
            vram_arbiter,
            ledger.clone(),
            job_queue.clone(),
            pool.clone(),
            8, // 最大 8 分身
            system_id,
        ));

        // ==========================================
        // P1: Task Supervisor Initialization
        // ==========================================
        let uow_manager = Arc::new(nurture_infra::economy::uow::SqliteUowManager::new(
            pool.clone(),
            &drm_master_key,
        ));

        let commerce_engine = Arc::new(nurture_infra::economy::bridge::NurtureCommerceBridge::new(
            ledger.clone(),
            settlement.clone(),
            marketplace.clone(),
            interceptor.clone(),
            csam_pipeline.clone(),
            job_queue.clone(),
            idempotency.clone(),
            license_store.clone(),
            karma_forge.clone(),
            shared_policy.clone(),
            pool.clone(),
            uow_manager,
        ));

        let supervisor = nurture_bridge::supervisor::TaskSupervisor::new(5, 300);

        // 1. Orphan Recovery Task (Run once and exit)
        struct OrphanRecoveryTask {
            cm: Arc<CloneManager>,
        }
        impl nurture_bridge::supervisor::SupervisedTask for OrphanRecoveryTask {
            fn name(&self) -> &'static str {
                "OrphanRecovery"
            }
            fn run(
                &self,
                _ct: tokio_util::sync::CancellationToken,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
                let cm = self.cm.clone();
                Box::pin(async move {
                    if let Err(e) = cm.recover_orphans().await {
                        tracing::error!(
                            "❌ [OrphanRecovery] CloneManager recovery failed: {:?}",
                            e
                        );
                    }
                    tracing::info!("✅ [OrphanRecovery] Completed successfully.");
                })
            }
        }
        supervisor.spawn_supervised(
            OrphanRecoveryTask {
                cm: clone_manager.clone(),
            },
            cancel_token.clone(),
        );

        // 2. CloneManager Maintenance Task
        struct CloneMaintTask {
            cm: Arc<CloneManager>,
        }
        impl nurture_bridge::supervisor::SupervisedTask for CloneMaintTask {
            fn name(&self) -> &'static str {
                "CloneManagerMaintenance"
            }
            fn run(
                &self,
                ct: tokio_util::sync::CancellationToken,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
                let cm = self.cm.clone();
                Box::pin(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                cm.run_maintenance().await;
                            }
                            _ = ct.cancelled() => {
                                tracing::info!("🛑 [CloneMaintTask] Cancellation received.");
                                break;
                            }
                        }
                    }
                })
            }
        }
        supervisor.spawn_supervised(
            CloneMaintTask {
                cm: clone_manager.clone(),
            },
            cancel_token.clone(),
        );

        // 3. Escrow Sweep Task
        struct EscrowSweepTask {
            ce: Arc<nurture_infra::economy::bridge::NurtureCommerceBridge>,
        }
        impl nurture_bridge::supervisor::SupervisedTask for EscrowSweepTask {
            fn name(&self) -> &'static str {
                "EscrowSweep"
            }
            fn run(
                &self,
                ct: tokio_util::sync::CancellationToken,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
                let ce = self.ce.clone();
                Box::pin(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                let mut retry_count = 0;
                                while let Err(e) = ce.process_expired_escrows().await {
                                    tracing::error!("❌ [EscrowSweep] Failed to process expired escrows (Attempt {}): {:?}", retry_count + 1, e);
                                    retry_count += 1;
                                    if retry_count >= 2 {
                                        tracing::error!("🚨 [EscrowSweep] Sweep failed permanently after retries: {:?}", e);
                                        break;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                }
                            }
                            _ = ct.cancelled() => {
                                tracing::info!("🛑 [EscrowSweep] Cancellation received.");
                                break;
                            }
                        }
                    }
                })
            }
        }
        supervisor.spawn_supervised(
            EscrowSweepTask {
                ce: commerce_engine.clone(),
            },
            cancel_token.clone(),
        );

        // 4. License GC Task
        struct LicenseGcTask {
            ls: Arc<dyn nurture_core::license::LicenseStore>,
        }
        impl nurture_bridge::supervisor::SupervisedTask for LicenseGcTask {
            fn name(&self) -> &'static str {
                "LicenseGC"
            }
            fn run(
                &self,
                ct: tokio_util::sync::CancellationToken,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
                let ls = self.ls.clone();
                Box::pin(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                match ls.purge_expired_licenses().await {
                                    Ok(count) if count > 0 => tracing::info!("🧹 [LicenseGC] Purged {} expired/revoked licenses.", count),
                                    Err(e) => {
                                        tracing::error!("❌ [LicenseGC] Failed to purge expired licenses: {:?}", e);
                                    }
                                    _ => {}
                                }
                            }
                            _ = ct.cancelled() => {
                                tracing::info!("🛑 [LicenseGC] Cancellation received.");
                                break;
                            }
                        }
                    }
                })
            }
        }
        supervisor.spawn_supervised(
            LicenseGcTask {
                ls: license_store.clone(),
            },
            cancel_token.clone(),
        );

        Ok(Arc::new(AppState {
            ledger,
            settlement,
            marketplace,
            interceptor,
            csam_pipeline,
            job_queue,
            idempotency,
            customer_store,
            stripe_handler,
            polar_handler,
            ekyc_store,
            pool,
            policy: shared_policy,
            system_actor_id: system_id,
            license_store,
            clone_manager,
            karma_forge,
            karma_immune_filter,
            immune_system,
            commerce_engine,
            internal_secret,
            auth_manager,
            asset_storage,
            python_executor,
            a2a_auth_token,
            shadow_clone_grpc_host,
            shadow_clone_grpc_port,
        }))
    }
}
