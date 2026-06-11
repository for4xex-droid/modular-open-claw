/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::job_queue::CostOps;
    use crate::job_queue::EvaluationOps;
    use crate::soul_store::UniversalSoulStore;
    use aiome_core_contracts::commune::CommuneMessage;
    use aiome_core_contracts::contracts::{
        ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict, SamsaraEvent,
    };
    use aiome_core_contracts::error::AiomeError;
    use aiome_core_contracts::security::PermissionManifest;
    use aiome_core_contracts::traits::{
        AgentEvolver, AuditStore, ChatStore, CommuneRegistry, Expression, FederationRegistry,
        ImmuneSystemOps, Job, JobQueue, JobStatus, KarmaRegistry, KarmaSearchResult, SettingsOps,
        SnsMetricsRecord, SoulStore, SystemStateOps, TaskRegistry,
    };
    use aiome_core_contracts::types::AgentStats;
    use async_trait::async_trait;
    use serde_json::Value;
    use soul::AgentSoul;
    use uuid::Uuid;

    #[derive(Debug)]
    struct BusyJQ;
    #[async_trait::async_trait]
    impl aiome_core_contracts::traits::SystemStateOps for BusyJQ {
        async fn store_system_state(
            &self,
            _: &str,
            _: &str,
        ) -> Result<(), aiome_core::error::AiomeError> {
            Ok(())
        }
        async fn fetch_system_state(
            &self,
            _: &str,
        ) -> Result<Option<String>, aiome_core::error::AiomeError> {
            Ok(None)
        }
    }
    #[async_trait]
    impl aiome_core_contracts::traits::SettingsOps for BusyJQ {
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
    impl JobQueue for BusyJQ {
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
    impl EvaluationOps for BusyJQ {
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
    impl TaskRegistry for BusyJQ {
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
            Ok("mock".into())
        }
        async fn dequeue(&self, _: &[&str]) -> Result<Option<Job>, AiomeError> {
            Ok(None)
        }
        async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> {
            Ok(None)
        }
        async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
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
            _: aiome_core_contracts::traits::JobStatus,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
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
            Ok(false)
        }
        async fn requeue_job(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl AuditStore for BusyJQ {
        async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn store_trajectory_step(
            &self,
            _: aiome_core_contracts::trajectory::TrajectoryStep,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_trajectory_steps(
            &self,
            _: &str,
        ) -> Result<Vec<aiome_core_contracts::trajectory::TrajectoryStep>, AiomeError> {
            Ok(Vec::new())
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
        async fn clear_trajectory_steps(&self, _: &str) -> Result<(), AiomeError> {
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
            Ok(None)
        }
        async fn store_diagnosis(
            &self,
            _: &str,
            _: aiome_core_contracts::trajectory::AgentDiagnosis,
        ) -> Result<(), aiome_core_contracts::error::AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ChatStore for BusyJQ {
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
    impl KarmaRegistry for BusyJQ {
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
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: bool,
        ) -> Result<(), AiomeError> {
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
            Ok(vec![])
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
    impl AgentEvolver for BusyJQ {
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
    impl ImmuneSystemOps for BusyJQ {
        async fn store_immune_rule(&self, _: &ImmuneRule) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
            Ok(vec![])
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
    impl FederationRegistry for BusyJQ {
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
    impl CommuneRegistry for BusyJQ {
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
    impl SoulStore for BusyJQ {
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
    impl aiome_core_contracts::traits::HarnessRegistryOps for BusyJQ {
        async fn store_harness_record(
            &self,
            _: &aiome_core_contracts::contracts::HarnessRecord,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_harness_records_by_status(
            &self,
            _: &str,
        ) -> Result<Vec<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
            Ok(vec![])
        }
        async fn update_harness_status(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn delete_harness_record(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_harness_record_by_id(
            &self,
            _: &str,
        ) -> Result<Option<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
            Ok(None)
        }
        async fn increment_harness_stats(&self, _: &str, _: bool) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_dream_execution() {
        // Mock LLM
        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let jq = BusyJQ;
        let sonar = ExternalTrendSonar::new(vec![], None);
        let llm = std::sync::Arc::new(MockLlm);
        let dream = DreamState::new(llm);
        let res = dream.dream(&jq, &sonar, 10).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_observability_dream() {
        use crate::job_queue::UniversalJobQueue;
        use crate::llm::evaluation_logger::{EvaluationLogEntry, EvaluationLogger};
        use std::sync::Arc;

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let logger = EvaluationLogger::new(std::sync::Arc::new(
            crate::llm::evaluation_logger::SqlEvalLogRepository::new(pool.clone()),
        ));
        logger
            .log(EvaluationLogEntry {
                prompt: "P1".into(),
                system: None,
                provider: "gemini".into(),
                model: "gemini-2.5-flash".into(),
                latency_ms: 2500, // HIGH LATENCY (> 2000)
                token_count_in: None,
                token_count_out: None,
                cost_usd: Some(2.5), // HIGH COST (> 1.0)
                cache_hit: false,
            })
            .await
            .expect("test seed data insertion must succeed");

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let llm = Arc::new(MockLlm);
        let dream = DreamState::new(llm).with_eval_logger(Arc::new(logger));

        let res = dream.observability_dream().await.unwrap();
        assert!(res.is_some(), "observability_dream should return insight");
        let insight = res.unwrap();
        assert!(
            insight.contains("gemini-2.5-flash"),
            "Insight must mention the high latency model"
        );
        assert!(
            insight.contains("high cost"),
            "Insight must mention the high cost indicator"
        );
    }

    #[tokio::test]
    async fn test_observability_dream_without_logger_returns_none() {
        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        // DreamState WITHOUT eval_logger → should return None
        let dream = DreamState::new(std::sync::Arc::new(MockLlm));
        let res = dream.observability_dream().await.unwrap();
        assert!(
            res.is_none(),
            "Without eval_logger, observability_dream must return None"
        );
    }

    #[tokio::test]
    async fn test_observability_dream_with_empty_stats_returns_none() {
        use crate::job_queue::UniversalJobQueue;
        use crate::llm::evaluation_logger::EvaluationLogger;
        use std::sync::Arc;

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts =
            Arc::new(crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()));
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let logger = EvaluationLogger::new(std::sync::Arc::new(
            crate::llm::evaluation_logger::SqlEvalLogRepository::new(pool.clone()),
        ));
        // No data inserted — empty stats

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let dream = DreamState::new(Arc::new(MockLlm)).with_eval_logger(Arc::new(logger));
        let res = dream.observability_dream().await.unwrap();
        assert!(
            res.is_none(),
            "With empty stats, observability_dream must return None"
        );
    }

    #[tokio::test]
    async fn test_aegis_sentinel_dream() {
        use crate::aegis::incident_repo::IncidentRepository;
        use crate::job_queue::trajectory_store::SqliteTrajectoryStore;
        use crate::job_queue::UniversalJobQueue;
        use tokio::sync::broadcast;

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(SqliteTrajectoryStore::new(pool.clone()));
        let _jq = UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .expect("jq");
        crate::job_queue::migrations::DbInitializer::init_db(&_jq)
            .await
            .unwrap();

        let repo = Arc::new(IncidentRepository::new(pool));
        let (tx, _) = broadcast::channel(16);

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let dream = DreamState::new(Arc::new(MockLlm))
            .with_incident_repo(repo)
            .with_event_sender(tx);

        let res = dream.aegis_sentinel_dream().await.unwrap();
        assert_eq!(
            res.unwrap().insight.unwrap(),
            "Aegis Sentinel dream complete: no incidents to process."
        );
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn test_aegis_sentinel_dream_batch_loop() {
        use crate::aegis::incident_repo::IncidentRepository;
        use crate::aegis::types::IncidentStatus;
        use std::sync::Arc;

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .unwrap();
        crate::job_queue::migrations::DbInitializer::init_db(&jq)
            .await
            .unwrap();

        let repo = Arc::new(IncidentRepository::new(pool));

        // Insert an incident that's already reached 2 retries
        let id = repo
            .insert_incident("test_skill", "hash", "{}", "err")
            .await
            .unwrap();
        repo.increment_retry_count(&id).await.unwrap();
        repo.increment_retry_count(&id).await.unwrap(); // Now at 2

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "fn patched() {}".into(), // Mock patch code
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let dream = DreamState::new(Arc::new(MockLlm)).with_incident_repo(repo.clone());

        // In non-stub mode, verify_with_kani will fail because Podman/Kani are not available.
        // This causes retry_count to increment and eventually reach MAX_KANI_RETRIES → WontFix.
        let _ = dream.aegis_sentinel_dream().await;

        let inc = repo.fetch_incident(&id).await.unwrap().unwrap();
        assert_eq!(inc.status, IncidentStatus::WontFix);
        assert_eq!(inc.retry_count, 3);
    }

    #[tokio::test]
    async fn test_reflective_dream_stores_karma() {
        use crate::job_queue::UniversalJobQueue;
        use aiome_core_contracts::traits::JobStatus;
        use aiome_core_contracts::traits::{KarmaRegistry, TaskRegistry};
        use std::sync::Arc;

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        // 1. Create a dummy job for the seed karma
        let dummy_job_id = jq
            .enqueue("data_processing", "dummy", "auto", None, None, None, 10)
            .await
            .unwrap();
        jq.store_karma(
            &dummy_job_id,
            "dummy_skill",
            "dummy_lesson",
            "Technical",
            "dummy_soul",
            None,
            None,
            None,
            false,
        )
        .await
        .unwrap();

        // 2. Create and fail another job
        let job_id = jq
            .enqueue(
                "data_processing",
                "test failure reflection",
                "auto",
                None,
                None,
                None,
                10,
            )
            .await
            .unwrap();
        jq.update_job_status(&job_id, JobStatus::Failed)
            .await
            .unwrap();
        jq.fail_job(&job_id, "test error").await.unwrap();

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait::async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let dream = DreamState::new(Arc::new(MockLlm));

        let result = dream.reflective_dream(&*jq).await.unwrap();
        assert!(result.is_some());

        // Verification Protocol: Assert that store_karma was called for dream_reflection
        let all_karma = jq.fetch_all_karma(100).await.unwrap();
        let has_reflection = all_karma
            .iter()
            .any(|k| k["karma_type"] == "Synthesized" && k["skill"] == "dream_reflection");
        assert!(
            has_reflection,
            "reflective_dream MUST store karma to write-back into the economic ecosystem"
        );
    }

    #[tokio::test]
    async fn test_biome_evolution_dream() {
        // 1. Setup DB and JobQueue
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = std::sync::Arc::new(
            crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        // 2. Initialize BiomeEngine
        let engine =
            std::sync::Arc::new(tokio::sync::RwLock::new(biome_engine::BiomeEngine::new(42)));

        // テストのために Legendary の発生を強制
        {
            let mut eng = engine.write().await;
            eng.debug_force_rarity(biome_engine::rarity::BiomeRarity::Legendary);
        }

        // 3. Initialize SoulStore and create system AgentSoul in DB
        let soul_store = std::sync::Arc::new(UniversalSoulStore::new(pool.clone()));
        let system_agent_id = jq.get_system_agent_id().await.unwrap();
        let soul = AgentSoul::new(system_agent_id.to_string());
        soul_store.save_soul(&soul).await.unwrap();

        // 4. Setup MockLlm returning valid JSON
        #[derive(Debug)]
        struct BiomeMockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for BiomeMockLlm {
            fn name(&self) -> &str {
                "biome-mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: r#"
                    {
                      "message": "A legendary silicon-based predator has evolved.",
                      "rarity": "Legendary",
                      "recommendation": "invest in silicon diffusion"
                    }
                    "#
                    .into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        // 5. Setup broadcast channel for events
        let (tx, mut rx) = tokio::sync::broadcast::channel(100);

        let dream = DreamState::new(std::sync::Arc::new(BiomeMockLlm))
            .with_biome_engine(engine)
            .with_soul_store(soul_store.clone())
            .with_event_sender(tx);

        // 6. Execute Biome Evolution dream
        let result = dream.biome_evolution_dream(&*jq).await.unwrap();
        assert!(result.is_some());

        // 7. Verify CoreEvent was sent
        let event = rx.recv().await.unwrap();
        if let CoreEvent::BiomeEvolution {
            run_id,
            generation,
            message,
            rarity,
            recommendation,
        } = event
        {
            assert_eq!(generation, 1);
            assert_eq!(rarity, Some("Legendary".to_string()));
            assert!(message.contains("legendary silicon-based predator"));
        } else {
            panic!("Expected CoreEvent::BiomeEvolution");
        }

        // 8. Verify Karma was stored
        let all_karma = jq.fetch_all_karma(100).await.unwrap();
        let has_biome_karma = all_karma
            .iter()
            .any(|k| k["skill"] == "biome" && k["karma_type"] == "Synthesized");
        assert!(
            has_biome_karma,
            "biome_evolution_dream MUST store karma for evolutionary progress"
        );

        // 9. Verify AgentSoul was updated with legendary experience
        let loaded_soul = soul_store
            .load_soul(&system_agent_id.to_string())
            .await
            .unwrap()
            .unwrap();
        let biome_exp = loaded_soul
            .experience_buffer
            .iter()
            .find(|e| e.is_core_memory && e.content.contains("Legendary"));
        assert!(
            biome_exp.is_some(),
            "Legendary achievement MUST update AgentSoul experience buffer"
        );

        // 10. Verify Experience.domain is "biome" (not "general")
        let exp = biome_exp.unwrap();
        assert_eq!(
            exp.domain, "biome",
            "Biome experience domain MUST be 'biome', not default 'general'"
        );
        assert!(
            (exp.outcome_valence - 1.0).abs() < f64::EPSILON,
            "Legendary outcome_valence MUST be 1.0"
        );
        assert!(
            (exp.original_prediction - 0.5).abs() < f64::EPSILON,
            "original_prediction MUST be 0.5 (initial neutral prediction)"
        );

        // 11. Verify PredictiveModel has "biome" domain entry
        assert!(
            loaded_soul.predictive_model.domains.contains_key("biome"),
            "PredictiveModel MUST have a 'biome' domain entry after Biome evolution"
        );
        let biome_dm = &loaded_soul.predictive_model.domains["biome"];
        assert_eq!(
            biome_dm.experience_count, 1,
            "experience_count MUST be 1 after single evolution"
        );
        // surprise = |1.0 - 0.5| = 0.5 → new_accuracy = 0.5*(1-α) + (1-0.5)*α = 0.5 (unchanged for default α=0.05)
        // The key invariant is that the domain was created and updated, not the exact value
        assert!(
            (biome_dm.last_surprise - 0.5).abs() < f64::EPSILON,
            "last_surprise MUST be 0.5 for Legendary (actual=1.0, predicted=0.5)"
        );
    }

    #[tokio::test]
    async fn test_biome_evolution_dream_with_boost_and_higgs() {
        // 1. Setup DB and JobQueue
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = std::sync::Arc::new(
            crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        // 2. ブーストのために 24 時間以内のカルマを登録する（news と commune の 2 ドメイン）
        let system_agent_id = jq.get_system_agent_id().await.unwrap();
        let dummy_job_id = jq
            .enqueue("data_processing", "dummy", "auto", None, None, None, 10)
            .await
            .unwrap();

        jq.store_karma(
            &dummy_job_id,
            "news_topic",
            "news lesson",
            "Synthesized",
            "dummy_soul",
            Some("news"),
            None,
            None,
            false,
        )
        .await
        .unwrap();
        jq.store_karma(
            &dummy_job_id,
            "commune_topic",
            "commune lesson",
            "Synthesized",
            "dummy_soul",
            Some("commune"),
            None,
            None,
            false,
        )
        .await
        .unwrap();

        // 3. Initialize BiomeEngine
        let engine =
            std::sync::Arc::new(tokio::sync::RwLock::new(biome_engine::BiomeEngine::new(42)));

        // テストのために Higgs の発生を強制
        {
            let mut eng = engine.write().await;
            eng.debug_force_substance(biome_engine::particle::SubstanceKind::Higgs);
        }

        // 4. Initialize SoulStore and create system AgentSoul in DB
        let soul_store = std::sync::Arc::new(UniversalSoulStore::new(pool.clone()));
        let soul = soul::AgentSoul::new(system_agent_id.to_string());
        soul_store.save_soul(&soul).await.unwrap();

        // 5. Setup MockLlm returning valid JSON
        #[derive(Debug)]
        struct BiomeMockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for BiomeMockLlm {
            fn name(&self) -> &str {
                "biome-mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: r#"
                    {
                      "message": "A legendary silicon-based predator has evolved.",
                      "rarity": "Legendary",
                      "recommendation": "invest in silicon diffusion"
                    }
                    "#
                    .into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        // 6. Setup broadcast channel for events
        let (tx, mut rx) = tokio::sync::broadcast::channel(100);

        let dream = DreamState::new(std::sync::Arc::new(BiomeMockLlm))
            .with_biome_engine(engine.clone())
            .with_soul_store(soul_store.clone())
            .with_event_sender(tx);

        // 7. Execute Biome Evolution dream
        let result = dream.biome_evolution_dream(&*jq).await.unwrap();
        assert!(result.is_some());

        // 8. エンジンに適用されたブースト値の検証（1.5x に設定されているか）
        {
            let eng = engine.read().await;
            assert!((eng.get_mutation_boost() - 1.5).abs() < 1e-5);
        }

        // 9. Soul への Higgs 凍結処理の適用検証
        let loaded_soul = soul_store
            .load_soul(&system_agent_id.to_string())
            .await
            .unwrap()
            .unwrap();
        let soul_obj: soul::AgentSoul = loaded_soul;

        // frozen_traits に Higgs 固定記録が 1 件追加されていることを検証
        assert_eq!(soul_obj.frozen_traits.len(), 1);
        let frozen = &soul_obj.frozen_traits[0];
        assert_eq!(frozen.frozen_at_generation, 1);

        // somatic_markers に Higgs に対応する Permanent な SomaticMarker が 1 件追加されていることを検証
        let matched_marker = soul_obj
            .somatic_markers
            .iter()
            .find(|m| m.id == frozen.somatic_marker_id);
        assert!(matched_marker.is_some());
        let marker = matched_marker.unwrap();
        assert!(marker.is_permanent);
    }

    #[derive(Debug)]
    struct MockCostOps {
        cost_24h: std::sync::atomic::AtomicU64,
    }

    #[async_trait::async_trait]
    impl SettingsOps for MockCostOps {
        async fn do_get_setting(&self, key: &str) -> Result<Option<String>, AiomeError> {
            if key == "cost_limit_24h" {
                return Ok(Some("10.0".to_string()));
            }
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

    #[async_trait::async_trait]
    impl CostOps for MockCostOps {
        async fn aggregate_cost_hours(&self, _hours: i64) -> Result<f64, AiomeError> {
            let bits = self.cost_24h.load(std::sync::atomic::Ordering::Relaxed);
            Ok(f64::from_bits(bits))
        }
        async fn aggregate_cost_days(&self, _days: i64) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
        async fn aggregate_cost_by_job(&self, _job_id: &str) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
    }

    #[tokio::test]
    async fn test_dream_cost_breaker_allows() {
        use std::sync::atomic::AtomicU64;

        let cost_ops = Arc::new(MockCostOps {
            cost_24h: AtomicU64::new(5.0f64.to_bits()),
        });

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait::async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let jq = BusyJQ;
        let sonar = ExternalTrendSonar::new(vec![], None);
        let llm = Arc::new(MockLlm);

        let dream = DreamState::new(llm).with_cost_ops(cost_ops as Arc<dyn CostOps>);
        let res = dream.dream(&jq, &sonar, 10).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_dream_cost_breaker_blocks() {
        use std::sync::atomic::AtomicU64;

        let cost_ops = Arc::new(MockCostOps {
            cost_24h: AtomicU64::new(15.0f64.to_bits()),
        });

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait::async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let jq = BusyJQ;
        let sonar = ExternalTrendSonar::new(vec![], None);
        let llm = Arc::new(MockLlm);

        let dream = DreamState::new(llm).with_cost_ops(cost_ops as Arc<dyn CostOps>);
        let res = dream.dream(&jq, &sonar, 10).await;

        assert!(res.is_ok());
        let opt = res.unwrap();
        assert!(opt.is_none(), "Dream should be skipped and return None");
    }

    #[tokio::test]
    async fn test_dream_llm_fallback() {
        #[derive(Debug)]
        struct ErrorLlm;
        #[async_trait::async_trait]
        impl aiome_core::llm_provider::LlmProvider for ErrorLlm {
            fn name(&self) -> &str {
                "error-llm"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Err(AiomeError::Infrastructure {
                    reason: "LLM API Error for testing".into(),
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = std::sync::Arc::new(
            crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let engine =
            std::sync::Arc::new(tokio::sync::RwLock::new(biome_engine::BiomeEngine::new(42)));
        let soul_store = std::sync::Arc::new(UniversalSoulStore::new(pool.clone()));
        let system_agent_id = jq.get_system_agent_id().await.unwrap();
        let soul = AgentSoul::new(system_agent_id.to_string());
        soul_store.save_soul(&soul).await.unwrap();

        let (tx, _) = broadcast::channel(16);
        let dream = DreamState::new(Arc::new(ErrorLlm))
            .with_biome_engine(engine)
            .with_soul_store(soul_store)
            .with_event_sender(tx);

        let res_biome = dream.biome_evolution_dream(&*jq).await;
        assert!(
            res_biome.is_ok(),
            "Biome evolution dream must succeed despite LLM failure"
        );
        let opt_biome = res_biome.unwrap();
        assert!(opt_biome.is_some());
        assert!(
            opt_biome.unwrap().contains("steadily"),
            "Insight should contain fallback message"
        );

        let res_sci = dream.scientific_dream(&*jq).await;
        assert!(
            res_sci.is_ok(),
            "Scientific dream must succeed despite LLM failure"
        );
        let opt_sci = res_sci.unwrap();
        assert!(opt_sci.is_some());
        assert!(
            opt_sci.unwrap().contains("Steady progress"),
            "Scientific fallback insight check"
        );
    }

    #[tokio::test]
    async fn test_guardian_llm_limit() {
        use std::sync::atomic::AtomicU32;

        #[derive(Debug)]
        struct CounterLlm {
            count: std::sync::atomic::AtomicU32,
        }
        #[async_trait::async_trait]
        impl aiome_core::llm_provider::LlmProvider for CounterLlm {
            fn name(&self) -> &str {
                "counter-llm"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "LLM Advice".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let llm = Arc::new(CounterLlm {
            count: AtomicU32::new(0),
        });
        let dream = DreamState::new(llm.clone());
        let epoch_counter = AtomicU32::new(0);

        for _ in 0..3 {
            let res = dream
                .biome_crisis_guardian(&epoch_counter, "meteor", "vulnerability_report")
                .await;
            assert!(res.is_ok());
            let opt = res.unwrap();
            assert!(opt.is_some());
            assert_eq!(opt.unwrap(), "LLM Advice");
        }

        assert_eq!(llm.count.load(std::sync::atomic::Ordering::Relaxed), 3);

        let res4 = dream
            .biome_crisis_guardian(&epoch_counter, "meteor", "vulnerability_report")
            .await;
        assert!(res4.is_ok());
        let opt4 = res4.unwrap();
        assert!(opt4.is_some());
        assert!(opt4.unwrap().contains("元素カタリスト"));

        assert_eq!(llm.count.load(std::sync::atomic::Ordering::Relaxed), 3);
    }
}
