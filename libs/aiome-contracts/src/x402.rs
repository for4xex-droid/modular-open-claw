/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#[doc(hidden)]
/// Phase 3: Migrated to alloy_primitives::U256
pub use alloy_primitives::U256;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AiomeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentProof {
    /// Historical field name. For OP-083-C this holds an EIP-191 signature hex
    /// (`0x…`), not an on-chain transaction hash (broadcast is out of scope).
    pub transaction_hash: String,
}

#[async_trait]
pub trait X402Negotiator: Send + Sync {
    /// HTTP 402 レスポンスを解釈し、自動決済を試みる
    async fn negotiate(&self, response: &reqwest::Response) -> Result<PaymentProof, AiomeError>;

    /// ウォレット残高を照会する
    async fn balance(&self) -> Result<U256, AiomeError>;
}
