/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#[cfg(test)]
mod tests {
    use crate::commerce_mock::MockCommerceEngine;
    use crate::db::DatabasePool;
    use crate::gig_engine::*;
    use aiome_contracts::{GigEngine, LlmResponse};
    use aiome_core::llm_provider::LlmProvider;
    use sqlx::Row;
    use std::sync::Arc;
    use uuid::Uuid;

    #[derive(Debug)]
    struct DummyLlm;
    #[async_trait::async_trait]
    impl LlmProvider for DummyLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<LlmResponse, aiome_core::error::AiomeError> {
            Ok(LlmResponse {
                content: "pass".into(),
                stop_reason: aiome_contracts::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn complete_with_cache(
            &self,
            _req: aiome_contracts::llm::LlmRequest,
        ) -> Result<LlmResponse, aiome_core::error::AiomeError> {
            self.complete("", None).await
        }
        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "dummy"
        }
        async fn stream_complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn tokio_stream::Stream<Item = Result<String, aiome_core::error::AiomeError>>
                        + Send,
                >,
            >,
            aiome_core::error::AiomeError,
        > {
            unimplemented!()
        }
    }

    async fn setup_gig_db() -> DatabasePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query("CREATE TABLE gig_intents (id TEXT PRIMARY KEY, requester_id TEXT, status TEXT, criteria TEXT)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE gig_bids (id TEXT PRIMARY KEY, intent_id TEXT, bidder_id TEXT, price_coins INTEGER, status TEXT)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE escrows (id TEXT PRIMARY KEY, payer_id TEXT, recipient_id TEXT, order_id TEXT, amount INTEGER, status TEXT)").execute(&pool).await.unwrap();

        DatabasePool::Sqlite(pool)
    }

    #[tokio::test]
    async fn test_accept_bid_race_condition_red() {
        let pool = setup_gig_db().await;
        let commerce = Arc::new(MockCommerceEngine::new()); // Simple mock
        let llm = Arc::new(DummyLlm);
        let engine = UniversalGigEngine::new(
            pool.clone(),
            commerce,
            llm,
            std::path::PathBuf::from("/tmp"),
        );

        let intent_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let bidder_id = Uuid::new_v4();
        let bid_id = Uuid::new_v4();

        // Seed data
        sqlx::query("INSERT INTO gig_intents (id, requester_id, status, criteria) VALUES (?, ?, 'Open', '{}')")
            .bind(intent_id.to_string()).bind(requester_id.to_string()).execute(pool.get_sqlite_pool_or_err().unwrap()).await.unwrap();
        sqlx::query("INSERT INTO gig_bids (id, intent_id, bidder_id, price_coins, status) VALUES (?, ?, ?, 100, 'Open')")
            .bind(bid_id.to_string()).bind(intent_id.to_string()).bind(bidder_id.to_string()).execute(pool.get_sqlite_pool_or_err().unwrap()).await.unwrap();

        // Simulate 2 threads accepting the SAME bid
        let engine = Arc::new(engine);
        let e1 = engine.clone();
        let e2 = engine.clone();

        let h1 = tokio::spawn(async move { e1.accept_bid(intent_id, bid_id).await });
        let h2 = tokio::spawn(async move { e2.accept_bid(intent_id, bid_id).await });

        let r1: Result<(), aiome_core::error::AiomeError> = h1.await.unwrap();
        let r2: Result<(), aiome_core::error::AiomeError> = h2.await.unwrap();

        // RED: In current implementation, BOTH might succeed (Ok) if they race.
        // We WANT one to succeed and one to FAIL.
        let results = vec![r1.is_ok(), r2.is_ok()];
        let success_count = results.iter().filter(|&&x| x).count();

        // If success_count > 1, the race condition is confirmed.
        assert_eq!(
            success_count, 1,
            "Exactly one acceptance should succeed. Results: {:?}",
            results
        );
    }
}
