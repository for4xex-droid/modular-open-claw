import sys

with open("apps/samsara-hub/src/main.rs", "r") as f:
    lines = f.readlines()

new_lines = []
skip = False
for i, line in enumerate(lines):
    if line.startswith("async fn health_handler()"):
        skip = True
    
    if line.startswith("pub fn build_app(state: Arc<HubState>) -> Router {"):
        skip = False

    if not skip:
        new_lines.append(line)

content = "".join(new_lines)

# Now replace the router setup in build_app
content = content.replace(
"""    let router = Router::new()
        .route("/api/v1/federation/sync", post(sync_handler))
        .route("/api/v1/federation/push", post(push_handler))
        .route("/api/v1/registry/agents", get(list_agents_handler))
        .route(
            "/api/v1/biome/topics",
            get(list_topics_handler).post(create_topic_handler),
        )
        .route("/api/v1/biome/relay", post(biome_relay_handler))
        .route("/api/v1/biome/ws", get(biome_ws_handler))
        .route("/api/v1/relay/timeline/sync", post(timeline_sync_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        // WS and Health handled outside middleware
        .route("/api/v1/federation/ws", get(ws_handler))
        .route("/api/v1/health", get(health_handler));""",
"""    use crate::handlers::biome::{biome_relay_handler, biome_ws_handler, create_topic_handler, list_topics_handler};
    use crate::handlers::system::{health_handler, list_agents_handler};
    use crate::handlers::timeline::timeline_sync_handler;
    use crate::handlers::middleware::auth_middleware;

    let router = Router::new()
        .route("/api/v1/federation/sync", post(sync_handler))
        .route("/api/v1/federation/push", post(push_handler))
        .route("/api/v1/registry/agents", get(list_agents_handler))
        .route(
            "/api/v1/biome/topics",
            get(list_topics_handler).post(create_topic_handler),
        )
        .route("/api/v1/biome/relay", post(biome_relay_handler))
        .route("/api/v1/biome/ws", get(biome_ws_handler))
        .route("/api/v1/relay/timeline/sync", post(timeline_sync_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        // WS and Health handled outside middleware
        .route("/api/v1/federation/ws", get(ws_handler))
        .route("/api/v1/health", get(health_handler));"""
)

with open("apps/samsara-hub/src/main.rs", "w") as f:
    f.write(content)
