#[cfg(test)]
mod tests {
    use crate::artifact_store::UniversalArtifactStore;
    use crate::db::DatabasePool;
    use aiome_contracts::traits::{ArtifactCategory, ArtifactStore, CreateArtifactRequest};
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
}
