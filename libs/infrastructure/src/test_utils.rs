/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
pub mod job_queue_mock {
    use crate::job_queue::{
        EvaluationOps, EvolutionOps, FederationOps, GuardrailOps, KarmaOps, SecurityOps,
        SettingsOps, SoulStoreOps, SwarmOps,
    };
    use aiome_core_contracts::contracts::{
        ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict, SamsaraEvent,
    };
    use aiome_core_contracts::error::AiomeError;
    use aiome_core_contracts::llm::{LlmProvider, LlmResponse};
    use aiome_core_contracts::security::PermissionManifest;
    use aiome_core_contracts::traits::{
        AgentEvolver, AuditStore, ChatStore, CommuneRegistry, Expression, FederationRegistry,
        ImmuneSystemOps, Job, JobQueue, JobStatus, KarmaRegistry, KarmaSearchResult, Publisher,
        SnsMetricsRecord, SoulStore, SystemStateOps, TaskRegistry,
    };
    use aiome_core_contracts::traits::{StrategicPlanner, ToolDiscoveryEngine};
    use aiome_core_contracts::trajectory::TrajectoryStep;
    use aiome_core_contracts::types::AgentStats;
    use async_trait::async_trait;
    use serde_json::Value;
    use uuid::Uuid;

    #[derive(Debug, Default)]
    pub struct GlobalMockJobQueue {
        pub job_to_return: std::sync::Mutex<Option<Job>>,
        pub fetched_job: std::sync::Mutex<Option<Job>>,
        pub completed: std::sync::Mutex<bool>,
        pub karmas: std::sync::Mutex<Vec<Value>>,
        pub diagnosis: std::sync::Mutex<Option<aiome_core_contracts::trajectory::AgentDiagnosis>>,
        pub trajectory: std::sync::Mutex<Vec<TrajectoryStep>>,
        pub failed_jobs: std::sync::Mutex<Vec<(String, String)>>,
        pub updated_status: std::sync::Mutex<Option<JobStatus>>,
        pub active_rules: std::sync::Mutex<Vec<ImmuneRule>>,
        pub fail_immune_fetch: std::sync::Mutex<bool>,
    }

    #[async_trait]
    impl SystemStateOps for GlobalMockJobQueue {
        async fn store_system_state(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_system_state(&self, _: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
    }

    #[async_trait]
    impl TaskRegistry for GlobalMockJobQueue {
        #[allow(clippy::too_many_arguments)]
        async fn enqueue(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<PermissionManifest>,
            _: Option<Uuid>,
            _: i32,
        ) -> Result<String, AiomeError> {
            Ok("mock-id".into())
        }
        async fn dequeue(&self, _categories: &[&str]) -> Result<Option<Job>, AiomeError> {
            Ok(self.job_to_return.lock().unwrap().take())
        }
        async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> {
            Ok(self.fetched_job.lock().unwrap().clone())
        }
        async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
            {
                *self.completed.lock().unwrap() = true;
            }
            Ok(())
        }
        async fn fail_job(&self, id: &str, reason: &str) -> Result<(), AiomeError> {
            self.failed_jobs
                .lock()
                .unwrap()
                .push((id.to_string(), reason.to_string()));
            Ok(())
        }
        async fn requeue_job(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn cancel_job(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn append_job_karma_directives(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn update_job_status(
            &self,
            _: &str,
            status: aiome_core_contracts::traits::JobStatus,
        ) -> Result<(), AiomeError> {
            {
                *self.updated_status.lock().unwrap() = Some(status);
            }
            Ok(())
        }
        async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_job_cost(&self, _: &str) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
        async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
            Ok(0)
        }
        async fn get_job_count_since(
            &self,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<i64, AiomeError> {
            Ok(0)
        }
        async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
            Ok(0)
        }
        async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
            Ok(true)
        }
    }

    #[async_trait]
    impl AuditStore for GlobalMockJobQueue {
        async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn store_trajectory_step(&self, step: TrajectoryStep) -> Result<(), AiomeError> {
            self.trajectory.lock().unwrap().push(step);
            Ok(())
        }
        async fn fetch_trajectory_steps(&self, _: &str) -> Result<Vec<TrajectoryStep>, AiomeError> {
            Ok(self.trajectory.lock().unwrap().clone())
        }
        async fn clear_trajectory_steps(&self, _: &str) -> Result<(), AiomeError> {
            self.trajectory.lock().unwrap().clear();
            Ok(())
        }
        async fn update_trajectory_reward(&self, _: &str, _: f64) -> Result<(), AiomeError> {
            Ok(())
        }

        async fn fetch_diagnosis(
            &self,
            _: &str,
        ) -> Result<
            Option<aiome_core_contracts::trajectory::AgentDiagnosis>,
            aiome_core_contracts::error::AiomeError,
        > {
            Ok(self.diagnosis.lock().unwrap().clone())
        }
        async fn store_diagnosis(
            &self,
            _: &str,
            diag: aiome_core_contracts::trajectory::AgentDiagnosis,
        ) -> Result<(), aiome_core_contracts::error::AiomeError> {
            {
                *self.diagnosis.lock().unwrap() = Some(diag);
            }
            Ok(())
        }

        async fn get_security_request_count(&self, _: Option<Uuid>) -> Result<u32, AiomeError> {
            Ok(0)
        }
        async fn increment_security_request_count(
            &self,
            _: Option<Uuid>,
        ) -> Result<u32, AiomeError> {
            Ok(1)
        }
    }

    #[async_trait]
    impl ChatStore for GlobalMockJobQueue {
        async fn fetch_chat_history(&self, _: &str, _: i64) -> Result<Vec<Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_chat_message(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: Option<serde_json::Value>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_chat_memory_summary(
            &self,
            _: &str,
        ) -> Result<Option<(String, Option<String>)>, AiomeError> {
            Ok(None)
        }
        async fn update_chat_memory_summary(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn mark_chats_as_distilled(&self, _: &str, _: i64) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn purge_old_distilled_chats(&self, _: i64) -> Result<u64, AiomeError> {
            Ok(0)
        }
    }

    #[async_trait]
    impl KarmaRegistry for GlobalMockJobQueue {
        async fn fetch_relevant_karma(
            &self,
            _: &str,
            _: &str,
            _: i64,
            _: &str,
        ) -> Result<KarmaSearchResult, AiomeError> {
            Ok(KarmaSearchResult::empty())
        }
        #[allow(clippy::too_many_arguments)]
        async fn store_karma(
            &self,
            job_id: &str,
            skill_id: &str,
            lesson: &str,
            karma_type: &str,
            soul_hash: &str,
            domain: Option<&str>,
            subtopic: Option<&str>,
            clone_origin_id: Option<&str>,
            is_private: bool,
        ) -> Result<(), AiomeError> {
            self.karmas.lock().unwrap().push(serde_json::json!({
                "job_id": job_id,
                "skill_id": skill_id,
                "lesson": lesson,
                "karma_type": karma_type,
                "soul_hash": soul_hash,
                "domain": domain,
                "subtopic": subtopic,
                "clone_origin_id": clone_origin_id,
                "is_private": is_private,
            }));
            Ok(())
        }
        async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_all_karma(&self, _: i64) -> Result<Vec<Value>, AiomeError> {
            Ok(self.karmas.lock().unwrap().clone())
        }
        async fn fetch_unincorporated_karma(
            &self,
            _: i64,
            _: &str,
        ) -> Result<Vec<Value>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_as_incorporated(
            &self,
            _: Vec<String>,
            _: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_relevant_karma_by_category(
            &self,
            _: &str,
            _: &str,
            _: i64,
        ) -> Result<KarmaSearchResult, AiomeError> {
            Ok(KarmaSearchResult::empty())
        }
    }

    #[async_trait]
    impl AgentEvolver for GlobalMockJobQueue {
        async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> {
            Ok(AgentStats::default())
        }
        async fn add_resonance(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn add_tech_exp(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn add_creativity(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn sync_samsara_level(&self) -> Result<Option<SamsaraEvent>, AiomeError> {
            Ok(None)
        }
        async fn record_evolution_event(
            &self,
            _: i32,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_evolution_history(&self, _: i64) -> Result<Vec<Value>, AiomeError> {
            Ok(vec![])
        }
        async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl CommuneRegistry for GlobalMockJobQueue {
        async fn get_commune_topic_status(
            &self,
            _: &str,
        ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
            Ok(None)
        }
        async fn advance_commune_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> {
            Ok(0)
        }
        async fn fetch_commune_messages(&self, _: &str, _: i64) -> Result<Vec<Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_commune_message(
            &self,
            _: &aiome_core_contracts::commune::CommuneMessage,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn update_commune_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
        async fn archive_commune_topic(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl SoulStore for GlobalMockJobQueue {
        async fn load_soul(&self, _: &str) -> Result<Option<Value>, AiomeError> {
            Ok(None)
        }
        async fn store_soul_fragment(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            Ok(None)
        }

        async fn archive_lora_model(
            &self,
            _soul_id: &str,
            _generation: u32,
            _lora_hash: &str,
            _adapter_path: &str,
            _base_model: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl aiome_core_contracts::traits::SettingsOps for GlobalMockJobQueue {
        async fn do_get_setting(&self, _key: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
        async fn do_set_setting(
            &self,
            _k: &str,
            _v: &str,
            _c: &str,
            _s: bool,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_get_all_settings(
            &self,
        ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
            Ok(vec![])
        }
        async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
            Ok(false)
        }
        async fn set_auto_expression_enabled(&self, _e: bool) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl JobQueue for GlobalMockJobQueue {
        async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> {
            Ok("".into())
        }
        async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn get_system_agent_id(&self) -> Result<Uuid, AiomeError> {
            Ok(Uuid::nil())
        }
        async fn store_expression(&self, _: &Expression) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_expressions(&self, _: i64) -> Result<Vec<Expression>, AiomeError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl ImmuneSystemOps for GlobalMockJobQueue {
        async fn store_immune_rule(&self, _: &ImmuneRule) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
            if let Ok(fail) = self.fail_immune_fetch.lock() {
                if *fail {
                    return Err(AiomeError::Infrastructure {
                        reason: "Simulated DB error for immune fetch".into(),
                    });
                }
            }
            Ok(self.active_rules.lock().unwrap().clone())
        }
        async fn record_arena_match(&self, _: &ArenaMatch) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_arena_matches(&self, _limit: i64) -> Result<Vec<ArenaMatch>, AiomeError> {
            Ok(vec![])
        }
        async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl FederationRegistry for GlobalMockJobQueue {
        async fn export_federated_data(
            &self,
            _: Option<&str>,
        ) -> Result<(Vec<KarmaEntry>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
            Ok((vec![], vec![], vec![]))
        }
        async fn import_federated_data(
            &self,
            _: Vec<KarmaEntry>,
            _: Vec<ImmuneRule>,
            _: Vec<ArenaMatch>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_peer_sync_time(&self, _: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
        async fn update_peer_sync_time(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_node_id(&self) -> Result<String, AiomeError> {
            Ok("mock".into())
        }
        async fn fetch_unfederated_data(
            &self,
        ) -> Result<(Vec<KarmaEntry>, Vec<ImmuneRule>), AiomeError> {
            Ok((vec![], vec![]))
        }
        async fn mark_as_federated(
            &self,
            _: Vec<String>,
            _: Vec<String>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_federated_metrics(&self) -> Result<FederatedMetrics, AiomeError> {
            Ok(FederatedMetrics::default())
        }
    }

    #[async_trait]
    impl EvaluationOps for GlobalMockJobQueue {
        async fn do_link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_record_sns_metrics(
            &self,
            _job_id: &str,
            _milestone_days: i64,
            _views: i64,
            _likes: i64,
            _comments_count: i64,
            _raw_comments: Option<&str>,
            _repost_count: Option<i64>,
            _quote_count: Option<i64>,
            _reply_count: Option<i64>,
            _impression_count: Option<i64>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_fetch_pending_evaluations(
            &self,
            _: i64,
        ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
            Ok(vec![])
        }
        async fn do_apply_final_verdict(
            &self,
            _: i64,
            _: OracleVerdict,
            _: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_fetch_jobs_for_evaluation(
            &self,
            _: i64,
            _: i64,
        ) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn do_fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl StrategicPlanner for GlobalMockJobQueue {
        async fn plan_goal(&self, _: &str, _: Value) -> Result<Vec<TrajectoryStep>, AiomeError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl ToolDiscoveryEngine for GlobalMockJobQueue {
        async fn discover_tools(&self) -> Result<Vec<Value>, AiomeError> {
            Ok(vec![])
        }
        async fn suggest_tools(&self, _: &str) -> Result<Vec<String>, AiomeError> {
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    pub struct GlobalMockLlm;

    #[async_trait]
    impl LlmProvider for GlobalMockLlm {
        async fn complete(&self, _: &str, _: Option<&str>) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: "mock".to_string(),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
        async fn complete_with_cache(
            &self,
            _: aiome_core_contracts::llm::LlmRequest,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: "mock".to_string(),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
    }

    #[async_trait]
    impl aiome_core_contracts::traits::HarnessRegistryOps for GlobalMockJobQueue {
        async fn store_harness_record(
            &self,
            _record: &aiome_core_contracts::contracts::HarnessRecord,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_harness_records_by_status(
            &self,
            _status: &str,
        ) -> Result<Vec<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
            Ok(vec![])
        }
        async fn update_harness_status(&self, _id: &str, _status: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn delete_harness_record(&self, _id: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_harness_record_by_id(
            &self,
            _id: &str,
        ) -> Result<Option<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
            Ok(None)
        }
        async fn increment_harness_stats(&self, _id: &str, _fire: bool) -> Result<(), AiomeError> {
            Ok(())
        }
    }
}

#[cfg(any(test, debug_assertions))]
pub mod mock_soul_store {
    use aiome_core_contracts::error::AiomeError;
    use aiome_core_contracts::traits::SoulStore;
    use async_trait::async_trait;
    use serde_json::Value;

    pub struct MockSoulStore;
    #[async_trait]
    impl SoulStore for MockSoulStore {
        async fn load_soul(&self, _: &str) -> Result<Option<Value>, AiomeError> {
            Ok(None)
        }
        async fn store_soul_fragment(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            Ok(None)
        }

        async fn archive_lora_model(
            &self,
            _soul_id: &str,
            _generation: u32,
            _lora_hash: &str,
            _adapter_path: &str,
            _base_model: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
    }
}

#[cfg(any(test, debug_assertions))]
pub mod cortex_mock {
    use crate::db::DatabasePool;

    pub async fn setup_db_pool() -> Result<DatabasePool, Box<dyn std::error::Error>> {
        let pool = DatabasePool::new_sqlite("sqlite::memory:").await?;
        let sqlite_pool = pool.get_sqlite_pool_or_err()?;

        // 1. Audit Ledger
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS audit_ledger_global (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                table_name TEXT NOT NULL,
                operation TEXT NOT NULL,
                record_id TEXT NOT NULL,
                new_data TEXT,
                prev_hash TEXT,
                current_hash TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ",
        )
        .execute(sqlite_pool)
        .await?;

        // 2. Wiki Articles
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS cortex_wiki_articles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content_md TEXT NOT NULL,
                concepts TEXT DEFAULT '[]',
                backlinks TEXT DEFAULT '[]',
                source_refs TEXT DEFAULT '[]',
                content_hash TEXT NOT NULL,
                version INTEGER DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ",
        )
        .execute(sqlite_pool)
        .await?;

        // 3. Cortex Documents
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS cortex_documents (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                source_url TEXT,
                content_md TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                source_type TEXT NOT NULL,
                tags TEXT DEFAULT '[]',
                summary TEXT,
                wiki_article_refs TEXT DEFAULT '[]',
                ingested_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                compiled BOOLEAN DEFAULT 0
            )
        ",
        )
        .execute(sqlite_pool)
        .await?;

        // 4. Concept Index
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS cortex_concept_index (
                concept TEXT PRIMARY KEY,
                article_ids TEXT DEFAULT '[]',
                document_ids TEXT DEFAULT '[]',
                summary TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ",
        )
        .execute(sqlite_pool)
        .await?;

        // 5. Activity Log (matches production migration 20260405000001)
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS cortex_activity_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                summary TEXT NOT NULL,
                detail_json TEXT DEFAULT '{}',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ",
        )
        .execute(sqlite_pool)
        .await?;

        // 6. Typed Links
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS cortex_typed_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_article_id TEXT NOT NULL,
                target_article_id TEXT NOT NULL,
                link_type TEXT NOT NULL DEFAULT 'references',
                confidence REAL DEFAULT 1.0,
                evidence_text TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (source_article_id) REFERENCES cortex_wiki_articles(id),
                FOREIGN KEY (target_article_id) REFERENCES cortex_wiki_articles(id),
                UNIQUE(source_article_id, target_article_id, link_type)
            )
        ",
        )
        .execute(sqlite_pool)
        .await?;

        // 7. FTS5 Setup
        sqlx::query(
            "
            CREATE VIRTUAL TABLE IF NOT EXISTS cortex_concept_fts USING fts5(
                concept,
                summary,
                article_ids UNINDEXED,
                document_ids UNINDEXED,
                content='cortex_concept_index',
                content_rowid='rowid'
            )
        ",
        )
        .execute(sqlite_pool)
        .await?;

        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS cortex_concept_fts_ai AFTER INSERT ON cortex_concept_index BEGIN
                INSERT INTO cortex_concept_fts(rowid, concept, summary, article_ids, document_ids)
                VALUES (new.rowid, new.concept, new.summary, new.article_ids, new.document_ids);
            END;"
        ).execute(sqlite_pool).await?;

        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS cortex_concept_fts_au AFTER UPDATE ON cortex_concept_index BEGIN
                INSERT INTO cortex_concept_fts(cortex_concept_fts, rowid, concept, summary, article_ids, document_ids)
                VALUES ('delete', old.rowid, old.concept, old.summary, old.article_ids, old.document_ids);
                INSERT INTO cortex_concept_fts(rowid, concept, summary, article_ids, document_ids)
                VALUES (new.rowid, new.concept, new.summary, new.article_ids, new.document_ids);
            END;"
        ).execute(sqlite_pool).await?;

        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS cortex_concept_fts_ad AFTER DELETE ON cortex_concept_index BEGIN
                INSERT INTO cortex_concept_fts(cortex_concept_fts, rowid, concept, summary, article_ids, document_ids)
                VALUES ('delete', old.rowid, old.concept, old.summary, old.article_ids, old.document_ids);
            END;"
        ).execute(sqlite_pool).await?;

        Ok(pool)
    }
}
