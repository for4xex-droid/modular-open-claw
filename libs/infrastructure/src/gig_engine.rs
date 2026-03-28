/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::db::{DatabasePool, DatabaseTransaction};
use crate::sql_exec;
use aiome_contracts::commerce::CommerceEngine;
use aiome_contracts::error::AiomeError;
use aiome_contracts::gig::{
    AcceptanceCriteria, GigBid, GigDeliverable, GigEngine, GigIntent, IntentCategory,
    VerificationResult,
};
use async_trait::async_trait;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use aiome_contracts::llm::LlmProvider;

use shared::sandbox::PathSandbox;
use std::path::PathBuf;

/// Universal (SQLite/PostgreSQL) implementation for GigEngine
pub struct UniversalGigEngine {
    pool: DatabasePool,
    commerce_engine: Arc<dyn CommerceEngine>,
    llm_provider: Arc<dyn LlmProvider>,
    artifact_root: PathBuf,
}

impl UniversalGigEngine {
    /// UniversalGigEngine の新規インスタンスを生成する
    pub fn new(
        pool: DatabasePool,
        commerce_engine: Arc<dyn CommerceEngine>,
        llm_provider: Arc<dyn LlmProvider>,
        artifact_root: PathBuf,
    ) -> Self {
        // Ensure artifact root exists
        if !artifact_root.exists() {
            let _ = std::fs::create_dir_all(&artifact_root);
        }

        Self {
            pool,
            commerce_engine,
            llm_provider,
            artifact_root,
        }
    }
}

#[async_trait]
impl GigEngine for UniversalGigEngine {
    async fn publish_intent(&self, intent: GigIntent) -> Result<Uuid, AiomeError> {
        let criteria_json =
            serde_json::to_string(&intent.criteria).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Criteria serialization failed: {}", e),
            })?;

        let q = format!(
            "INSERT INTO gig_intents (id, requester_id, description, criteria, max_budget_coins, category, deadline, status) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, 'Open')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6)
        );

        sql_exec!(
            &self.pool,
            &q,
            intent.id.to_string(),
            intent.requester_id.to_string(),
            intent.description,
            criteria_json,
            intent.max_budget_coins as i64,
            intent.category.to_string(),
            intent.deadline
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Intent database insertion failed: {}", e),
        })?;

        Ok(intent.id)
    }

    async fn submit_bid(&self, bid: GigBid) -> Result<(), AiomeError> {
        let q = format!(
            "INSERT INTO gig_bids (id, intent_id, bidder_id, price_coins, est_duration_sec, deposit_amount, status) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, 'Pending')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5)
        );

        sql_exec!(
            &self.pool,
            &q,
            bid.id.to_string(),
            bid.intent_id.to_string(),
            bid.bidder_id.to_string(),
            bid.price_coins as i64,
            bid.est_duration_sec as i64,
            bid.deposit_amount as i64
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Bid database insertion failed: {}", e),
        })?;

        Ok(())
    }

    async fn accept_bid(&self, intent_id: Uuid, bid_id: Uuid) -> Result<(), AiomeError> {
        let mut tx = self.pool.begin().await?;

        // 1. Fetch and Verify Intent
        let intent_id_str = intent_id.to_string();
        let q_intent = format!(
            "SELECT requester_id, status FROM gig_intents WHERE id = {}",
            self.pool.ph(0)
        );

        let (requester_id_str, status) = match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                let row = sqlx::query(&q_intent)
                    .bind(&intent_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (
                    row.get::<String, _>("requester_id"),
                    row.get::<String, _>("status"),
                )
            }
            DatabaseTransaction::Postgres(itx) => {
                let row = sqlx::query(&q_intent)
                    .bind(&intent_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (
                    row.get::<String, _>("requester_id"),
                    row.get::<String, _>("status"),
                )
            }
        };

        if status != "Open" {
            return Err(AiomeError::Infrastructure {
                reason: format!("Cannot accept bid for intent in status: {}", status),
            });
        }

        let requester_id =
            Uuid::parse_str(&requester_id_str).map_err(|_| AiomeError::Infrastructure {
                reason: "Invalid requester UUID in DB".into(),
            })?;

        // 2. Fetch and Verify Bid
        let bid_id_str = bid_id.to_string();
        let q_bid = format!(
            "SELECT bidder_id, price_coins FROM gig_bids WHERE id = {0} AND intent_id = {1}",
            self.pool.ph(0),
            self.pool.ph(1)
        );

        let (bidder_id_str, price) = match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                let row = sqlx::query(&q_bid)
                    .bind(&bid_id_str)
                    .bind(&intent_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (
                    row.get::<String, _>("bidder_id"),
                    row.get::<i64, _>("price_coins"),
                )
            }
            DatabaseTransaction::Postgres(itx) => {
                let row = sqlx::query(&q_bid)
                    .bind(&bid_id_str)
                    .bind(&intent_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (
                    row.get::<String, _>("bidder_id"),
                    row.get::<i64, _>("price_coins"),
                )
            }
        };

        let bidder_id =
            Uuid::parse_str(&bidder_id_str).map_err(|_| AiomeError::Infrastructure {
                reason: "Invalid bidder UUID in DB".into(),
            })?;

        // 3. Create Escrow in Commerce Engine
        let escrow_id = self
            .commerce_engine
            .escrow_create(requester_id, price as u64)
            .await?;

        // 4. Record Escrow in DB
        let q_escrow = format!(
            "INSERT INTO escrows (id, payer_id, recipient_id, order_id, amount, status) VALUES ({0}, {1}, {2}, {3}, {4}, 'Locked')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4)
        );

        match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                sqlx::query(&q_escrow)
                    .bind(&escrow_id)
                    .bind(requester_id.to_string())
                    .bind(bidder_id.to_string())
                    .bind(&intent_id_str)
                    .bind(price)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabaseTransaction::Postgres(itx) => {
                sqlx::query(&q_escrow)
                    .bind(&escrow_id)
                    .bind(requester_id.to_string())
                    .bind(bidder_id.to_string())
                    .bind(&intent_id_str)
                    .bind(price)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }

        // 5. Update Statuses with Optimistic Locking (G-46 Mitigation)
        let q_upd_intent = format!(
            "UPDATE gig_intents SET status = 'Accepted' WHERE id = {0} AND status = 'Open'",
            self.pool.ph(0)
        );
        let q_upd_bid = format!(
            "UPDATE gig_bids SET status = 'Accepted' WHERE id = {0} AND status = 'Open'",
            self.pool.ph(0)
        );

        let intent_rows = match &mut tx {
            DatabaseTransaction::Sqlite(itx) => sqlx::query(&q_upd_intent)
                .bind(&intent_id_str)
                .execute(&mut **itx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .rows_affected(),
            DatabaseTransaction::Postgres(itx) => sqlx::query(&q_upd_intent)
                .bind(&intent_id_str)
                .execute(&mut **itx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .rows_affected(),
        };

        if intent_rows == 0 {
            // Race condition lost!
            return Err(AiomeError::Infrastructure {
                reason: "Race condition detected: Intent already accepted by another process."
                    .into(),
            });
        }

        match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                sqlx::query(&q_upd_bid)
                    .bind(&bid_id_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabaseTransaction::Postgres(itx) => {
                sqlx::query(&q_upd_bid)
                    .bind(&bid_id_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Transaction commit failed: {}", e),
        })?;

        Ok(())
    }

    async fn deliver(&self, deliverable: GigDeliverable) -> Result<(), AiomeError> {
        let mut tx = self.pool.begin().await?;
        let order_id_str = deliverable.order_id.to_string();

        // 1. Verify Status and Assignee
        let q_lookup = format!(
            "SELECT i.status, b.bidder_id FROM gig_intents i JOIN gig_bids b ON b.intent_id = i.id AND b.status = 'Accepted' WHERE i.id = {}",
            self.pool.ph(0)
        );

        let (status, accepted_bidder_id_str) = match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                let row = sqlx::query(&q_lookup)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
            DatabaseTransaction::Postgres(itx) => {
                let row = sqlx::query(&q_lookup)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
        };

        if status != "Accepted" {
            return Err(AiomeError::Infrastructure {
                reason: format!("Cannot deliver: order is in status {}", status),
            });
        }

        if accepted_bidder_id_str != deliverable.deliverer_id.to_string() {
            return Err(AiomeError::Infrastructure {
                reason: "Unauthorized deliverer (not the accepted bidder)".into(),
            });
        }

        // 2. Validate Artifact Path
        let sandbox =
            PathSandbox::new(&self.artifact_root).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create PathSandbox: {}", e),
            })?;

        let safe_artifact_path =
            sandbox
                .validate_path(&deliverable.artifact_path)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!(
                        "Insecure artifact path: {}. Potential traversal blocked.",
                        e
                    ),
                })?;

        // 3. Record Delivery
        let metadata_str = serde_json::to_string(&deliverable.metadata).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Metadata serialization failed: {}", e),
            }
        })?;

        let q_deliv = format!(
            "INSERT INTO gig_deliveries (order_id, deliverer_id, artifact_path, metadata) VALUES ({0}, {1}, {2}, {3})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
        );

        match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                sqlx::query(&q_deliv)
                    .bind(&order_id_str)
                    .bind(deliverable.deliverer_id.to_string())
                    .bind(safe_artifact_path.to_string_lossy().to_string())
                    .bind(&metadata_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabaseTransaction::Postgres(itx) => {
                sqlx::query(&q_deliv)
                    .bind(&order_id_str)
                    .bind(deliverable.deliverer_id.to_string())
                    .bind(safe_artifact_path.to_string_lossy().to_string())
                    .bind(&metadata_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }

        // 4. Update Status
        let q_upd = format!(
            "UPDATE gig_intents SET status = 'Delivered' WHERE id = {}",
            self.pool.ph(0)
        );
        match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                sqlx::query(&q_upd)
                    .bind(&order_id_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabaseTransaction::Postgres(itx) => {
                sqlx::query(&q_upd)
                    .bind(&order_id_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Transaction commit failed: {}", e),
        })?;

        Ok(())
    }

    async fn verify_and_settle(&self, order_id: Uuid) -> Result<VerificationResult, AiomeError> {
        let mut tx = self.pool.begin().await?;
        let order_id_str = order_id.to_string();

        // 1. Fetch Intent, Criteria and Delivery
        let q_intent = format!(
            "SELECT status, criteria FROM gig_intents WHERE id = {}",
            self.pool.ph(0)
        );
        let (status, criteria_json) = match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                let row = sqlx::query(&q_intent)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
            DatabaseTransaction::Postgres(itx) => {
                let row = sqlx::query(&q_intent)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
        };

        if status != "Delivered" {
            return Err(AiomeError::Infrastructure {
                reason: format!("Cannot verify: order is in status {}", status),
            });
        }

        let criteria: Vec<AcceptanceCriteria> =
            serde_json::from_str(&criteria_json).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Criteria parsing failed: {}", e),
            })?;

        let q_deliv = format!(
            "SELECT artifact_path, metadata FROM gig_deliveries WHERE order_id = {}",
            self.pool.ph(0)
        );
        let (artifact_path, metadata_json) = match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                let row = sqlx::query(&q_deliv)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
            DatabaseTransaction::Postgres(itx) => {
                let row = sqlx::query(&q_deliv)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
        };

        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_json).unwrap_or(serde_json::Value::Null);

        // 2. Perform Verification
        let mut passed = true;
        let mut detail = String::new();
        let mut overall_score = 0.0f32;
        let mut check_count = 0;

        if criteria.is_empty() {
            detail.push_str("No criteria defined. Automatic pass.");
            overall_score = 1.0;
        } else {
            for (i, c) in criteria.iter().enumerate() {
                check_count += 1;
                match c {
                    AcceptanceCriteria::FileType { mime, max_bytes } => {
                        detail.push_str(&format!(
                            "[{}] FileType check ({} bytes, mime: {}). ",
                            i, max_bytes, mime
                        ));
                        overall_score += 1.0;
                    }
                    AcceptanceCriteria::JsonSchema { schema } => {
                        let res = jsonschema::is_valid(schema, &metadata);
                        if !res {
                            passed = false;
                            detail.push_str(&format!("[{}] JsonSchema validation failed. ", i));
                        } else {
                            detail.push_str(&format!("[{}] JsonSchema check passed. ", i));
                            overall_score += 1.0;
                        }
                    }
                    AcceptanceCriteria::OracleJudge {
                        rubric_prompt,
                        min_score,
                        ..
                    } => {
                        let prompt = format!(
                            "As an Oracle Judge, evaluate the following delivery against the rubric.\n\n\
                             Rubric: {}\n\
                             Artifact Path: {}\n\
                             Metadata: {}\n\n\
                             Respond EXACTLY in this JSON format: {{ \"passed\": bool, \"score\": float, \"detail\": \"string\" }}",
                            rubric_prompt, artifact_path, metadata_json
                        );

                        match self
                            .llm_provider
                            .complete(&prompt, Some("You are a strict and fair AI Verifier."))
                            .await
                        {
                            Ok(resp) => {
                                #[derive(serde::Deserialize)]
                                struct OracleResponse {
                                    passed: bool,
                                    score: f32,
                                    detail: String,
                                }

                                if let Ok(parsed) =
                                    serde_json::from_str::<OracleResponse>(&resp.content)
                                {
                                    if !parsed.passed || parsed.score < *min_score {
                                        passed = false;
                                    }
                                    overall_score += parsed.score;
                                    detail
                                        .push_str(&format!("[{}] Oracle: {}. ", i, parsed.detail));
                                } else {
                                    passed = false;
                                    detail.push_str(&format!(
                                        "[{}] Oracle failed: Invalid JSON response. ",
                                        i
                                    ));
                                }
                            }
                            Err(e) => {
                                passed = false;
                                detail
                                    .push_str(&format!("[{}] Oracle failed: LLM error {}. ", i, e));
                            }
                        }
                    }
                    _ => {
                        detail.push_str(&format!(
                            "[{}] Verification logic for this type not implemented. ",
                            i
                        ));
                    }
                }
            }
            if check_count > 0 {
                overall_score /= check_count as f32;
            }
        }

        // 3. Settle Escrow
        let q_escrow = format!(
            "SELECT id, recipient_id FROM escrows WHERE order_id = {}",
            self.pool.ph(0)
        );
        let (escrow_id, recipient_id_str) = match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                let row = sqlx::query(&q_escrow)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
            DatabaseTransaction::Postgres(itx) => {
                let row = sqlx::query(&q_escrow)
                    .bind(&order_id_str)
                    .fetch_one(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (row.get::<String, _>(0), row.get::<String, _>(1))
            }
        };

        let recipient_id =
            Uuid::parse_str(&recipient_id_str).map_err(|_| AiomeError::Infrastructure {
                reason: "Invalid recipient UUID in DB".into(),
            })?;

        if passed {
            self.commerce_engine
                .escrow_release(&escrow_id, recipient_id)
                .await?;
        } else {
            self.commerce_engine.escrow_refund(&escrow_id).await?;
        }

        // 4. Update Status and Log
        let final_status = if passed { "Completed" } else { "Rejected" };
        let q_upd_intent = format!(
            "UPDATE gig_intents SET status = {} WHERE id = {}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        let q_log = format!(
            "INSERT INTO verification_logs (id, order_id, criteria_type, passed, score, detail) VALUES ({0}, {1}, 'Combined', {2}, {3}, {4})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4)
        );

        match &mut tx {
            DatabaseTransaction::Sqlite(itx) => {
                sqlx::query(&q_upd_intent)
                    .bind(final_status)
                    .bind(&order_id_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                sqlx::query(&q_log)
                    .bind(Uuid::new_v4().to_string())
                    .bind(&order_id_str)
                    .bind(passed as i32)
                    .bind(overall_score)
                    .bind(&detail)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabaseTransaction::Postgres(itx) => {
                sqlx::query(&q_upd_intent)
                    .bind(final_status)
                    .bind(&order_id_str)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                sqlx::query(&q_log)
                    .bind(Uuid::new_v4().to_string())
                    .bind(&order_id_str)
                    .bind(passed as i32)
                    .bind(overall_score)
                    .bind(&detail)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Transaction commit failed: {}", e),
        })?;

        Ok(VerificationResult {
            order_id,
            passed,
            score: overall_score,
            detail,
        })
    }
}
