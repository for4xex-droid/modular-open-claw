/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#[cfg(test)]
mod tests {
    use crate::db::DatabasePool;
    use crate::soul_store::UniversalSoulStore;
    use aiome_core::error::AiomeError;
    use soul::AgentSoul;

    async fn setup_db() -> DatabasePool {
        let jq = crate::job_queue::UniversalJobQueue::new("sqlite::memory:")
            .await
            .expect("Failed to create in-memory job queue");
        jq.get_pool().clone()
    }

    #[tokio::test]
    async fn test_lora_persistence_roundtrip() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let store = UniversalSoulStore::new(pool);

        let mut soul = AgentSoul::new("test-agent-1".to_string());
        soul.lora_adapter_path = Some("/path/to/adapter".to_string());
        soul.lora_base_model = Some("llama-3-8b".to_string());

        // Save
        store.save_soul(&soul).await?;

        // Load
        let loaded = store
            .load_soul("test-agent-1")
            .await?
            .expect("Soul not found");

        assert_eq!(
            loaded.lora_adapter_path,
            Some("/path/to/adapter".to_string())
        );
        assert_eq!(loaded.lora_base_model, Some("llama-3-8b".to_string()));

        // Update to None
        soul.lora_adapter_path = None;
        store.save_soul(&soul).await?;

        let loaded2 = store
            .load_soul("test-agent-1")
            .await?
            .expect("Soul not found");
        assert_eq!(loaded2.lora_adapter_path, None);
        assert_eq!(loaded2.lora_base_model, Some("llama-3-8b".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_begging_persistence_roundtrip() -> Result<(), AiomeError> {
        use chrono::{TimeZone, Utc};
        let pool = setup_db().await;
        let store = UniversalSoulStore::new(pool);

        let mut soul = AgentSoul::new("test-agent-begging".to_string());
        // SQLite text precision typically ignores nano-seconds in simple ISO strings, so we use s precision
        let begging_time = Utc.with_ymd_and_hms(2026, 3, 21, 10, 30, 0).unwrap();
        soul.last_begging_at = Some(begging_time);

        // Save
        store.save_soul(&soul).await?;

        // Load
        let loaded = store
            .load_soul("test-agent-begging")
            .await?
            .expect("Soul not found");

        assert_eq!(loaded.last_begging_at, Some(begging_time));

        Ok(())
    }

    #[tokio::test]
    async fn test_soul_snapshot_lora_cache_sync() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let store = UniversalSoulStore::new(pool);

        let mut soul = AgentSoul::new("test-cache-lora".to_string());
        soul.lora_hash = Some("sha256:lora789".to_string());
        soul.lora_adapter_path = Some("/path/to/lora789".to_string());
        soul.lora_base_model = Some("sdxl-v1-0".to_string());

        // Save soul (this should update the cache)
        store.save_soul(&soul).await?;

        // Get snapshot from cache
        let snapshot = store
            .get_snapshot()
            .await
            .expect("Snapshot should be in cache");

        assert_eq!(snapshot.lora_hash, Some("sha256:lora789".to_string()));
        assert_eq!(
            snapshot.lora_adapter_path,
            Some("/path/to/lora789".to_string())
        );
        assert_eq!(snapshot.lora_base_model, Some("sdxl-v1-0".to_string()));

        Ok(())
    }
}
