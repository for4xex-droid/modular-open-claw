/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::UniversalJobQueue;
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
use aiome_core::error::AiomeError;
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
        let q = format!(
            "INSERT INTO chat_history (channel_id, role, content, metadata) VALUES ({0}, {1}, {2}, {3})",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2),
            self.pool.ph(3)
        );
        sql_exec!(&self.pool, &q, channel_id, role, content, meta_str).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to insert chat history: {}", e),
            }
        })?;
        Ok(())
    }

    async fn do_fetch_chat_history(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        let q = format!("SELECT id, role, content, metadata FROM chat_history WHERE channel_id = {0} AND is_distilled = 0 ORDER BY id DESC LIMIT {1}", self.pool.ph(0), self.pool.ph(1));
        let mut messages = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(channel_id)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let meta_str: Option<String> = row.try_get("metadata").ok().flatten();
                    let metadata: Option<serde_json::Value> =
                        meta_str.and_then(|s| serde_json::from_str(&s).ok());
                    messages.push(serde_json::json!({ "id": row.get::<i64, _>("id"), "role": row.get::<String, _>("role"), "content": row.get::<String, _>("content"), "metadata": metadata }));
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(channel_id)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let meta_str: Option<String> = row.try_get("metadata").ok().flatten();
                    let metadata: Option<serde_json::Value> =
                        meta_str.and_then(|s| serde_json::from_str(&s).ok());
                    messages.push(serde_json::json!({ "id": row.get::<i64, _>("id"), "role": row.get::<String, _>("role"), "content": row.get::<String, _>("content"), "metadata": metadata }));
                }
            }
        }
        messages.reverse();
        Ok(messages)
    }

    async fn do_get_chat_memory_summary(
        &self,
        channel_id: &str,
    ) -> Result<Option<(String, Option<String>)>, AiomeError> {
        let q = format!(
            "SELECT summary, last_interaction_id FROM chat_memory_summaries WHERE channel_id = {}",
            self.pool.ph(0)
        );
        let opt: Option<(String, Option<String>)> =
            crate::sql_fetch_optional!(&self.pool, (String, Option<String>), &q, channel_id)
                .unwrap_or(None);
        Ok(opt)
    }

    async fn do_update_chat_memory_summary(
        &self,
        channel_id: &str,
        summary: &str,
        last_interaction_id: Option<&str>,
    ) -> Result<(), AiomeError> {
        let q = match &self.pool {
            crate::db::DatabasePool::Sqlite(_) => format!("INSERT OR REPLACE INTO chat_memory_summaries (channel_id, summary, last_interaction_id, updated_at) VALUES ({0}, {1}, {2}, {3})", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.now_fn()),
            crate::db::DatabasePool::Postgres(_) => format!("INSERT INTO chat_memory_summaries (channel_id, summary, last_interaction_id, updated_at) VALUES ({0}, {1}, {2}, {3}) ON CONFLICT (channel_id) DO UPDATE SET summary = EXCLUDED.summary, last_interaction_id = EXCLUDED.last_interaction_id, updated_at = EXCLUDED.updated_at", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.now_fn()),
        };
        sql_exec!(&self.pool, &q, channel_id, summary, last_interaction_id)?;
        Ok(())
    }

    async fn do_fetch_undistilled_chats_by_channel(
        &self,
    ) -> Result<HashMap<String, Vec<(i64, String, String)>>, AiomeError> {
        let q = "SELECT id, channel_id, role, content FROM chat_history WHERE is_distilled = 0 ORDER BY channel_id ASC, id ASC";
        let mut map = HashMap::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows =
                    sqlx::query(q)
                        .fetch_all(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                for row in rows {
                    let id: i64 = row.get("id");
                    let channel_id: String = row.get("channel_id");
                    let role: String = row.get("role");
                    let content: String = row.get("content");
                    map.entry(channel_id)
                        .or_insert_with(Vec::new)
                        .push((id, role, content));
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows =
                    sqlx::query(q)
                        .fetch_all(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                for row in rows {
                    let id: i64 = row.get("id");
                    let channel_id: String = row.get("channel_id");
                    let role: String = row.get("role");
                    let content: String = row.get("content");
                    map.entry(channel_id)
                        .or_insert_with(Vec::new)
                        .push((id, role, content));
                }
            }
        }
        Ok(map)
    }

    async fn do_mark_chats_as_distilled(
        &self,
        channel_id: &str,
        up_to_id: i64,
    ) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE chat_history SET is_distilled = 1 WHERE channel_id = {0} AND id <= {1}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        sql_exec!(&self.pool, &q, channel_id, up_to_id)?;
        Ok(())
    }

    async fn do_purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError> {
        let ts_expr = self.pool.now_with_dynamic_days_interval(0);
        let q = format!(
            "DELETE FROM chat_history WHERE is_distilled = 1 AND created_at < {}",
            ts_expr
        );
        let res = sql_exec!(&self.pool, &q, -days)?;
        Ok(res)
    }

    async fn do_fetch_skills_for_distillation(
        &self,
        threshold: i64,
    ) -> Result<Vec<String>, AiomeError> {
        let q = format!(
            "SELECT related_skill FROM karma_logs GROUP BY related_skill HAVING COUNT(id) > {}",
            self.pool.ph(0)
        );
        let keys: Vec<String> = crate::sql_fetch_all!(&self.pool, (String,), &q, threshold)
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
        let q = format!(
            "SELECT id, lesson FROM karma_logs WHERE related_skill = {}",
            self.pool.ph(0)
        );
        let pairs: Vec<(String, String)> =
            crate::sql_fetch_all!(&self.pool, (String, String), &q, skill).unwrap_or_default();
        Ok(pairs)
    }

    async fn do_adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError> {
        let q = format!("UPDATE karma_logs SET weight = CASE WHEN weight + {0} < 0 THEN 0 WHEN weight + {1} > 100 THEN 100 ELSE weight + {2} END WHERE id = {3}", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3));
        sql_exec!(&self.pool, &q, delta, delta, delta, karma_id)?;
        Ok(())
    }

    async fn do_karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        let ts_expr = self.pool.now_with_dynamic_days_interval(0);

        // 1. 既存の時間・重みベースのアーカイブ (COLD 遷移)
        let q_time = format!("UPDATE karma_logs SET is_archived = 1 WHERE weight < 5 AND (last_applied_at IS NULL OR last_applied_at < {}) AND is_archived = 0", ts_expr);
        let res_time = sql_exec!(&self.pool, &q_time, -90)?;

        // 2. Phase 4: Poincare ベースの重要度パージ (CR-1: バッチ化)
        let mut res_slm = 0;
        if let Some(slm) = &self.slm_bridge {
            tracing::info!(
                "🧬 [PoincareGC] Analyzing low-importance karma via SLM (batch mode)..."
            );
            // 重要度の低そうな記憶を recall でリストアップ
            if let Ok(results) = slm
                .recall("low importance logic artifacts redundant", 100)
                .await
            {
                let queries: Vec<String> = results.iter().map(|r| r.content.clone()).collect();

                if !queries.is_empty() {
                    // バッチで重要度を一括算出 (1回のプロセス起動 or フォールバック)
                    let targets: Vec<String> = match slm.calculate_importance_batch(&queries).await
                    {
                        Ok(scored) => scored
                            .into_iter()
                            .filter(|(_, importance)| *importance < 0.3) // 閾値: 極めて低い
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
                        let q_slm = format!(
                            "UPDATE karma_logs SET is_archived = 1 WHERE content IN ({}) AND is_archived = 0",
                            targets.iter().map(|_| self.pool.ph(0)).collect::<Vec<_>>().join(", ")
                        );

                        // 動的バインドが必要なため、各 db 実装に合わせて処理
                        match &self.pool {
                            crate::db::DatabasePool::Sqlite(p) => {
                                let mut query = sqlx::query(&q_slm);
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
                                let mut query = sqlx::query(&q_slm);
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
        let q1 = format!(
            "UPDATE karma_logs SET is_archived = 1 WHERE id = {}",
            self.pool.ph(0)
        );
        for id in old_karma_ids {
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    sqlx::query(&q1)
                        .bind(id)
                        .execute(&mut **t)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    sqlx::query(&q1)
                        .bind(id)
                        .execute(&mut **t)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                }
            }
        }
        let new_id = uuid::Uuid::new_v4().to_string();
        let domain = domain.unwrap_or("general");
        let q2 = format!("INSERT INTO karma_logs (id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, domain, subtopic, clone_origin_id) VALUES ({0}, 'Synthesized', {1}, {2}, 100, {3}, {4}, {5}, {6}, {7})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.now_fn(), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6));
        match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query(&q2)
                    .bind(&new_id)
                    .bind(skill)
                    .bind(distilled_lesson)
                    .bind(soul_hash)
                    .bind(domain)
                    .bind(subtopic)
                    .bind(clone_origin_id)
                    .execute(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            crate::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query(&q2)
                    .bind(&new_id)
                    .bind(skill)
                    .bind(distilled_lesson)
                    .bind(soul_hash)
                    .bind(domain)
                    .bind(subtopic)
                    .bind(clone_origin_id)
                    .execute(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }
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
        let q1 = format!(
            "UPDATE sns_metrics_history SET retry_count = retry_count + 1 WHERE id = {}",
            self.pool.ph(0)
        );
        let q2 = format!(
            "SELECT retry_count FROM sns_metrics_history WHERE id = {}",
            self.pool.ph(0)
        );
        let count: i64 = match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query(&q1)
                    .bind(record_id)
                    .execute(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                sqlx::query(&q2)
                    .bind(record_id)
                    .fetch_one(&mut **t)
                    .await
                    .map(|r| r.get("retry_count"))
                    .unwrap_or(0)
            }
            crate::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query(&q1)
                    .bind(record_id)
                    .execute(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                sqlx::query(&q2)
                    .bind(record_id)
                    .fetch_one(&mut **t)
                    .await
                    .map(|r| r.get("retry_count"))
                    .unwrap_or(0)
            }
        };

        if count >= 3 {
            let q3 = format!("UPDATE sns_metrics_history SET is_finalized = 1, oracle_reason = 'Poison Pill Activated: LLM Evaluation continually fails.' WHERE id = {}", self.pool.ph(0));
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q3).bind(record_id).execute(&mut **t).await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q3).bind(record_id).execute(&mut **t).await;
                }
            }
            let _ = tx.commit().await;
            Ok(true)
        } else {
            let _ = tx.commit().await;
            Ok(false)
        }
    }
}
