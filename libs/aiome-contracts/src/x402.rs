/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use reqwest;
use alloy_primitives::U256;
use async_trait::async_trait;
use thiserror::Error;
use anyhow::Result;
use serde::{Deserialize, Serialize};

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
