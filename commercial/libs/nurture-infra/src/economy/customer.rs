/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_bridge::db::DatabasePool;
use nurture_bridge::error::AiomeError;
use nurture_bridge::{sql_exec, sql_fetch_optional_map};
use nurture_core::customer::CustomerStore;
use sqlx::Row;
use uuid::Uuid;

pub struct SQLiteCustomerStore {
    pool: DatabasePool,
}

impl SQLiteCustomerStore {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomerStore for SQLiteCustomerStore {
    async fn get_actor_id(
        &self,
        stripe_customer_id: &str,
    ) -> Result<Option<ActorId>, NurtureError> {
        let row_opt: Option<String> = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT actor_id FROM nurture_customers WHERE stripe_customer_id = ?",
            |row| Ok::<String, AiomeError>(row.get("actor_id")),
            pg: "SELECT actor_id FROM nurture_customers WHERE stripe_customer_id = $1",
            |row| Ok::<String, AiomeError>(row.get("actor_id")),
            stripe_customer_id
        )
        .map_err(|e| NurtureError::Infrastructure(format!("顧客検索エラー: {}", e)))?;

        match row_opt {
            Some(id_str) => {
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
        sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO nurture_customers (stripe_customer_id, actor_id) VALUES (?, ?)",
            pg: "INSERT INTO nurture_customers (stripe_customer_id, actor_id) VALUES ($1, $2)",
            stripe_customer_id,
            actor_id.0.to_string()
        )
        .map(|_| ())
        .map_err(|e| NurtureError::Infrastructure(format!("顧客紐付けエラー: {}", e)))
    }
}
