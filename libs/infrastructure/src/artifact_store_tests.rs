/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#[cfg(test)]
mod tests {
    use crate::artifact_store::UniversalArtifactStore;
    use crate::db::DatabasePool;
    use aiome_core_contracts::traits::{ArtifactCategory, ArtifactStore, CreateArtifactRequest};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_artifact_protected_isolation() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();

        // Setup table for test
        if let DatabasePool::Sqlite(p) = &pool {
            sqlx::query(
                "CREATE TABLE ai_artifacts (
                    id TEXT PRIMARY KEY,
                    title TEXT,
                    category TEXT,
                    tags TEXT,
                    created_by TEXT,
                    dir_path TEXT,
                    file_manifest TEXT,
                    karma_refs TEXT,
                    job_ref TEXT,
                    signature TEXT,
                    soul_version_hash TEXT,
                    embedding BLOB,
                    text_content TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )",
            )
            .execute(p)
            .await
            .unwrap();
        }

        let base_temp = tempdir().unwrap();
        let vault_temp = tempdir().unwrap();

        let base_dir = base_temp.path().to_path_buf();
        let vault_dir = vault_temp.path().to_path_buf();

        // UniversalArtifactStore に vault_path を持たせる必要がある (まだ実装前)
        let store =
            UniversalArtifactStore::new(pool, base_dir.clone()).with_vault(vault_dir.clone()); // このメソッドを後で実装する

        let req = CreateArtifactRequest {
            title: "Secret Strategy".to_string(),
            category: ArtifactCategory::Knowledge,
            tags: vec!["internal".to_string()],
            created_by: "system".to_string(),
            files: vec![(
                "strategy.md".to_string(),
                b"TOP SECRET".to_vec(),
                "text/markdown".to_string(),
            )],
            karma_refs: vec![],
            text_content: Some("TOP SECRET".to_string()),
            job_ref: None,
            parent_refs: vec![],
            is_protected: true, // これが true なら vault_dir に保存されるべき
        };

        let root_temp = tempdir().unwrap();
        let jail = bastion::fs_guard::Jail::init(root_temp.path().to_path_buf()).unwrap();

        let _id = store.save_artifact(req, &jail).await.unwrap();

        // 検証: ファイルが vault_dir の中にあるか？
        let artifacts_in_vault = vault_dir.join("artifacts");
        assert!(
            artifacts_in_vault.exists(),
            "Protected artifact should be stored in the vault directory (artifacts subdirectory)"
        );

        // base_dir には無いことを確認
        let artifacts_in_base = base_dir.join("artifacts");
        assert!(
            !artifacts_in_base.exists(),
            "Protected artifact should NOT be stored in the base directory"
        );
    }

    #[tokio::test]
    async fn test_save_artifact_file_size_limit() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        let base_temp = tempdir().unwrap();
        let store = UniversalArtifactStore::new(pool, base_temp.path().to_path_buf());

        // 11MB file (limit is 10MB)
        let large_content = vec![0u8; 11 * 1024 * 1024];
        let req = CreateArtifactRequest {
            title: "Large File".to_string(),
            category: ArtifactCategory::Report,
            tags: vec![],
            created_by: "tester".to_string(),
            files: vec![(
                "large.bin".to_string(),
                large_content,
                "application/octet-stream".to_string(),
            )],
            karma_refs: vec![],
            text_content: None,
            job_ref: None,
            parent_refs: vec![],
            is_protected: false,
        };

        let jail_dir = tempdir().unwrap();
        let jail = bastion::fs_guard::Jail::init(jail_dir.path().to_path_buf()).unwrap();

        let result = store.save_artifact(req, &jail).await;
        assert!(result.is_err(), "Should fail for files larger than 10MB");
        if let Err(aiome_core_contracts::error::AiomeError::Infrastructure { reason }) = result {
            assert!(
                reason.contains("File size limit exceeded"),
                "Error message should mention limit"
            );
        } else {
            panic!("Expected Infrastructure error for size limit");
        }
    }

    #[tokio::test]
    async fn test_save_artifact_file_count_limit() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        let base_temp = tempdir().unwrap();
        let store = UniversalArtifactStore::new(pool, base_temp.path().to_path_buf());

        // 21 files (limit is 20)
        let mut files = Vec::new();
        for i in 0..21 {
            files.push((
                format!("file{}.txt", i),
                b"content".to_vec(),
                "text/plain".to_string(),
            ));
        }

        let req = CreateArtifactRequest {
            title: "Too Many Files".to_string(),
            category: ArtifactCategory::Report,
            tags: vec![],
            created_by: "tester".to_string(),
            files,
            karma_refs: vec![],
            text_content: None,
            job_ref: None,
            parent_refs: vec![],
            is_protected: false,
        };

        let jail_dir = tempdir().unwrap();
        let jail = bastion::fs_guard::Jail::init(jail_dir.path().to_path_buf()).unwrap();

        let result = store.save_artifact(req, &jail).await;
        assert!(result.is_err(), "Should fail for more than 20 files");
    }

    #[tokio::test]
    async fn test_list_artifacts_pagination_clamp() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        // Setup table
        if let DatabasePool::Sqlite(p) = &pool {
            sqlx::query("CREATE TABLE ai_artifacts (id TEXT, title TEXT, category TEXT, tags TEXT, created_by TEXT, dir_path TEXT, file_manifest TEXT, karma_refs TEXT, job_ref TEXT, signature TEXT, soul_version_hash TEXT, embedding BLOB, text_content TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)").execute(p).await.unwrap();

            // Insert 120 dummy records
            for i in 0..120 {
                sqlx::query("INSERT INTO ai_artifacts (id, title, category, tags, created_by, dir_path, file_manifest, karma_refs, job_ref, signature, soul_version_hash) VALUES (?, ?, 'Report', '[]', 'tester', 'path', '[]', '[]', 'job', 'sig', 'hash')")
                    .bind(format!("id{}", i))
                    .bind(format!("title{}", i))
                    .execute(p)
                    .await
                    .unwrap();
            }
        }

        let base_temp = tempdir().unwrap();
        let store = UniversalArtifactStore::new(pool, base_temp.path().to_path_buf());

        // Request with huge limit
        let result = store.list_artifacts(None, 999999).await.unwrap();

        // Should be clamped to 100
        assert_eq!(result.len(), 100, "Should be clamped to 100 records");
    }

    #[tokio::test]
    async fn test_save_artifact_enqueues_csam_scan() {
        use aiome_core_contracts::error::AiomeError;
        use aiome_core_contracts::traits::TaskRegistry;
        use async_trait::async_trait;
        use std::sync::Arc;
        use std::sync::Mutex;
        use uuid::Uuid;

        #[derive(Default, Debug)]
        struct MockTracker {
            enqueued_jobs: Mutex<Vec<(String, String)>>, // (category, topic)
        }

        #[async_trait]
        impl TaskRegistry for MockTracker {
            async fn enqueue(
                &self,
                category: &str,
                topic: &str,
                _style: &str,
                _karma_directives: Option<&str>,
                _permission_manifest: Option<aiome_core_contracts::security::PermissionManifest>,
                _agent_id: Option<Uuid>,
                _priority: i32,
            ) -> Result<String, AiomeError> {
                self.enqueued_jobs
                    .lock()
                    .unwrap()
                    .push((category.to_string(), topic.to_string()));
                Ok("job-id".to_string())
            }
            async fn dequeue(
                &self,
                _categories: &[&str],
            ) -> Result<Option<aiome_core_contracts::traits::Job>, AiomeError> {
                Ok(None)
            }
            async fn fetch_job(
                &self,
                _: &str,
            ) -> Result<Option<aiome_core_contracts::traits::Job>, AiomeError> {
                Ok(None)
            }
            async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn requeue_job(&self, _: &str) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn cancel_job(&self, _: &str) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn update_job_status(
                &self,
                _: &str,
                _: aiome_core_contracts::traits::JobStatus,
            ) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
                Ok(0)
            }
            async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
                Ok(0)
            }
            async fn get_job_count_since(
                &self,
                _: chrono::DateTime<chrono::Utc>,
            ) -> Result<i64, AiomeError> {
                Ok(0)
            }
            async fn fetch_recent_jobs(
                &self,
                _: i64,
            ) -> Result<Vec<aiome_core_contracts::traits::Job>, AiomeError> {
                Ok(vec![])
            }
            async fn fetch_top_performing_jobs(
                &self,
                _: i64,
            ) -> Result<Vec<aiome_core_contracts::traits::Job>, AiomeError> {
                Ok(vec![])
            }
            async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
                Ok(0)
            }
            async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
                Ok(true)
            }
            async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
                Ok(0)
            }
            async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let pool = crate::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        if let crate::db::DatabasePool::Sqlite(p) = &pool {
            sqlx::query("CREATE TABLE ai_artifacts (id TEXT PRIMARY KEY, title TEXT, category TEXT, tags TEXT, created_by TEXT, dir_path TEXT, file_manifest TEXT, karma_refs TEXT, job_ref TEXT, signature TEXT, soul_version_hash TEXT, embedding BLOB, text_content TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)")
                .execute(p)
                .await
                .unwrap();
        }

        let base_temp = tempdir().unwrap();
        let tracker = Arc::new(MockTracker::default());

        let store = UniversalArtifactStore::new(pool, base_temp.path().to_path_buf())
            .with_job_queue(tracker.clone());

        let req = CreateArtifactRequest {
            title: "Test Image".to_string(),
            category: ArtifactCategory::Image,
            tags: vec![],
            created_by: "system".to_string(),
            files: vec![(
                "image.png".to_string(),
                b"data".to_vec(),
                "image/png".to_string(),
            )],
            karma_refs: vec![],
            text_content: None,
            job_ref: None,
            parent_refs: vec![],
            is_protected: false,
        };

        let root_temp = tempdir().unwrap();
        let jail = bastion::fs_guard::Jail::init(root_temp.path().to_path_buf()).unwrap();

        let id = store.save_artifact(req, &jail).await.unwrap();

        let jobs = tracker.enqueued_jobs.lock().unwrap();
        assert_eq!(jobs.len(), 1, "Should enqueue exactly one job");
        assert_eq!(jobs[0].0, "csam_scan", "Job category should be csam_scan");
        assert_eq!(jobs[0].1, id, "Job topic should be artifact_id");
    }
}
