/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::UniversalJobQueue;
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::SettingsOps;
use async_trait::async_trait;
use sqlx::Row;
use std::collections::HashMap;
use tracing::warn;
#[async_trait]
pub trait WatchtowerOps {
    async fn do_insert_chat_message(
        &self,
        channel_id: &str,
        role: &str,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_chat_history(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_get_chat_memory_summary(
        &self,
        channel_id: &str,
    ) -> Result<Option<(String, Option<String>)>, AiomeError>;
    async fn do_update_chat_memory_summary(
        &self,
        channel_id: &str,
        summary: &str,
        last_interaction_id: Option<&str>,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_undistilled_chats_by_channel(
        &self,
    ) -> Result<HashMap<String, Vec<(i64, String, String)>>, AiomeError>;
    async fn do_mark_chats_as_distilled(
        &self,
        channel_id: &str,
        up_to_id: i64,
    ) -> Result<(), AiomeError>;
    async fn do_purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError>;
    async fn do_fetch_skills_for_distillation(
        &self,
        threshold: i64,
    ) -> Result<Vec<String>, AiomeError>;
    async fn do_fetch_raw_karma_for_skill(
        &self,
        skill: &str,
    ) -> Result<Vec<(String, String)>, AiomeError>;
    async fn do_apply_distilled_karma(
        &self,
        skill: &str,
        distilled_lesson: &str,
        old_karma_ids: &[String],
        soul_hash: &str,
        domain: Option<&str>,
        subtopic: Option<&str>,
        clone_origin_id: Option<&str>,
    ) -> Result<(), AiomeError>;
    async fn do_adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError>;
    async fn do_karma_decay_sweep(&self) -> Result<u64, AiomeError>;
    async fn do_increment_oracle_retry_count(&self, record_id: i64) -> Result<bool, AiomeError>;
}

#[async_trait]
impl WatchtowerOps for UniversalJobQueue {
    async fn do_insert_chat_message(
        &self,
        channel_id: &str,
        role: &str,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), AiomeError> {
        let meta_str = metadata.map(|m| m.to_string());

        const Q_INSERT_SQLITE: &str =
            "INSERT INTO chat_history (channel_id, role, content, metadata) VALUES (?, ?, ?, ?)";
        const Q_INSERT_PG: &str = "INSERT INTO chat_history (channel_id, role, content, metadata) VALUES ($1, $2, $3, $4)";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_INSERT_SQLITE,
            pg: Q_INSERT_PG,
            channel_id,
            role,
            content,
            meta_str
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to insert chat history: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_chat_history(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        const Q_FETCH_SQLITE: &str = "SELECT id, role, content, metadata FROM chat_history WHERE channel_id = ? AND is_distilled = 0 ORDER BY id DESC LIMIT ?";
        const Q_FETCH_PG: &str = "SELECT id, role, content, metadata FROM chat_history WHERE channel_id = $1 AND is_distilled = 0 ORDER BY id DESC LIMIT $2";

        let rows: Vec<(i64, String, String, Option<String>)> = crate::sql_fetch_all!(
            &self.pool,
            (i64, String, String, Option<String>),
            sqlite: Q_FETCH_SQLITE,
            pg: Q_FETCH_PG,
            channel_id,
            limit
        )?;

        let mut messages = Vec::new();
        for row in rows {
            let metadata: Option<serde_json::Value> =
                row.3.and_then(|s| serde_json::from_str(&s).ok());
            messages.push(serde_json::json!({ "id": row.0, "role": row.1, "content": row.2, "metadata": metadata }));
        }
        messages.reverse();
        Ok(messages)
    }

    async fn do_get_chat_memory_summary(
        &self,
        channel_id: &str,
    ) -> Result<Option<(String, Option<String>)>, AiomeError> {
        const Q_SUM_SQLITE: &str =
            "SELECT summary, last_interaction_id FROM chat_memory_summaries WHERE channel_id = ?";
        const Q_SUM_PG: &str =
            "SELECT summary, last_interaction_id FROM chat_memory_summaries WHERE channel_id = $1";

        let opt: Option<(String, Option<String>)> = crate::sql_fetch_optional!(
            &self.pool,
            (String, Option<String>),
            sqlite: Q_SUM_SQLITE,
            pg: Q_SUM_PG,
            channel_id
        )
        .unwrap_or(None);
        Ok(opt)
    }

    async fn do_update_chat_memory_summary(
        &self,
        channel_id: &str,
        summary: &str,
        last_interaction_id: Option<&str>,
    ) -> Result<(), AiomeError> {
        const SQLITE_Q: &str = "INSERT OR REPLACE INTO chat_memory_summaries (channel_id, summary, last_interaction_id, updated_at) VALUES (?, ?, ?, datetime('now'))";
        const PG_Q: &str = "INSERT INTO chat_memory_summaries (channel_id, summary, last_interaction_id, updated_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (channel_id) DO UPDATE SET summary = EXCLUDED.summary, last_interaction_id = EXCLUDED.last_interaction_id, updated_at = EXCLUDED.updated_at";

        crate::sql_exec!(
            &self.pool,
            sqlite: SQLITE_Q,
            pg: PG_Q,
            channel_id,
            summary,
            last_interaction_id
        )?;
        Ok(())
    }

    async fn do_fetch_undistilled_chats_by_channel(
        &self,
    ) -> Result<HashMap<String, Vec<(i64, String, String)>>, AiomeError> {
        const Q_UND_SQLITE: &str = "SELECT id, channel_id, role, content FROM chat_history WHERE is_distilled = 0 ORDER BY channel_id ASC, id ASC";
        const Q_UND_PG: &str = "SELECT id, channel_id, role, content FROM chat_history WHERE is_distilled = 0 ORDER BY channel_id ASC, id ASC";

        let mut map = HashMap::new();
        let rows: Vec<(i64, String, String, String)> = crate::sql_fetch_all!(
            &self.pool,
            (i64, String, String, String),
            sqlite: Q_UND_SQLITE,
            pg: Q_UND_PG
        )?;

        for row in rows {
            map.entry(row.1)
                .or_insert_with(Vec::new)
                .push((row.0, row.2, row.3));
        }
        Ok(map)
    }

    async fn do_mark_chats_as_distilled(
        &self,
        channel_id: &str,
        up_to_id: i64,
    ) -> Result<(), AiomeError> {
        const Q_MARK_SQLITE: &str =
            "UPDATE chat_history SET is_distilled = 1 WHERE channel_id = ? AND id <= ?";
        const Q_MARK_PG: &str =
            "UPDATE chat_history SET is_distilled = 1 WHERE channel_id = $1 AND id <= $2";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_MARK_SQLITE,
            pg: Q_MARK_PG,
            channel_id,
            up_to_id
        )?;
        Ok(())
    }

    async fn do_purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError> {
        let ts_expr = self.pool.now_with_dynamic_days_interval(0);
        let sqlite_q = format!(
            "DELETE FROM chat_history WHERE is_distilled = 1 AND created_at < {}",
            ts_expr
        );
        let pg_q = format!(
            "DELETE FROM chat_history WHERE is_distilled = 1 AND created_at < {}",
            ts_expr
        );

        let res = crate::sql_exec!(&self.pool, sqlite: &sqlite_q, pg: &pg_q, -days)?;
        Ok(res)
    }

    async fn do_fetch_skills_for_distillation(
        &self,
        threshold: i64,
    ) -> Result<Vec<String>, AiomeError> {
        const Q_FETCH_SKILLS_SQLITE: &str =
            "SELECT related_skill FROM karma_logs GROUP BY related_skill HAVING COUNT(id) > ?";
        const Q_FETCH_SKILLS_PG: &str =
            "SELECT related_skill FROM karma_logs GROUP BY related_skill HAVING COUNT(id) > $1";

        let keys: Vec<String> = crate::sql_fetch_all!(
            &self.pool,
            (String,),
            sqlite: Q_FETCH_SKILLS_SQLITE,
            pg: Q_FETCH_SKILLS_PG,
            threshold
        )
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();
        Ok(keys)
    }

    async fn do_fetch_raw_karma_for_skill(
        &self,
        skill: &str,
    ) -> Result<Vec<(String, String)>, AiomeError> {
        const Q_RAW_SQLITE: &str = "SELECT id, lesson FROM karma_logs WHERE related_skill = ?";
        const Q_RAW_PG: &str = "SELECT id, lesson FROM karma_logs WHERE related_skill = $1";

        let pairs: Vec<(String, String)> = crate::sql_fetch_all!(
            &self.pool,
            (String, String),
            sqlite: Q_RAW_SQLITE,
            pg: Q_RAW_PG,
            skill
        )
        .unwrap_or_default();
        Ok(pairs)
    }

    async fn do_adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError> {
        const Q_ADJ_SQLITE: &str = "UPDATE karma_logs SET weight = CASE WHEN weight + ? < 0 THEN 0 WHEN weight + ? > 100 THEN 100 ELSE weight + ? END WHERE id = ?";
        const Q_ADJ_PG: &str = "UPDATE karma_logs SET weight = CASE WHEN weight + $1 < 0 THEN 0 WHEN weight + $2 > 100 THEN 100 ELSE weight + $3 END WHERE id = $4";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_ADJ_SQLITE,
            pg: Q_ADJ_PG,
            delta,
            delta,
            delta,
            karma_id
        )?;
        Ok(())
    }

    async fn do_karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        let ts_expr = self.pool.now_with_dynamic_days_interval(0);

        // 1. 既存の時間・重みベースのアーカイブ (COLD 遷移)
        let q_time = format!("UPDATE karma_logs SET is_archived = 1 WHERE weight < 5 AND (last_applied_at IS NULL OR last_applied_at < {}) AND is_archived = 0", ts_expr);
        let res_time = sql_exec!(&self.pool, &q_time, -90)?;

        // 2. Phase 4: Poincare ベースの重要度パージ (CR-1: バッチ化)
        let mut res_slm = 0;

        let threshold_str = self
            .get_setting_value("poincare_gc_threshold")
            .await
            .unwrap_or_default()
            .unwrap_or_else(|| "0.3".to_string());
        let threshold: f64 = threshold_str.parse().unwrap_or(0.3);

        let batch_size_str = self
            .get_setting_value("poincare_gc_batch_size")
            .await
            .unwrap_or_default()
            .unwrap_or_else(|| "100".to_string());
        let batch_size: usize = batch_size_str.parse().unwrap_or(100);

        metrics::gauge!("aiome_poincare_gc_threshold").set(threshold);
        metrics::gauge!("aiome_poincare_gc_batch_size").set(batch_size as f64);

        if let Some(slm) = &self.slm_bridge {
            tracing::info!(
                "🧬 [PoincareGC] Analyzing low-importance karma via SLM (batch mode)..."
            );
            // 重要度の低そうな記憶を recall でリストアップ
            if let Ok(results) = slm
                .recall(
                    "low importance logic artifacts redundant",
                    batch_size as i64,
                )
                .await
            {
                let queries: Vec<String> = results.iter().map(|r| r.content.clone()).collect();

                if !queries.is_empty() {
                    // バッチで重要度を一括算出 (1回のプロセス起動 or フォールバック)
                    let targets: Vec<String> = match slm.calculate_importance_batch(&queries).await
                    {
                        Ok(scored) => scored
                            .into_iter()
                            .filter(|(_, importance)| *importance < threshold) // 動的閾値
                            .map(|(content, _)| content)
                            .collect(),
                        Err(e) => {
                            tracing::warn!(
                                "⚠️ [PoincareGC] Batch importance calculation failed: {}",
                                e
                            );
                            Vec::new()
                        }
                    };

                    if !targets.is_empty() {
                        // 該当するコンテンツをアーカイブ。
                        let q_slm_sqlite = format!(
                            "UPDATE karma_logs SET is_archived = 1 WHERE content IN ({}) AND is_archived = 0",
                            targets.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
                        );
                        let q_slm_pg = format!(
                            "UPDATE karma_logs SET is_archived = 1 WHERE content IN ({}) AND is_archived = 0",
                            targets.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect::<Vec<_>>().join(", ")
                        );

                        // 動的バインドが必要なため、各 db 実装に合わせて処理
                        match &self.pool {
                            crate::db::DatabasePool::Sqlite(p) => {
                                let mut query = sqlx::query(&q_slm_sqlite);
                                for t in &targets {
                                    query = query.bind(t);
                                }
                                res_slm = query
                                    .execute(p)
                                    .await
                                    .map(|r| r.rows_affected())
                                    .unwrap_or(0);
                            }
                            crate::db::DatabasePool::Postgres(p) => {
                                let mut query = sqlx::query(&q_slm_pg);
                                for t in &targets {
                                    query = query.bind(t);
                                }
                                res_slm = query
                                    .execute(p)
                                    .await
                                    .map(|r| r.rows_affected())
                                    .unwrap_or(0);
                            }
                        }
                        if res_slm > 0 {
                            tracing::info!(
                                "✅ [PoincareGC] Archived {} low-importance karma entries.",
                                res_slm
                            );
                            metrics::counter!("aiome_poincare_gc_archived_total")
                                .increment(res_slm);
                        }
                    }
                }
            }
        }

        Ok(res_time + res_slm)
    }

    async fn do_apply_distilled_karma(
        &self,
        skill: &str,
        distilled_lesson: &str,
        old_karma_ids: &[String],
        soul_hash: &str,
        domain: Option<&str>,
        subtopic: Option<&str>,
        clone_origin_id: Option<&str>,
    ) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        const Q1_SQLITE: &str = "UPDATE karma_logs SET is_archived = 1 WHERE id = ?";
        const Q1_PG: &str = "UPDATE karma_logs SET is_archived = 1 WHERE id = $1";

        for id in old_karma_ids {
            crate::sql_tx_exec!(
                &mut tx,
                sqlite: Q1_SQLITE,
                pg: Q1_PG,
                id
            )?;
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        let domain = domain.unwrap_or("general");
        const Q2_SQLITE: &str = "INSERT INTO karma_logs (id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, domain, subtopic, clone_origin_id) VALUES (?, 'Synthesized', ?, ?, 100, ?, datetime('now'), ?, ?, ?)";
        const Q2_PG: &str = "INSERT INTO karma_logs (id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, domain, subtopic, clone_origin_id) VALUES ($1, 'Synthesized', $2, $3, 100, $4, NOW(), $5, $6, $7)";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q2_SQLITE,
            pg: Q2_PG,
            &new_id,
            skill,
            distilled_lesson,
            soul_hash,
            domain,
            subtopic,
            clone_origin_id
        )?;
        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn do_increment_oracle_retry_count(&self, record_id: i64) -> Result<bool, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        const Q1_SQLITE: &str =
            "UPDATE sns_metrics_history SET retry_count = retry_count + 1 WHERE id = ?";
        const Q1_PG: &str =
            "UPDATE sns_metrics_history SET retry_count = retry_count + 1 WHERE id = $1";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            record_id
        )?;

        const Q2_SQLITE: &str = "SELECT retry_count FROM sns_metrics_history WHERE id = ?";
        const Q2_PG: &str = "SELECT retry_count FROM sns_metrics_history WHERE id = $1";

        let count: i64 = crate::sql_tx_fetch_one!(
            &mut tx,
            (i64,),
            sqlite: Q2_SQLITE,
            pg: Q2_PG,
            record_id
        )
        .map(|r| r.0)
        .unwrap_or(0);

        if count >= 3 {
            const Q3_SQLITE: &str = "UPDATE sns_metrics_history SET is_finalized = 1, oracle_reason = 'Poison Pill Activated: LLM Evaluation continually fails.' WHERE id = ?";
            const Q3_PG: &str = "UPDATE sns_metrics_history SET is_finalized = 1, oracle_reason = 'Poison Pill Activated: LLM Evaluation continually fails.' WHERE id = $1";

            let _ = crate::sql_tx_exec!(
                &mut tx,
                sqlite: Q3_SQLITE,
                pg: Q3_PG,
                record_id
            );

            let _ = tx.commit().await;
            Ok(true)
        } else {
            let _ = tx.commit().await;
            Ok(false)
        }
    }
}
