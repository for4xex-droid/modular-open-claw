/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! Agent Playbooks (F-1): 公式業務テンプレートの同梱レジストリと
//! list / install / import API。
//! 公式 Playbook は `include_str!` によるバイナリ同梱であり、
//! `~/.aiome` への書き出しは行わない。

use crate::app_state::AppState;
use crate::auth::Authenticated;
use crate::error::AppError;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use infrastructure::validator::DefaultConstitutionalValidator;
use infrastructure::workflow::playbook::PlaybookManifest;
use infrastructure::workflow::store::WorkflowStore;
use infrastructure::workflow::validator::WorkflowValidator;
use tracing::warn;
use uuid::Uuid;

/// バイナリ同梱の公式 Playbook（id, JSON 本文）
static BUNDLED_PLAYBOOKS: &[(&str, &str)] = &[
    (
        "seo-operations",
        include_str!("../../assets/playbooks/seo-operations.json"),
    ),
    (
        "sns-operations",
        include_str!("../../assets/playbooks/sns-operations.json"),
    ),
    (
        "competitor-research",
        include_str!("../../assets/playbooks/competitor-research.json"),
    ),
    (
        "support-triage",
        include_str!("../../assets/playbooks/support-triage.json"),
    ),
];

/// 同梱 Playbook をすべてパースして返す。
/// パースに失敗したアセットは warn ログを出して除外する（パニックしない）。
pub(crate) fn load_bundled() -> Vec<PlaybookManifest> {
    BUNDLED_PLAYBOOKS
        .iter()
        .filter_map(|(id, raw)| match serde_json::from_str::<PlaybookManifest>(raw) {
            Ok(manifest) => Some(manifest),
            Err(e) => {
                warn!(
                    "⚠️ [Playbooks] Bundled playbook asset {:?} failed to parse and was excluded: {}",
                    id, e
                );
                None
            }
        })
        .collect()
}

/// Playbook 一覧の1要素
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PlaybookSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub workflow_count: usize,
    pub required_skills: Vec<String>,
    pub required_mcp_servers: Vec<String>,
}

/// install / import 成功時のレスポンス
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PlaybookInstallResponse {
    pub playbook_id: String,
    pub created_workflow_ids: Vec<String>,
}

/// [GET] /api/v1/playbooks — 同梱 Playbook の一覧
#[utoipa::path(
    get,
    path = "/api/v1/playbooks",
    responses(
        (status = 200, description = "List bundled official playbooks", body = [PlaybookSummary]),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn list_playbooks(_auth: Authenticated) -> Result<impl IntoResponse, AppError> {
    let list: Vec<PlaybookSummary> = load_bundled()
        .into_iter()
        .map(|pb| PlaybookSummary {
            workflow_count: pb.workflows.len(),
            id: pb.id,
            name: pb.name,
            description: pb.description,
            tags: pb.tags,
            required_skills: pb.required_skills,
            required_mcp_servers: pb.required_mcp_servers,
        })
        .collect();
    Ok((StatusCode::OK, Json(list)))
}

/// [POST] /api/v1/playbooks/:id/install — 同梱 Playbook の導入
#[utoipa::path(
    post,
    path = "/api/v1/playbooks/{id}/install",
    responses(
        (status = 200, description = "Playbook installed", body = PlaybookInstallResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Unknown playbook id"),
        (status = 422, description = "Missing dependencies (skills / MCP servers)")
    ),
    security(("api_key" = []))
)]
pub async fn install_playbook(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::response::Response, AppError> {
    let manifest = load_bundled()
        .into_iter()
        .find(|pb| pb.id == id)
        .ok_or_else(|| AppError::not_found(format!("Playbook not found: {}", id)))?;

    install_manifest(&state, &auth, manifest).await
}

/// [POST] /api/v1/playbooks/import — 任意マニフェストの導入
#[utoipa::path(
    post,
    path = "/api/v1/playbooks/import",
    responses(
        (status = 200, description = "Playbook imported", body = PlaybookInstallResponse),
        (status = 400, description = "Manifest structure violation"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Missing dependencies (skills / MCP servers)")
    ),
    security(("api_key" = []))
)]
pub async fn import_playbook(
    auth: Authenticated,
    State(state): State<AppState>,
    Json(manifest): Json<PlaybookManifest>,
) -> Result<axum::response::Response, AppError> {
    manifest
        .validate_structure()
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    install_manifest(&state, &auth, manifest).await
}

/// 導入の共通処理: (a) 依存検査 → (b) 全ワークフロー事前検証 → (c) 作成
/// → (d) 途中失敗時はベストエフォートでロールバック。
/// 依存欠落時は 422 + 欠落一覧の JSON を返す（DB には一切書かない）。
async fn install_manifest(
    state: &AppState,
    auth: &Authenticated,
    manifest: PlaybookManifest,
) -> Result<axum::response::Response, AppError> {
    use axum::response::IntoResponse as _;

    // (a) 依存検査
    let available_skills: Vec<String> = state
        .wasm_skill_manager
        .list_skills_with_metadata()
        .into_iter()
        .map(|m| m.name)
        .collect();
    let missing_skills: Vec<&String> = manifest
        .required_skills
        .iter()
        .filter(|s| !available_skills.contains(s))
        .collect();

    let configured_mcp_servers = configured_mcp_server_names();
    let missing_mcp_servers: Vec<&String> = manifest
        .required_mcp_servers
        .iter()
        .filter(|s| !configured_mcp_servers.contains(*s))
        .collect();

    if !missing_skills.is_empty() || !missing_mcp_servers.is_empty() {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "missing_skills": missing_skills,
                "missing_mcp_servers": missing_mcp_servers,
            })),
        )
            .into_response());
    }

    // (b) 全ワークフローの事前検証（1つでも失敗したら DB には書かない）
    let validator = DefaultConstitutionalValidator::new(state.provider.get_inner().clone(), None);
    for wf in &manifest.workflows {
        WorkflowValidator::validate(wf, &validator)
            .await
            .map_err(|e| {
                AppError::bad_request(format!(
                    "Playbook {:?} workflow {:?} failed validation: {:?}",
                    manifest.id, wf.name, e
                ))
            })?;
    }

    // (c) 作成（新規 UUID を採番。マニフェスト内の id は使わない）
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());
    let creator_id = auth.agent_id.to_string();
    let mut created_ids: Vec<Uuid> = Vec::new();

    for wf in &manifest.workflows {
        let new_id = Uuid::new_v4();
        let mut new_def = wf.clone();
        new_def.id = new_id;
        new_def.version = 1;

        let result = async {
            store
                .create_workflow(
                    new_id,
                    &creator_id,
                    &wf.name,
                    &wf.description,
                    "private",
                    manifest.tags.clone(),
                )
                .await?;
            store
                .save_version(
                    new_id,
                    1,
                    &new_def,
                    &format!("Imported from playbook {}", manifest.id),
                )
                .await
        }
        .await;

        match result {
            Ok(()) => created_ids.push(new_id),
            Err(e) => {
                // (d) ベストエフォートのロールバック（部分適用を残さない）
                for rollback_id in &created_ids {
                    if let Err(re) = store.delete_workflow(*rollback_id).await {
                        warn!(
                            "⚠️ [Playbooks] Rollback failed for workflow {}: {}",
                            rollback_id, re
                        );
                    }
                }
                return Err(AppError::internal(format!(
                    "Failed to install playbook {:?} at workflow {:?}: {}",
                    manifest.id, wf.name, e
                )));
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(PlaybookInstallResponse {
            playbook_id: manifest.id,
            created_workflow_ids: created_ids.iter().map(|u| u.to_string()).collect(),
        }),
    )
        .into_response())
}

/// `~/.aiome/mcp_servers.json` に設定済みの MCP サーバー名を返す。
/// ファイル不存在・パース不能は「設定なし」として空を返す（= 全 MCP 欠落扱い）。
fn configured_mcp_server_names() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = std::path::PathBuf::from(home).join(".aiome/mcp_servers.json");
    let Ok(raw) = std::fs::read_to_string(&config_path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    value
        .get("mcp_servers")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use infrastructure::workflow::schema::{NodeType, TriggerType};

    /// アセット品質ゲート: 同梱4本すべてがパース・構造検証を通過し、
    /// 全 Start ノードが Manual トリガーであること。
    #[test]
    fn test_bundled_playbooks_all_parse_and_validate() {
        let playbooks = load_bundled();
        assert_eq!(
            playbooks.len(),
            BUNDLED_PLAYBOOKS.len(),
            "all bundled playbook assets must parse"
        );

        for pb in &playbooks {
            pb.validate_structure().unwrap_or_else(|e| {
                panic!("bundled playbook {:?} failed validation: {}", pb.id, e)
            });
            assert!(
                pb.required_skills.is_empty() && pb.required_mcp_servers.is_empty(),
                "official playbooks must run without external dependencies: {:?}",
                pb.id
            );
            for wf in &pb.workflows {
                for node in &wf.nodes {
                    if let NodeType::Start { trigger } = &node.node_type {
                        assert_eq!(
                            *trigger,
                            TriggerType::Manual,
                            "playbook {:?} workflow {:?} must use Manual trigger",
                            pb.id,
                            wf.name
                        );
                    }
                }
            }
        }
    }
}
