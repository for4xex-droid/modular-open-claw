use commerce_protocol::identity::ActorId;
use nurture_api::state::{AppState, EconomyPolicy};
use nurture_bridge::traits::JobQueue;
use nurture_infra::storage::MockAssetStorage;
use secrecy::SecretString;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
async fn test_db_initialized_in_wal_mode() {
    // Arrange
    let tdir = tempdir().unwrap();
    let db_path = tdir.path().join("test_nurture.db");
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        // Note: we intentionally do NOT set journal_mode=WAL here
        // so we can test that `AppState::init` sets it.
        ;
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let store = Arc::new(
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::new(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
        ),
    ) as Arc<dyn nurture_bridge::trajectory::TrajectoryStore>;
    let job_queue: Arc<dyn JobQueue> =
        Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            store,
        ));
    let policy = EconomyPolicy::default();
    let system_id = ActorId(Uuid::new_v4());
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let internal_secret = SecretString::from("test_internal_secret".to_string());

    let drm_master_key = SecretString::from("test_drm_master_key".to_string());
    let asset_storage = Arc::new(MockAssetStorage::new());

    // Act
    let state = AppState::init(
        pool.clone(),
        job_queue,
        policy,
        system_id,
        cancel_token,
        internal_secret,
        None,
        None,
        std::sync::Arc::new(nurture_bridge::auth::MockAuthManager::new()),
        drm_master_key,
        asset_storage,
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .expect("Failed to initialize AppState");

    // Assert
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&state.pool)
        .await
        .expect("Failed to query journal_mode");

    assert_eq!(
        journal_mode.to_lowercase(),
        "wal",
        "Database should be in WAL mode"
    );

    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&state.pool)
        .await
        .expect("Failed to query busy_timeout");

    assert_eq!(busy_timeout, 5000, "Database busy_timeout should be 5000ms");
}
