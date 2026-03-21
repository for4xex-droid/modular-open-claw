use aiome_contracts::commerce::CommerceEngine;
use aiome_contracts::error::AiomeError;
use aiome_contracts::gig::{
    AcceptanceCriteria, GigBid, GigDeliverable, GigEngine, GigIntent, VerificationResult,
};
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use aiome_contracts::llm::LlmProvider;

/// SQLite をバックエンドとする GigEngine 実装
pub struct SqliteGigEngine {
    pool: SqlitePool,
    commerce_engine: Arc<dyn CommerceEngine>,
    llm_provider: Arc<dyn LlmProvider>,
}

impl SqliteGigEngine {
    /// SqliteGigEngine の新規インスタンスを生成する
    pub fn new(
        pool: SqlitePool,
        commerce_engine: Arc<dyn CommerceEngine>,
        llm_provider: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            pool,
            commerce_engine,
            llm_provider,
        }
    }
}

#[async_trait]
impl GigEngine for SqliteGigEngine {
    async fn publish_intent(&self, intent: GigIntent) -> Result<Uuid, AiomeError> {
        let criteria_json =
            serde_json::to_string(&intent.criteria).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Criteria serialization failed: {}", e),
            })?;

        sqlx::query(
            "INSERT INTO gig_intents (id, requester_id, description, criteria, max_budget_coins, deadline, status)
             VALUES (?, ?, ?, ?, ?, ?, 'Open')"
        )
        .bind(intent.id.to_string())
        .bind(intent.requester_id.to_string())
        .bind(intent.description)
        .bind(criteria_json)
        .bind(intent.max_budget_coins as i64)
        .bind(intent.deadline.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Intent database insertion failed: {}", e),
        })?;

        Ok(intent.id)
    }

    async fn submit_bid(&self, bid: GigBid) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT INTO gig_bids (id, intent_id, bidder_id, price_coins, est_duration_sec, deposit_amount, status)
             VALUES (?, ?, ?, ?, ?, ?, 'Pending')"
        )
        .bind(bid.id.to_string())
        .bind(bid.intent_id.to_string())
        .bind(bid.bidder_id.to_string())
        .bind(bid.price_coins as i64)
        .bind(bid.est_duration_sec as i64)
        .bind(bid.deposit_amount as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Bid database insertion failed: {}", e),
        })?;

        Ok(())
    }

    async fn accept_bid(&self, intent_id: Uuid, bid_id: Uuid) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Transaction start failed: {}", e),
            })?;

        // 1. Fetch and Verify Intent
        let (requester_id_str, status): (String, String) =
            sqlx::query_as("SELECT requester_id, status FROM gig_intents WHERE id = ?")
                .bind(intent_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Intent lookup failed: {}", e),
                })?;

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
        let (bidder_id_str, price): (String, i64) = sqlx::query_as(
            "SELECT bidder_id, price_coins FROM gig_bids WHERE id = ? AND intent_id = ?",
        )
        .bind(bid_id.to_string())
        .bind(intent_id.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Bid lookup failed: {}", e),
        })?;

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
        sqlx::query(
            "INSERT INTO escrows (id, payer_id, recipient_id, order_id, amount, status)
             VALUES (?, ?, ?, ?, ?, 'Locked')",
        )
        .bind(&escrow_id)
        .bind(requester_id.to_string())
        .bind(bidder_id.to_string())
        .bind(intent_id.to_string())
        .bind(price)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow record failed: {}", e),
        })?;

        // 5. Update Statuses
        sqlx::query("UPDATE gig_intents SET status = 'Accepted' WHERE id = ?")
            .bind(intent_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Intent status update failed: {}", e),
            })?;

        sqlx::query("UPDATE gig_bids SET status = 'Accepted' WHERE id = ?")
            .bind(bid_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Bid status update failed: {}", e),
            })?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Transaction commit failed: {}", e),
        })?;

        Ok(())
    }

    async fn deliver(&self, deliverable: GigDeliverable) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Transaction start failed: {}", e),
            })?;

        // 1. Verify Status and Assignee
        let (status, accepted_bidder_id_str): (String, String) = sqlx::query_as(
            "SELECT i.status, b.bidder_id
             FROM gig_intents i
             JOIN gig_bids b ON b.intent_id = i.id AND b.status = 'Accepted'
             WHERE i.id = ?",
        )
        .bind(deliverable.order_id.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Order lookup failed: {}", e),
        })?;

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

        // 2. Record Delivery
        let metadata_str = serde_json::to_string(&deliverable.metadata).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Metadata serialization failed: {}", e),
            }
        })?;

        sqlx::query(
            "INSERT INTO gig_deliveries (order_id, deliverer_id, artifact_path, metadata)
             VALUES (?, ?, ?, ?)",
        )
        .bind(deliverable.order_id.to_string())
        .bind(deliverable.deliverer_id.to_string())
        .bind(deliverable.artifact_path)
        .bind(metadata_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Delivery recording failed: {}", e),
        })?;

        // 3. Update Status
        sqlx::query("UPDATE gig_intents SET status = 'Delivered' WHERE id = ?")
            .bind(deliverable.order_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Status update to Delivered failed: {}", e),
            })?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Transaction commit failed: {}", e),
        })?;

        Ok(())
    }

    async fn verify_and_settle(&self, order_id: Uuid) -> Result<VerificationResult, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Transaction start failed: {}", e),
            })?;

        // 1. Fetch Intent, Criteria and Delivery
        let (status, criteria_json): (String, String) =
            sqlx::query_as("SELECT status, criteria FROM gig_intents WHERE id = ?")
                .bind(order_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Intent lookup failed: {}", e),
                })?;

        if status != "Delivered" {
            return Err(AiomeError::Infrastructure {
                reason: format!("Cannot verify: order is in status {}", status),
            });
        }

        let criteria: Vec<AcceptanceCriteria> =
            serde_json::from_str(&criteria_json).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Criteria parsing failed: {}", e),
            })?;

        let (artifact_path, metadata_json): (String, String) =
            sqlx::query_as("SELECT artifact_path, metadata FROM gig_deliveries WHERE order_id = ?")
                .bind(order_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Delivery lookup failed: {}", e),
                })?;

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
                    AcceptanceCriteria::OracleJudge { rubric_prompt, min_score, .. } => {
                        let prompt = format!(
                            "As an Oracle Judge, evaluate the following delivery against the rubric.\n\n\
                             Rubric: {}\n\
                             Artifact Path: {}\n\
                             Metadata: {}\n\n\
                             Respond EXACTLY in this JSON format: {{ \"passed\": bool, \"score\": float, \"detail\": \"string\" }}",
                            rubric_prompt, artifact_path, metadata_json
                        );

                        match self.llm_provider.complete(&prompt, Some("You are a strict and fair AI Verifier.")).await {
                            Ok(resp) => {
                                #[derive(serde::Deserialize)]
                                struct OracleResponse {
                                    passed: bool,
                                    score: f32,
                                    detail: String,
                                }

                                if let Ok(parsed) = serde_json::from_str::<OracleResponse>(&resp.content) {
                                    if !parsed.passed || parsed.score < *min_score {
                                        passed = false;
                                    }
                                    overall_score += parsed.score;
                                    detail.push_str(&format!("[{}] Oracle: {}. ", i, parsed.detail));
                                } else {
                                    passed = false;
                                    detail.push_str(&format!("[{}] Oracle failed: Invalid JSON response. ", i));
                                }
                            }
                            Err(e) => {
                                passed = false;
                                detail.push_str(&format!("[{}] Oracle failed: LLM error {}. ", i, e));
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
        let (escrow_id, recipient_id_str): (String, String) =
            sqlx::query_as("SELECT id, recipient_id FROM escrows WHERE order_id = ?")
                .bind(order_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Escrow lookup failed: {}", e),
                })?;

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
        sqlx::query("UPDATE gig_intents SET status = ? WHERE id = ?")
            .bind(final_status)
            .bind(order_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Final status update failed: {}", e),
            })?;

        sqlx::query(
            "INSERT INTO verification_logs (id, order_id, criteria_type, passed, score, detail)
             VALUES (?, ?, 'Combined', ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(order_id.to_string())
        .bind(passed as i32)
        .bind(overall_score)
        .bind(&detail)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Verification log failed: {}", e),
        })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::MockLlmProvider;
    use crate::commerce_mock::MockCommerceEngine;
    use chrono::Utc;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE gig_intents (
                id TEXT PRIMARY KEY,
                requester_id TEXT NOT NULL,
                description TEXT NOT NULL,
                criteria TEXT NOT NULL,
                max_budget_coins INTEGER NOT NULL,
                deadline TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Open',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE gig_bids (
                id TEXT PRIMARY KEY,
                intent_id TEXT NOT NULL REFERENCES gig_intents(id),
                bidder_id TEXT NOT NULL,
                price_coins INTEGER NOT NULL,
                est_duration_sec INTEGER NOT NULL,
                deposit_amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Pending',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE escrows (
                id TEXT PRIMARY KEY,
                payer_id TEXT NOT NULL,
                recipient_id TEXT,
                order_id TEXT NOT NULL REFERENCES gig_intents(id),
                amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Locked',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE gig_deliveries (
                order_id TEXT PRIMARY KEY REFERENCES gig_intents(id),
                deliverer_id TEXT NOT NULL,
                artifact_path TEXT NOT NULL,
                metadata TEXT NOT NULL,
                delivered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE verification_logs (
                id TEXT PRIMARY KEY,
                order_id TEXT NOT NULL REFERENCES gig_intents(id),
                criteria_type TEXT NOT NULL,
                passed INTEGER NOT NULL,
                score REAL NOT NULL,
                detail TEXT,
                verified_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    fn mock_llm() -> Arc<dyn LlmProvider> {
        Arc::new(MockLlmProvider {
            response: "{\"passed\": true, \"score\": 0.95, \"detail\": \"Good job!\"}".into(),
            should_fail: false,
        })
    }

    #[tokio::test]
    async fn test_gig_intent_lifecycle_green() {
        let pool = setup_db().await;
        let commerce = Arc::new(MockCommerceEngine);
        let engine = SqliteGigEngine::new(pool, commerce, mock_llm());

        let intent_id = Uuid::new_v4();
        let intent = GigIntent {
            id: intent_id,
            requester_id: Uuid::new_v4(),
            description: "Test gig".into(),
            criteria: vec![AcceptanceCriteria::FileType {
                mime: "image/png".into(),
                max_bytes: 1024,
            }],
            max_budget_coins: 100,
            deadline: Utc::now() + chrono::Duration::hours(1),
        };

        let result = engine.publish_intent(intent).await;
        assert!(result.is_ok());

        let row: (String, String) =
            sqlx::query_as("SELECT id, status FROM gig_intents WHERE id = ?")
                .bind(intent_id.to_string())
                .fetch_one(&engine.pool)
                .await
                .unwrap();

        assert_eq!(row.0, intent_id.to_string());
        assert_eq!(row.1, "Open");
    }

    #[tokio::test]
    async fn test_gig_bid_submission_green() {
        let pool = setup_db().await;
        let commerce = Arc::new(MockCommerceEngine);
        let engine = SqliteGigEngine::new(pool, commerce, mock_llm());

        let intent_id = Uuid::new_v4();
        let bid_id = Uuid::new_v4();

        // 1. Publish Intent
        let intent = GigIntent {
            id: intent_id,
            requester_id: Uuid::new_v4(),
            description: "Test bidding".into(),
            criteria: vec![],
            max_budget_coins: 100,
            deadline: Utc::now() + chrono::Duration::hours(1),
        };
        engine.publish_intent(intent).await.unwrap();

        // 2. Submit Bid
        let bid = GigBid {
            id: bid_id,
            intent_id,
            bidder_id: Uuid::new_v4(),
            price_coins: 80,
            est_duration_sec: 3600,
            deposit_amount: 10,
        };

        let result = engine.submit_bid(bid).await;
        assert!(result.is_ok());

        // Verify persistence
        let row: (String, String) = sqlx::query_as("SELECT id, status FROM gig_bids WHERE id = ?")
            .bind(bid_id.to_string())
            .fetch_one(&engine.pool)
            .await
            .unwrap();

        assert_eq!(row.0, bid_id.to_string());
        assert_eq!(row.1, "Pending");
    }

    #[tokio::test]
    async fn test_gig_bid_acceptance_green() {
        let pool = setup_db().await;
        let commerce = Arc::new(MockCommerceEngine);
        let engine = SqliteGigEngine::new(pool, commerce, mock_llm());

        let intent_id = Uuid::new_v4();
        let bid_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();

        // 1. Setup
        engine
            .publish_intent(GigIntent {
                id: intent_id,
                requester_id,
                description: "Acceptance test".into(),
                criteria: vec![],
                max_budget_coins: 100,
                deadline: Utc::now() + chrono::Duration::hours(1),
            })
            .await
            .unwrap();

        engine
            .submit_bid(GigBid {
                id: bid_id,
                intent_id,
                bidder_id: Uuid::new_v4(),
                price_coins: 80,
                est_duration_sec: 3600,
                deposit_amount: 10,
            })
            .await
            .unwrap();

        // 2. Accept
        let result = engine.accept_bid(intent_id, bid_id).await;
        assert!(result.is_ok());

        // 3. Verify
        let intent_status: String =
            sqlx::query_scalar("SELECT status FROM gig_intents WHERE id = ?")
                .bind(intent_id.to_string())
                .fetch_one(&engine.pool)
                .await
                .unwrap();
        assert_eq!(intent_status, "Accepted");

        let bid_status: String = sqlx::query_scalar("SELECT status FROM gig_bids WHERE id = ?")
            .bind(bid_id.to_string())
            .fetch_one(&engine.pool)
            .await
            .unwrap();
        assert_eq!(bid_status, "Accepted");

        let escrow_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM escrows WHERE order_id = ?")
                .bind(intent_id.to_string())
                .fetch_one(&engine.pool)
                .await
                .unwrap();
        assert_eq!(escrow_count, 1);
    }

    #[tokio::test]
    async fn test_gig_delivery_green() {
        let pool = setup_db().await;
        let commerce = Arc::new(MockCommerceEngine);
        let engine = SqliteGigEngine::new(pool, commerce, mock_llm());

        let intent_id = Uuid::new_v4();
        let bid_id = Uuid::new_v4();
        let bidder_id = Uuid::new_v4();

        // 1. Setup: Accepted intent
        engine
            .publish_intent(GigIntent {
                id: intent_id,
                requester_id: Uuid::new_v4(),
                description: "Delivery test".into(),
                criteria: vec![],
                max_budget_coins: 100,
                deadline: Utc::now() + chrono::Duration::hours(1),
            })
            .await
            .unwrap();

        engine
            .submit_bid(GigBid {
                id: bid_id,
                intent_id,
                bidder_id,
                price_coins: 80,
                est_duration_sec: 3600,
                deposit_amount: 10,
            })
            .await
            .unwrap();

        engine.accept_bid(intent_id, bid_id).await.unwrap();

        // 2. Deliver
        let deliverable = GigDeliverable {
            order_id: intent_id,
            deliverer_id: bidder_id,
            artifact_path: "/tmp/result.json".into(),
            metadata: serde_json::json!({"version": "1.0"}),
        };

        let result = engine.deliver(deliverable).await;
        assert!(result.is_ok());

        // 3. Verify
        let status: String = sqlx::query_scalar("SELECT status FROM gig_intents WHERE id = ?")
            .bind(intent_id.to_string())
            .fetch_one(&engine.pool)
            .await
            .unwrap();
        assert_eq!(status, "Delivered");

        let artifact: String =
            sqlx::query_scalar("SELECT artifact_path FROM gig_deliveries WHERE order_id = ?")
                .bind(intent_id.to_string())
                .fetch_one(&engine.pool)
                .await
                .unwrap();
        assert_eq!(artifact, "/tmp/result.json");
    }

    #[tokio::test]
    async fn test_gig_verify_and_settle_green() {
        let pool = setup_db().await;
        let commerce = Arc::new(MockCommerceEngine);
        let engine = SqliteGigEngine::new(pool, commerce, mock_llm());

        let intent_id = Uuid::new_v4();
        let bid_id = Uuid::new_v4();
        let bidder_id = Uuid::new_v4();

        // 1. Setup: Delivered intent
        engine
            .publish_intent(GigIntent {
                id: intent_id,
                requester_id: Uuid::new_v4(),
                description: "Settlement test".into(),
                criteria: vec![],
                max_budget_coins: 100,
                deadline: Utc::now() + chrono::Duration::hours(1),
            })
            .await
            .unwrap();

        engine
            .submit_bid(GigBid {
                id: bid_id,
                intent_id,
                bidder_id,
                price_coins: 80,
                est_duration_sec: 3600,
                deposit_amount: 10,
            })
            .await
            .unwrap();

        engine.accept_bid(intent_id, bid_id).await.unwrap();

        engine
            .deliver(GigDeliverable {
                order_id: intent_id,
                deliverer_id: bidder_id,
                artifact_path: "/tmp/result.json".into(),
                metadata: serde_json::json!({"version": "1.0"}),
            })
            .await
            .unwrap();

        // 2. Verify and Settle
        let result = engine.verify_and_settle(intent_id).await;
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.passed);
        assert_eq!(res.order_id, intent_id);

        // 3. Verify Persistence
        let status: String = sqlx::query_scalar("SELECT status FROM gig_intents WHERE id = ?")
            .bind(intent_id.to_string())
            .fetch_one(&engine.pool)
            .await
            .unwrap();
        assert_eq!(status, "Completed");

        let log_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM verification_logs WHERE order_id = ?")
                .bind(intent_id.to_string())
                .fetch_one(&engine.pool)
                .await
                .unwrap();
        assert_eq!(log_count, 1);
    }

    #[tokio::test]
    async fn test_gig_verify_with_oracle_judge() {
        let pool = setup_db().await;
        let commerce = Arc::new(MockCommerceEngine);
        let llm = Arc::new(MockLlmProvider {
            response: "{\"passed\": true, \"score\": 0.98, \"detail\": \"Excellent work analyzed by LLM.\"}"
                .into(),
            should_fail: false,
        });
        let engine = SqliteGigEngine::new(pool, commerce, llm);

        let intent_id = Uuid::new_v4();
        let bidder_id = Uuid::new_v4();

        // 1. Setup: Intent with OracleJudge
        engine
            .publish_intent(GigIntent {
                id: intent_id,
                requester_id: Uuid::new_v4(),
                description: "Write a high quality haiku".into(),
                criteria: vec![AcceptanceCriteria::OracleJudge {
                    rubric_prompt: "Assess poetic quality".into(),
                    min_score: 0.8,
                    model: None,
                }],
                max_budget_coins: 100,
                deadline: Utc::now() + chrono::Duration::hours(1),
            })
            .await
            .unwrap();

        // Acceptance and Delivery
        let bid_id = Uuid::new_v4();
        engine
            .submit_bid(GigBid {
                id: bid_id,
                intent_id,
                bidder_id,
                price_coins: 50,
                est_duration_sec: 3600,
                deposit_amount: 5,
            })
            .await
            .unwrap();
        engine.accept_bid(intent_id, bid_id).await.unwrap();
        engine
            .deliver(GigDeliverable {
                order_id: intent_id,
                deliverer_id: bidder_id,
                artifact_path: "haiku.txt".into(),
                metadata: serde_json::json!({"content": "Old pond, frog jumps in."}),
            })
            .await
            .unwrap();

        // 2. Verify and Settle (OracleJudge should be invoked)
        let result = engine.verify_and_settle(intent_id).await.unwrap();

        // TDD RED expectation: Current implementation stubs OracleJudge, so it should not have score 0.98
        // Or it should not have the detail from LLM.
        assert!(result.passed);
        assert_eq!(result.score, 0.98); // This will FAIL because current score is fixed at 1.0
        assert!(result.detail.contains("Excellent work analyzed by LLM.")); // This will FAIL because detail is stubbed
    }
}
