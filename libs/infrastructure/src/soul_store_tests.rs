/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {
    use crate::db::DatabasePool;
    use crate::soul_store::UniversalSoulStore;
    use aiome_core::error::AiomeError;
    use aiome_core_contracts::traits::SoulStore;
    use soul::AgentSoul;

    async fn setup_db() -> DatabasePool {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let _jq = crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .expect("Failed to create in-memory job queue");
        pool
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

    #[tokio::test]
    async fn test_archive_lora_model_on_rebirth() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        // In order for this test to pass we will need to ensure the schema includes `archived_lora_models` table.
        let store = UniversalSoulStore::new(pool);

        let mut soul = AgentSoul::new("test-archive-lora".to_string());
        soul.generation = 1;
        soul.lora_hash = Some("sha256:legacy_model".to_string());
        soul.lora_adapter_path = Some("/vault/lora/legacy_1".to_string());
        soul.lora_base_model = Some("mlx_lm.llama".to_string());

        // Save active soul initially
        store.save_soul(&soul).await?;

        // Archive it
        store
            .archive_lora_model(
                &soul.id,
                soul.generation,
                soul.lora_hash.as_ref().unwrap(),
                soul.lora_adapter_path.as_ref().unwrap(),
                soul.lora_base_model.as_ref().unwrap(),
            )
            .await?;

        // Verify we can fetch it back
        let archived = store.get_archived_lora_models("test-archive-lora").await?;
        assert_eq!(archived.len(), 1);
        let record = &archived[0];
        assert_eq!(record.generation, 1);
        assert_eq!(record.lora_hash, "sha256:legacy_model");
        assert_eq!(record.adapter_path, "/vault/lora/legacy_1");
        assert_eq!(record.base_model, "mlx_lm.llama");

        Ok(())
    }

    #[tokio::test]
    async fn test_frozen_traits_persistence_roundtrip() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let store = UniversalSoulStore::new(pool);

        let mut soul = AgentSoul::new("test-agent-frozen".to_string());

        let marker = soul::SomaticMarker {
            id: "m_permanent".to_string(),
            embedding: vec![0.1, 0.2],
            valence: 0.8,
            arousal: 0.5,
            intensity: 0.9,
            created_at: "2026-06-10T00:00:00Z".to_string(),
            is_permanent: true,
        };
        soul.somatic_markers.push(marker);

        let ft = soul::FrozenTraitSnapshot {
            trait_index: 3,
            frozen_value: 0.75,
            somatic_marker_id: "m_permanent".to_string(),
            frozen_at_generation: 2,
            created_at: "2026-06-10T00:00:00Z".to_string(),
        };
        soul.frozen_traits.push(ft.clone());

        // Save
        store.save_soul(&soul).await?;

        // Load
        let loaded = store
            .load_soul("test-agent-frozen")
            .await?
            .expect("Soul not found");

        assert_eq!(loaded.somatic_markers.len(), 1);
        assert!(loaded.somatic_markers[0].is_permanent);
        assert_eq!(loaded.somatic_markers[0].id, "m_permanent");

        assert_eq!(loaded.frozen_traits.len(), 1);
        assert_eq!(loaded.frozen_traits[0], ft);

        Ok(())
    }
}
