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
        crate::routes::general::get_logs,
        crate::routes::general::list_wiki_files,
        crate::routes::general::get_wiki_content,
        crate::routes::settings::get_settings,
        crate::routes::settings::update_setting,
        crate::routes::settings::get_ollama_models,
        crate::routes::skill::list_skills,
        crate::routes::skill::import_skill,
        crate::routes::skill::spawn_mcp_server,
        crate::routes::general::get_health_status,
        // Agent
        crate::routes::agent::trigger_agent_chat,
        crate::routes::agent::handle_karma_feedback,
        // Jobs
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
        // Biome
        crate::routes::biome::biome_status,
        crate::routes::biome::list_topics,
        crate::routes::biome::create_topic,
        crate::routes::biome::autonomous_start,
        crate::routes::biome::autonomous_stop,
        crate::routes::biome::autonomous_status,
        crate::routes::biome::list_messages,
        crate::routes::biome::send_message,
        // Expression
        crate::routes::expression::expression_status,
        crate::routes::expression::generate_expression,
        crate::routes::expression::list_expressions,
        crate::routes::expression::toggle_auto_expression,
        // Commerce
        crate::routes::commerce::get_balance,
        crate::routes::commerce::execute_purchase,
        // Gift
        crate::routes::gift::send_gift,
        crate::routes::gift::get_gift_policy,
        // Voice & Avatar
        crate::routes::voice::list_voice_assets_handler,
        // Artifacts
        crate::routes::artifacts::list_artifacts_handler,
        crate::routes::artifacts::get_artifact_handler,
        crate::routes::artifacts::download_artifact_file_handler,
        crate::routes::artifacts::delete_artifact_handler,
        crate::routes::artifacts::get_artifact_edges_handler,
        // EKYC
        crate::routes::ekyc::create_ekyc_session_handler,
        // Audit & Trends (Phase 8.6)
        crate::routes::general::get_audit_ledger,
        crate::routes::general::get_diagnoses,
        crate::routes::general::get_trends,
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
        crate::routes::cortex::delete_document_handler
    ),
    components(
        schemas(
            crate::routes::general::LogEntryResponse,
            crate::routes::settings::UpdateSettingsRequest,
            crate::routes::settings::TestConnectionRequest,
            crate::routes::settings::TestConnectionResponse,
            crate::routes::skill::SkillSummary,
            crate::routes::skill::ImportRequest,
            crate::routes::skill::McpSpawnRequest,
            shared::health::ResourceStatus,
            aiome_core::contracts::SystemSetting,
            aiome_core::contracts::ImmuneRule,
            crate::routes::agent::AgentChatRequest,
            crate::routes::agent::ChatMessage,
            crate::routes::agent::KarmaFeedbackRequest,
            crate::routes::karma::GraphNode,
            crate::routes::karma::GraphEdge,
            crate::routes::karma::GraphData,
            crate::routes::biome::SendBiomeRequest,
            crate::routes::biome::StartAutonomousRequest,
            crate::routes::expression::ListParams,
            crate::routes::expression::AutoToggle,
            crate::routes::commerce::PurchaseRequest,
            aiome_core_contracts::commerce::GiftRequest,
            crate::routes::gift::GiftResponse,
            crate::routes::gift::GiftPolicyResponse,
            crate::routes::artifacts::ListArtifactsParams,
            crate::routes::general::AuditLedgerResponse,
            crate::routes::general::DiagnosisResponse,
            crate::routes::general::TrendsResponse,
            aiome_core_contracts::traits::TrendItem,
            crate::routes::ekyc::EkycSessionResponse,
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
#[openapi(paths(
    crate::routes::karma::trigger_failure_demo,
    crate::routes::karma::trigger_security_demo,
    crate::routes::karma::trigger_federation_demo,
    crate::routes::settings::test_connection,
))]
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
            let mut demo_doc = DemoApiDoc::openapi();
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

        let docs_dir = std::path::Path::new("../../docs");
        if !docs_dir.exists() {
            std::fs::create_dir_all(docs_dir).unwrap();
        }
        std::fs::write(docs_dir.join("openapi.json"), schema)
            .expect("Failed to write OpenAPI schema");
    }
}
