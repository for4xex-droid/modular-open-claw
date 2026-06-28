/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use super::schema::{NodeType, WorkflowDefinition};
use aiome_core_contracts::traits::ConstitutionalValidator;
use std::collections::{HashMap, HashSet, VecDeque};

/// バリデーションエラーの種類
#[derive(Debug, PartialEq, Clone)]
pub enum ValidationError {
    NoStartNode,
    MultipleStartNodes,
    CycleDetected,
    IsolatedNode(String),
    InvalidLoopIterations(String, u32),
    SelfReferentialSubWorkflow(String),
    SecurityViolation(String),
    InvalidEdge { source: String, target: String },
    InvalidTimerDelay(String, u64),
    InvalidWasmCode(String, String),
}

fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip4) => {
            ip4.is_loopback()
                || ip4.is_private()
                || ip4.is_link_local()
                || ip4.is_unspecified()
                || ip4.is_broadcast()
        }
        std::net::IpAddr::V6(ip6) => {
            ip6.is_loopback()
                || ip6.is_unspecified()
                || (ip6.segments()[0] & 0xfe00) == 0xfc00
                || (ip6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

pub struct WorkflowValidator;

impl WorkflowValidator {
    /// ワークフロー全体のバリデーションを実行する
    pub async fn validate<V>(
        definition: &WorkflowDefinition,
        constitutional_validator: &V,
    ) -> Result<(), ValidationError>
    where
        V: ConstitutionalValidator + ?Sized,
    {
        // 1. Startノードの検出と検証
        let start_nodes: Vec<&super::schema::WorkflowNode> = definition
            .nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Start { .. }))
            .collect();

        if start_nodes.is_empty() {
            return Err(ValidationError::NoStartNode);
        }
        if start_nodes.len() > 1 {
            return Err(ValidationError::MultipleStartNodes);
        }
        let start_node = start_nodes[0];

        // ノードIDのセットを作成
        let node_ids: HashSet<&String> = definition.nodes.iter().map(|n| &n.id).collect();

        // 隣接リストと入次数の構築
        let mut adj_list: HashMap<&String, Vec<&String>> = HashMap::new();
        let mut in_degrees: HashMap<&String, usize> = HashMap::new();

        for node in &definition.nodes {
            adj_list.insert(&node.id, Vec::new());
            in_degrees.insert(&node.id, 0);
        }

        for edge in &definition.edges {
            if !node_ids.contains(&edge.source) || !node_ids.contains(&edge.target) {
                return Err(ValidationError::InvalidEdge {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                });
            }
            adj_list
                .get_mut(&edge.source)
                .ok_or_else(|| ValidationError::InvalidEdge {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                })?
                .push(&edge.target);
            let in_degree =
                in_degrees
                    .get_mut(&edge.target)
                    .ok_or_else(|| ValidationError::InvalidEdge {
                        source: edge.source.clone(),
                        target: edge.target.clone(),
                    })?;
            *in_degree += 1;
        }

        // 2. 循環（Cycle）検出 - Kahn's Algorithm
        let mut temp_in_degrees = in_degrees.clone();
        let mut queue = VecDeque::new();
        for (&node_id, &degree) in &temp_in_degrees {
            if degree == 0 {
                queue.push_back(node_id);
            }
        }

        let mut visited_count = 0;
        while let Some(node_id) = queue.pop_front() {
            visited_count += 1;
            if let Some(neighbors) = adj_list.get(node_id) {
                for neighbor in neighbors {
                    let degree = temp_in_degrees
                        .get_mut(neighbor)
                        .ok_or(ValidationError::CycleDetected)?;
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if visited_count < definition.nodes.len() {
            return Err(ValidationError::CycleDetected);
        }

        // 3. 孤立ノード検出 (Startノードからの到達可能性確認)
        let mut reached = HashSet::new();
        let mut bfs_queue = VecDeque::new();
        bfs_queue.push_back(&start_node.id);
        reached.insert(&start_node.id);

        while let Some(current_id) = bfs_queue.pop_front() {
            if let Some(neighbors) = adj_list.get(current_id) {
                for neighbor in neighbors {
                    if !reached.contains(neighbor) {
                        reached.insert(*neighbor);
                        bfs_queue.push_back(neighbor);
                    }
                }
            }
        }

        for node in &definition.nodes {
            if !reached.contains(&node.id) {
                return Err(ValidationError::IsolatedNode(node.id.clone()));
            }
        }

        // 4. 個別ノード構成ルール検証
        for node in &definition.nodes {
            match &node.node_type {
                NodeType::Loop {
                    max_iterations: Some(max_iter),
                    ..
                } => {
                    if *max_iter > 1000 {
                        return Err(ValidationError::InvalidLoopIterations(
                            node.id.clone(),
                            *max_iter,
                        ));
                    }
                }
                NodeType::SubWorkflow { workflow_id, .. } => {
                    if *workflow_id == definition.id {
                        return Err(ValidationError::SelfReferentialSubWorkflow(node.id.clone()));
                    }
                }
                NodeType::HttpRequest { url_template, .. } => {
                    let mut is_malicious = false;
                    let mut error_msg = String::new();

                    let has_variables = url_template.contains("{{") || url_template.contains("}");

                    if let Ok(parsed_url) = url::Url::parse(url_template) {
                        if let Some(host) = parsed_url.host_str() {
                            let host_lower = host.to_lowercase();

                            if host_lower == "localhost"
                                || host_lower.ends_with(".local")
                                || host_lower.ends_with(".localhost")
                                || host_lower.ends_with(".test")
                                || host_lower.ends_with(".example")
                                || host_lower.ends_with(".invalid")
                            {
                                is_malicious = true;
                                error_msg = format!("Blocked reserved domain: {}", host);
                            } else if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                                if is_private_ip(ip) {
                                    is_malicious = true;
                                    error_msg = format!("Blocked private IP: {}", ip);
                                }
                            } else {
                                let dev_mode = std::env::var("AIOME_DEV_MODE")
                                    .map(|v| v == "true")
                                    .unwrap_or(false)
                                    || std::env::var("CI").map(|v| v == "true").unwrap_or(false);

                                let host_to_resolve = host.to_string();
                                let resolve_result = tokio::time::timeout(
                                    std::time::Duration::from_millis(1000),
                                    tokio::task::spawn_blocking(move || {
                                        use std::net::ToSocketAddrs;
                                        format!("{}:80", host_to_resolve).to_socket_addrs()
                                    }),
                                )
                                .await;

                                match resolve_result {
                                    Ok(Ok(Ok(addrs))) => {
                                        let mut found_private = false;
                                        for addr in addrs {
                                            if is_private_ip(addr.ip()) {
                                                found_private = true;
                                                error_msg = format!(
                                                    "Blocked private resolved IP: {} for host {}",
                                                    addr.ip(),
                                                    host
                                                );
                                                break;
                                            }
                                        }
                                        if found_private {
                                            is_malicious = true;
                                        }
                                    }
                                    _ => {
                                        if !dev_mode {
                                            is_malicious = true;
                                            error_msg = format!(
                                                "DNS resolution failed or timed out for host: {}",
                                                host
                                            );
                                        } else {
                                            tracing::warn!("⚠️ DNS resolution failed or timed out for host '{}' in dev mode. Passing anyway.", host);
                                        }
                                    }
                                }
                            }
                        }
                    } else if !has_variables {
                        is_malicious = true;
                        error_msg = format!("Invalid URL: {}", url_template);
                    }

                    if !is_malicious {
                        let url_lower = url_template.to_lowercase();
                        if url_lower.contains("localhost")
                            || url_lower.contains("127.0.0.1")
                            || url_lower.contains("0.0.0.0")
                            || url_lower.contains("10.")
                            || url_lower.contains("192.168.")
                            || url_lower.contains("172.16.")
                            || url_lower.contains("172.17.")
                            || url_lower.contains("172.18.")
                            || url_lower.contains("172.19.")
                            || url_lower.contains("172.20.")
                            || url_lower.contains("172.21.")
                            || url_lower.contains("172.22.")
                            || url_lower.contains("172.23.")
                            || url_lower.contains("172.24.")
                            || url_lower.contains("172.25.")
                            || url_lower.contains("172.26.")
                            || url_lower.contains("172.27.")
                            || url_lower.contains("172.28.")
                            || url_lower.contains("172.29.")
                            || url_lower.contains("172.30.")
                            || url_lower.contains("172.31.")
                        {
                            is_malicious = true;
                            error_msg = format!(
                                "Fallback blocked potentially private address: {}",
                                url_template
                            );
                        }
                    }

                    if is_malicious {
                        return Err(ValidationError::SecurityViolation(format!(
                            "SSRF vulnerability detected: {}",
                            error_msg
                        )));
                    }
                }
                NodeType::Timer { delay_seconds } => {
                    if *delay_seconds == 0 || *delay_seconds > 86400 {
                        return Err(ValidationError::InvalidTimerDelay(
                            node.id.clone(),
                            *delay_seconds,
                        ));
                    }
                }
                NodeType::WasmCode { code, language } => {
                    if code.trim().is_empty() {
                        return Err(ValidationError::InvalidWasmCode(
                            node.id.clone(),
                            "Empty code".to_string(),
                        ));
                    }
                    let lang = language.to_lowercase();
                    if lang != "javascript" && lang != "rust" && lang != "typescript" {
                        return Err(ValidationError::InvalidWasmCode(
                            node.id.clone(),
                            format!("Unsupported language '{}'", language),
                        ));
                    }
                }
                _ => {}
            }
        }

        // 5. ConstitutionalValidator による倫理・安全検証 (CSAM/SSRF防止を含む)
        // ワークフローのメタデータとノードプロンプト等の安全検証
        let mut content_to_verify = format!(
            "Workflow Name: {}\nDescription: {}\n",
            definition.name, definition.description
        );

        for node in &definition.nodes {
            content_to_verify.push_str(&format!("Node Label: {}\n", node.label));
            if let NodeType::LlmPrompt {
                model: _,
                temperature: _,
            } = &node.node_type
            {
                if let Some(prompt) = node.config.get("prompt").and_then(|p| p.as_str()) {
                    content_to_verify.push_str(&format!("Prompt: {}\n", prompt));
                }
            }
            if let NodeType::HumanApproval { prompt_message, .. } = &node.node_type {
                content_to_verify.push_str(&format!("Approval Msg: {}\n", prompt_message));
            }
        }

        // 憲法（Soul.md）に基づき検証
        if let Err(e) = constitutional_validator
            .verify_constitutional(&content_to_verify, "System security and safety policy")
            .await
        {
            return Err(ValidationError::SecurityViolation(e.to_string()));
        }

        Ok(())
    }
}
