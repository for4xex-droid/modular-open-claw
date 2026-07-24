/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::auth;
use crate::routes;
use crate::AppState;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::{
    http::{
        header::{
            CONTENT_SECURITY_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
            X_FRAME_OPTIONS,
        },
        HeaderValue, StatusCode,
    },
    routing::{delete, get, post, put},
    Router,
};
use std::time::Duration;
use std::time::Instant;
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, services::ServeDir,
    set_header::SetResponseHeaderLayer, timeout::TimeoutLayer,
};
#[cfg(debug_assertions)]
use utoipa::OpenApi;

pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let path = req.uri().path().to_owned();
    let method = req.method().clone();

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    metrics::counter!("aiome_api_requests_total", "method" => method.to_string(), "path" => path.clone(), "status" => status.clone()).increment(1);
    metrics::histogram!("aiome_api_request_duration_seconds", "method" => method.to_string(), "path" => path, "status" => status).record(latency);

    response
}

pub fn build_app(
    state: AppState,
    cors_layer: CorsLayer,
    static_path: String,
    mut plugin_registry: crate::plugin_loader::PluginRegistry,
    metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
) -> Router {
    let internal_router = Router::new()
        .route(
            "/api/v1/metrics",
            get(move || std::future::ready(metrics_handle.render())),
        )
        .route(
            "/api/v1/whisper/monologue",
            get(routes::whisper::get_monologue_history),
        )
        .route(
            "/api/v1/ollama/models",
            get(routes::settings::get_ollama_models),
        )
        .route(
            "/api/v1/commerce/balance/:agent_id",
            get(routes::commerce::get_balance),
        )
        .route(
            "/api/v1/commerce/points/:agent_id",
            get(routes::commerce::get_points),
        )
        .route(
            "/api/v1/commerce/history/:agent_id",
            get(routes::commerce::get_transaction_history),
        )
        .route(
            "/api/v1/commerce/wishlist/:agent_id",
            get(routes::commerce::get_wishlist),
        )
        .route(
            "/api/v1/commerce/convert-points",
            post(routes::commerce::convert_points),
        )
        .route(
            "/api/v1/commerce/transfer",
            post(routes::commerce::transfer),
        )
        .route(
            "/api/v1/commerce/escrow/history/:agent_id",
            get(routes::commerce::list_escrows),
        )
        .route(
            "/api/v1/commerce/escrow/:escrow_id/release",
            post(routes::commerce::release_escrow),
        )
        .route(
            "/api/v1/commerce/purchase/:agent_id",
            axum::routing::post(routes::commerce::execute_purchase).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(2)), // 1 purchase per 2s
            ),
        )
        .route(
            "/api/v1/commerce/checkout-session/create",
            axum::routing::post(routes::commerce::create_checkout_session).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(5)), // 1 create per 5s
            ),
        )
        .route(
            "/api/v1/commerce/customer-portal/create",
            axum::routing::post(routes::commerce::create_portal_session).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(5)), // 1 create per 5s
            ),
        )
        .route("/api/v1/admin/ban", post(routes::admin::create_ban))
        .route("/api/v1/admin/unban", post(routes::admin::remove_ban))
        .route("/api/v1/admin/bans", get(routes::admin::list_bans))
        .route(
            "/api/v1/commerce/subscription/create",
            axum::routing::post(routes::commerce::create_subscription).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(5)), // 1 create per 5s
            ),
        )
        .route(
            "/api/v1/commerce/subscription/cancel",
            axum::routing::post(routes::commerce::cancel_subscription).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(2)), // 1 cancel per 2s
            ),
        )
        .route(
            "/api/v1/commerce/subscription/:agent_id",
            axum::routing::get(routes::commerce::get_subscription_status),
        )
        .route(
            "/api/v1/logs",
            get(routes::audit::get_logs).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth::admin_only_middleware,
            )),
        )
        .route(
            "/api/v1/system/spec-export",
            get(routes::general::export_spec_kit).route_layer(
                axum::middleware::from_fn_with_state(state.clone(), auth::admin_only_middleware),
            ),
        )
        .route(
            "/api/v1/quality-gate/history",
            get(routes::quality_gate::get_quality_gate_history),
        )
        .route(
            "/api/v1/gift/send/:agent_id",
            axum::routing::post(routes::gift::send_gift).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(30)), // 1 gift per 30s
            ),
        )
        .route(
            "/api/v1/gift/policy/:agent_id",
            get(routes::gift::get_gift_policy),
        )
        .nest(
            "/api/v1/audit",
            Router::new()
                .route("/ledger", get(routes::audit::get_audit_ledger))
                .route("/prompt-stats", get(routes::audit::get_audit_prompt_stats))
                .route("/diagnostics", get(routes::audit::get_diagnoses))
                .route(
                    "/diagnostics/summary",
                    get(routes::audit::get_diagnostics_summary),
                )
                .route("/quarantine", get(routes::audit::get_quarantined_assets))
                .route(
                    "/quarantine/:id/release",
                    post(routes::audit::release_quarantined_asset),
                )
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    auth::admin_only_middleware,
                )),
        )
        .nest(
            "/api/v1/vault",
            Router::new()
                .route("/status", get(routes::vault::vault_status))
                .route("/secrets", put(routes::vault::vault_upsert))
                .route("/secrets/:key", delete(routes::vault::vault_delete))
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    auth::admin_only_middleware,
                )),
        )
        // Bootstrap: Factory Reset moved to debug_assertions block
        .route("/api/v1/trends", get(routes::general::get_trends))
        .route(
            "/api/v1/gig/publish",
            axum::routing::post(routes::gig::publish_intent).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(30)), // 1 publish per 30s
            ),
        )
        .route(
            "/api/v1/gig/bid",
            axum::routing::post(routes::gig::submit_bid).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(5)), // 1 bid per 5s
            ),
        )
        .route(
            "/api/v1/gig/accept/:intent_id/:bid_id",
            post(routes::gig::accept_bid),
        )
        .route("/api/v1/gig/deliver", post(routes::gig::deliver))
        .route("/api/v1/gig/verify/:order_id", post(routes::gig::verify))
        // --- LoRA Marketplace ---
        .route("/api/v1/lora/market", get(routes::lora_market::list_market))
        .route(
            "/api/v1/lora/market/publish",
            post(routes::lora_market::publish_listing),
        )
        .route(
            "/api/v1/lora/market/purchase",
            post(routes::lora_market::purchase_listing),
        )
        .route(
            "/api/v1/lora/market/complete/:purchase_id",
            post(routes::lora_market::complete_purchase),
        )
        .route(
            "/api/v1/lora/market/:listing_id",
            axum::routing::delete(routes::lora_market::delist_listing),
        )
        .route(
            "/api/v1/lora/market/my-listings",
            get(routes::lora_market::my_listings),
        )
        .route("/api/commune/status", get(routes::commune::commune_status))
        .route(
            "/api/commune/topics",
            get(routes::commune::list_topics)
                .post(routes::commune::create_topic)
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(1, std::time::Duration::from_secs(5)), // 1 topic per 5s
                ),
        )
        .route("/api/commune/list", get(routes::commune::list_messages))
        .route(
            "/api/commune/send",
            axum::routing::post(routes::commune::send_message).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(2, std::time::Duration::from_secs(1)), // 2 messages per sec (p2p)
            ),
        )
        .route(
            "/api/commune/send/metadata-free",
            axum::routing::post(routes::commune::send_message_metadata_free).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(2, std::time::Duration::from_secs(1)), // 2 messages per sec (p2p)
            ),
        )
        .route(
            "/api/commune/autonomous/start",
            axum::routing::post(routes::commune::autonomous_start).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(2)), // 1 toggle per 2s
            ),
        )
        .route(
            "/api/commune/autonomous/stop",
            axum::routing::post(routes::commune::autonomous_stop),
        )
        .route(
            "/api/commune/autonomous/status",
            get(routes::commune::autonomous_status),
        )
        .route(
            "/api/commune/:topic/genome",
            post(routes::commune::share_genome),
        )
        .route(
            "/api/commune/:topic/genomes",
            get(routes::commune::list_shared_genomes),
        )
        .route(
            "/api/synergy/graph",
            get(routes::karma::synergy_graph_handler),
        )
        .route(
            "/api/v1/jobs/awaiting-input",
            get(routes::jobs::get_awaiting_input_jobs),
        )
        .route(
            "/api/v1/jobs/:id/cancel",
            post(routes::jobs::cancel_job_handler),
        )
        .route(
            "/api/v1/jobs/:id/logs",
            get(routes::jobs::get_job_logs_handler),
        )
        .route(
            "/api/v1/jobs/:id/review",
            post(routes::jobs::submit_job_review),
        )
        .route(
            "/api/v1/trajectory/:id",
            get(routes::jobs::get_trajectory_handler),
        )
        .route(
            "/api/v1/trajectory/:id/diagnosis",
            get(routes::jobs::get_diagnosis_handler),
        )
        .route("/api/v1/soul/status", get(routes::soul::get_soul_status))
        .route("/api/v1/soul/init", post(routes::soul::init_soul))
        .route(
            "/api/v1/a2ui/action",
            post(routes::a2ui::submit_a2ui_action),
        )
        .route(
            "/api/v1/biome/runs",
            get(routes::biome::list_runs).post(routes::biome::save_run),
        )
        .route(
            "/api/v1/biome/specimens",
            get(routes::biome::list_specimens).post(routes::biome::save_specimen),
        )
        .route(
            "/api/v1/biome/analytics/:run_id",
            get(routes::biome::get_analytics),
        );

    #[cfg(any(debug_assertions, feature = "demo"))]
    let internal_router =
        internal_router.route("/api/v1/demo/start", post(routes::demo::start_demo));

    #[cfg(debug_assertions)]
    let internal_router = internal_router
        .route(
            "/api/synergy/test/failure",
            post(routes::karma::trigger_failure_demo),
        )
        .route(
            "/api/synergy/test/security",
            post(routes::karma::trigger_security_demo),
        )
        .route(
            "/api/synergy/test/federation",
            post(routes::karma::trigger_federation_demo),
        )
        .route(
            "/api/v1/bootstrap/factory-reset",
            post(routes::bootstrap::factory_reset),
        )
        .route(
            "/api/v1/settings/test",
            post(routes::settings::test_connection),
        );

    let internal_router = internal_router
        .route(
            "/api/synergy/rules",
            get(routes::karma::get_immune_rules_handler)
                .post(routes::karma::add_immune_rule_handler),
        )
        .route(
            "/api/synergy/rules/:id",
            axum::routing::delete(routes::karma::delete_immune_rule_handler),
        )
        .route(
            "/api/system/evolution",
            get(routes::karma::get_evolution_history_handler),
        )
        .route(
            "/api/v1/voice/list",
            get(routes::voice::list_voice_assets_handler),
        )
        .route(
            "/api/v1/voice/synthesize",
            post(routes::voice::synthesize_voice_handler),
        )
        .route(
            "/api/v1/models/status",
            get(routes::model_setup::get_model_status),
        )
        .route("/api/v1/models/pull", post(routes::model_setup::pull_model))
        .route("/api/v1/lora/train", post(routes::lora::train_lora_handler))
        .route(
            "/api/v1/lora/status/:job_id",
            get(routes::lora::status_lora_handler),
        )
        .route("/api/wiki", get(routes::general::list_wiki_files))
        .route("/api/wiki/content", get(routes::general::get_wiki_content))
        .nest(
            "/api/v1/cortex",
            Router::new()
                .route("/ingest", post(routes::cortex::ingest_url_handler))
                .route("/ingest/text", post(routes::cortex::ingest_text_handler))
                .route("/documents", get(routes::cortex::list_documents_handler))
                .route(
                    "/documents/:id",
                    delete(routes::cortex::delete_document_handler),
                )
                .route("/wiki", get(routes::cortex::list_wiki_articles_handler))
                .route("/wiki/:id", get(routes::cortex::get_wiki_article_handler))
                .route("/query", post(routes::cortex::query_handler))
                .route(
                    "/suggestions",
                    get(routes::cortex::suggest_questions_handler),
                )
                .route("/synth", post(routes::cortex::synth_dataset_handler))
                .route(
                    "/dpo/dataset",
                    get(routes::cortex::export_dpo_dataset_handler),
                )
                .route("/god-nodes", get(routes::cortex::god_nodes_handler))
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(5, std::time::Duration::from_secs(1)),
                ),
        )
        .nest(
            "/api/v1/workflows",
            Router::new()
                .route(
                    "/",
                    get(routes::workflow::list_workflows).post(routes::workflow::create_workflow),
                )
                .route(
                    "/:id",
                    get(routes::workflow::get_workflow)
                        .put(routes::workflow::update_workflow)
                        .delete(routes::workflow::delete_workflow),
                )
                .route("/:id/execute", post(routes::workflow::execute_workflow))
                .route("/:id/validate", post(routes::workflow::validate_workflow))
                .route("/:id/fork", post(routes::workflow::fork_workflow))
                .route("/:id/versions", get(routes::workflow::list_versions))
                .route("/:id/executions", get(routes::workflow::list_executions))
                .route("/:id/export", get(routes::workflow::export_workflow)),
        )
        .nest(
            "/api/v1/playbooks",
            Router::new()
                .route("/", get(routes::playbook::list_playbooks))
                .route("/import", post(routes::playbook::import_playbook))
                .route("/:id/install", post(routes::playbook::install_playbook)),
        )
        .route(
            "/api/v1/settings",
            get(routes::settings::get_settings)
                .put(routes::settings::update_setting)
                .post(routes::settings::update_setting)
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(1, std::time::Duration::from_secs(2)), // 1 write per 2s
                ),
        )
        .route(
            "/api/v1/settings/identity",
            get(routes::settings::get_identity),
        )
        .route(
            "/api/v1/syndicate/guilds",
            post(crate::routes::syndicate::create_guild)
                .get(crate::routes::syndicate::list_guilds)
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(1, std::time::Duration::from_secs(1)) // 1 guild op per sec
                        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024)), // 2MB limit (G-8.6)
                ),
        )
        .route(
            "/api/v1/syndicate/guilds/:id",
            delete(crate::routes::syndicate::delete_guild).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(2)), // 1 delete per 2s
            ),
        )
        .route(
            "/api/v1/syndicate/guilds/:id/members",
            post(crate::routes::syndicate::add_member)
                .get(crate::routes::syndicate::list_members)
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(2, std::time::Duration::from_secs(1)), // 2 member ops per sec
                ),
        );

    // test_connection moved to debug_assertions block

    let internal_router = internal_router
        .route(
            "/api/v1/ekyc/session",
            post(routes::ekyc::create_ekyc_session_handler).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(5)), // 1 session per 5s
            ),
        )
        // GDPR Right to be Forgotten (A-1)
        .route(
            "/api/v1/auth/delete",
            axum::routing::delete(routes::auth::delete_account_handler).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(10)), // 1 deletion per 10s (irreversible)
            ),
        )
        .route(
            "/api/v1/ekyc/status",
            // U0-B1: このハンドラは `Extension<AuthenticatedUser>` を要求するため、
            // extension を注入する jwt_auth_middleware を明示的に適用する
            // （internal_router の auth_middleware は検証のみで extension を注入しない）
            get(routes::avatar::get_ekyc_status_handler).route_layer(
                axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::auth::jwt_auth_middleware,
                ),
            ),
        )
        .route(
            "/api/expression/status",
            get(routes::expression::expression_status),
        )
        .route(
            "/api/expression/generate",
            axum::routing::post(routes::expression::generate_expression).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(10)), // 1 generation per 10s (costly)
            ),
        )
        .route(
            "/api/expression/list",
            get(routes::expression::list_expressions),
        )
        .route(
            "/api/artifacts",
            get(routes::artifacts::list_artifacts_handler),
        )
        .route(
            "/api/artifacts/:id",
            get(routes::artifacts::get_artifact_handler)
                .delete(routes::artifacts::delete_artifact_handler),
        )
        .route(
            "/api/artifacts/:id/files/:filename",
            get(routes::artifacts::download_artifact_file_handler),
        )
        .route(
            "/api/artifacts/:id/edges",
            get(routes::artifacts::get_artifact_edges_handler),
        )
        .route(
            "/api/v1/blueprints/:id/deploy",
            post(routes::blueprint::deploy_blueprint_handler),
        )
        .route(
            "/api/expression/auto",
            axum::routing::post(routes::expression::toggle_auto_expression),
        )
        .route("/api/skills", get(routes::skill::list_skills))
        .route(
            "/api/skills/verify-proof",
            axum::routing::post(routes::proof_verifier::verify_skill_proof).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(10)), // 1 verify per 10s
            ),
        )
        .route(
            "/api/skills/import",
            axum::routing::post(routes::skill::import_skill).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(10)), // 1 import per 10s
            ),
        )
        .route(
            "/api/skills/mcp/spawn",
            axum::routing::post(routes::skill::spawn_mcp_server),
        )
        .route(
            "/api/skills/mcp/config",
            axum::routing::put(routes::skill::update_mcp_config).get(routes::skill::get_mcp_config),
        )
        .nest("/api/v1/forecast", routes::forecast::router())
        .nest(
            "/api/v1/buzz",
            Router::new()
                .route("/generate", post(routes::buzz::generate))
                .route("/pending", get(routes::buzz::list_pending))
                .route("/approve/:id", post(routes::buzz::approve))
                .route("/reject/:id", post(routes::buzz::reject))
                .route("/publish/:id", post(routes::buzz::publish))
                .route("/history", get(routes::buzz::history))
                .route(
                    "/draft/:id",
                    axum::routing::patch(routes::buzz::update_draft),
                ),
        )
        .merge(routes::security::router())
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    let streaming_router = Router::new()
        .route("/api/synergy/karma", get(routes::karma::get_karma_stream))
        .route(
            "/api/stream/chat",
            // MC `useAgentChat` posts JSON+SSE; keep GET for older clients/tools.
            get(crate::stream::trigger_agent_chat_stream)
                .post(crate::stream::trigger_agent_chat_stream),
        )
        .route("/api/stream/history", get(crate::stream::get_chat_history))
        .route(
            "/api/stream/vitality",
            get(crate::stream::trigger_system_vitality_stream),
        )
        .nest("/api/v1/mcp", crate::mcp::router())
        .nest("/api/v1/nurture-mcp", routes::nurture_mcp_proxy::router())
        .route("/api/v1/watchtower/ws", get(routes::watchtower::ws_handler))
        .route(
            "/api/agent/feedback",
            post(routes::agent::handle_karma_feedback),
        );

    let state_copy = state.clone();
    let state_for_auth = state.clone();

    // OP-088 P1: Plugin は Router<()> — with_state 後に JWT 付き merge。
    // S2S /internal は JWT 外（nest_service）。merge_routes(AppState) には載せない。
    let s2s_router = plugin_registry.take_s2s_router();
    let plugin_unit_routers = plugin_registry.plugin_unit_routers();

    // 1. Create the base authenticated router (WITHOUT global limit yet)
    let authed_router =
        internal_router
            .merge(streaming_router)
            .route_layer(axum::middleware::from_fn_with_state(
                state_for_auth,
                auth::auth_middleware,
            ));

    // 2. Create the base public router
    let mut public_router = Router::new()
        .route("/api/health", get(routes::general::get_health_status))
        .route("/health", get(routes::general::get_health_status))
        .route(
            // DEPRECATED Phase E E5: Inochi frozen — keep for PathSandbox / compat (do not expand)
            "/api/v1/avatar/inochi2d/:filename",
            get(routes::avatar::serve_inochi2d_asset),
        )
        // Bootstrap Mode (Phase 2B-CORE) — 認証不要
        .route(
            "/api/v1/bootstrap/status",
            get(routes::bootstrap::bootstrap_status),
        )
        .route(
            "/api/v1/setup/init",
            axum::routing::post(routes::setup::setup_init).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(5)),
            ),
        )
        .route(
            "/api/v1/bootstrap/detect-ollama",
            get(routes::bootstrap::detect_ollama),
        )
        .route(
            "/api/v1/auth/authorize",
            get(routes::auth::authorize_handler),
        )
        .route(
            "/api/v1/auth/token",
            post(routes::auth::token_handler).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(5, std::time::Duration::from_secs(60)),
            ),
        )
        .route(
            "/api/v1/commerce/webhook",
            axum::routing::post(routes::commerce_webhook::stripe_webhook),
        )
        .route(
            "/api/v1/commerce/webhook/polar",
            axum::routing::post(routes::commerce_webhook::polar_webhook),
        );

    // OP-088 P1-1: JWT 外で /internal（Bearer secret + OXP）。Plugin nurture_routes 配下禁止。
    if let Some(s2s) = s2s_router {
        tracing::info!("🔐 [Router] Nesting S2S /internal outside JWT (InProcess)");
        public_router = public_router.nest_service("/internal", s2s);
    }

    #[cfg(debug_assertions)]
    let public_router = public_router.merge(
        utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", crate::api::ApiDoc::openapi())
            .url("/api-docs/demo.json", crate::api::DemoApiDoc::openapi()),
    );

    let public_router = public_router
        .fallback_service(ServeDir::new(static_path).append_index_html_on_directories(true));

    // 3. Apply 2MB limit to the combined base router
    let limited_base = public_router
        .merge(authed_router)
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024));

    // 4. Create high-limit router (Must disable DEFAULT limit to allow higher ones)
    let high_limit_router = Router::new()
        .route(
            "/api/v1/voice/upload",
            axum::routing::post(routes::voice::upload_voice_handler).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(2)
                    .rate_limit(1, std::time::Duration::from_secs(1))
                    .layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        crate::auth::auth_middleware,
                    ))
                    .layer(RequestBodyLimitLayer::new(500 * 1024 * 1024)),
            ),
        )
        .route(
            "/api/avatar/upload",
            axum::routing::post(routes::avatar::upload_avatar_handler).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        crate::auth::jwt_auth_middleware,
                    ))
                    .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024)),
            ),
        )
        .route(
            "/api/avatar/ekyc-status",
            get(routes::avatar::get_ekyc_status_handler).route_layer(
                axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::auth::jwt_auth_middleware,
                ),
            ),
        )
        .route(
            // DEPRECATED Phase E E5: Inochi frozen — UI unmounted; removal needs explicit approval
            "/api/v1/avatar/inochi2d/upload",
            axum::routing::post(routes::inochi2d::upload_inochi2d_handler).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        crate::auth::jwt_auth_middleware,
                    ))
                    .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024)),
            ),
        )
        .nest(
            "/api/v1/treasure",
            Router::new()
                .route("/", get(routes::treasure::get_treasure))
                .route("/feedback", post(routes::treasure::record_feedback))
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::auth::jwt_auth_middleware,
                )),
        )
        .layer(axum::extract::DefaultBodyLimit::disable());

    // 5. Merge them - limited_base will handle its routes with 2MB, high_limit_router will handle its with 500MB/50MB
    let final_router = limited_base.merge(high_limit_router);

    // Plugin は Router<()> — with_state 後に JWT 付き merge（型不一致で silent-drop しない）
    let mut final_router = final_router.with_state(state_copy.clone());
    for plugin_router in plugin_unit_routers {
        final_router = final_router.merge(plugin_router.route_layer(
            axum::middleware::from_fn_with_state(state_copy.clone(), auth::auth_middleware),
        ));
    }

    // Assembly with Global Config Layers (CORS, Headers, etc.)
    final_router
        .layer(axum::middleware::from_fn(metrics_middleware))
        .layer(SetResponseHeaderLayer::if_not_present(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000; includeSubDomains")))
        // NOTE: 'wasm-unsafe-eval' は biome-engine (WASM) を Web Worker 内で
        // コンパイルするために必須（Worker の CSP はスクリプト応答ヘッダー由来）。
        // eval/new Function は引き続き禁止される。
        .layer(SetResponseHeaderLayer::if_not_present(CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' ws: wss: http: https:; object-src 'none'; base-uri 'self';")))
        .layer(cors_layer)
        .layer(
            tower::ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(|err: tower::BoxError| async move {
                    let error_id = uuid::Uuid::new_v4().to_string();
                    tracing::error!("Security Layer Error [Error ID: {}]: {}", error_id, err);

                    let msg = if cfg!(not(debug_assertions)) {
                        format!("An internal service error occurred. Error ID: {}", error_id)
                    } else {
                        format!("Security Layer Error: {}", err)
                    };

                    (StatusCode::INTERNAL_SERVER_ERROR, msg)
                }))
                .buffer(1024)
                .rate_limit(50, std::time::Duration::from_secs(1))
                .into_inner()
        )
}

pub async fn handle_rate_limit(_err: tower::BoxError) -> (StatusCode, &'static str) {
    (StatusCode::TOO_MANY_REQUESTS, "Rate Limit Exceeded")
}
