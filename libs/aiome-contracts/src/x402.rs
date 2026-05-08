/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#[doc(hidden)]
/// Phase 3: Migrated to alloy_primitives::U256
pub use alloy_primitives::U256;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentProof {
    pub transaction_hash: String,
}

#[async_trait]
pub trait X402Negotiator: Send + Sync {
    /// HTTP 402 レスポンスを解釈し、自動決済を試みる
    async fn negotiate(&self, response: &reqwest::Response) -> Result<PaymentProof>;

    /// ウォレット残高を照会する
    async fn balance(&self) -> Result<U256>;
}
