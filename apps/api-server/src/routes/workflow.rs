/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::app_state::AppState;
use crate::auth::Authenticated;
use crate::error::AppError;
use aiome_core_contracts::TaskRegistry;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use infrastructure::validator::DefaultConstitutionalValidator;
use infrastructure::workflow::schema::WorkflowDefinition;
use infrastructure::workflow::store::WorkflowStore;
use infrastructure::workflow::transpiler::WorkflowTranspiler;
use infrastructure::workflow::validator::WorkflowValidator;
use uuid::Uuid;

/// [POST] /api/v1/workflows
pub async fn create_workflow(
    auth: Authenticated,
    State(state): State<AppState>,
    Json(req): Json<WorkflowDefinition>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    // 1. workflows テーブルへの基本情報保存
    store
        .create_workflow(
            req.id,
            &auth.agent_id.to_string(),
            &req.name,
            &req.description,
            "private", // デフォルトの可視性
            vec![],    // デフォルトのタグ
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to create workflow: {}", e)))?;

    // 2. workflow_versions テーブルへの初期定義保存
    store
        .save_version(req.id, req.version, &req, "Initial version")
        .await
        .map_err(|e| AppError::internal(format!("Failed to save workflow version: {}", e)))?;

    Ok(StatusCode::OK)
}

/// [GET] /api/v1/workflows
pub async fn list_workflows(
    auth: Authenticated,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());
    let list = store
        .list_workflows(&auth.agent_id.to_string())
        .await
        .map_err(|e| AppError::internal(format!("Failed to list workflows: {}", e)))?;

    Ok((StatusCode::OK, Json(list)))
}

/// [GET] /api/v1/workflows/{id}
pub async fn get_workflow(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());
    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?;

    if let Some(w) = wf {
        // BOLA チェック (作成者本人用または公開設定のもの以外は403)
        if w.creator_id != auth.agent_id.to_string() && w.visibility == "private" {
            return Err(AppError::forbidden("Access denied"));
        }

        // 最新の定義（バージョン）を取得して返す
        let version = w.current_version as u32;
        let def = store
            .get_version(id, version)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch workflow version: {}", e)))?;

        if let Some(d) = def {
            let mut val = serde_json::to_value(&d).map_err(|e| {
                AppError::internal(format!("Failed to serialize workflow definition: {}", e))
            })?;
            if let Some(obj) = val.as_object_mut() {
                obj.insert(
                    "creator_id".to_string(),
                    serde_json::Value::String(w.creator_id),
                );
                obj.insert(
                    "visibility".to_string(),
                    serde_json::Value::String(w.visibility),
                );
                obj.insert(
                    "is_template".to_string(),
                    serde_json::Value::Bool(w.is_template != 0),
                );
                obj.insert(
                    "fork_source_id".to_string(),
                    w.fork_source_id
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
                obj.insert(
                    "execution_count".to_string(),
                    serde_json::Value::Number(w.execution_count.into()),
                );
                obj.insert(
                    "last_executed_at".to_string(),
                    w.last_executed_at
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
            }
            Ok((StatusCode::OK, Json(val)))
        } else {
            let val = serde_json::to_value(&w).map_err(|e| {
                AppError::internal(format!("Failed to serialize workflow record: {}", e))
            })?;
            Ok((StatusCode::OK, Json(val)))
        }
    } else {
        Err(AppError::not_found("Workflow not found"))
    }
}

/// [PUT] /api/v1/workflows/{id}
pub async fn update_workflow(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<WorkflowDefinition>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?;

    if let Some(w) = wf {
        if w.creator_id != auth.agent_id.to_string() {
            return Err(AppError::forbidden("Access denied"));
        }
    } else {
        return Err(AppError::not_found("Workflow not found"));
    }

    // workflows テーブルの基本情報更新
    store
        .update_workflow(
            id,
            &req.name,
            &req.description,
            "private",
            vec![],
            req.version,
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to update workflow: {}", e)))?;

    // 新しいバージョン定義を追加
    store
        .save_version(id, req.version, &req, "Updated version")
        .await
        .map_err(|e| AppError::internal(format!("Failed to save workflow version: {}", e)))?;

    Ok(StatusCode::OK)
}

/// [DELETE] /api/v1/workflows/{id}
pub async fn delete_workflow(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?;

    if let Some(w) = wf {
        if w.creator_id != auth.agent_id.to_string() {
            return Err(AppError::forbidden("Access denied"));
        }
    } else {
        return Err(AppError::not_found("Workflow not found"));
    }

    store
        .delete_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to delete workflow: {}", e)))?;

    Ok(StatusCode::OK)
}

// === 未実装エンドポイントの仮スケルトン & 実装 ===

/// [POST] /api/v1/workflows/:id/execute
pub async fn execute_workflow(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    // 1. 既存のワークフローを取得
    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow not found"))?;

    // 所有者チェック
    if wf.creator_id != auth.agent_id.to_string() {
        return Err(AppError::forbidden("Access denied"));
    }

    // 最新のバージョン定義を取得
    let version = wf.current_version as u32;
    let def = store
        .get_version(id, version)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow version: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow version definition not found"))?;

    // 2. 構造バリデーションを実行
    let validator = DefaultConstitutionalValidator::new(state.provider.get_inner().clone(), None);
    WorkflowValidator::validate(&def, &validator)
        .await
        .map_err(|e| AppError::bad_request(format!("Workflow is invalid: {:?}", e)))?;

    // 3. 実行IDの作成と transpiler による Job 変換
    let execution_id = Uuid::new_v4();
    let jobs = WorkflowTranspiler::transpile(&def, execution_id)
        .map_err(|e| AppError::internal(format!("Transpilation failed: {:?}", e)))?;

    // 4. JobQueue への登録
    let mut job_ids: Vec<String> = Vec::new();
    for job in &jobs {
        job_ids.push(job.id.clone());
        let _ = state
            .job_queue
            .enqueue(
                &job.category,
                &job.topic,
                &job.style,
                job.karma_directives.as_deref(),
                None, // permission_manifest
                Some(auth.agent_id),
                job.priority,
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to enqueue job: {}", e)))?;
    }

    // 5. 実行履歴レコードを作成
    store
        .create_execution(
            execution_id,
            id,
            version,
            serde_json::json!({}), // 入力変数 (デフォルトは空)
        )
        .await
        .map_err(|e| {
            AppError::internal(format!("Failed to create workflow execution record: {}", e))
        })?;

    Ok((
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "execution_id": execution_id.to_string(),
            "job_ids": job_ids,
        })),
    ))
}

/// [POST] /api/v1/workflows/:id/validate
pub async fn validate_workflow(
    _auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(_id): axum::extract::Path<Uuid>,
    Json(req): Json<WorkflowDefinition>,
) -> Result<impl IntoResponse, AppError> {
    let validator = DefaultConstitutionalValidator::new(state.provider.get_inner().clone(), None);

    WorkflowValidator::validate(&req, &validator)
        .await
        .map_err(|e| AppError::bad_request(format!("Validation failed: {:?}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "valid": true }))))
}

/// [POST] /api/v1/workflows/:id/fork
pub async fn fork_workflow(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    // 1. フォーク元のワークフローを取得
    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow not found"))?;

    // BOLAチェック (作成者本人用または公開設定のもの以外は403)
    if wf.creator_id != auth.agent_id.to_string() && wf.visibility == "private" {
        return Err(AppError::forbidden("Access denied"));
    }

    // 2. 元の定義（最新バージョン）を取得
    let version = wf.current_version as u32;
    let def = store
        .get_version(id, version)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow version: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow version definition not found"))?;

    // 3. 新しいワークフロー情報を作成
    let new_workflow_id = Uuid::new_v4();
    let new_name = format!("Fork of {}", wf.name);
    let tags: Vec<String> = serde_json::from_str(&wf.tags).unwrap_or_default();

    // データベースに新しいワークフローを登録
    store
        .create_workflow_fork(
            new_workflow_id,
            &auth.agent_id.to_string(),
            &new_name,
            &wf.description,
            "private", // フォークしたものはデフォルトでprivate
            tags,
            id, // 元のID
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to create forked workflow: {}", e)))?;

    // 4. 新しいバージョン定義を保存
    let mut new_def = def.clone();
    new_def.id = new_workflow_id;
    new_def.name = new_name;
    new_def.version = 1;
    new_def.created_at = chrono::Utc::now().to_rfc3339();
    new_def.updated_at = chrono::Utc::now().to_rfc3339();

    store
        .save_version(new_workflow_id, 1, &new_def, "Forked version")
        .await
        .map_err(|e| {
            AppError::internal(format!("Failed to save forked workflow version: {}", e))
        })?;

    // 5. 新しく作成されたワークフロー情報を返す
    let response_body = serde_json::json!({
        "id": new_workflow_id.to_string(),
        "name": new_def.name,
        "description": new_def.description,
        "version": 1,
        "fork_source_id": id.to_string()
    });

    Ok((StatusCode::OK, Json(response_body)))
}

/// [GET] /api/v1/workflows/:id/export — 単一ワークフローを Playbook マニフェスト v1 として出力
#[utoipa::path(
    get,
    path = "/api/v1/workflows/{id}/export",
    responses(
        (status = 200, description = "Workflow exported as playbook manifest v1"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied"),
        (status = 404, description = "Workflow not found")
    ),
    security(("api_key" = []))
)]
pub async fn export_workflow(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow not found"))?;

    // BOLA チェック (作成者本人用または公開設定のもの以外は403)
    if wf.creator_id != auth.agent_id.to_string() && wf.visibility == "private" {
        return Err(AppError::forbidden("Access denied"));
    }

    let version = wf.current_version as u32;
    let def = store
        .get_version(id, version)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow version: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow version definition not found"))?;

    let tags: Vec<String> = serde_json::from_str(&wf.tags).unwrap_or_default();
    let manifest = infrastructure::workflow::playbook::PlaybookManifest {
        playbook_version: infrastructure::workflow::playbook::PLAYBOOK_MANIFEST_VERSION,
        id: format!("wf-{}", id.simple()),
        name: wf.name,
        description: wf.description,
        tags,
        required_skills: vec![],
        required_mcp_servers: vec![],
        workflows: vec![def],
    };

    Ok((StatusCode::OK, Json(manifest)))
}

/// [GET] /api/v1/workflows/:id/versions
pub async fn list_versions(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    // ワークフロー取得と所有権チェック
    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow not found"))?;

    if wf.creator_id != auth.agent_id.to_string() {
        return Err(AppError::forbidden("Access denied"));
    }

    let list = store
        .list_versions(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch versions: {}", e)))?;

    Ok((StatusCode::OK, Json(list)))
}

/// [GET] /api/v1/workflows/:id/executions
pub async fn list_executions(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = WorkflowStore::new((**state.db_pool.get_inner()).clone());

    // ワークフロー取得と所有権チェック
    let wf = store
        .get_workflow(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch workflow: {}", e)))?
        .ok_or_else(|| AppError::not_found("Workflow not found"))?;

    if wf.creator_id != auth.agent_id.to_string() {
        return Err(AppError::forbidden("Access denied"));
    }

    let list = store
        .list_executions(id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch executions: {}", e)))?;

    Ok((StatusCode::OK, Json(list)))
}
