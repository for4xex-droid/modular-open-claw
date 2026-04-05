/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::auth;
use crate::routes;
use crate::AppState;
use axum::{
    http::{
        header::{
            CONTENT_SECURITY_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
            X_FRAME_OPTIONS,
        },
        HeaderValue, StatusCode,
    },
    routing::{delete, get, post},
    Router,
};
use std::time::Duration;
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, services::ServeDir,
    set_header::SetResponseHeaderLayer, timeout::TimeoutLayer,
};
use utoipa::OpenApi;

pub fn build_app(
    state: AppState,
    cors_layer: CorsLayer,
    static_path: &str,
    #[cfg(feature = "nurture")] nurture_state: nurture_api::NurtureState,
    plugin_registry: crate::plugin_loader::PluginRegistry,
    metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
) -> Router {
    let internal_router = Router::new()
        .route(
            "/api/v1/metrics",
            get(move || std::future::ready(metrics_handle.render())),
        )
        .route("/", get(routes::general::get_health_status))
        .route(
            "/api/v1/ollama/models",
            get(routes::settings::get_ollama_models),
        )
        .route(
            "/api/v1/commerce/balance/:agent_id",
            get(routes::commerce::get_balance),
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
            "/api/v1/commerce/subscription/create",
            axum::routing::post(routes::commerce::create_subscription),
        )
        .route(
            "/api/v1/commerce/subscription/cancel",
            axum::routing::post(routes::commerce::cancel_subscription),
        )
        .route(
            "/api/v1/commerce/subscription/:agent_id",
            axum::routing::get(routes::commerce::get_subscription_status),
        )
        .route("/api/v1/logs", get(routes::general::get_logs))
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
        .route(
            "/api/v1/audit/ledger",
            get(routes::general::get_audit_ledger),
        )
        .route(
            "/api/v1/audit/diagnostics",
            get(routes::general::get_diagnoses),
        )
        .route(
            "/api/v1/audit/quarantine",
            get(routes::general::get_quarantined_assets),
        )
        .route(
            "/api/v1/audit/quarantine/:id/release",
            post(routes::general::release_quarantined_asset),
        )
        // Bootstrap: Factory Reset (Phase 2B-4) — 認証必須 (System Admin)
        .route(
            "/api/v1/bootstrap/factory-reset",
            post(routes::bootstrap::factory_reset),
        )
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
        .route("/api/biome/status", get(routes::biome::biome_status))
        .route(
            "/api/biome/topics",
            get(routes::biome::list_topics)
                .post(routes::biome::create_topic)
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(1, std::time::Duration::from_secs(5)), // 1 topic per 5s
                ),
        )
        .route("/api/biome/list", get(routes::biome::list_messages))
        .route(
            "/api/biome/send",
            axum::routing::post(routes::biome::send_message).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(2, std::time::Duration::from_secs(1)), // 2 messages per sec (p2p)
            ),
        )
        .route(
            "/api/biome/autonomous/start",
            axum::routing::post(routes::biome::autonomous_start).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(2)), // 1 toggle per 2s
            ),
        )
        .route(
            "/api/biome/autonomous/stop",
            axum::routing::post(routes::biome::autonomous_stop),
        )
        .route(
            "/api/biome/autonomous/status",
            get(routes::biome::autonomous_status),
        )
        .route(
            "/api/synergy/graph",
            get(routes::karma::synergy_graph_handler),
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
        );

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
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(5, std::time::Duration::from_secs(1)),
                ),
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
        )
        .route("/api/v1/demo/start", post(routes::demo::start_demo));

    #[cfg(debug_assertions)]
    let internal_router = internal_router.route(
        "/api/v1/settings/test",
        post(routes::settings::test_connection),
    );

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
        .route(
            "/api/v1/ekyc/status",
            get(routes::avatar::get_ekyc_status_handler),
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
            "/api/expression/auto",
            axum::routing::post(routes::expression::toggle_auto_expression),
        )
        .route("/api/skills", get(routes::skill::list_skills))
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
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    let streaming_router = Router::new()
        .route("/api/synergy/karma", get(routes::karma::get_karma_stream))
        .route(
            "/api/stream/chat",
            get(crate::stream::trigger_agent_chat_stream),
        )
        .route(
            "/api/stream/vitality",
            get(crate::stream::trigger_system_vitality_stream),
        )
        .nest("/api/v1/mcp", crate::mcp::router())
        .route("/api/v1/watchtower/ws", get(routes::watchtower::ws_handler))
        .route(
            "/api/agent/feedback",
            post(routes::agent::handle_karma_feedback),
        );

    let state_copy = state.clone();
    let state_for_auth = state.clone();

    // 1. Create the base authenticated router (WITHOUT global limit yet)
    let mut authed_router = internal_router.merge(streaming_router);

    #[cfg(feature = "nurture")]
    {
        authed_router = authed_router.merge(nurture_api::routes::nurture_routes(nurture_state));
    }

    let authed_router = plugin_registry.merge_routes(authed_router).route_layer(
        axum::middleware::from_fn_with_state(state_for_auth, auth::auth_middleware),
    );

    // 2. Create the base public router
    let public_router = Router::new()
        .route("/api/health", get(routes::general::get_health_status))
        .route("/health", get(routes::general::get_health_status))
        // Bootstrap Mode (Phase 2B-CORE) — 認証不要
        .route(
            "/api/v1/bootstrap/status",
            get(routes::bootstrap::bootstrap_status),
        )
        .route(
            "/api/v1/bootstrap/detect-ollama",
            get(routes::bootstrap::detect_ollama),
        )
        .route(
            "/api/v1/auth/authorize",
            get(routes::auth::authorize_handler),
        )
        .route("/api/v1/auth/token", post(routes::auth::token_handler))
        .route(
            "/api/v1/commerce/webhook",
            axum::routing::post(routes::commerce_webhook::stripe_webhook),
        )
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", crate::api::ApiDoc::openapi()),
        )
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

    // Assembly with Global Config Layers (CORS, Headers, etc.)
    final_router
        .layer(SetResponseHeaderLayer::if_not_present(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000; includeSubDomains")))
        .layer(SetResponseHeaderLayer::if_not_present(CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' ws: wss: http: https:; object-src 'none'; base-uri 'self';")))
        .layer(cors_layer)
        .layer(
            tower::ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(|err: tower::BoxError| async move {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Security Layer Error: {}", err))
                }))
                .buffer(1024)
                .rate_limit(50, std::time::Duration::from_secs(1))
                .into_inner()
        )
        .with_state(state_copy)
}

pub async fn handle_rate_limit(_err: tower::BoxError) -> (StatusCode, &'static str) {
    (StatusCode::TOO_MANY_REQUESTS, "Rate Limit Exceeded")
}
