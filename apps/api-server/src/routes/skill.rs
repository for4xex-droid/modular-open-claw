/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

#[derive(Serialize, utoipa::ToSchema)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub source: String, // "wasm", "mcp", "marketplace"
    pub status: String, // "Active", "Installed", "Available"
    pub layer: u8,
    pub tools: Vec<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ImportSkillRequest {
    pub url: String,
}

#[utoipa::path(
    get,
    path = "/api/skills",
    responses(
        (status = 200, description = "List all active skills in the Swarm", body = [SkillSummary]),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn list_skills(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<Vec<SkillSummary>>, AppError> {
    let mut skills = Vec::new();

    // 1. Wasm Skills
    let wasm_meta = state.wasm_skill_manager.list_skills_with_metadata();
    for meta in wasm_meta {
        skills.push(SkillSummary {
            name: meta.name.clone(),
            description: meta.description,
            source: "wasm".to_string(),
            status: "Active".to_string(),
            layer: 3,
            tools: meta.capabilities,
        });
    }

    // 2. MCP Skills (Active Clients)
    let mcp_ids = state.mcp_manager.active_client_ids().await;
    for id in mcp_ids {
        if let Some(client) = state.mcp_manager.get_client(&id).await {
            let tools = client.list_tools().await.unwrap_or_default();
            skills.push(SkillSummary {
                name: id.clone(),
                description: format!("Running MCP Server: {}", id),
                source: "mcp".to_string(),
                status: "Active".to_string(),
                layer: 4,
                tools: tools.into_iter().map(|t| t.name).collect(),
            });
        }
    }

    // 3. Marketplace Skills (Local Discovery)
    let marketplace_path = state.config.resolver.resolve("skills/marketplace");
    if let Ok(entries) = std::fs::read_dir(&marketplace_path) {
        use infrastructure::skills::importer::SkillImporter;
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let mut manifests = Vec::new();
                    match ext {
                        "md" => {
                            if let Some(m) = SkillImporter::parse_skill_md(&content) {
                                manifests.push(m);
                            }
                        }
                        "yaml" | "yml" => {
                            if let Some(m) = SkillImporter::parse_agency_yaml(&content) {
                                manifests.push(m);
                            }
                        }
                        "json" => {
                            manifests.extend(SkillImporter::parse_openapi(&content));
                        }
                        _ => {}
                    }

                    for m in manifests {
                        skills.push(SkillSummary {
                            name: m.l1.name.clone(),
                            description: m.l1.trigger_description.clone(),
                            source: "marketplace".to_string(),
                            status: "Available".to_string(),
                            layer: 4,
                            tools: vec![m.l1.name.to_lowercase().replace(' ', "_")],
                        });
                    }
                }
            }
        }
    }

    Ok(Json(skills))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ImportRequest {
    pub url: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct McpSpawnRequest {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/skills/import",
    request_body = ImportRequest,
    responses(
        (status = 200, description = "Skill imported successfully"),
        (status = 400, description = "Invalid URL or fetch failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Security Block (SSRF)")
    ),
    security(("api_key" = []))
)]
pub async fn import_skill(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<ImportRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!(
        "👹 [Vampire Attack] Attempting to import skill from: {}",
        payload.url
    );

    // 1. SSRF Validation
    state
        .security_policy
        .validate_url(&payload.url)
        .await
        .map_err(|e| aiome_core::error::AiomeError::SecurityViolation {
            reason: format!("SSRF Blocked: {}", e),
        })?;

    // 2. Fetch the content with size limit (W-8 Fix)
    let resp = state
        .http_client
        .get(&payload.url)
        .send()
        .await
        .map_err(|e| aiome_core::error::AiomeError::RemoteServiceError {
            url: payload.url.clone(),
            source: e.into(),
        })?;

    if let Some(cl) = resp.content_length() {
        if cl > 1_048_576 {
            return Err(aiome_core::error::AiomeError::SecurityViolation {
                reason: format!("Imported skill content length ({}) exceeds 1MB limit", cl),
            }
            .into());
        }
    }

    use futures::StreamExt;
    let mut body_stream = resp.bytes_stream();
    let mut content_bytes = Vec::new();

    while let Some(chunk_res) = body_stream.next().await {
        let chunk =
            chunk_res.map_err(
                |e| aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                    reason: format!("Failed to read stream: {}", e),
                },
            )?;

        content_bytes.extend_from_slice(&chunk);

        if content_bytes.len() > 1_048_576 {
            return Err(aiome_core::error::AiomeError::SecurityViolation {
                reason: "Imported skill content exceeds 1MB limit (streaming detected)".into(),
            }
            .into());
        }
    }

    let raw_content = String::from_utf8(content_bytes).map_err(|e| {
        aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
            reason: format!("Content is not valid UTF-8: {}", e),
        }
    })?;

    // 🛡️ [GlassWorm Shield] Strip invisible unicode before compiling imported code
    let content = shared::guardrails::strip_invisible_unicode(&raw_content).into_owned();

    // 2. Parse using SkillImporter (Infrastructure)
    use infrastructure::skills::cleanroom::Cleanroom;
    use infrastructure::skills::importer::SkillImporter;

    let manifests = if payload.url.ends_with(".yaml") || payload.url.ends_with(".yml") {
        SkillImporter::parse_agency_yaml(&content)
            .into_iter()
            .collect::<Vec<_>>()
    } else if payload.url.ends_with(".json") {
        SkillImporter::parse_openapi(&content)
    } else {
        SkillImporter::parse_skill_md(&content)
            .into_iter()
            .collect::<Vec<_>>()
    };

    if manifests.is_empty() {
        return Err(
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: "No valid skills found in the content.".to_string(),
            }
            .into(),
        );
    }

    // 3. Process via Cleanroom (N2)
    let cleanroom = Cleanroom::new(
        (**state.skill_forge).clone(),
        state.config.resolver.resolve("cleanroom"),
        Some((*state.provider).clone()),
    );

    let mut imported_skills = Vec::new();
    let mut errors = Vec::new();

    for manifest in manifests {
        let skill_name = manifest.l1.name.clone();
        match cleanroom.process_import(manifest).await {
            Ok(_) => {
                info!(
                    "✅ [Vampire Attack] Successfully imported and forged skill: {}",
                    skill_name
                );
                imported_skills.push(skill_name);
            }
            Err(e) => {
                error!(
                    "❌ [Vampire Attack] Failed to import skill '{}': {}",
                    skill_name, e
                );
                errors.push(format!("{}: {}", skill_name, e));
            }
        }
    }

    if imported_skills.is_empty() && !errors.is_empty() {
        return Err(
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: format!("All skill imports failed: {:?}", errors),
            }
            .into(),
        );
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "imported_count": imported_skills.len(),
        "skills": imported_skills,
        "errors": if errors.is_empty() { None } else { Some(errors) },
        "message": "Skills successfully imported and forged in cleanroom."
    })))
}

#[utoipa::path(
    post,
    path = "/api/skills/mcp/spawn",
    request_body = McpSpawnRequest,
    responses(
        (status = 200, description = "MCP Server spawned", body = serde_json::Value),
        (status = 500, description = "Spawn failed")
    ),
    security(("api_key" = []))
)]
pub async fn spawn_mcp_server(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<McpSpawnRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // SEC: Command whitelist to prevent arbitrary process execution
    const ALLOWED_MCP_COMMANDS: &[&str] = &["npx", "node", "python3", "uvx"];
    if !ALLOWED_MCP_COMMANDS.contains(&payload.command.as_str()) {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: format!("MCP command '{}' is not whitelisted", payload.command),
        }
        .into());
    }

    // SEC: Block path traversal and injection in args
    let dangerous_parts = [
        "..", "$(", "`", "${", ";", "&&", "||", ">", "<", "|", "\n", "\r", "%0a", "%0d",
    ];
    for arg in &payload.args {
        for part in dangerous_parts {
            if arg.contains(part) {
                return Err(aiome_core::error::AiomeError::SecurityViolation {
                    reason: format!("MCP arg contains prohibited sequence: {}", part),
                }
                .into());
            }
        }
    }

    state
        .mcp_manager
        .spawn_stdio_server(payload.id.clone(), &payload.command, payload.args, std::collections::HashMap::new())
        .await
        .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
            reason: format!("MCP Spawn Error: {}", e),
        })?;

    Ok(Json(
        serde_json::json!({"status": "success", "id": payload.id}),
    ))
}

#[utoipa::path(
    put,
    path = "/api/skills/mcp/config",
    request_body(content = inline(crate::mcp::discovery::McpDiscoveryFile), description = "The entire MCP discovery config file content"),
    responses(
        (status = 200, description = "Config updated and MCP processes hot-reloaded")
    ),
    security(("api_key" = []))
)]
pub async fn update_mcp_config(
    State(state): State<AppState>,
    Json(payload): Json<crate::mcp::discovery::McpDiscoveryFile>,
) -> Result<Json<serde_json::Value>, AppError> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = std::path::PathBuf::from(home).join(".aiome/mcp_servers.json");
    
    if let Some(parent) = config_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    
    let serialized = serde_json::to_string_pretty(&payload)
        .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
        
    tokio::fs::write(&config_path, serialized)
        .await
        .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;

    state.mcp_manager.kill_all().await;
    let _ = state.registry.clear_mcp_servers().await;

    crate::mcp::discovery::discover_and_connect(&state.mcp_manager, &state.registry)
        .await
        .map_err(|e| AppError::internal(format!("Failed to reload Discovery: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Config updated and MCP processes hot-reloaded"
    })))
}

#[utoipa::path(
    get,
    path = "/api/skills/mcp/config",
    responses(
        (status = 200, description = "Return the current MCP discovery config file content")
    ),
    security(("api_key" = []))
)]
pub async fn get_mcp_config(
    State(_state): State<AppState>,
) -> Result<Json<crate::mcp::discovery::McpDiscoveryFile>, AppError> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = std::path::PathBuf::from(home).join(".aiome/mcp_servers.json");
    
    let content = tokio::fs::read_to_string(&config_path).await.unwrap_or_else(|_| {
        r#"{"mcp_servers": {}}"#.to_string()
    });
    
    let config: crate::mcp::discovery::McpDiscoveryFile = serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Invalid MCP configuration file: {}", e)))?;
        
    Ok(Json(config))
}
