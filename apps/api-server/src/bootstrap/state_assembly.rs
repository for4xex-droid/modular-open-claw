/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]

use crate::app_state::Component;

use std::sync::Arc;

use super::*;

pub async fn assemble_app_state(
    preflight: &PreflightResult,
    db: &DatabaseResult,
    llm: &ProviderResult,
    core: &CoreServicesResult,
    oxilean_power: std::sync::Arc<std::sync::atomic::AtomicU32>,
) -> anyhow::Result<crate::app_state::AppState> {
    let docs_path = "../../docs";

    let state = crate::app_state::AppState {
        oxilean_power,
        hook_chain: Default::default(),
        hook_manager: Component::new(db.hook_manager.clone()),
        db_pool: Component::new(std::sync::Arc::new(db.db_pool.clone())),
        health_monitor: Component::new(preflight.health_monitor.clone()),
        job_queue: Component::new(db.job_queue.clone()),
        wasm_skill_manager: Component::new(core.wasm_skill_manager.clone()),
        skill_forge: Component::new(core.skill_forge.clone()),
        docs_path: docs_path.to_string(),
        llm_semaphore: Component::new(core.llm_semaphore.clone()),
        forge_semaphore: Component::new(core.forge_semaphore.clone()),
        mcp_sessions: Component::new(std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ))),
        mcp_manager: Component::new(core.mcp_manager.clone()),
        artifact_store: Component::new(
            core.artifact_store.clone() as std::sync::Arc<dyn aiome_core::traits::ArtifactStore>
        ),
        event_sender: Component::new(core.event_sender.clone()),
        context_engine: Component::new(core.context_engine.clone()),
        soul_mutator: Component::new(core.soul_mutator.clone()),
        soul_store: Component::new(core.soul_store.clone()),
        provider: Component::new(core.router_provider.clone()),
        fast_provider: Component::new(llm.fast_provider.clone()),
        autonomous_running: Component::new(core.autonomous_running.clone()),
        autonomous_config: Component::new(core.autonomous_config.clone()),
        http_client: Component::new(db.http_client.clone()),
        docker_failures: Component::new(core.docker_failures.clone()),
        security_policy: {
            let mut policy = shared::security::SecurityPolicy::default();
            for tool in preflight.plugin_registry.registered_tools() {
                policy.register_tool(&tool);
            }
            policy
        },
        commerce_engine: Component::new(
            core.commerce_engine
                .clone()
                .ok_or_else(|| anyhow::anyhow!("commerce engine must be initialized"))?,
        ),
        gig_engine: Component::new(core.gig_engine.clone()),
        circuit_breaker: Component::new(db.circuit_breaker.clone()),
        rate_limiter: Component::new(db.rate_limiter.clone()),
        slo_engine: Component::new(db.slo_engine.clone()),
        alert_manager: Component::new(db.alert_manager.clone()),
        skill_arena: Component::new(std::sync::Arc::new(
            infrastructure::skills::skill_arena::SkillArena::new().with_db_pool(db.db_pool.clone()),
        )),
        api_server_secret: Component::new(core.api_server_secret.clone()),
        federation_secret: Component::new(
            core.federation_secret
                .clone()
                .unwrap_or_else(|| Arc::new(secrecy::SecretString::from(String::new()))),
        ),
        config: Component::new(preflight.config.clone()),
        gift_engine: Component::new(core.gift_engine.clone()),
        ekyc_engine: Component::new(core.ekyc_engine.clone()),
        ekyc_session_store: Component::new(core.ekyc_session_store.clone()),
        quarantine_store: Component::new(core.quarantine_store.clone()),
        ban_store: Component::new(core.ban_store.clone()),
        auth_manager: Component::new(core.auth_manager.clone()),
        system_agent_id: db.system_agent_id,
        voice_drm: Component::new(core.voice_drm.clone()),
        registry: Component::new(core.registry.clone()),
        intent_generator: Component::new(core.intent_generator.clone()),
        intent_firewall: Component::new(core.intent_firewall.clone()),
        audit_logger: Component::new(db.audit_logger.clone()),
        affiliate_adapter: Component::new({
            #[cfg(debug_assertions)]
            {
                std::sync::Arc::new(infrastructure::intent::MockAffiliateAdapter::new())
                    as std::sync::Arc<dyn aiome_core_contracts::traits::AffiliateAdapter>
            }
            #[cfg(not(debug_assertions))]
            {
                std::sync::Arc::new(infrastructure::intent::DisabledAffiliateAdapter::new())
                    as std::sync::Arc<dyn aiome_core_contracts::traits::AffiliateAdapter>
            }
        }),
        soul_pipeline: Component::new(core.soul_pipeline.clone()),
        transcription_engine: Component::new(core.transcription_engine.clone()),
        task_dispatcher: Component::new(core.task_dispatcher.clone()),
        lora_engine: {
            let core_engine = std::sync::Arc::new(aiome_core::lora::engine::LoraEngine::new());
            let engine =
                std::sync::Arc::new(infrastructure::lora_training::LoraTrainingService::new(
                    core_engine,
                    Some(core.soul_mutator.clone()),
                    Some(db.job_queue.clone()),
                    Some(core.event_sender.clone()),
                    Some(core.compute_semaphore.clone()),
                ));
            Component::new(engine as std::sync::Arc<dyn aiome_core_contracts::traits::LoraEngine>)
        },
        tts_provider: {
            let tts_type = std::env::var("TTS_PROVIDER").unwrap_or_else(|_| "mock".to_string());
            let provider: std::sync::Arc<dyn aiome_core_contracts::traits::TtsProvider> =
                match tts_type.as_str() {
                    "openai" => {
                        let key: secrecy::SecretString =
                            match &core.tts_openai_api_key_raw {
                                Some(raw) => secrecy::SecretString::from(raw.clone()),
                                None => {
                                    tracing::warn!(
                                        "⚠️ [TTS] TTS_OPENAI_API_KEY missing, OpenAI TTS will fail"
                                    );
                                    preflight.config.openai_api_key.clone().unwrap_or_else(|| {
                                        secrecy::SecretString::from(String::new())
                                    })
                                }
                            };
                        let model = std::env::var("TTS_OPENAI_MODEL")
                            .unwrap_or_else(|_| "tts-1".to_string());
                        std::sync::Arc::new(infrastructure::tts::OpenAiTtsProvider::new(
                            key,
                            model,
                            std::env::var("OPENAI_TTS_ENDPOINT").ok(),
                        ))
                    }
                    "xtts" => {
                        let endpoint = preflight
                            .config
                            .xtts_endpoint
                            .clone()
                            .unwrap_or_else(|| format!("http://127.0.0.1:{}", 18020));
                        std::sync::Arc::new(infrastructure::tts::XttsProvider::new(endpoint))
                    }
                    _ => {
                        #[cfg(debug_assertions)]
                        {
                            std::sync::Arc::new(infrastructure::tts::MockTtsProvider::default())
                        }
                        #[cfg(not(debug_assertions))]
                        {
                            std::sync::Arc::new(infrastructure::tts::DisabledTtsProvider::default())
                        }
                    }
                };
            Component::new(provider)
        },
        news_service: {
            let rss = std::sync::Arc::new(infrastructure::rss_collector::RssCollector::new(
                std::sync::Arc::new(infrastructure::rss_collector::SqlTrendCacheRepository::new(
                    db.db_pool.clone(),
                )),
            ));
            Component::new(rss as std::sync::Arc<dyn aiome_core_contracts::traits::NewsService>)
        },
        live_session_manager: Component(preflight.live_manager.clone()),
        syndicate_store: Component::new(std::sync::Arc::new(
            aiome_commerce::syndicate::UniversalSyndicateStore::new(db.db_pool.clone()),
        )),
        hierarchical_router: Component::new(std::sync::Arc::new(
            infrastructure::hierarchical_router::HierarchicalRouter::new(
                llm.fast_provider.clone(),
                db.db_pool.get_sqlite_pool_or_err()?.clone(),
            ),
        )),
        rlm_client: Component::new(core.rlm_client.clone()),
        formal_proof_gate: Component::new(core.formal_proof_gate.clone()),
        a2a_client: Component::new(core.a2a_client.clone()),
        compute_semaphore: Component::new(core.compute_semaphore.clone()),
        disk_quota: Component::new(core.disk_quota_mgr.clone()),
        publish_pipeline: Component::new(core.publish_pipeline.clone()),
        cortex_projector: Component::new(core.cortex_projector.clone()),
        lora_marketplace: {
            let vault_root = preflight.config.resolver.resolve("vault");
            let commerce_for_marketplace = core.commerce_engine.clone().ok_or_else(|| {
                anyhow::anyhow!("commerce engine must be initialized for lora marketplace")
            })?;
            let marketplace = Arc::new(
                infrastructure::lora_marketplace::UniversalLoraMarketplace::new(
                    db.db_pool.clone(),
                    commerce_for_marketplace,
                    vault_root,
                ),
            );
            Component::new(
                marketplace
                    as std::sync::Arc<dyn aiome_core_contracts::lora_marketplace::LoraMarketplace>,
            )
        },
        quality_gate_store: Component::new(core.quality_gate_store.clone()),
        ws_active_connections: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        harness_cache: Component::new(std::sync::Arc::new(
            infrastructure::skills::harness::HarnessCache::new(),
        )),
        upload_semaphore: Component::new(std::sync::Arc::new(tokio::sync::Semaphore::new(2))),
        generative_engine: {
            let engine_type =
                std::env::var("GENERATIVE_ENGINE").unwrap_or_else(|_| "mock".to_string());
            let engine: std::sync::Arc<dyn aiome_core_contracts::traits::GenerativeEngine> =
                match engine_type.as_str() {
                    "comfyui" => {
                        let base_url = preflight.config.comfyui_url.clone();
                        std::sync::Arc::new(
                            infrastructure::generative_engine::ComfyUiGenerativeEngine::new(
                                base_url,
                                Some(core.compute_semaphore.clone()),
                            ),
                        )
                    }
                    "falai" => {
                        let api_key = std::env::var("FAL_KEY").unwrap_or_default();
                        shared::security::scrub_env("FAL_KEY");
                        std::sync::Arc::new(
                            infrastructure::generative_engine::FalAiGenerativeEngine::new(
                                secrecy::SecretString::from(api_key),
                                std::env::var("FAL_AI_ENDPOINT").ok(),
                            ),
                        )
                    }
                    _ => {
                        #[cfg(any(test, debug_assertions))]
                        {
                            tracing::warn!(
                                "⚠️ [GenerativeEngine] Using Mock engine for development."
                            );
                            std::sync::Arc::new(
                            infrastructure::generative_engine::mock::MockGenerativeEngine::default(
                            ),
                        )
                        }
                        #[cfg(not(any(test, debug_assertions)))]
                        {
                            tracing::error!("🚨 [FATAL] GenerativeEngine must be explicitly configured in production (GENERATIVE_ENGINE=comfyui|falai).");
                            std::process::exit(1);
                        }
                    }
                };
            Component::new(engine)
        },
        cortex_ingester: Component::new(std::sync::Arc::new(
            infrastructure::cortex_ingester::CortexIngester::new(
                llm.fast_provider.clone(),
                db.db_pool.clone(),
            ),
        )),
        project_rules_cache: Component::new(std::sync::Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(30))
                .build(),
        )),
        cortex_query: Component::new(std::sync::Arc::new(
            infrastructure::cortex_query::CortexQueryEngine::new(
                core.router_provider.clone(),
                db.db_pool.clone(),
            )
            .with_rlm_provider(core.rlm_client.clone()),
        )),
        feature_flags_cache: Component::new(std::sync::Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(60))
                .build(),
        )),
        eval_logger: Component::new(db.eval_logger.clone()),
        a2ui_catalog: {
            let mut catalog = infrastructure::a2ui::AiomeCatalog::default();
            catalog.register_component(
                "text",
                serde_json::json!({"content": "string", "markdown": "boolean"}),
            );
            catalog.register_component("button", serde_json::json!({"label": "string", "action": "string (whitelist: approve_job:<uuid>, run_skill:<name>, cancel_job:<uuid>, navigate:<tab>)", "variant": "string (optional)"}));
            catalog.register_component("list", serde_json::json!({"ordered": "boolean"}));
            catalog.register_component(
                "form",
                serde_json::json!({"action": "string", "submitLabel": "string"}),
            );
            catalog.register_component(
                "input",
                serde_json::json!({"name": "string", "label": "string", "type": "string"}),
            );
            catalog.register_component("taskApproval", serde_json::json!({"title": "string", "description": "string", "riskLevel": "string"}));
            catalog.register_component(
                "taskResult",
                serde_json::json!({"success": "boolean", "message": "string"}),
            );
            catalog.register_component(
                "treasureItem",
                serde_json::json!({"name": "string", "value": "number", "currency": "string"}),
            );
            catalog.register_component("progressBar", serde_json::json!({"progress": "number", "label": "string", "status": "string (optional)"}));
            catalog.register_component("dataTable", serde_json::json!({"headers": "array of strings", "rows": "array of arrays of strings"}));
            catalog.register_component("chart", serde_json::json!({"type": "string (line, bar, pie)", "data": "object", "title": "string"}));
            catalog.register_component("alert", serde_json::json!({"type": "string (info, warning, error, success)", "message": "string"}));
            catalog.register_component(
                "cellStatus",
                serde_json::json!({"cellId": "string", "status": "string", "metrics": "object"}),
            );
            catalog.register_component(
                "timeline",
                serde_json::json!({"events": "array of objects {date, title, description}"}),
            );
            catalog.register_component(
                "codeBlock",
                serde_json::json!({"code": "string", "language": "string"}),
            );
            catalog.register_component(
                "card",
                serde_json::json!({"title": "string (optional)", "content": "string (optional)"}),
            );
            catalog.register_component("voiceStore", serde_json::json!({}));
            catalog.register_component("loraMarket", serde_json::json!({}));
            catalog.register_component(
                "walletWidget",
                serde_json::json!({"label": "string (optional)"}),
            );
            catalog.register_component(
                "marketplaceItem",
                serde_json::json!({
                    "title": "string",
                    "price": "number",
                    "currency": "string (optional, default KC)",
                    "description": "string (optional)"
                }),
            );
            Component::new(std::sync::Arc::new(catalog))
        },
        nurture_url: std::env::var("NURTURE_API_URL").ok(),
        nurture_internal_secret: preflight.secrets.nurture_secret.clone(),
        gig_updater: Component::new(std::sync::Arc::new(
            infrastructure::gig_metadata_updater::DbGigUpdater::new(
                db.db_pool.get_sqlite_pool_or_err()?.clone(),
            ),
        )
            as std::sync::Arc<dyn aiome_contracts::gig_metadata::GigMetadataUpdater>),
        pkce_cache: Component::new(Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(600))
                .max_capacity(10_000)
                .build(),
        )),
        mcp_oauth_secrets: {
            let mut secrets = std::collections::HashMap::new();
            for provider in crate::mcp::discovery::ALLOWED_OAUTH_PROVIDERS {
                let id_env = format!("{}_CLIENT_ID", provider.to_uppercase());
                let sec_env = format!("{}_CLIENT_SECRET", provider.to_uppercase());
                if let (Ok(client_id), Ok(client_secret)) =
                    (std::env::var(&id_env), std::env::var(&sec_env))
                {
                    shared::security::scrub_env(&id_env);
                    shared::security::scrub_env(&sec_env);
                    secrets.insert(
                        provider.to_string(),
                        crate::mcp::discovery::OAuthCredentials {
                            client_id,
                            client_secret: secrecy::SecretString::from(client_secret),
                        },
                    );
                }
            }
            secrets
        },
        vault_backend: Component::new(core.vault_backend.clone()),
        prompt_registry: Component::new(core.prompt_registry.clone()),
        spec_provider: Component::new(core.spec_provider.clone()),
        tokens_css: core.tokens_css.clone(),
        buzz_generator: Component::new(std::sync::Arc::new(
            infrastructure::buzz::generator::BuzzContentGenerator::new(llm.fast_provider.clone()),
        )),
        buzz_scheduler: Component::new(std::sync::Arc::new(
            infrastructure::buzz::scheduler::BuzzScheduler::new(90, 4),
        )),
        stripe_price_subscription_monthly: preflight
            .secrets
            .stripe_price_subscription_monthly
            .clone(),
        stripe_api_key: core.stripe_key_raw.clone(),
        biome_engine: Component::new(std::sync::Arc::new(tokio::sync::RwLock::new(
            biome_engine::BiomeEngine::new(42),
        ))),
        workflow_execution_tracker: Component::new(core.workflow_execution_tracker.clone()),
    };

    Ok(state)
}
