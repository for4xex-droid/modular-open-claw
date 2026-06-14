pub mod schema;
pub mod store;
pub mod transpiler;
pub mod validator;

#[cfg(test)]
mod tests {
    use super::schema::*;
    use super::transpiler::*;
    use super::validator::*;
    use aiome_core_contracts::error::AiomeError;
    use aiome_core_contracts::traits::{ConstitutionalValidator, Job};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use uuid::Uuid;

    struct MockConstitutionalValidator {
        should_fail: bool,
    }

    #[async_trait]
    impl ConstitutionalValidator for MockConstitutionalValidator {
        async fn verify_constitutional(
            &self,
            output: &str,
            _soul_md: &str,
        ) -> Result<(), AiomeError> {
            if self.should_fail || output.contains("harmful") {
                return Err(AiomeError::Validation {
                    reason: "Constitutional validation failed".to_string(),
                });
            }
            Ok(())
        }
    }

    fn create_test_node(id: &str, node_type: NodeType) -> WorkflowNode {
        WorkflowNode {
            id: id.to_string(),
            node_type,
            label: format!("Node {}", id),
            config: serde_json::json!({}),
            position: Position { x: 0.0, y: 0.0 },
        }
    }

    fn create_test_edge(source: &str, target: &str) -> WorkflowEdge {
        WorkflowEdge {
            source: source.to_string(),
            target: target.to_string(),
            source_handle: None,
            target_handle: None,
        }
    }

    #[test]
    fn test_deserialize_valid_workflow() {
        let workflow_json = r#"{
            "id": "8437dfb3-c4e2-4da6-bb4a-262de6e1099c",
            "name": "Test Workflow",
            "description": "A workflow to test JSON deserialization",
            "version": 1,
            "nodes": [
                {
                    "id": "start-1",
                    "node_type": { "Start": { "trigger": "Manual" } },
                    "label": "Start Node",
                    "config": {},
                    "position": { "x": 10.0, "y": 20.0 }
                },
                {
                    "id": "llm-1",
                    "node_type": { "LlmPrompt": { "model": "gemini-2.5", "temperature": 0.7 } },
                    "label": "LLM Node",
                    "config": { "prompt": "Hello World" },
                    "position": { "x": 100.0, "y": 200.0 }
                }
            ],
            "edges": [
                {
                    "source": "start-1",
                    "target": "llm-1",
                    "source_handle": null,
                    "target_handle": null
                }
            ],
            "variables": {},
            "created_at": "2026-06-13T00:00:00Z",
            "updated_at": "2026-06-13T00:00:00Z"
        }"#;

        let decoded: Result<WorkflowDefinition, _> = serde_json::from_str(workflow_json);
        assert!(
            decoded.is_ok(),
            "Failed to deserialize valid workflow JSON: {:?}",
            decoded.err()
        );
        let wf = decoded.unwrap();
        assert_eq!(wf.name, "Test Workflow");
        assert_eq!(wf.nodes.len(), 2);
        assert_eq!(wf.edges.len(), 1);
    }

    #[tokio::test]
    async fn test_validation_success() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let wf_id = Uuid::new_v4();

        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
            create_test_node(
                "http-1",
                NodeType::HttpRequest {
                    method: "GET".to_string(),
                    url_template: "https://api.example.com".to_string(),
                },
            ),
        ];

        let edges = vec![
            create_test_edge("start-1", "llm-1"),
            create_test_edge("llm-1", "http-1"),
        ];

        let wf = WorkflowDefinition {
            id: wf_id,
            name: "Valid Workflow".to_string(),
            description: "No loops, single start, valid nodes".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_ok(), "Validation failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_validation_no_start() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "No Start".to_string(),
            description: "No start node".to_string(),
            version: 1,
            nodes: vec![create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            )],
            edges: vec![],
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::NoStartNode => {}
            other => panic!("Expected NoStartNode error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_multiple_starts() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Multiple Starts".to_string(),
            description: "Multiple start nodes".to_string(),
            version: 1,
            nodes: vec![
                create_test_node(
                    "start-1",
                    NodeType::Start {
                        trigger: TriggerType::Manual,
                    },
                ),
                create_test_node(
                    "start-2",
                    NodeType::Start {
                        trigger: TriggerType::Webhook,
                    },
                ),
            ],
            edges: vec![],
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::MultipleStartNodes => {}
            other => panic!("Expected MultipleStartNodes error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_cycle() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
            create_test_node(
                "llm-2",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
        ];

        let edges = vec![
            create_test_edge("start-1", "llm-1"),
            create_test_edge("llm-1", "llm-2"),
            create_test_edge("llm-2", "llm-1"), // Cycle: llm-1 -> llm-2 -> llm-1
        ];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Cycle Workflow".to_string(),
            description: "Has a cycle".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::CycleDetected => {}
            other => panic!("Expected CycleDetected error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_isolated_node() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
            create_test_node(
                "isolated-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
        ];

        let edges = vec![create_test_edge("start-1", "llm-1")];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Isolated Node Workflow".to_string(),
            description: "Has isolated node".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::IsolatedNode(ref id) => {
                assert_eq!(id, "isolated-1");
            }
            other => panic!("Expected IsolatedNode error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_loop_max_iterations() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "loop-1",
                NodeType::Loop {
                    iterator_expression: "$.items".to_string(),
                    max_iterations: Some(2000),
                },
            ), // Over limit 1000
        ];
        let edges = vec![create_test_edge("start-1", "loop-1")];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Invalid Loop".to_string(),
            description: "Loop exceeds limit".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::InvalidLoopIterations(ref id, max_iter) => {
                assert_eq!(id, "loop-1");
                assert_eq!(max_iter, 2000);
            }
            other => panic!("Expected InvalidLoopIterations error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_self_subworkflow() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let wf_id = Uuid::new_v4();
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "sub-1",
                NodeType::SubWorkflow {
                    workflow_id: wf_id,
                    version: None,
                },
            ), // Reference to self wf_id
        ];
        let edges = vec![create_test_edge("start-1", "sub-1")];

        let wf = WorkflowDefinition {
            id: wf_id,
            name: "Self Subworkflow".to_string(),
            description: "Refers to itself".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::SelfReferentialSubWorkflow(ref id) => {
                assert_eq!(id, "sub-1");
            }
            other => panic!("Expected SelfReferentialSubWorkflow error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_constitutional_fail() {
        let validator = MockConstitutionalValidator { should_fail: true };
        let wf_id = Uuid::new_v4();
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
        ];
        let edges = vec![create_test_edge("start-1", "llm-1")];

        let wf = WorkflowDefinition {
            id: wf_id,
            name: "Harmful Workflow".to_string(),
            description: "This description is fine, but validator fails".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::SecurityViolation(ref msg) => {
                assert!(msg.contains("Constitutional validation failed"));
            }
            other => panic!("Expected SecurityViolation error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_ssrf_blocking() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "http-1",
                NodeType::HttpRequest {
                    method: "GET".to_string(),
                    url_template: "http://127.0.0.1:8080/sensitive".to_string(),
                },
            ), // SSRF target
        ];
        let edges = vec![create_test_edge("start-1", "http-1")];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "SSRF Workflow".to_string(),
            description: "Attempts SSRF".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err(), "Expected SSRF to be blocked");
        match result.err().unwrap() {
            ValidationError::SecurityViolation(ref msg) => {
                assert!(
                    msg.contains("SSRF"),
                    "Expected SSRF error message, got: {}",
                    msg
                );
            }
            other => panic!("Expected SecurityViolation for SSRF, got {:?}", other),
        }
    }

    #[test]
    fn test_transpile_simple_chain() {
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
            create_test_node(
                "http-1",
                NodeType::HttpRequest {
                    method: "GET".to_string(),
                    url_template: "https://api.example.com".to_string(),
                },
            ),
        ];
        let edges = vec![
            create_test_edge("start-1", "llm-1"),
            create_test_edge("llm-1", "http-1"),
        ];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Chain".to_string(),
            description: "Simple chain".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let execution_id = Uuid::new_v4();
        let jobs = WorkflowTranspiler::transpile(&wf, execution_id).expect("Transpile failed");

        // Start ノードは Job を生成しないため、LLM と HTTP の 2つの Job が生成されるはず
        assert_eq!(jobs.len(), 2);

        // 最初の Job (LLM)
        let job1 = &jobs[0];
        assert_eq!(job1.category, "wf_llm");
        let directives1: serde_json::Value =
            serde_json::from_str(job1.karma_directives.as_ref().unwrap()).unwrap();
        assert_eq!(
            directives1["workflow_execution_id"],
            execution_id.to_string()
        );
        assert_eq!(directives1["node_id"], "llm-1");

        // 2番目の Job (HTTP)
        let job2 = &jobs[1];
        assert_eq!(job2.category, "wf_http");
        let directives2: serde_json::Value =
            serde_json::from_str(job2.karma_directives.as_ref().unwrap()).unwrap();
        assert_eq!(
            directives2["workflow_execution_id"],
            execution_id.to_string()
        );
        assert_eq!(directives2["node_id"], "http-1");
        assert_eq!(directives2["parent_job_id"], job1.id); // 依存関係
    }

    #[test]
    fn test_transpile_loop() {
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "loop-1",
                NodeType::Loop {
                    iterator_expression: "$.items".to_string(),
                    max_iterations: Some(3),
                },
            ),
        ];
        let edges = vec![create_test_edge("start-1", "loop-1")];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Loop".to_string(),
            description: "Loop test".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let execution_id = Uuid::new_v4();
        let jobs = WorkflowTranspiler::transpile(&wf, execution_id).expect("Transpile failed");

        // Loop ノードは max_iterations = 3 のため 3つの Job が生成される
        assert_eq!(jobs.len(), 3);
        for (i, job) in jobs.iter().enumerate() {
            assert_eq!(job.category, "wf_loop");
            let directives: serde_json::Value =
                serde_json::from_str(job.karma_directives.as_ref().unwrap()).unwrap();
            assert_eq!(
                directives["workflow_execution_id"],
                execution_id.to_string()
            );
            assert_eq!(directives["node_id"], "loop-1");
            assert_eq!(directives["loop_index"], i);
        }
    }

    #[test]
    fn test_transpile_parallel() {
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
            create_test_node(
                "llm-2",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
            create_test_node(
                "parallel-1",
                NodeType::Parallel {
                    wait_mode: ParallelWaitMode::All,
                },
            ),
        ];
        let edges = vec![
            create_test_edge("start-1", "llm-1"),
            create_test_edge("start-1", "llm-2"),
            create_test_edge("llm-1", "parallel-1"),
            create_test_edge("llm-2", "parallel-1"),
        ];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Parallel".to_string(),
            description: "Parallel test".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let execution_id = Uuid::new_v4();
        let jobs = WorkflowTranspiler::transpile(&wf, execution_id).expect("Transpile failed");

        // 2つの並列 Job + 1つの Barrier Job = 3
        assert_eq!(jobs.len(), 3);
        let barrier_job = &jobs[2];
        assert_eq!(barrier_job.category, "wf_parallel");
        let directives: serde_json::Value =
            serde_json::from_str(barrier_job.karma_directives.as_ref().unwrap()).unwrap();

        // Barrier Job の parent_job_ids に並列実行された Job の ID が含まれること
        let parents = directives["parent_job_ids"].as_array().unwrap();
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&serde_json::json!(jobs[0].id)));
        assert!(parents.contains(&serde_json::json!(jobs[1].id)));
    }

    #[test]
    fn test_transpile_subworkflow_recursion_limit() {
        let wf_id = Uuid::new_v4();
        // 深さ 6 の循環参照（自身を呼び出すSubWorkflowが連鎖）
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "sub-1",
                NodeType::SubWorkflow {
                    workflow_id: wf_id,
                    version: None,
                },
            ),
        ];
        let edges = vec![create_test_edge("start-1", "sub-1")];

        let wf = WorkflowDefinition {
            id: wf_id,
            name: "Recursion Wf".to_string(),
            description: "Recursion".to_string(),
            version: 1,
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let execution_id = Uuid::new_v4();
        let result = WorkflowTranspiler::transpile_with_depth(&wf, execution_id, 6); // 深さ6で呼び出す
        assert!(result.is_err());
        match result.err().unwrap() {
            TranspilerError::RecursionLimitExceeded => {}
            other => panic!("Expected RecursionLimitExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_workflow_conductor_capable_categories() {
        use crate::task_orchestrator::workflow_conductor::WorkflowConductor;
        use crate::task_orchestrator::TaskConductor;

        let conductor = WorkflowConductor::new();
        let categories = conductor.capable_categories();

        assert!(categories.contains(&"wf_llm".to_string()));
        assert!(categories.contains(&"wf_http".to_string()));
        assert!(categories.contains(&"wf_mcp".to_string()));
        assert!(categories.contains(&"wf_loop".to_string()));
        assert!(categories.contains(&"wf_parallel".to_string()));
    }

    #[tokio::test]
    async fn test_store_workflow_crud_and_cascade() {
        use crate::db::DatabasePool;
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();

        let sqlite_pool = match &pool {
            DatabasePool::Sqlite(p) => p,
            _ => panic!("Expected Sqlite pool"),
        };

        sqlx::migrate!("migrations/sqlite")
            .run(sqlite_pool)
            .await
            .unwrap();

        let store = super::store::WorkflowStore::new(pool);
        let wf_id = Uuid::new_v4();
        let creator_id = "test_creator";

        // 1. Create
        store
            .create_workflow(
                wf_id,
                creator_id,
                "Test Flow",
                "Description",
                "private",
                vec!["tag1".to_string()],
            )
            .await
            .unwrap();

        // 2. Get & Verify
        let record = store
            .get_workflow(wf_id)
            .await
            .unwrap()
            .expect("Workflow not found");
        assert_eq!(record.name, "Test Flow");
        assert_eq!(record.creator_id, creator_id);

        // 3. Save version
        let nodes = vec![create_test_node(
            "start-1",
            NodeType::Start {
                trigger: TriggerType::Manual,
            },
        )];
        let wf_def = WorkflowDefinition {
            id: wf_id,
            name: "Test Flow".to_string(),
            description: "Description".to_string(),
            version: 1,
            nodes,
            edges: vec![],
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };
        store
            .save_version(wf_id, 1, &wf_def, "Initial version")
            .await
            .unwrap();

        // Get version
        let loaded_def = store
            .get_version(wf_id, 1)
            .await
            .unwrap()
            .expect("Version not found");
        assert_eq!(loaded_def.name, "Test Flow");

        // 4. Create execution
        let exec_id = Uuid::new_v4();
        store
            .create_execution(exec_id, wf_id, 1, serde_json::json!({}))
            .await
            .unwrap();

        // Get execution
        let exec_record = store
            .get_execution(exec_id)
            .await
            .unwrap()
            .expect("Execution not found");
        assert_eq!(exec_record.status, "Running");

        // Update execution status
        store
            .update_execution_status(
                exec_id,
                "Completed",
                Some(serde_json::json!({"output": "ok"})),
            )
            .await
            .unwrap();
        let exec_record_updated = store
            .get_execution(exec_id)
            .await
            .unwrap()
            .expect("Execution not found");
        assert_eq!(exec_record_updated.status, "Completed");

        // 5. Cascade Delete
        store.delete_workflow(wf_id).await.unwrap();
        let record_deleted = store.get_workflow(wf_id).await.unwrap();
        assert!(record_deleted.is_none());

        // Cascaded versions
        let version_deleted = store.get_version(wf_id, 1).await.unwrap();
        assert!(version_deleted.is_none());

        // Cascaded executions
        let exec_deleted = store.get_execution(exec_id).await.unwrap();
        assert!(exec_deleted.is_none());
    }

    #[test]
    fn test_estimate_cost() {
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ), // 0.003
            create_test_node(
                "mcp-1",
                NodeType::McpToolCall {
                    server_name: "test".to_string(),
                    tool_name: "test".to_string(),
                },
            ), // 0.001
            create_test_node(
                "http-1",
                NodeType::HttpRequest {
                    method: "GET".to_string(),
                    url_template: "https://api.example.com".to_string(),
                },
            ), // 0.0005
            create_test_node(
                "loop-1",
                NodeType::Loop {
                    iterator_expression: "$.items".to_string(),
                    max_iterations: Some(5),
                },
            ), // 0.003 * 5 = 0.015
        ];

        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Estimate Test".to_string(),
            description: "Testing cost estimation".to_string(),
            version: 1,
            nodes,
            edges: vec![],
            variables: HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let est = wf.estimate_cost();
        assert_eq!(est.nodes, 5);
        // 0.0 + 0.003 + 0.001 + 0.0005 + 0.015 = 0.0195
        assert!((est.estimated_usd - 0.0195).abs() < 1e-6);
    }

    #[test]
    fn test_deserialize_timer_and_wasm_code() {
        let workflow_json = r#"{
            "id": "8437dfb3-c4e2-4da6-bb4a-262de6e1099c",
            "name": "Extended Workflow",
            "description": "Workflow with timer and wasm",
            "version": 1,
            "nodes": [
                {
                    "id": "start-1",
                    "node_type": { "Start": { "trigger": "Manual" } },
                    "label": "Start Node",
                    "config": {},
                    "position": { "x": 0.0, "y": 0.0 }
                },
                {
                    "id": "timer-1",
                    "node_type": { "Timer": { "delay_seconds": 10 } },
                    "label": "Timer Node",
                    "config": {},
                    "position": { "x": 100.0, "y": 0.0 }
                },
                {
                    "id": "wasm-1",
                    "node_type": { "WasmCode": { "code": "console.log('hello')", "language": "javascript" } },
                    "label": "Wasm Node",
                    "config": {},
                    "position": { "x": 200.0, "y": 0.0 }
                }
            ],
            "edges": [
                {
                    "source": "start-1",
                    "target": "timer-1",
                    "source_handle": null,
                    "target_handle": null
                },
                {
                    "source": "timer-1",
                    "target": "wasm-1",
                    "source_handle": null,
                    "target_handle": null
                }
            ],
            "variables": {},
            "created_at": "2026-06-13T00:00:00Z",
            "updated_at": "2026-06-13T00:00:00Z"
        }"#;

        let decoded: Result<WorkflowDefinition, _> = serde_json::from_str(workflow_json);
        assert!(
            decoded.is_ok(),
            "Failed to deserialize extended workflow JSON: {:?}",
            decoded.err()
        );
        let wf = decoded.unwrap();
        assert_eq!(wf.nodes.len(), 3);

        match &wf.nodes[1].node_type {
            NodeType::Timer { delay_seconds } => assert_eq!(*delay_seconds, 10),
            other => panic!("Expected NodeType::Timer, got {:?}", other),
        }

        match &wf.nodes[2].node_type {
            NodeType::WasmCode { code, language } => {
                assert_eq!(code, "console.log('hello')");
                assert_eq!(language, "javascript");
            }
            other => panic!("Expected NodeType::WasmCode, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_invalid_timer() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "timer-1",
                NodeType::Timer {
                    delay_seconds: 90000,
                },
            ), // 24時間(86400)超
        ];
        let edges = vec![create_test_edge("start-1", "timer-1")];
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Invalid Timer".to_string(),
            description: "Timer exceeds limit".to_string(),
            version: 1,
            nodes,
            edges,
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::InvalidTimerDelay(ref id, delay) => {
                assert_eq!(id, "timer-1");
                assert_eq!(delay, 90000);
            }
            other => panic!("Expected InvalidTimerDelay, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_invalid_wasm_code() {
        let validator = MockConstitutionalValidator { should_fail: false };
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "wasm-1",
                NodeType::WasmCode {
                    code: "".to_string(),
                    language: "python".to_string(),
                },
            ), // 空コード、無効言語
        ];
        let edges = vec![create_test_edge("start-1", "wasm-1")];
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Invalid Wasm".to_string(),
            description: "Wasm invalid".to_string(),
            version: 1,
            nodes,
            edges,
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let result = WorkflowValidator::validate(&wf, &validator).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            ValidationError::InvalidWasmCode(ref id, ref reason) => {
                assert_eq!(id, "wasm-1");
                assert!(reason.contains("Empty code") || reason.contains("Unsupported language"));
            }
            other => panic!("Expected InvalidWasmCode, got {:?}", other),
        }
    }

    #[test]
    fn test_transpile_timer_and_wasm_code() {
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node("timer-1", NodeType::Timer { delay_seconds: 5 }),
            create_test_node(
                "wasm-1",
                NodeType::WasmCode {
                    code: "console.log()".to_string(),
                    language: "javascript".to_string(),
                },
            ),
        ];
        let edges = vec![
            create_test_edge("start-1", "timer-1"),
            create_test_edge("timer-1", "wasm-1"),
        ];
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Transpile Ext".to_string(),
            description: "Transpiling timer/wasm".to_string(),
            version: 1,
            nodes,
            edges,
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let execution_id = Uuid::new_v4();
        let jobs = WorkflowTranspiler::transpile(&wf, execution_id).expect("Transpile failed");

        assert_eq!(jobs.len(), 2);

        let job_timer = &jobs[0];
        assert_eq!(job_timer.category, "wf_timer");
        let dir_timer: serde_json::Value =
            serde_json::from_str(job_timer.karma_directives.as_ref().unwrap()).unwrap();
        assert_eq!(dir_timer["node_id"], "timer-1");

        let job_wasm = &jobs[1];
        assert_eq!(job_wasm.category, "wf_wasm");
        let dir_wasm: serde_json::Value =
            serde_json::from_str(job_wasm.karma_directives.as_ref().unwrap()).unwrap();
        assert_eq!(dir_wasm["node_id"], "wasm-1");
        assert_eq!(dir_wasm["parent_job_id"], job_timer.id);
    }

    #[test]
    fn test_estimate_cost_extended() {
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node("timer-1", NodeType::Timer { delay_seconds: 10 }), // 0.0001
            create_test_node(
                "wasm-1",
                NodeType::WasmCode {
                    code: "1+1".to_string(),
                    language: "javascript".to_string(),
                },
            ), // 0.002
        ];
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Cost Ext".to_string(),
            description: "Cost".to_string(),
            version: 1,
            nodes,
            edges: vec![],
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let est = wf.estimate_cost();
        assert_eq!(est.nodes, 3);
        // 0.0001 + 0.002 = 0.0021
        assert!((est.estimated_usd - 0.0021).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_workflow_conductor_timer_and_wasm() {
        use crate::task_orchestrator::workflow_conductor::WorkflowConductor;
        use crate::task_orchestrator::{TaskConductor, TaskEvent};
        use tokio::sync::mpsc;

        let conductor = WorkflowConductor::new();
        let categories = conductor.capable_categories();
        assert!(categories.contains(&"wf_timer".to_string()));
        assert!(categories.contains(&"wf_wasm".to_string()));

        let (tx, mut rx) = mpsc::channel(10);

        // 1. Timer job execution
        let job_timer = Job {
            id: "job-timer-1".to_string(),
            category: "wf_timer".to_string(),
            topic: serde_json::json!({ "delay_seconds": 1 }).to_string(),
            ..Default::default()
        };

        let start = std::time::Instant::now();
        let res_timer = conductor.conduct(job_timer, tx.clone()).await;
        let duration = start.elapsed();

        assert!(res_timer.is_ok());
        assert!(duration >= std::time::Duration::from_secs(1));

        // 2. Wasm job execution
        let job_wasm = Job {
            id: "job-wasm-1".to_string(),
            category: "wf_wasm".to_string(),
            topic: serde_json::json!({ "code": "console.log('hi')", "language": "javascript" })
                .to_string(),
            ..Default::default()
        };

        let res_wasm = conductor.conduct(job_wasm, tx).await;
        assert!(res_wasm.is_ok());
        let (output, _) = res_wasm.unwrap();
        assert_eq!(output, "WASM execution simulated successfully");

        // イベントが送られているか確認
        let mut events = vec![];
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_validation_timer_boundaries() {
        let validator = MockConstitutionalValidator { should_fail: false };

        // 86400s (OK)
        let nodes_ok = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "timer-1",
                NodeType::Timer {
                    delay_seconds: 86400,
                },
            ),
        ];
        let wf_ok = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Timer Bound OK".to_string(),
            description: "Timer boundary test".to_string(),
            version: 1,
            nodes: nodes_ok,
            edges: vec![create_test_edge("start-1", "timer-1")],
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };
        assert!(WorkflowValidator::validate(&wf_ok, &validator)
            .await
            .is_ok());

        // 86401s (NG)
        let nodes_ng = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "timer-1",
                NodeType::Timer {
                    delay_seconds: 86401,
                },
            ),
        ];
        let wf_ng = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Timer Bound NG".to_string(),
            description: "Timer boundary test".to_string(),
            version: 1,
            nodes: nodes_ng,
            edges: vec![create_test_edge("start-1", "timer-1")],
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };
        let res_ng = WorkflowValidator::validate(&wf_ng, &validator).await;
        assert!(res_ng.is_err());
        match res_ng.err().unwrap() {
            ValidationError::InvalidTimerDelay(ref id, delay) => {
                assert_eq!(id, "timer-1");
                assert_eq!(delay, 86401);
            }
            other => panic!("Expected InvalidTimerDelay, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_wasm_isolated_errors() {
        let validator = MockConstitutionalValidator { should_fail: false };

        // Empty code
        let nodes_empty = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "wasm-1",
                NodeType::WasmCode {
                    code: "  ".to_string(),
                    language: "javascript".to_string(),
                },
            ),
        ];
        let wf_empty = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Empty Wasm".to_string(),
            description: "Empty wasm".to_string(),
            version: 1,
            nodes: nodes_empty,
            edges: vec![create_test_edge("start-1", "wasm-1")],
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };
        let res_empty = WorkflowValidator::validate(&wf_empty, &validator).await;
        assert!(res_empty.is_err());
        match res_empty.err().unwrap() {
            ValidationError::InvalidWasmCode(ref id, ref reason) => {
                assert_eq!(id, "wasm-1");
                assert_eq!(reason, "Empty code");
            }
            other => panic!("Expected Empty code ValidationError, got {:?}", other),
        }

        // Unsupported language
        let nodes_lang = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "wasm-1",
                NodeType::WasmCode {
                    code: "console.log()".to_string(),
                    language: "python".to_string(),
                },
            ),
        ];
        let wf_lang = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Python Wasm".to_string(),
            description: "Python wasm".to_string(),
            version: 1,
            nodes: nodes_lang,
            edges: vec![create_test_edge("start-1", "wasm-1")],
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };
        let res_lang = WorkflowValidator::validate(&wf_lang, &validator).await;
        assert!(res_lang.is_err());
        match res_lang.err().unwrap() {
            ValidationError::InvalidWasmCode(ref id, ref reason) => {
                assert_eq!(id, "wasm-1");
                assert_eq!(reason, "Unsupported language 'python'");
            }
            other => panic!(
                "Expected Unsupported language ValidationError, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_transpile_parallel_n_mode() {
        let nodes = vec![
            create_test_node(
                "start-1",
                NodeType::Start {
                    trigger: TriggerType::Manual,
                },
            ),
            create_test_node(
                "llm-1",
                NodeType::LlmPrompt {
                    model: None,
                    temperature: None,
                },
            ),
            create_test_node(
                "parallel-1",
                NodeType::Parallel {
                    wait_mode: ParallelWaitMode::N(3),
                },
            ),
        ];
        let edges = vec![
            create_test_edge("start-1", "llm-1"),
            create_test_edge("llm-1", "parallel-1"),
        ];
        let wf = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "Parallel N".to_string(),
            description: "Parallel N test".to_string(),
            version: 1,
            nodes,
            edges,
            variables: std::collections::HashMap::new(),
            created_at: "2026".to_string(),
            updated_at: "2026".to_string(),
        };

        let execution_id = Uuid::new_v4();
        let jobs = WorkflowTranspiler::transpile(&wf, execution_id).expect("Transpile failed");

        assert_eq!(jobs.len(), 2);
        let job_parallel = &jobs[1];
        assert_eq!(job_parallel.category, "wf_parallel");

        let directives: serde_json::Value =
            serde_json::from_str(job_parallel.karma_directives.as_ref().unwrap()).unwrap();
        assert_eq!(directives["wait_mode"], "N");
        assert_eq!(directives["wait_mode_n"], 3);
    }
}
