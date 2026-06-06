/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;

/// Stripe 顧客 ID とシステム ActorId の紐付けを管理するストア
#[async_trait]
pub trait CustomerStore: Send + Sync {
    /// Stripe 顧客 ID から ActorId を取得する
    async fn get_actor_id(&self, stripe_customer_id: &str)
        -> Result<Option<ActorId>, NurtureError>;

    /// Stripe 顧客 ID と ActorId を紐付ける
    async fn link_customer(
        &self,
        stripe_customer_id: &str,
        actor_id: &ActorId,
    ) -> Result<(), NurtureError>;
}
