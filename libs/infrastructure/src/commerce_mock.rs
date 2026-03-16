/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use aiome_core::commerce::{CommerceEngine, EconomicContext};
use async_trait::async_trait;
use uuid::Uuid;

/// OSS 版向けのモック経済エンジン
pub struct MockCommerceEngine;

#[async_trait]
impl CommerceEngine for MockCommerceEngine {
    async fn get_balance(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(1000) // 常に 1000 コイン
    }

    async fn validate_activity(
        &self,
        _agent_id: Uuid,
        _activity_type: &str,
        _amount: u64,
    ) -> Result<(), AiomeError> {
        Ok(()) // 常に許可
    }

    async fn execute_autonomous_purchase(
        &self,
        _agent_id: Uuid,
        _item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        Ok(Uuid::new_v4().to_string()) // ダミーのトランザクションID
    }
}
