/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use axum_test::TestServer;
use chrono::Utc;
use commerce_protocol::identity::ActorId;
use commerce_protocol::mcp_commerce::BuyRequest;
use nurture_api::routes::nurture_routes;
use nurture_api::state::AppState;
use nurture_bridge::auth::MockAuthManager;
use serial_test::serial;
use sqlx::SqlitePool;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_full_buy_flow_e2e() {
    // Setup Auth
    std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_test");

    let tdir = tempdir().unwrap();
    let db_path = tdir.path().join("test_nurture.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    // Initial data: Buyer with 1000 coins, Item priced at 100
    let buyer_id = Uuid::new_v4();
    let seller_id = Uuid::new_v4();
    let item_id = Uuid::new_v4();
    let system_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(buyer_id.to_string())
        .bind(1000i64)
        .bind(1000i64)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
        .bind(seller_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_items (id, kind, name, description, price_coins, creator_id, metadata, created_at, sale_mode, drm_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(item_id.to_string())
        .bind("VrmAvatar")
        .bind("Test Avatar")
        .bind("A test item")
        .bind(100i64)
        .bind(seller_id.to_string())
        .bind(format!("{{\"actor_id\":\"{}\"}}", seller_id))
        .bind(Utc::now())
        .bind("Instant")
        .bind(1i32) // DRM Enable
        .execute(&pool).await.unwrap();

    let token = format!("mock_valid_token_{}", buyer_id);

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let db_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());
    let store = std::sync::Arc::new(
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&db_url)
            .await
            .unwrap(),
    ) as std::sync::Arc<dyn nurture_bridge::trajectory::TrajectoryStore>;
    let job_queue: std::sync::Arc<dyn nurture_bridge::traits::JobQueue> =
        std::sync::Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            store,
        ));

    // Setup state and app
    let state = AppState::init(
        pool.clone(),
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        ActorId(system_id),
        cancel_token,
        "test_nurture_secret".to_string().into(),
        None,
        None,
        std::sync::Arc::new(MockAuthManager::new()),
        "test_drm_master_key".to_string().into(),
        std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new()),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();
    let app = nurture_routes(state);
    let server = TestServer::new(app).unwrap();

    // Execute Buy
    let buy_req = BuyRequest {
        buyer: ActorId(buyer_id),
        item_id,
        idempotency_key: Some(format!("e2e-test-{}", Uuid::new_v4())),
        use_escrow: None,
    };

    let response = server
        .post("/marketplace/buy") // Path fixed (TestServer directly uses nurture_routes)
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&buy_req)
        .await;

    response.assert_status_success();

    let buy_res: commerce_protocol::mcp_commerce::BuyResponse = response.json();
    assert!(buy_res.license_id.is_some());

    // Verify License in DB
    let license_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM nurture_licenses WHERE id = ?)")
            .bind(buy_res.license_id.unwrap().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(license_exists);

    // Verify DB State
    // Buyer balance: 1000 - 100 = 900
    let buyer_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(buyer_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(buyer_balance, 900);

    // Seller balance: 100 - burn(5%) - fee(30% of 95) = 100 - 5 - 28 = 67
    let seller_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(seller_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seller_balance, 67);

    // System balance: 28 (System Fee)
    let system_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(system_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(system_balance, 28);

    // Burn balance (nil Uuid): 5
    let burn_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(Uuid::nil().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(burn_balance, 5);

    // Seller points: 70% of 100 = 70 (Policy points rate is against original price)
    let seller_points: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_points WHERE actor_id = ?")
            .bind(seller_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seller_points, 70);
}

#[tokio::test]
#[serial]
async fn test_idempotent_buy_flow() {
    // [Reflexion Sprint A v2] 回帰テスト: 冪等性処理（unwrap_or_defaultによる空文字列バグ修正）

    let tdir = tempdir().unwrap();
    let db_path = tdir.path().join("test_idemp.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let buyer_id = Uuid::new_v4();
    let seller_id = Uuid::new_v4();
    let item_id = Uuid::new_v4();
    let system_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(buyer_id.to_string())
        .bind(1000i64)
        .bind(1000i64)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
        .bind(seller_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_items (id, kind, name, description, price_coins, creator_id, metadata, created_at, sale_mode, drm_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(item_id.to_string())
        .bind("VrmAvatar")
        .bind("Test Avatar")
        .bind("A test item")
        .bind(100i64)
        .bind(seller_id.to_string())
        .bind(format!("{{\"actor_id\":\"{}\"}}", seller_id))
        .bind(Utc::now())
        .bind("Instant")
        .bind(0i32)
        .execute(&pool).await.unwrap();

    let token = format!("mock_valid_token_{}", buyer_id);

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let db_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());
    let store = std::sync::Arc::new(
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&db_url)
            .await
            .unwrap(),
    ) as std::sync::Arc<dyn nurture_bridge::trajectory::TrajectoryStore>;
    let job_queue: std::sync::Arc<dyn nurture_bridge::traits::JobQueue> =
        std::sync::Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            store,
        ));

    let state = AppState::init(
        pool.clone(),
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        ActorId(system_id),
        cancel_token,
        "test_nurture_secret".to_string().into(),
        None,
        None,
        std::sync::Arc::new(MockAuthManager::new()),
        "test_drm_master_key".to_string().into(),
        std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new()),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();

    let app = nurture_routes(state);
    let server = TestServer::new(app).unwrap();

    let idem_key = format!("test-idemp-{}", Uuid::new_v4());
    let buy_req = BuyRequest {
        buyer: ActorId(buyer_id),
        item_id,
        idempotency_key: Some(idem_key.clone()),
        use_escrow: None,
    };

    // 初回リクエスト
    let res1 = server
        .post("/marketplace/buy")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&buy_req)
        .await;

    res1.assert_status_success();
    let txt1 = res1.text();

    // 2回目リクエスト。同じレスポンスを返すはずであり、コインが2回引かれるべきではない。
    let res2 = server
        .post("/marketplace/buy")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&buy_req)
        .await;

    res2.assert_status_success();
    let txt2 = res2.text();

    assert_eq!(txt1, txt2, "Idempotency responses must match exactly");

    // 引かれたコインの量が1回分 (100) であることを確認 (1000 -> 900)
    let buyer_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(buyer_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(buyer_balance, 900, "Balance should only be deducted once");
}

#[tokio::test]
#[serial]
async fn test_buy_flow_with_escrow() {
    let tdir = tempdir().unwrap();
    let db_path = tdir.path().join("test_nurture_escrow.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let buyer_id = Uuid::new_v4();
    let seller_id = Uuid::new_v4();
    let item_id = Uuid::new_v4();
    let system_id = Uuid::new_v4();

    // Buyer balance: 1000, Item price: 300
    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(buyer_id.to_string())
        .bind(1000i64)
        .bind(1000i64)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(seller_id.to_string())
        .bind(0i64)
        .bind(1000i64)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
        .bind(seller_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_items (id, kind, name, description, price_coins, creator_id, metadata, created_at, sale_mode, drm_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(item_id.to_string())
        .bind("VrmAvatar")
        .bind("Test Escrow Avatar")
        .bind("A test item for escrow")
        .bind(300i64)
        .bind(seller_id.to_string())
        .bind(format!("{{\"actor_id\":\"{}\"}}", seller_id))
        .bind(Utc::now())
        .bind("Instant")
        .bind(1i32) // DRM Enable
        .execute(&pool).await.unwrap();

    let token = format!("mock_valid_token_{}", buyer_id);

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let db_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());
    let store = std::sync::Arc::new(
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&db_url)
            .await
            .unwrap(),
    ) as std::sync::Arc<dyn nurture_bridge::trajectory::TrajectoryStore>;
    let job_queue: std::sync::Arc<dyn nurture_bridge::traits::JobQueue> =
        std::sync::Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            store,
        ));

    let state = AppState::init(
        pool.clone(),
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        ActorId(system_id),
        cancel_token,
        "test_nurture_secret".to_string().into(),
        None,
        None,
        std::sync::Arc::new(MockAuthManager::new()),
        "test_drm_master_key".to_string().into(),
        std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new()),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();
    let app = nurture_routes(state);
    let server = TestServer::new(app).unwrap();

    let buy_req = BuyRequest {
        buyer: ActorId(buyer_id),
        item_id,
        idempotency_key: Some(format!("escrow-e2e-{}", Uuid::new_v4())),
        use_escrow: Some(true),
    };

    let response = server
        .post("/marketplace/buy")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&buy_req)
        .await;

    response.assert_status_success();

    let buy_res: commerce_protocol::mcp_commerce::BuyResponse = response.json();
    assert!(buy_res.escrow_id.is_some());
    let escrow_id = buy_res.escrow_id.clone().unwrap();
    assert!(escrow_id.starts_with("escrow-"));

    // Verify License in DB
    assert!(buy_res.license_id.is_some());
    let license_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM nurture_licenses WHERE id = ?)")
            .bind(buy_res.license_id.unwrap().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(license_exists);

    // Verify Escrow Record in SQLite (Status should be 'pending')
    let escrow_status: String =
        sqlx::query_scalar("SELECT status FROM nurture_escrows WHERE escrow_id = ?")
            .bind(&escrow_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(escrow_status, "pending");

    // Buyer balance: 1000 - 300 = 700
    let buyer_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(buyer_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(buyer_balance, 700);

    // Escrow resolution resolved seller balance should still be 0 (since it is pending, not released)
    let seller_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(seller_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seller_balance, 0);
}
