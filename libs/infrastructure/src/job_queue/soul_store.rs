/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::UniversalJobQueue;
use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::SoulStore;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
pub trait SoulStoreOps {
    async fn do_load_soul(&self, id: &str) -> Result<Option<serde_json::Value>, AiomeError>;
    async fn do_store_soul_fragment(
        &self,
        fragment_yaml: &str,
        version_hash: &str,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError>;
}

#[async_trait]
impl SoulStore for UniversalJobQueue {
    async fn load_soul(&self, id: &str) -> Result<Option<serde_json::Value>, AiomeError> {
        self.do_load_soul(id).await
    }

    async fn store_soul_fragment(
        &self,
        fragment_yaml: &str,
        version_hash: &str,
    ) -> Result<(), AiomeError> {
        self.do_store_soul_fragment(fragment_yaml, version_hash)
            .await
    }

    async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
        self.do_fetch_latest_soul_fragment().await
    }
}

#[async_trait]
impl SoulStoreOps for UniversalJobQueue {
    async fn do_load_soul(&self, id: &str) -> Result<Option<serde_json::Value>, AiomeError> {
        let q = format!("SELECT data_json FROM souls WHERE id = {}", self.pool.ph(0));
        let opt = crate::sql_fetch_optional!(&self.pool, (String,), &q, id).unwrap_or(None);
        if let Some((s,)) = opt {
            Ok(Some(
                serde_json::from_str(&s).unwrap_or(serde_json::Value::Null),
            ))
        } else {
            Ok(None)
        }
    }

    async fn do_store_soul_fragment(
        &self,
        fragment_yaml: &str,
        version_hash: &str,
    ) -> Result<(), AiomeError> {
        let now = chrono::Utc::now().to_rfc3339();
        let q = format!("INSERT INTO soul_fragments (version_hash, fragment_yaml, created_at) VALUES ({}, {}, {})", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2));
        sql_exec!(&self.pool, &q, version_hash, fragment_yaml, &now).map_err(|e| {
            AiomeError::Infrastructure {
                reason: e.to_string(),
            }
        })?;
        Ok(())
    }

    async fn do_fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
        let q = "SELECT version_hash, fragment_yaml FROM soul_fragments ORDER BY created_at DESC LIMIT 1";
        let opt = crate::sql_fetch_optional!(&self.pool, (String, String), q).unwrap_or(None);
        Ok(opt)
    }
}
