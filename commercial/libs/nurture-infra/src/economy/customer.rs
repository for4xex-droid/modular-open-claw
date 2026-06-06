/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_core::customer::CustomerStore;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub struct SQLiteCustomerStore {
    pool: SqlitePool,
}

impl SQLiteCustomerStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomerStore for SQLiteCustomerStore {
    async fn get_actor_id(
        &self,
        stripe_customer_id: &str,
    ) -> Result<Option<ActorId>, NurtureError> {
        let row =
            sqlx::query("SELECT actor_id FROM nurture_customers WHERE stripe_customer_id = ?")
                .bind(stripe_customer_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| NurtureError::Infrastructure(format!("顧客検索エラー: {}", e)))?;

        match row {
            Some(row) => {
                let id_str: String = row.get("actor_id");
                let uuid = Uuid::parse_str(&id_str).map_err(|e| {
                    NurtureError::Infrastructure(format!("ActorId パースエラー: {}", e))
                })?;
                Ok(Some(ActorId(uuid)))
            }
            None => Ok(None),
        }
    }

    async fn link_customer(
        &self,
        stripe_customer_id: &str,
        actor_id: &ActorId,
    ) -> Result<(), NurtureError> {
        sqlx::query("INSERT INTO nurture_customers (stripe_customer_id, actor_id) VALUES (?, ?)")
            .bind(stripe_customer_id)
            .bind(actor_id.0.to_string())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| NurtureError::Infrastructure(format!("顧客紐付けエラー: {}", e)))
    }
}
