/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core_contracts::invariant::InvariantDagNode;
use chrono::Utc;
use sha2::{Digest, Sha256};

/// 不変条件の状態遷移をハッシュチェーンで管理するエンジン
#[derive(Default)]
pub struct InvariantDag {
    nodes: Vec<InvariantDagNode>,
}

impl InvariantDag {
    /// 新しい DAG インスタンスを作成する
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// 最新のノード数を取得する
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 新しいノードを追加し、ハッシュチェーンを更新する
    pub fn append(
        &mut self,
        step_id: u32,
        job_id: &str,
        action: &str,
        invariants: Vec<String>,
    ) -> InvariantDagNode {
        let parent_hash = self
            .nodes
            .last()
            .map(|n| n.hash.clone())
            .unwrap_or_else(|| "0".to_string());
        let timestamp = Utc::now().to_rfc3339();

        // ハッシュ計算用のシリアライズ文字列
        // parent_hash + step_id + job_id + action + invariants
        let mut hasher = Sha256::new();
        hasher.update(parent_hash.as_bytes());
        hasher.update(step_id.to_be_bytes());
        hasher.update(job_id.as_bytes());
        hasher.update(action.as_bytes());
        for inv in &invariants {
            hasher.update(inv.as_bytes());
        }
        let hash = format!("{:x}", hasher.finalize());

        let node = InvariantDagNode {
            hash: hash.clone(),
            parent_hash,
            step_id,
            job_id: job_id.to_string(),
            action: action.to_string(),
            verified_invariants: invariants,
            timestamp,
        };

        self.nodes.push(node.clone());
        node
    }

    /// チェーン全体の整合性を検証する
    pub fn verify_chain(&self) -> Result<(), String> {
        let mut expected_parent = "0".to_string();

        for (i, node) in self.nodes.iter().enumerate() {
            // 1. 親ハッシュの整合性
            if node.parent_hash != expected_parent {
                return Err(format!(
                    "Chain Break at node {}: Expected parent {}, got {}",
                    i, expected_parent, node.parent_hash
                ));
            }

            // 2. 自身のハッシュの整合性 (改竄検知)
            let mut hasher = Sha256::new();
            hasher.update(node.parent_hash.as_bytes());
            hasher.update(node.step_id.to_be_bytes());
            hasher.update(node.job_id.as_bytes());
            hasher.update(node.action.as_bytes());
            for inv in &node.verified_invariants {
                hasher.update(inv.as_bytes());
            }
            let computed_hash = format!("{:x}", hasher.finalize());

            if node.hash != computed_hash {
                return Err(format!(
                    "Tamper Detected at node {}: Stored hash {}, but computed {}",
                    i, node.hash, computed_hash
                ));
            }

            expected_parent = node.hash.clone();
        }

        Ok(())
    }

    /// 指定したハッシュまでロールバックし、削除されたノードを返す
    pub fn rollback_to(&mut self, target_hash: &str) -> Vec<InvariantDagNode> {
        if let Some(pos) = self.nodes.iter().position(|n| n.hash == target_hash) {
            // target_hash のノード自体は残し、その次からを削除
            if pos + 1 < self.nodes.len() {
                return self.nodes.split_off(pos + 1);
            }
        }
        vec![]
    }

    /// JSON 文字列に変換する
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.nodes).unwrap_or_default()
    }

    /// JSON 文字列から復元する
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let nodes = serde_json::from_str(json)?;
        Ok(Self { nodes })
    }

    /// ノードリストへの可変参照を取得する (テスト用)
    #[cfg(test)]
    pub fn nodes_mut(&mut self) -> &mut Vec<InvariantDagNode> {
        &mut self.nodes
    }
}
