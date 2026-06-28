/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// ワークフロー全体の定義
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowDefinition {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: u32,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub variables: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// ワークフローの各ノード
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub config: serde_json::Value,
    pub position: Position,
}

/// 10種類のノードタイプ
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum NodeType {
    /// 1. Start (トリガー) - ワークフローのエントリポイント
    Start { trigger: TriggerType },
    /// 2. LLM Prompt - テキスト生成、プロンプト指示
    LlmPrompt {
        model: Option<String>,
        temperature: Option<f32>,
    },
    /// 3. MCP Tool Call - 外部ツールの呼び出し
    McpToolCall {
        server_name: String,
        tool_name: String,
    },
    /// 4. HTTP Request - 外部 REST API の呼び出し
    HttpRequest {
        method: String,
        url_template: String,
    },
    /// 5. Transform - データ加工、JSONマッピング
    Transform { expression: String },
    /// 6. Condition - if/else 分岐判定
    Condition {
        expression: String,
        mode: ConditionMode,
    },
    /// 7. Human Approval - 人間の手動承認待ち
    HumanApproval {
        prompt_message: String,
        timeout_seconds: Option<u64>,
    },
    /// 8. Loop - イテレータによる繰り返し処理
    Loop {
        iterator_expression: String,
        max_iterations: Option<u32>,
    },
    /// 9. Parallel - 並列分岐と Barrier 合流
    Parallel { wait_mode: ParallelWaitMode },
    /// 10. SubWorkflow - 他のワークフローのサブルーチン呼び出し
    SubWorkflow {
        #[schemars(with = "String")]
        workflow_id: Uuid,
        version: Option<u32>,
    },
    /// 11. Timer - 指定時間待機
    Timer { delay_seconds: u64 },
    /// 12. WasmCode - 隔離された Wasm 実行
    WasmCode {
        code: String,
        language: String, // "javascript" | "rust" など
    },
}

/// トリガーの種別
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum TriggerType {
    Manual,
    Cron { expression: String },
    Webhook,
    Event { event_name: String },
}

/// 分岐の評価モード
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum ConditionMode {
    Expression,
    LlmJudge,
}

/// 並列合流の待機モード
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum ParallelWaitMode {
    All,
    Any,
    N(usize),
}

/// 接続線（エッジ）
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowEdge {
    pub source: String,
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
}

/// 画面上のノード描画座標
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// コスト見積もりの結果
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CostEstimate {
    pub estimated_usd: f64,
    pub nodes: usize,
}

impl WorkflowDefinition {
    /// ワークフロー実行コストの推定
    pub fn estimate_cost(&self) -> CostEstimate {
        let mut total = 0.0;
        for node in &self.nodes {
            total += match &node.node_type {
                NodeType::LlmPrompt { .. } => 0.003, // ~1K tokens相当
                NodeType::McpToolCall { .. } => 0.001,
                NodeType::HttpRequest { .. } => 0.0005,
                NodeType::Loop { max_iterations, .. } => {
                    0.003 * max_iterations.unwrap_or(10) as f64 // 安全上限で見積もり
                }
                NodeType::Timer { .. } => 0.0001,
                NodeType::WasmCode { .. } => 0.002,
                _ => 0.0,
            };
        }
        CostEstimate {
            estimated_usd: total,
            nodes: self.nodes.len(),
        }
    }
}
