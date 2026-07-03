/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! Agent Playbooks (F-1): 業務テンプレート一式を1つのマニフェスト（JSON）として
//! パッケージ化するための型と構造バリデーション。
//! v1 では同梱アセットとローカル JSON のみを対象とし、署名検証や
//! SubWorkflow の UUID remap はスコープ外（F-3 / v2 課題）。

use super::schema::{NodeType, WorkflowDefinition};
use aiome_core::error::AiomeError;
use serde::{Deserialize, Serialize};

/// 現在サポートするマニフェスト形式バージョン
pub const PLAYBOOK_MANIFEST_VERSION: u32 = 1;

/// Playbook マニフェスト v1
///
/// ワークフロー定義（1〜10個）と、導入に必要なスキル・MCP サーバーの宣言を持つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookManifest {
    pub playbook_version: u32,
    /// `^[a-z0-9-]{1,64}$` に一致すること（パストラバーサル対策）
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub required_skills: Vec<String>,
    #[serde(default)]
    pub required_mcp_servers: Vec<String>,
    pub workflows: Vec<WorkflowDefinition>,
}

impl PlaybookManifest {
    /// 構造バリデーション。違反はすべて列挙して1つの `AiomeError::Validation` で返す。
    pub fn validate_structure(&self) -> Result<(), AiomeError> {
        let mut violations: Vec<String> = Vec::new();

        if self.playbook_version != PLAYBOOK_MANIFEST_VERSION {
            violations.push(format!(
                "playbook_version must be {} (got {})",
                PLAYBOOK_MANIFEST_VERSION, self.playbook_version
            ));
        }

        if !is_valid_playbook_id(&self.id) {
            violations.push(format!(
                "id must match ^[a-z0-9-]{{1,64}}$ (got {:?})",
                self.id
            ));
        }

        if self.name.is_empty() || self.name.chars().count() > 100 {
            violations.push(format!(
                "name must be 1..=100 characters (got {} characters)",
                self.name.chars().count()
            ));
        }

        if self.description.chars().count() > 1000 {
            violations.push(format!(
                "description must be at most 1000 characters (got {})",
                self.description.chars().count()
            ));
        }

        if self.workflows.is_empty() || self.workflows.len() > 10 {
            violations.push(format!(
                "workflows must contain 1..=10 definitions (got {})",
                self.workflows.len()
            ));
        }

        for wf in &self.workflows {
            if wf
                .nodes
                .iter()
                .any(|n| matches!(n.node_type, NodeType::SubWorkflow { .. }))
            {
                violations.push(format!(
                    "workflow {:?} contains a SubWorkflow node, which is not supported in playbook manifest v1",
                    wf.name
                ));
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(AiomeError::Validation {
                reason: format!("playbook validation failed: {}", violations.join("; ")),
            })
        }
    }
}

/// Playbook ID の妥当性検査（`^[a-z0-9-]{1,64}$` 相当）
fn is_valid_playbook_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::schema::{Position, TriggerType, WorkflowEdge, WorkflowNode};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn minimal_workflow(name: &str) -> WorkflowDefinition {
        WorkflowDefinition {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: "test workflow".to_string(),
            version: 1,
            nodes: vec![WorkflowNode {
                id: "start".to_string(),
                node_type: NodeType::Start {
                    trigger: TriggerType::Manual,
                },
                label: "Start".to_string(),
                config: serde_json::json!({}),
                position: Position { x: 0.0, y: 0.0 },
            }],
            edges: Vec::<WorkflowEdge>::new(),
            variables: HashMap::new(),
            created_at: "2026-07-03T00:00:00Z".to_string(),
            updated_at: "2026-07-03T00:00:00Z".to_string(),
        }
    }

    fn valid_manifest() -> PlaybookManifest {
        PlaybookManifest {
            playbook_version: 1,
            id: "seo-operations".to_string(),
            name: "SEO 運用".to_string(),
            description: "テスト用".to_string(),
            tags: vec!["seo".to_string()],
            required_skills: vec![],
            required_mcp_servers: vec![],
            workflows: vec![minimal_workflow("audit")],
        }
    }

    #[test]
    fn test_playbook_valid_manifest_ok() {
        assert!(valid_manifest().validate_structure().is_ok());
    }

    #[test]
    fn test_playbook_rejects_bad_id() {
        for bad_id in ["../../etc", "UPPER-case", &"a".repeat(65), ""] {
            let mut m = valid_manifest();
            m.id = bad_id.to_string();
            let err = m.validate_structure().expect_err("id should be rejected");
            assert!(
                err.to_string().contains("id must match"),
                "unexpected error for {:?}: {}",
                bad_id,
                err
            );
        }
    }

    #[test]
    fn test_playbook_rejects_subworkflow_node() {
        let mut m = valid_manifest();
        m.workflows[0].nodes.push(WorkflowNode {
            id: "sub".to_string(),
            node_type: NodeType::SubWorkflow {
                workflow_id: Uuid::new_v4(),
                version: None,
            },
            label: "Sub".to_string(),
            config: serde_json::json!({}),
            position: Position { x: 100.0, y: 0.0 },
        });
        let err = m
            .validate_structure()
            .expect_err("SubWorkflow should be rejected");
        assert!(err.to_string().contains("SubWorkflow"));
    }

    #[test]
    fn test_playbook_rejects_empty_and_11_workflows() {
        let mut m = valid_manifest();
        m.workflows.clear();
        assert!(m.validate_structure().is_err(), "empty must be rejected");

        let mut m = valid_manifest();
        m.workflows = (0..11)
            .map(|i| minimal_workflow(&format!("wf-{i}")))
            .collect();
        assert!(m.validate_structure().is_err(), "11 must be rejected");
    }

    #[test]
    fn test_playbook_error_lists_all_violations() {
        let mut m = valid_manifest();
        m.playbook_version = 2;
        m.id = "Bad/Id".to_string();
        m.name = String::new();
        let err = m.validate_structure().expect_err("must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("playbook_version"),
            "missing version violation: {msg}"
        );
        assert!(msg.contains("id must match"), "missing id violation: {msg}");
        assert!(
            msg.contains("name must be"),
            "missing name violation: {msg}"
        );
    }
}
