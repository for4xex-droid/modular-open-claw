/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::db::DatabasePool;
use crate::workflow::schema::WorkflowDefinition;
use aiome_core::error::AiomeError;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowRecord {
    pub id: String,
    pub creator_id: String,
    pub name: String,
    pub description: String,
    pub tags: String,
    pub visibility: String,
    pub current_version: i64,
    pub is_template: i64,
    pub fork_source_id: Option<String>,
    pub execution_count: i64,
    pub last_executed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub workflow_id: String,
    pub version: i64,
    pub status: String,
    pub input_variables: String,
    pub output_result: Option<String>,
    pub root_job_id: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

pub struct WorkflowStore {
    pool: DatabasePool,
}

impl WorkflowStore {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    /// ワークフローの新規作成
    pub async fn create_workflow(
        &self,
        id: Uuid,
        creator_id: &str,
        name: &str,
        description: &str,
        visibility: &str,
        tags: Vec<String>,
    ) -> Result<(), AiomeError> {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        let id_str = id.to_string();
        let q = "INSERT INTO workflows (id, creator_id, name, description, tags, visibility) VALUES (?, ?, ?, ?, ?, ?)";

        crate::sql_exec!(
            &self.pool,
            q,
            id_str,
            creator_id.to_string(),
            name.to_string(),
            description.to_string(),
            tags_json,
            visibility.to_string()
        )?;
        Ok(())
    }

    /// ワークフローのフォークによる新規作成
    pub async fn create_workflow_fork(
        &self,
        id: Uuid,
        creator_id: &str,
        name: &str,
        description: &str,
        visibility: &str,
        tags: Vec<String>,
        fork_source_id: Uuid,
    ) -> Result<(), AiomeError> {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        let id_str = id.to_string();
        let fork_source_id_str = fork_source_id.to_string();
        let q = "INSERT INTO workflows (id, creator_id, name, description, tags, visibility, fork_source_id) VALUES (?, ?, ?, ?, ?, ?, ?)";

        crate::sql_exec!(
            &self.pool,
            q,
            id_str,
            creator_id.to_string(),
            name.to_string(),
            description.to_string(),
            tags_json,
            visibility.to_string(),
            fork_source_id_str
        )?;
        Ok(())
    }

    /// ワークフローの取得
    pub async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRecord>, AiomeError> {
        let id_str = id.to_string();
        let q = "SELECT id, creator_id, name, description, tags, visibility, current_version, is_template, fork_source_id, execution_count, last_executed_at, created_at, updated_at FROM workflows WHERE id = ?";

        let record = crate::sql_fetch_optional_map!(
            &self.pool,
            sqlite: q,
            |row| {
                use sqlx::Row;
                Ok::<WorkflowRecord, AiomeError>(WorkflowRecord {
                    id: row.get("id"),
                    creator_id: row.get("creator_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    tags: row.get("tags"),
                    visibility: row.get("visibility"),
                    current_version: row.get("current_version"),
                    is_template: row.get("is_template"),
                    fork_source_id: row.get("fork_source_id"),
                    execution_count: row.get("execution_count"),
                    last_executed_at: row.get("last_executed_at"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            },
            pg: q,
            |row| {
                use sqlx::Row;
                Ok::<WorkflowRecord, AiomeError>(WorkflowRecord {
                    id: row.get("id"),
                    creator_id: row.get("creator_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    tags: row.get("tags"),
                    visibility: row.get("visibility"),
                    current_version: row.get("current_version"),
                    is_template: row.get("is_template"),
                    fork_source_id: row.get("fork_source_id"),
                    execution_count: row.get("execution_count"),
                    last_executed_at: row.get("last_executed_at"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            },
            &id_str
        )?;
        Ok(record)
    }

    /// ワークフローの削除 (ON DELETE CASCADE によりバージョンと実行履歴も削除されます)
    pub async fn delete_workflow(&self, id: Uuid) -> Result<(), AiomeError> {
        let id_str = id.to_string();
        let q = "DELETE FROM workflows WHERE id = ?";
        crate::sql_exec!(&self.pool, q, id_str)?;
        Ok(())
    }

    /// バージョンの保存
    pub async fn save_version(
        &self,
        workflow_id: Uuid,
        version: u32,
        definition: &WorkflowDefinition,
        change_summary: &str,
    ) -> Result<(), AiomeError> {
        let def_json = serde_json::to_string(definition)?;
        let wf_id_str = workflow_id.to_string();
        let version_i = version as i64;
        let id_str = Uuid::new_v4().to_string();
        let q = "INSERT INTO workflow_versions (id, workflow_id, version, definition, change_summary) VALUES (?, ?, ?, ?, ?)";

        crate::sql_exec!(
            &self.pool,
            q,
            id_str,
            wf_id_str,
            version_i,
            def_json,
            change_summary.to_string()
        )?;
        Ok(())
    }

    /// バージョンの取得
    pub async fn get_version(
        &self,
        workflow_id: Uuid,
        version: u32,
    ) -> Result<Option<WorkflowDefinition>, AiomeError> {
        let wf_id_str = workflow_id.to_string();
        let version_i = version as i64;
        let q = "SELECT definition FROM workflow_versions WHERE workflow_id = ? AND version = ?";

        let row = crate::sql_fetch_optional_map!(
            &self.pool,
            sqlite: q,
            |row| {
                use sqlx::Row;
                Ok::<String, AiomeError>(row.get("definition"))
            },
            pg: q,
            |row| {
                use sqlx::Row;
                Ok::<String, AiomeError>(row.get("definition"))
            },
            &wf_id_str,
            version_i
        )?;

        if let Some(def_str) = row {
            let def: WorkflowDefinition = serde_json::from_str(&def_str)?;
            Ok(Some(def))
        } else {
            Ok(None)
        }
    }

    /// 実行履歴の新規作成
    pub async fn create_execution(
        &self,
        execution_id: Uuid,
        workflow_id: Uuid,
        version: u32,
        input_variables: serde_json::Value,
    ) -> Result<(), AiomeError> {
        let exec_id_str = execution_id.to_string();
        let wf_id_str = workflow_id.to_string();
        let version_i = version as i64;
        let vars_str = serde_json::to_string(&input_variables)?;
        let q = "INSERT INTO workflow_executions (id, workflow_id, version, input_variables) VALUES (?, ?, ?, ?)";

        crate::sql_exec!(&self.pool, q, exec_id_str, wf_id_str, version_i, vars_str)?;
        Ok(())
    }

    /// 実行履歴のステータス更新
    pub async fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: &str,
        output_result: Option<serde_json::Value>,
    ) -> Result<(), AiomeError> {
        let exec_id_str = execution_id.to_string();
        let output_str = output_result
            .map(|v| serde_json::to_string(&v))
            .transpose()?;
        let q = "UPDATE workflow_executions SET status = ?, output_result = ?, completed_at = datetime('now') WHERE id = ?";

        crate::sql_exec!(&self.pool, q, status.to_string(), output_str, exec_id_str)?;
        Ok(())
    }

    /// 実行履歴の取得
    pub async fn get_execution(
        &self,
        execution_id: Uuid,
    ) -> Result<Option<ExecutionRecord>, AiomeError> {
        let exec_id_str = execution_id.to_string();
        let q = "SELECT id, workflow_id, version, status, input_variables, output_result, root_job_id, started_at, completed_at FROM workflow_executions WHERE id = ?";

        let record = crate::sql_fetch_optional_map!(
            &self.pool,
            sqlite: q,
            |row| {
                use sqlx::Row;
                Ok::<ExecutionRecord, AiomeError>(ExecutionRecord {
                    id: row.get("id"),
                    workflow_id: row.get("workflow_id"),
                    version: row.get("version"),
                    status: row.get("status"),
                    input_variables: row.get("input_variables"),
                    output_result: row.get("output_result"),
                    root_job_id: row.get("root_job_id"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                })
            },
            pg: q,
            |row| {
                use sqlx::Row;
                Ok::<ExecutionRecord, AiomeError>(ExecutionRecord {
                    id: row.get("id"),
                    workflow_id: row.get("workflow_id"),
                    version: row.get("version"),
                    status: row.get("status"),
                    input_variables: row.get("input_variables"),
                    output_result: row.get("output_result"),
                    root_job_id: row.get("root_job_id"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                })
            },
            &exec_id_str
        )?;
        Ok(record)
    }

    /// ワークフローの更新
    pub async fn update_workflow(
        &self,
        id: Uuid,
        name: &str,
        description: &str,
        visibility: &str,
        tags: Vec<String>,
        current_version: u32,
    ) -> Result<(), AiomeError> {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        let id_str = id.to_string();
        let version_i = current_version as i64;
        let q = "UPDATE workflows SET name = ?, description = ?, visibility = ?, tags = ?, current_version = ?, updated_at = datetime('now') WHERE id = ?";

        crate::sql_exec!(
            &self.pool,
            q,
            name.to_string(),
            description.to_string(),
            visibility.to_string(),
            tags_json,
            version_i,
            id_str
        )?;
        Ok(())
    }

    /// ワークフロー一覧の取得 (作成者本人用または公開設定のもの)
    pub async fn list_workflows(
        &self,
        creator_id: &str,
    ) -> Result<Vec<WorkflowRecord>, AiomeError> {
        let creator_str = creator_id.to_string();
        let q = "SELECT id, creator_id, name, description, tags, visibility, current_version, is_template, fork_source_id, execution_count, last_executed_at, created_at, updated_at FROM workflows WHERE creator_id = ? OR visibility = 'community' OR visibility = 'marketplace'";

        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
            i64,
            Option<String>,
            i64,
            Option<String>,
            String,
            String,
        )> = crate::sql_fetch_all!(
            &self.pool,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                i64,
                i64,
                Option<String>,
                i64,
                Option<String>,
                String,
                String
            ),
            sqlite: q,
            pg: q,
            &creator_str
        )?;

        let records = rows
            .into_iter()
            .map(|row| WorkflowRecord {
                id: row.0,
                creator_id: row.1,
                name: row.2,
                description: row.3,
                tags: row.4,
                visibility: row.5,
                current_version: row.6,
                is_template: row.7,
                fork_source_id: row.8,
                execution_count: row.9,
                last_executed_at: row.10,
                created_at: row.11,
                updated_at: row.12,
            })
            .collect();

        Ok(records)
    }

    /// 実行履歴一覧の取得
    pub async fn list_executions(
        &self,
        workflow_id: Uuid,
    ) -> Result<Vec<ExecutionRecord>, AiomeError> {
        let wf_id_str = workflow_id.to_string();
        let q = "SELECT id, workflow_id, version, status, input_variables, output_result, root_job_id, started_at, completed_at FROM workflow_executions WHERE workflow_id = ? ORDER BY started_at DESC";

        let rows = crate::sql_fetch_all!(
            &self.pool,
            (
                String,
                String,
                i64,
                String,
                String,
                Option<String>,
                Option<String>,
                String,
                Option<String>
            ),
            sqlite: q,
            pg: q,
            &wf_id_str
        )?;

        let records = rows
            .into_iter()
            .map(|row| ExecutionRecord {
                id: row.0,
                workflow_id: row.1,
                version: row.2,
                status: row.3,
                input_variables: row.4,
                output_result: row.5,
                root_job_id: row.6,
                started_at: row.7,
                completed_at: row.8,
            })
            .collect();

        Ok(records)
    }

    /// バージョン履歴一覧の取得
    pub async fn list_versions(&self, workflow_id: Uuid) -> Result<Vec<VersionRecord>, AiomeError> {
        let wf_id_str = workflow_id.to_string();
        let q = "SELECT id, workflow_id, version, definition, change_summary, created_at FROM workflow_versions WHERE workflow_id = ? ORDER BY version DESC";

        let rows = crate::sql_fetch_all!(
            &self.pool,
            (
                String,
                String,
                i64,
                String,
                String,
                String
            ),
            sqlite: q,
            pg: q,
            &wf_id_str
        )?;

        let records = rows
            .into_iter()
            .map(|row| VersionRecord {
                id: row.0,
                workflow_id: row.1,
                version: row.2,
                definition: row.3,
                change_summary: row.4,
                created_at: row.5,
            })
            .collect();

        Ok(records)
    }

    /// ステータスが Running の実行履歴を全件取得（再起動時の orphan 復旧用）
    pub async fn list_running_executions(&self) -> Result<Vec<ExecutionRecord>, AiomeError> {
        let q = "SELECT id, workflow_id, version, status, input_variables, output_result, root_job_id, started_at, completed_at FROM workflow_executions WHERE status = 'Running' ORDER BY started_at ASC";

        let rows = crate::sql_fetch_all!(
            &self.pool,
            (
                String,
                String,
                i64,
                String,
                String,
                Option<String>,
                Option<String>,
                String,
                Option<String>
            ),
            sqlite: q,
            pg: q
        )?;

        Ok(rows
            .into_iter()
            .map(|row| ExecutionRecord {
                id: row.0,
                workflow_id: row.1,
                version: row.2,
                status: row.3,
                input_variables: row.4,
                output_result: row.5,
                root_job_id: row.6,
                started_at: row.7,
                completed_at: row.8,
            })
            .collect())
    }

    /// karma_directives.workflow_execution_id に紐づくジョブ ID とステータスを取得
    pub async fn list_jobs_for_execution(
        &self,
        execution_id: Uuid,
    ) -> Result<Vec<(String, String)>, AiomeError> {
        let exec_id = execution_id.to_string();
        let pattern = format!("%\"workflow_execution_id\":\"{exec_id}\"%");
        let q = "SELECT id, status FROM jobs WHERE karma_directives LIKE ?";

        let rows = crate::sql_fetch_all!(
            &self.pool,
            (String, String),
            sqlite: q,
            pg: q,
            &pattern
        )?;

        Ok(rows)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionRecord {
    pub id: String,
    pub workflow_id: String,
    pub version: i64,
    pub definition: String,
    pub change_summary: String,
    pub created_at: String,
}
