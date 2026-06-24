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
    sqlx::migrate!("../../migrations/sqlite")
        .run(&pool)
        .await
        .unwrap();

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
        nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
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
        .fetch_one(state.pool.get_sqlite_pool().unwrap())
        .await
        .expect("Failed to query journal_mode");

    assert_eq!(
        journal_mode.to_lowercase(),
        "wal",
        "Database should be in WAL mode"
    );

    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(state.pool.get_sqlite_pool().unwrap())
        .await
        .expect("Failed to query busy_timeout");

    assert_eq!(busy_timeout, 5000, "Database busy_timeout should be 5000ms");
}

#[tokio::test]
async fn test_postgres_pool_migration_and_basic_ops() {
    use nurture_bridge::db::DatabasePool;

    let pg_url = std::env::var("TEST_PG_URL").unwrap_or_else(|_| {
        "postgres://aiome_test_user:aiome_test_password@localhost:5433/aiome_test_db".to_string()
    });

    let pool = match DatabasePool::new_postgres(&pg_url).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping Postgres test: connection failed: {:?}", e);
            return;
        }
    };

    if let DatabasePool::Postgres(pg_pool) = &pool {
        sqlx::migrate!("../../migrations/postgres")
            .run(pg_pool)
            .await
            .expect("PostgreSQL migration failed");
    }

    let actor_id = Uuid::new_v4().to_string();

    // Clean up if somehow remaining
    let _ = nurture_bridge::sql_exec!(
        &pool,
        sqlite: "DELETE FROM nurture_wallets WHERE actor_id = ?",
        pg: "DELETE FROM nurture_wallets WHERE actor_id = $1",
        &actor_id
    );

    // Act: Insert (Positive Test)
    let insert_res = nurture_bridge::sql_exec!(
        &pool,
        sqlite: "INSERT INTO nurture_wallets (actor_id, balance, version, last_reset) VALUES (?, 100, 1, CURRENT_TIMESTAMP)",
        pg: "INSERT INTO nurture_wallets (actor_id, balance, version, last_reset) VALUES ($1, 100, 1, CURRENT_TIMESTAMP)",
        &actor_id
    );
    assert!(
        insert_res.is_ok(),
        "PostgreSQL insert failed: {:?}",
        insert_res.err()
    );

    // Act: Fetch via EconomyLedger (Positive Test)
    use nurture_core::ledger::EconomyLedger;
    use nurture_infra::economy::ledger::DatabaseEconomyLedger;

    let ledger = DatabaseEconomyLedger::new(pool.clone());
    let actor = commerce_protocol::identity::ActorId(Uuid::parse_str(&actor_id).unwrap());

    let wallet_res = ledger.get_balance(&actor).await;
    assert!(
        wallet_res.is_ok(),
        "Failed to get balance via ledger: {:?}",
        wallet_res.err()
    );
    let wallet = wallet_res.unwrap();
    assert_eq!(wallet.coin.balance, 100);
}

#[tokio::test]
async fn test_postgres_negative_violation() {
    use nurture_bridge::db::DatabasePool;

    let pg_url = std::env::var("TEST_PG_URL").unwrap_or_else(|_| {
        "postgres://aiome_test_user:aiome_test_password@localhost:5433/aiome_test_db".to_string()
    });

    let pool = match DatabasePool::new_postgres(&pg_url).await {
        Ok(p) => p,
        Err(_) => return, // Skip
    };

    if let DatabasePool::Postgres(pg_pool) = &pool {
        let _ = sqlx::migrate!("../../migrations/postgres")
            .run(pg_pool)
            .await;
    }

    let actor_id = Uuid::new_v4().to_string();

    // First insert should succeed
    let insert1 = nurture_bridge::sql_exec!(
        &pool,
        sqlite: "INSERT INTO nurture_wallets (actor_id, balance, version, last_reset) VALUES (?, 100, 1, CURRENT_TIMESTAMP)",
        pg: "INSERT INTO nurture_wallets (actor_id, balance, version, last_reset) VALUES ($1, 100, 1, CURRENT_TIMESTAMP)",
        &actor_id
    );
    assert!(insert1.is_ok());

    // Second insert with duplicate actor_id (Negative Test: Unique constraint violation)
    let insert2 = nurture_bridge::sql_exec!(
        &pool,
        sqlite: "INSERT INTO nurture_wallets (actor_id, balance, version, last_reset) VALUES (?, 200, 1, CURRENT_TIMESTAMP)",
        pg: "INSERT INTO nurture_wallets (actor_id, balance, version, last_reset) VALUES ($1, 200, 1, CURRENT_TIMESTAMP)",
        &actor_id
    );
    assert!(
        insert2.is_err(),
        "Duplicate insert should fail under unique constraint"
    );
}
