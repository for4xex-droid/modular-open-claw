/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use super::schema::{NodeType, ParallelWaitMode, WorkflowDefinition};
use aiome_core_contracts::traits::{Job, JobStatus};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// トランスパイル時のエラー
#[derive(Debug, PartialEq, Clone)]
pub enum TranspilerError {
    RecursionLimitExceeded,
    ValidationError(String),
}

pub struct WorkflowTranspiler;

impl WorkflowTranspiler {
    /// ワークフロー定義を Job リストに変換する
    pub fn transpile(
        definition: &WorkflowDefinition,
        execution_id: Uuid,
    ) -> Result<Vec<Job>, TranspilerError> {
        Self::transpile_with_resolver(definition, execution_id, &HashMap::new())
    }

    /// 事前解決済みサブワークフローを使ってトランスパイルする
    pub fn transpile_with_resolver(
        definition: &WorkflowDefinition,
        execution_id: Uuid,
        resolved: &HashMap<Uuid, WorkflowDefinition>,
    ) -> Result<Vec<Job>, TranspilerError> {
        Self::transpile_inner(definition, execution_id, 0, resolved)
    }

    /// 再帰の深さを追跡しながらトランスパイルを実行する
    pub fn transpile_with_depth(
        definition: &WorkflowDefinition,
        execution_id: Uuid,
        depth: usize,
    ) -> Result<Vec<Job>, TranspilerError> {
        Self::transpile_inner(definition, execution_id, depth, &HashMap::new())
    }

    fn transpile_inner(
        definition: &WorkflowDefinition,
        execution_id: Uuid,
        depth: usize,
        resolved: &HashMap<Uuid, WorkflowDefinition>,
    ) -> Result<Vec<Job>, TranspilerError> {
        if depth > 5 {
            return Err(TranspilerError::RecursionLimitExceeded);
        }

        // Start ノードを見つける
        let start_node = definition
            .nodes
            .iter()
            .find(|n| matches!(n.node_type, NodeType::Start { .. }))
            .ok_or_else(|| TranspilerError::ValidationError("No start node found".to_string()))?;

        // 依存関係（edges）から隣接リストを構築
        let mut adj_list: HashMap<&String, Vec<&String>> = HashMap::new();
        let mut in_degrees: HashMap<&String, usize> = HashMap::new();

        for node in &definition.nodes {
            adj_list.insert(&node.id, Vec::new());
            in_degrees.insert(&node.id, 0);
        }

        for edge in &definition.edges {
            adj_list
                .get_mut(&edge.source)
                .ok_or_else(|| {
                    TranspilerError::ValidationError(format!(
                        "Invalid edge source: {}",
                        edge.source
                    ))
                })?
                .push(&edge.target);

            let in_degree = in_degrees.get_mut(&edge.target).ok_or_else(|| {
                TranspilerError::ValidationError(format!("Invalid edge target: {}", edge.target))
            })?;
            *in_degree += 1;
        }

        // トポロジカルソートで処理順を決める（Jobの順序決定）
        let mut queue = VecDeque::new();
        // Start ノードの隣接ノードを queue に入れる
        if let Some(neighbors) = adj_list.get(&start_node.id) {
            for neighbor in neighbors {
                queue.push_back(*neighbor);
            }
        }

        let mut sorted_node_ids = Vec::new();
        let mut local_in_degrees = in_degrees.clone();

        let mut visited = std::collections::HashSet::new();
        visited.insert(&start_node.id);

        while let Some(current_id) = queue.pop_front() {
            if !visited.insert(current_id) {
                continue;
            }
            sorted_node_ids.push(current_id);

            if let Some(neighbors) = adj_list.get(current_id) {
                for neighbor in neighbors {
                    let in_degree = local_in_degrees.get_mut(neighbor).ok_or_else(|| {
                        TranspilerError::ValidationError(format!("Neighbor {} not found", neighbor))
                    })?;
                    *in_degree -= 1;
                    queue.push_back(neighbor);
                }
            }
        }

        let mut jobs = Vec::new();
        let mut node_to_job_id: HashMap<String, String> = HashMap::new();

        for &node_id in &sorted_node_ids {
            let node = definition
                .nodes
                .iter()
                .find(|n| &n.id == node_id)
                .ok_or_else(|| {
                    TranspilerError::ValidationError(format!("Node {} not found", node_id))
                })?;

            match &node.node_type {
                NodeType::Start { .. } => {
                    // Start ノードは Job を生成しない
                }
                NodeType::Loop { max_iterations, .. } => {
                    let count = max_iterations.unwrap_or(10);
                    let mut last_job_id = String::new();
                    for i in 0..count {
                        let job_id = Uuid::new_v4().to_string();
                        let karma_directives = build_karma_directives(
                            execution_id,
                            &node.id,
                            &definition.edges,
                            &node_to_job_id,
                            json!({ "loop_index": i }),
                        );

                        let job = Job {
                            id: job_id.clone(),
                            category: "wf_loop".to_string(),
                            topic: build_job_topic(node),
                            style: String::new(),
                            status: JobStatus::Pending,
                            priority: 0,
                            karma_directives: Some(karma_directives),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            updated_at: chrono::Utc::now().to_rfc3339(),
                            ..Default::default()
                        };
                        jobs.push(job);
                        last_job_id = job_id;
                    }
                    if !last_job_id.is_empty() {
                        node_to_job_id.insert(node.id.clone(), last_job_id);
                    }
                }
                NodeType::Parallel { wait_mode } => {
                    let job_id = Uuid::new_v4().to_string();

                    let incoming_edges: Vec<&super::schema::WorkflowEdge> = definition
                        .edges
                        .iter()
                        .filter(|e| e.target == node.id)
                        .collect();

                    let mut parent_job_ids = Vec::new();
                    for edge in incoming_edges {
                        if let Some(pid) = node_to_job_id.get(&edge.source) {
                            parent_job_ids.push(pid.clone());
                        }
                    }

                    let mut wait_mode_n = None;
                    let wait_mode_str = match wait_mode {
                        ParallelWaitMode::All => "All",
                        ParallelWaitMode::Any => "Any",
                        ParallelWaitMode::N(n) => {
                            wait_mode_n = Some(*n);
                            "N"
                        }
                    };

                    let karma_directives = build_karma_directives(
                        execution_id,
                        &node.id,
                        &definition.edges,
                        &node_to_job_id,
                        json!({
                            "parent_job_ids": parent_job_ids,
                            "wait_mode": wait_mode_str,
                            "wait_mode_n": wait_mode_n,
                        }),
                    );

                    let job = Job {
                        id: job_id.clone(),
                        category: "wf_parallel".to_string(),
                        topic: build_job_topic(node),
                        style: String::new(),
                        status: JobStatus::Pending,
                        priority: 0,
                        karma_directives: Some(karma_directives),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        ..Default::default()
                    };
                    jobs.push(job);
                    node_to_job_id.insert(node.id.clone(), job_id);
                }
                NodeType::SubWorkflow {
                    workflow_id,
                    version: _version,
                } => {
                    let sub_def = resolved.get(workflow_id).ok_or_else(|| {
                        TranspilerError::ValidationError(format!(
                            "SubWorkflow {} is not resolved",
                            workflow_id
                        ))
                    })?;
                    let sub_jobs =
                        Self::transpile_inner(sub_def, execution_id, depth + 1, resolved)?;
                    jobs.extend(sub_jobs);
                }
                NodeType::Timer { .. } => {
                    let job_id = Uuid::new_v4().to_string();
                    let karma_directives = build_karma_directives(
                        execution_id,
                        &node.id,
                        &definition.edges,
                        &node_to_job_id,
                        json!({}),
                    );
                    let job = Job {
                        id: job_id.clone(),
                        category: "wf_timer".to_string(),
                        topic: build_job_topic(node),
                        style: String::new(),
                        status: JobStatus::Pending,
                        priority: 0,
                        karma_directives: Some(karma_directives),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        ..Default::default()
                    };
                    jobs.push(job);
                    node_to_job_id.insert(node.id.clone(), job_id);
                }
                NodeType::WasmCode { .. } => {
                    let job_id = Uuid::new_v4().to_string();
                    let karma_directives = build_karma_directives(
                        execution_id,
                        &node.id,
                        &definition.edges,
                        &node_to_job_id,
                        json!({}),
                    );
                    let job = Job {
                        id: job_id.clone(),
                        category: "wf_wasm".to_string(),
                        topic: build_job_topic(node),
                        style: String::new(),
                        status: JobStatus::Pending,
                        priority: 0,
                        karma_directives: Some(karma_directives),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        ..Default::default()
                    };
                    jobs.push(job);
                    node_to_job_id.insert(node.id.clone(), job_id);
                }
                other => {
                    let job_id = Uuid::new_v4().to_string();

                    let category = match other {
                        NodeType::LlmPrompt { .. } => "wf_llm",
                        NodeType::HttpRequest { .. } => "wf_http",
                        NodeType::McpToolCall { .. } => "wf_mcp",
                        NodeType::Transform { .. } => "wf_transform",
                        NodeType::HumanApproval { .. } => "wf_approval",
                        NodeType::Condition { .. } => "wf_condition",
                        _ => "wf_generic",
                    };

                    let karma_directives = build_karma_directives(
                        execution_id,
                        &node.id,
                        &definition.edges,
                        &node_to_job_id,
                        json!({}),
                    );

                    let job = Job {
                        id: job_id.clone(),
                        category: category.to_string(),
                        topic: build_job_topic(node),
                        style: String::new(),
                        status: JobStatus::Pending,
                        priority: 0,
                        karma_directives: Some(karma_directives),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        ..Default::default()
                    };
                    jobs.push(job);
                    node_to_job_id.insert(node.id.clone(), job_id);
                }
            }
        }

        Ok(jobs)
    }
}

fn find_parent_job_id(
    node_id: &str,
    edges: &[super::schema::WorkflowEdge],
    node_to_job_id: &HashMap<String, String>,
) -> Option<String> {
    edges
        .iter()
        .find(|e| e.target == node_id)
        .and_then(|e| node_to_job_id.get(&e.source))
        .cloned()
}

fn find_incoming_edge<'a>(
    node_id: &str,
    edges: &'a [super::schema::WorkflowEdge],
) -> Option<&'a super::schema::WorkflowEdge> {
    edges.iter().find(|e| e.target == node_id)
}

/// karma_directives JSON を構築する（branch / parent_job_id を含む）
fn build_karma_directives(
    execution_id: Uuid,
    node_id: &str,
    edges: &[super::schema::WorkflowEdge],
    node_to_job_id: &HashMap<String, String>,
    extra: serde_json::Value,
) -> String {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "workflow_execution_id".to_string(),
        json!(execution_id.to_string()),
    );
    obj.insert("node_id".to_string(), json!(node_id));

    if let Some(extra_obj) = extra.as_object() {
        for (k, v) in extra_obj {
            obj.insert(k.clone(), v.clone());
        }
    }

    if !obj.contains_key("parent_job_id") && !obj.contains_key("parent_job_ids") {
        if let Some(pid) = find_parent_job_id(node_id, edges, node_to_job_id) {
            obj.insert("parent_job_id".to_string(), json!(pid));
        }
    }

    if let Some(branch) = find_incoming_edge(node_id, edges).and_then(|e| e.source_handle.as_ref())
    {
        obj.entry("branch".to_string()).or_insert(json!(branch));
    }

    serde_json::Value::Object(obj).to_string()
}

/// node_type フィールドと node.config をマージした topic JSON を生成する
fn build_job_topic(node: &super::schema::WorkflowNode) -> String {
    use super::schema::NodeType;

    let mut obj = node.config.as_object().cloned().unwrap_or_default();

    match &node.node_type {
        NodeType::LlmPrompt { model, temperature } => {
            if let Some(m) = model {
                obj.insert("model".to_string(), json!(m));
            }
            if let Some(t) = temperature {
                obj.insert("temperature".to_string(), json!(t));
            }
        }
        NodeType::HttpRequest {
            method,
            url_template,
        } => {
            obj.insert("method".to_string(), json!(method));
            obj.insert("url_template".to_string(), json!(url_template));
        }
        NodeType::McpToolCall {
            server_name,
            tool_name,
        } => {
            obj.insert("server_name".to_string(), json!(server_name));
            obj.insert("tool_name".to_string(), json!(tool_name));
        }
        NodeType::Transform { expression } => {
            obj.insert("expression".to_string(), json!(expression));
        }
        NodeType::Condition { expression, mode } => {
            obj.insert("expression".to_string(), json!(expression));
            obj.insert(
                "mode".to_string(),
                serde_json::to_value(mode).unwrap_or(json!("Expression")),
            );
        }
        NodeType::HumanApproval {
            prompt_message,
            timeout_seconds,
        } => {
            obj.insert("prompt_message".to_string(), json!(prompt_message));
            if let Some(t) = timeout_seconds {
                obj.insert("timeout_seconds".to_string(), json!(t));
            }
        }
        NodeType::Loop {
            iterator_expression,
            max_iterations,
        } => {
            obj.insert(
                "iterator_expression".to_string(),
                json!(iterator_expression),
            );
            if let Some(m) = max_iterations {
                obj.insert("max_iterations".to_string(), json!(m));
            }
        }
        NodeType::Parallel { wait_mode } => {
            obj.insert(
                "wait_mode".to_string(),
                serde_json::to_value(wait_mode).unwrap_or(json!("All")),
            );
        }
        NodeType::Timer { delay_seconds } => {
            obj.insert("delay_seconds".to_string(), json!(delay_seconds));
        }
        NodeType::WasmCode { code, language } => {
            obj.insert("code".to_string(), json!(code));
            obj.insert("language".to_string(), json!(language));
        }
        NodeType::Start { .. } | NodeType::SubWorkflow { .. } => {}
    }

    serde_json::Value::Object(obj).to_string()
}

/// enqueue 後の実ジョブ ID で karma_directives 内の親参照を書き換える
pub fn remap_karma_directives(
    karma_directives: Option<&str>,
    id_map: &HashMap<String, String>,
) -> Option<String> {
    let raw = karma_directives?;
    let Ok(mut v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Some(raw.to_string());
    };

    if let Some(parent) = v.get("parent_job_id").and_then(|x| x.as_str()) {
        if let Some(mapped) = id_map.get(parent) {
            v["parent_job_id"] = json!(mapped);
        }
    }

    if let Some(parents) = v.get_mut("parent_job_ids").and_then(|x| x.as_array_mut()) {
        for item in parents.iter_mut() {
            if let Some(pid) = item.as_str() {
                if let Some(mapped) = id_map.get(pid) {
                    *item = json!(mapped);
                }
            }
        }
    }

    Some(v.to_string())
}
