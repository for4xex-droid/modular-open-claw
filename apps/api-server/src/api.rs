/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::audit::get_logs,
        crate::routes::general::list_wiki_files,
        crate::routes::general::get_wiki_content,
        crate::routes::settings::get_settings,
        crate::routes::settings::update_setting,
        crate::routes::settings::get_ollama_models,
        crate::routes::skill::list_skills,
        crate::routes::skill::import_skill,
        crate::routes::skill::spawn_mcp_server,
        crate::routes::proof_verifier::verify_skill_proof,
        crate::routes::skill::update_mcp_config,
        crate::routes::skill::get_mcp_config,
        crate::routes::general::get_health_status,
        // Playbooks (F-1)
        crate::routes::playbook::list_playbooks,
        crate::routes::playbook::install_playbook,
        crate::routes::playbook::import_playbook,
        crate::routes::workflow::export_workflow,
        // Agent
        crate::routes::agent::trigger_agent_chat,
        crate::routes::agent::handle_karma_feedback,
        // Jobs
        crate::routes::jobs::get_awaiting_input_jobs,
        crate::routes::jobs::submit_job_review,
        crate::routes::jobs::cancel_job_handler,
        crate::routes::jobs::get_job_logs_handler,
        crate::routes::jobs::get_trajectory_handler,
        crate::routes::jobs::get_diagnosis_handler,
        // Karma
        crate::routes::karma::get_karma_stream,
        crate::routes::karma::synergy_graph_handler,
        crate::routes::karma::get_immune_rules_handler,
        crate::routes::karma::add_immune_rule_handler,
        crate::routes::karma::delete_immune_rule_handler,
        crate::routes::karma::get_evolution_history_handler,
        // Commune
        crate::routes::commune::commune_status,
        crate::routes::commune::list_topics,
        crate::routes::commune::create_topic,
        crate::routes::commune::autonomous_start,
        crate::routes::commune::autonomous_stop,
        crate::routes::commune::autonomous_status,
        crate::routes::commune::list_messages,
        crate::routes::commune::send_message,
        // Expression
        crate::routes::expression::expression_status,
        crate::routes::expression::generate_expression,
        crate::routes::expression::list_expressions,
        crate::routes::expression::toggle_auto_expression,
        // A2UI
        crate::routes::a2ui::submit_a2ui_action,
        // Commerce
        crate::routes::commerce::get_balance,
        crate::routes::commerce::get_points,
        crate::routes::commerce::get_transaction_history,
        crate::routes::commerce::get_wishlist,
        crate::routes::commerce::convert_points,
        crate::routes::commerce::transfer,
        crate::routes::commerce::execute_purchase,
        crate::routes::commerce::list_escrows,
        crate::routes::commerce::release_escrow,
        crate::routes::commerce::create_subscription,
        crate::routes::commerce::cancel_subscription,
        crate::routes::commerce::get_subscription_status,
        crate::routes::commerce::create_checkout_session,
        crate::routes::commerce::create_portal_session,
        // Gift
        crate::routes::gift::send_gift,
        crate::routes::gift::get_gift_policy,
        // Voice & Avatar
        crate::routes::voice::list_voice_assets_handler,
        crate::routes::model_setup::get_model_status,
        // Artifacts
        crate::routes::artifacts::list_artifacts_handler,
        crate::routes::artifacts::get_artifact_handler,
        crate::routes::artifacts::download_artifact_file_handler,
        crate::routes::artifacts::delete_artifact_handler,
        crate::routes::artifacts::get_artifact_edges_handler,
        // EKYC
        crate::routes::ekyc::create_ekyc_session_handler,
        // Auth / Right to be forgotten
        crate::routes::auth::delete_account_handler,
        // Audit & Trends (Phase 8.6)
        crate::routes::audit::get_audit_ledger,
        crate::routes::audit::get_audit_prompt_stats,
        crate::routes::audit::get_diagnoses,
        crate::routes::audit::get_diagnostics_summary,
        crate::routes::audit::get_quarantined_assets,
        crate::routes::audit::release_quarantined_asset,
        crate::routes::general::get_trends,
        crate::routes::quality_gate::get_quality_gate_history,
        // Gig Economy
        crate::routes::gig::publish_intent,
        crate::routes::gig::submit_bid,
        crate::routes::gig::accept_bid,
        crate::routes::gig::deliver,
        crate::routes::gig::verify,
        // Syndicate
        crate::routes::syndicate::create_guild,
        crate::routes::syndicate::list_guilds,
        crate::routes::syndicate::delete_guild,
        crate::routes::syndicate::add_member,
        crate::routes::syndicate::list_members,
        // Cortex
        crate::routes::cortex::ingest_url_handler,
        crate::routes::cortex::ingest_text_handler,
        crate::routes::cortex::list_documents_handler,
        crate::routes::cortex::delete_document_handler,
        crate::routes::cortex::list_wiki_articles_handler,
        crate::routes::cortex::get_wiki_article_handler,
        crate::routes::cortex::query_handler,
        crate::routes::cortex::suggest_questions_handler,
        crate::routes::cortex::synth_dataset_handler,
        crate::routes::cortex::export_dpo_dataset_handler,
        crate::routes::cortex::god_nodes_handler,
        // LoRA Marketplace
        crate::routes::lora_market::list_market,
        crate::routes::lora_market::publish_listing,
        crate::routes::lora_market::purchase_listing,
        crate::routes::lora_market::complete_purchase,
        crate::routes::lora_market::delist_listing,
        crate::routes::lora_market::my_listings,
        // Soul
        crate::routes::soul::init_soul,
        crate::routes::soul::get_soul_status,
        // Security
        crate::routes::security::get_oxilean_power,
        // Forecast
        crate::routes::forecast::get_forecast
    ),
    components(
        schemas(
            crate::routes::general::LogEntryResponse,
            crate::routes::settings::UpdateSettingsRequest,
            crate::routes::settings::TestConnectionRequest,
            crate::routes::settings::TestConnectionResponse,
            crate::routes::playbook::PlaybookSummary,
            crate::routes::playbook::PlaybookInstallResponse,
            crate::routes::commerce::ReleaseEscrowRequest,
            aiome_core_contracts::commerce::EscrowRecord,
            crate::routes::skill::SkillSummary,
            crate::routes::skill::ImportRequest,
            crate::routes::skill::McpSpawnRequest,
            crate::routes::proof_verifier::VerifyProofRequest,
            crate::routes::proof_verifier::VerifyProofResponse,
            shared::health::ResourceStatus,
            shared::health::CircuitBreakerStatus,
            shared::health::CircuitState,
            shared::health::LoraStatus,
            shared::health::IncidentStats,
            aiome_core::contracts::SystemSetting,
            aiome_core::contracts::ImmuneRule,
            crate::routes::agent::AgentChatRequest,
            crate::routes::agent::ChatMessage,
            crate::routes::agent::KarmaFeedbackRequest,
            crate::routes::karma::GraphNode,
            crate::routes::karma::GraphEdge,
            crate::routes::karma::GraphData,
            crate::routes::commune::SendCommuneRequest,
            crate::routes::commune::StartAutonomousRequest,
            crate::routes::expression::ListParams,
            crate::routes::expression::AutoToggle,
            crate::routes::commerce::PurchaseRequest,
            aiome_core_contracts::commerce::PointsBalance,
            aiome_core_contracts::commerce::TransactionRecord,
            crate::routes::commerce::WithdrawRequest,
            crate::routes::commerce::TransferRequest,
            crate::routes::commerce::CreateSubscriptionRequest,
            crate::routes::commerce::SubscriptionResponse,
            crate::routes::commerce::CancelSubscriptionRequest,
            aiome_core_contracts::commerce::SubscriptionStatus,
            crate::routes::commerce::CreateCheckoutSessionRequest,
            crate::routes::commerce::CreateCheckoutSessionResponse,
            crate::routes::commerce::CreatePortalSessionRequest,
            crate::routes::commerce::CreatePortalSessionResponse,
            aiome_core_contracts::commerce::GiftRequest,
            crate::routes::gift::GiftResponse,
            crate::routes::gift::GiftPolicyResponse,
            crate::routes::artifacts::ListArtifactsParams,
            crate::routes::model_setup::ModelStatusResponse,
            crate::routes::general::AuditLedgerResponse,
            crate::routes::general::PromptStatsResponse,
            infrastructure::llm::evaluation_logger::ProviderEvalStat,
            crate::routes::general::DiagnosisResponse,
            crate::routes::general::DiagnosisSummaryResponse,
            crate::routes::general::CategoryCount,
            crate::routes::general::TrendsResponse,
            aiome_core_contracts::traits::TrendItem,
            crate::routes::ekyc::EkycSessionResponse,
            infrastructure::quality_gate_store::QualityGateEntry,
            // Gig Economy
            aiome_core_contracts::gig::GigIntent,
            aiome_core_contracts::gig::GigBid,
            aiome_core_contracts::gig::GigDeliverable,
            aiome_core_contracts::gig::VerificationResult,
            aiome_core_contracts::gig::AcceptanceCriteria,
            aiome_core_contracts::gig::GigOrderStatus,
            aiome_core::trajectory::TrajectoryStep,
            aiome_core::trajectory::AgentDiagnosis,
            aiome_core::trajectory::FailureCategory,
            aiome_core::trajectory::StepCategory,
            aiome_core::trajectory::ConstraintViolation,
            // Syndicate
            aiome_core_contracts::syndicate::Guild,
            aiome_core_contracts::syndicate::GuildMember,
            crate::routes::syndicate::CreateGuildRequest,
            crate::routes::syndicate::AddMemberRequest,
            // Cortex
            crate::routes::cortex::IngestUrlReq,
            crate::routes::cortex::IngestTextReq,
            crate::routes::cortex::IngestResp,
            infrastructure::cortex_ingester::CortexDocument,
            infrastructure::cortex_ingester::SourceType,
            infrastructure::cortex_compiler::WikiArticle,
            crate::routes::cortex::WikiArticleSummary,
            crate::routes::cortex::QueryReq,
            infrastructure::cortex_query::CortexAnswer,
            crate::routes::cortex::SynthReq,
            crate::routes::cortex::GodNode,
            // LoRA Marketplace
            aiome_core_contracts::lora_marketplace::ListingStatus,
            aiome_core_contracts::lora_marketplace::PurchaseStatus,
            aiome_core_contracts::lora_marketplace::LoraListing,
            aiome_core_contracts::lora_marketplace::LoraPurchase,
            crate::routes::lora_market::PublishListingRequest,
            crate::routes::lora_market::PurchaseRequest,
            crate::routes::lora_market::PurchaseResponse,
            // Soul
            crate::routes::soul::SoulStatusResponse,
            crate::routes::soul::InitSoulResponse,
            crate::routes::soul::InitSoulRequest,
            // Security
            crate::routes::security::OxiLeanPowerResponse,
            // Forecast
            crate::routes::forecast::ForecastResponse
        )
    ),
    info(
        title = "Aiome Management Console API",
        version = "0.1.0",
        description = "Core API for the Autonomous AI Operating System"
    ),
    modifiers(&SecurityAddon)
)
]
pub struct ApiDoc;

#[cfg(debug_assertions)]
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::karma::trigger_failure_demo,
        crate::routes::karma::trigger_security_demo,
        crate::routes::karma::trigger_federation_demo,
        crate::routes::settings::test_connection,
        crate::routes::bootstrap::factory_reset,
    ),
    components(schemas(crate::routes::bootstrap::FactoryResetResponse))
)]
pub struct DebugApiDoc;

#[cfg(any(debug_assertions, feature = "demo"))]
#[derive(OpenApi)]
#[openapi(paths(crate::routes::demo::start_demo))]
pub struct DemoApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("API_SERVER_SECRET")
                        .build(),
                ),
            );
        }

        #[cfg(debug_assertions)]
        {
            let mut debug_doc = DebugApiDoc::openapi();
            for (path, item) in debug_doc.paths.paths {
                openapi.paths.paths.insert(path, item);
            }
            if let (Some(debug_components), Some(components)) =
                (debug_doc.components.take(), openapi.components.as_mut())
            {
                for (name, schema) in debug_components.schemas {
                    components.schemas.insert(name, schema);
                }
            }
        }

        #[cfg(any(debug_assertions, feature = "demo"))]
        {
            let demo_doc = DemoApiDoc::openapi();
            for (path, item) in demo_doc.paths.paths {
                openapi.paths.paths.insert(path, item);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_schema_generation() {
        let schema = ApiDoc::openapi().to_pretty_json().unwrap();
        assert!(!schema.is_empty());

        // Ensure the 5 commerce endpoints are permanently registered in OpenAPI paths
        let schema_json: serde_json::Value = serde_json::from_str(&schema).unwrap();
        let paths = schema_json["paths"].as_object().expect("paths object");

        assert!(
            paths.contains_key("/api/v1/commerce/subscription/create"),
            "Missing subscription create"
        );
        assert!(
            paths.contains_key("/api/v1/commerce/subscription/cancel"),
            "Missing subscription cancel"
        );
        assert!(
            paths.contains_key("/api/v1/commerce/subscription/{agent_id}"),
            "Missing subscription get"
        );
        assert!(
            paths.contains_key("/api/v1/commerce/checkout-session/create"),
            "Missing checkout session create"
        );
        assert!(
            paths.contains_key("/api/v1/commerce/customer-portal/create"),
            "Missing customer portal create"
        );

        let docs_dir = std::path::Path::new("../../docs");
        if !docs_dir.exists() {
            std::fs::create_dir_all(docs_dir).unwrap();
        }
        std::fs::write(docs_dir.join("openapi.json"), schema)
            .expect("Failed to write OpenAPI schema");
    }
}
