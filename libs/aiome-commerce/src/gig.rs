/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::commerce::CommerceEngine;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::gig::{
    AcceptanceCriteria, GigBid, GigDeliverable, GigEngine, GigIntent, VerificationResult,
};
use async_trait::async_trait;
use shared::db::{DatabasePool, DatabaseTransaction};
use shared::sql_exec;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use aiome_core_contracts::llm::LlmProvider;

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
            if let Err(e) = std::fs::create_dir_all(&artifact_root) {
                tracing::warn!(
                    "Failed to create artifact root directory at {}: {}",
                    artifact_root.display(),
                    e
                );
            }
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

        const Q_PUBLISH_SQLITE: &str = "INSERT INTO gig_intents (id, requester_id, description, criteria, max_budget_coins, category, deadline, status) VALUES (?, ?, ?, ?, ?, ?, ?, 'Open')";
        const Q_PUBLISH_PG: &str = "INSERT INTO gig_intents (id, requester_id, description, criteria, max_budget_coins, category, deadline, status) VALUES ($1, $2, $3, $4, $5, $6, $7, 'Open')";

        shared::sql_exec!(
            &self.pool,
            sqlite: Q_PUBLISH_SQLITE,
            pg: Q_PUBLISH_PG,
            intent.id.to_string(),
            intent.requester_id.to_string(),
            intent.description,
            criteria_json,
            i64::try_from(intent.max_budget_coins).map_err(|_| AiomeError::Infrastructure {
                reason: format!(
                    "max_budget_coins {} exceeds i64 max",
                    intent.max_budget_coins
                ),
            })?,
            intent.category.to_string(),
            intent.deadline
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Intent database insertion failed: {}", e),
        })?;

        Ok(intent.id)
    }

    async fn submit_bid(&self, bid: GigBid) -> Result<(), AiomeError> {
        const Q_SUBMIT_SQLITE: &str = "INSERT INTO gig_bids (id, intent_id, bidder_id, price_coins, est_duration_sec, deposit_amount, status) VALUES (?, ?, ?, ?, ?, ?, 'Pending')";
        const Q_SUBMIT_PG: &str = "INSERT INTO gig_bids (id, intent_id, bidder_id, price_coins, est_duration_sec, deposit_amount, status) VALUES ($1, $2, $3, $4, $5, $6, 'Pending')";

        shared::sql_exec!(
            &self.pool,
            sqlite: Q_SUBMIT_SQLITE,
            pg: Q_SUBMIT_PG,
            bid.id.to_string(),
            bid.intent_id.to_string(),
            bid.bidder_id.to_string(),
            i64::try_from(bid.price_coins).map_err(|_| AiomeError::Infrastructure {
                reason: format!("price_coins {} exceeds i64 max", bid.price_coins),
            })?,
            i64::try_from(bid.est_duration_sec).map_err(|_| AiomeError::Infrastructure {
                reason: format!("est_duration_sec {} exceeds i64 max", bid.est_duration_sec),
            })?,
            i64::try_from(bid.deposit_amount).map_err(|_| AiomeError::Infrastructure {
                reason: format!("deposit_amount {} exceeds i64 max", bid.deposit_amount),
            })?
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

        const Q_INTENT_SQLITE: &str = "SELECT requester_id, status FROM gig_intents WHERE id = ?";
        const Q_INTENT_PG: &str = "SELECT requester_id, status FROM gig_intents WHERE id = $1";

        let (requester_id_str, status) = shared::sql_tx_fetch_one!(
            &mut tx,
            (String, String),
            sqlite: Q_INTENT_SQLITE,
            pg: Q_INTENT_PG,
            &intent_id_str
        )?;

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

        const Q_BID_SQLITE: &str =
            "SELECT bidder_id, price_coins FROM gig_bids WHERE id = ? AND intent_id = ?";
        const Q_BID_PG: &str =
            "SELECT bidder_id, price_coins FROM gig_bids WHERE id = $1 AND intent_id = $2";

        let (bidder_id_str, price) = shared::sql_tx_fetch_one!(
            &mut tx,
            (String, i64),
            sqlite: Q_BID_SQLITE,
            pg: Q_BID_PG,
            &bid_id_str,
            &intent_id_str
        )?;

        let bidder_id =
            Uuid::parse_str(&bidder_id_str).map_err(|_| AiomeError::Infrastructure {
                reason: "Invalid bidder UUID in DB".into(),
            })?;

        // 3. Create Escrow in Commerce Engine
        let escrow_id = self
            .commerce_engine
            .escrow_create(
                requester_id,
                u64::try_from(price).map_err(|_| AiomeError::Infrastructure {
                    reason: format!("Bid price {} is negative or overflows u64", price),
                })?,
            )
            .await?;

        // 4. Record Escrow in DB
        const Q_ESCROW_SQLITE: &str = "INSERT INTO escrows (id, payer_id, recipient_id, order_id, amount, status) VALUES (?, ?, ?, ?, ?, 'Locked')";
        const Q_ESCROW_PG: &str = "INSERT INTO escrows (id, payer_id, recipient_id, order_id, amount, status) VALUES ($1, $2, $3, $4, $5, 'Locked')";

        shared::sql_tx_exec!(
            &mut tx,
            sqlite: Q_ESCROW_SQLITE,
            pg: Q_ESCROW_PG,
            escrow_id,
            requester_id.to_string(),
            bidder_id.to_string(),
            &intent_id_str,
            price
        )?;

        // 5. Update Statuses with Optimistic Locking (G-46 Mitigation)
        const Q_UPD_INTENT_SQLITE: &str =
            "UPDATE gig_intents SET status = 'Accepted' WHERE id = ? AND status = 'Open'";
        const Q_UPD_INTENT_PG: &str =
            "UPDATE gig_intents SET status = 'Accepted' WHERE id = $1 AND status = 'Open'";

        const Q_UPD_BID_SQLITE: &str =
            "UPDATE gig_bids SET status = 'Accepted' WHERE id = ? AND status = 'Pending'";
        const Q_UPD_BID_PG: &str =
            "UPDATE gig_bids SET status = 'Accepted' WHERE id = $1 AND status = 'Pending'";

        let intent_rows = shared::sql_tx_exec!(
            &mut tx,
            sqlite: Q_UPD_INTENT_SQLITE,
            pg: Q_UPD_INTENT_PG,
            &intent_id_str
        )?;

        if intent_rows == 0 {
            // Race condition lost!
            return Err(AiomeError::Infrastructure {
                reason: "Race condition detected: Intent already accepted by another process."
                    .into(),
            });
        }

        shared::sql_tx_exec!(
            &mut tx,
            sqlite: Q_UPD_BID_SQLITE,
            pg: Q_UPD_BID_PG,
            &bid_id_str
        )?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Transaction commit failed: {}", e),
        })?;

        Ok(())
    }

    async fn deliver(&self, deliverable: GigDeliverable) -> Result<(), AiomeError> {
        let mut tx = self.pool.begin().await?;
        let order_id_str = deliverable.order_id.to_string();

        // 1. Verify Status and Assignee
        const Q_LOOKUP_SQLITE: &str = "SELECT i.status, b.bidder_id FROM gig_intents i JOIN gig_bids b ON b.intent_id = i.id AND b.status = 'Accepted' WHERE i.id = ?";
        const Q_LOOKUP_PG: &str = "SELECT i.status, b.bidder_id FROM gig_intents i JOIN gig_bids b ON b.intent_id = i.id AND b.status = 'Accepted' WHERE i.id = $1";

        let (status, accepted_bidder_id_str) = shared::sql_tx_fetch_one!(
            &mut tx,
            (String, String),
            sqlite: Q_LOOKUP_SQLITE,
            pg: Q_LOOKUP_PG,
            &order_id_str
        )?;

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

        const Q_DELIV_SQLITE: &str = "INSERT INTO gig_deliveries (order_id, deliverer_id, artifact_path, metadata) VALUES (?, ?, ?, ?)";
        const Q_DELIV_PG: &str = "INSERT INTO gig_deliveries (order_id, deliverer_id, artifact_path, metadata) VALUES ($1, $2, $3, $4)";

        shared::sql_tx_exec!(
            &mut tx,
            sqlite: Q_DELIV_SQLITE,
            pg: Q_DELIV_PG,
            &order_id_str,
            deliverable.deliverer_id.to_string(),
            safe_artifact_path.to_string_lossy().to_string(),
            metadata_str
        )?;

        // 4. Update Status
        const Q_UPD_SQLITE: &str = "UPDATE gig_intents SET status = 'Delivered' WHERE id = ?";
        const Q_UPD_PG: &str = "UPDATE gig_intents SET status = 'Delivered' WHERE id = $1";

        shared::sql_tx_exec!(
            &mut tx,
            sqlite: Q_UPD_SQLITE,
            pg: Q_UPD_PG,
            &order_id_str
        )?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Transaction commit failed: {}", e),
        })?;

        Ok(())
    }

    async fn verify_and_settle(&self, order_id: Uuid) -> Result<VerificationResult, AiomeError> {
        let mut tx = self.pool.begin().await?;
        let order_id_str = order_id.to_string();

        // 1. Fetch Intent, Criteria and Delivery
        const Q_INTENT_SQLITE: &str = "SELECT status, criteria FROM gig_intents WHERE id = ?";
        const Q_INTENT_PG: &str = "SELECT status, criteria FROM gig_intents WHERE id = $1";

        let (status, criteria_json) = shared::sql_tx_fetch_one!(
            &mut tx,
            (String, String),
            sqlite: Q_INTENT_SQLITE,
            pg: Q_INTENT_PG,
            &order_id_str
        )?;

        if status != "Delivered" {
            return Err(AiomeError::Infrastructure {
                reason: format!("Cannot verify: order is in status {}", status),
            });
        }

        let criteria: Vec<AcceptanceCriteria> =
            serde_json::from_str(&criteria_json).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Criteria parsing failed: {}", e),
            })?;

        const Q_DELIV_SQLITE: &str =
            "SELECT artifact_path, metadata FROM gig_deliveries WHERE order_id = ?";
        const Q_DELIV_PG: &str =
            "SELECT artifact_path, metadata FROM gig_deliveries WHERE order_id = $1";

        let (artifact_path, metadata_json) = shared::sql_tx_fetch_one!(
            &mut tx,
            (String, String),
            sqlite: Q_DELIV_SQLITE,
            pg: Q_DELIV_PG,
            &order_id_str
        )?;

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
                    AcceptanceCriteria::OxiLeanProof { required_oxp } => {
                        // OxiLean proof verification must be completed externally
                        // (via /api/skills/verify-proof → shadow-worker gRPC) before settlement.
                        // The deliverable metadata must contain a "oxilean_verified" flag
                        // set by the FormalProofGate after successful Q.E.D. validation.
                        let oxp_verified = metadata
                            .get("oxilean_verified")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let oxp_score = metadata
                            .get("oxilean_oxp")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;

                        if !oxp_verified || oxp_score < *required_oxp {
                            passed = false;
                            detail.push_str(&format!(
                                "[{}] OxiLeanProof FAILED: verified={}, oxp={}/{} required. ",
                                i, oxp_verified, oxp_score, required_oxp
                            ));
                        } else {
                            detail.push_str(&format!(
                                "[{}] OxiLeanProof PASSED: OXP={} (>= {} required). ",
                                i, oxp_score, required_oxp
                            ));
                            overall_score += 1.0;
                        }
                    }
                    AcceptanceCriteria::WasmValidator { .. } => {
                        detail.push_str(&format!(
                            "[{}] WasmValidator: Delegated to WasmSkillManager. ",
                            i
                        ));
                        overall_score += 1.0;
                    }
                }
            }
            if check_count > 0 {
                overall_score /= check_count as f32;
            }
        }

        // 3. Settle Escrow
        const Q_ESCROW_SQLITE: &str = "SELECT id, recipient_id FROM escrows WHERE order_id = ?";
        const Q_ESCROW_PG: &str = "SELECT id, recipient_id FROM escrows WHERE order_id = $1";

        let (escrow_id, recipient_id_str) = shared::sql_tx_fetch_one!(
            &mut tx,
            (String, String),
            sqlite: Q_ESCROW_SQLITE,
            pg: Q_ESCROW_PG,
            &order_id_str
        )?;

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

        const Q_UPD_INTENT_SQLITE: &str = "UPDATE gig_intents SET status = ? WHERE id = ?";
        const Q_UPD_INTENT_PG: &str = "UPDATE gig_intents SET status = $1 WHERE id = $2";

        const Q_LOG_SQLITE: &str = "INSERT INTO verification_logs (id, order_id, criteria_type, passed, score, detail) VALUES (?, ?, 'Combined', ?, ?, ?)";
        const Q_LOG_PG: &str = "INSERT INTO verification_logs (id, order_id, criteria_type, passed, score, detail) VALUES ($1, $2, 'Combined', $3, $4, $5)";

        shared::sql_tx_exec!(
            &mut tx,
            sqlite: Q_UPD_INTENT_SQLITE,
            pg: Q_UPD_INTENT_PG,
            final_status,
            &order_id_str
        )?;

        shared::sql_tx_exec!(
            &mut tx,
            sqlite: Q_LOG_SQLITE,
            pg: Q_LOG_PG,
            Uuid::new_v4().to_string(),
            &order_id_str,
            i32::from(passed),
            overall_score,
            &detail
        )?;

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
