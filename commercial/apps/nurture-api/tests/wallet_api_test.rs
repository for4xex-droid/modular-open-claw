/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use axum_test::TestServer;
use commerce_protocol::identity::ActorId;
use nurture_api::routes::nurture_routes;
use nurture_api::state::AppState;
use nurture_bridge::auth::MockAuthManager;
use serde::Serialize;
use serial_test::serial;
use sqlx::SqlitePool;
use tempfile::tempdir;
use uuid::Uuid;

#[derive(Serialize)]
struct TransferRequest {
    to_actor_id: Uuid,
    amount: u64,
}

async fn setup_test_app() -> (TestServer, SqlitePool, tempfile::TempDir) {
    let tdir = tempdir().unwrap();
    let db_path = tdir.path().join("test_wallet.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    sqlx::migrate!("../../migrations/sqlite")
        .run(&pool)
        .await
        .unwrap();

    let system_id = Uuid::new_v4();
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
        nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
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

    (server, pool, tdir)
}

fn create_token(actor_id: Uuid) -> String {
    format!("mock_valid_token_{}", actor_id)
}

#[tokio::test]
#[serial]
async fn test_transfer_api_happy_path() {
    let (server, pool, _tdir) = setup_test_app().await;

    let from_id = Uuid::new_v4();
    let to_id = Uuid::new_v4();

    // Setup wallets
    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(from_id.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(to_id.to_string())
        .bind(200i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    // KYC Verification for sender
    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
        .bind(from_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let token = create_token(from_id);
    let req = TransferRequest {
        to_actor_id: to_id,
        amount: 300,
    };

    let res = server
        .post("/wallet/transfer")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .await;

    res.assert_status_success();

    let res_json: serde_json::Value = res.json();
    assert_eq!(res_json["status"], "success");
    assert!(res_json["transaction_id"].is_string());

    // Check DB balances
    let from_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(from_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(from_balance, 700);

    let to_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(to_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(to_balance, 500);
}

#[tokio::test]
#[serial]
async fn test_transfer_api_requires_kyc() {
    let (server, pool, _tdir) = setup_test_app().await;

    let from_id = Uuid::new_v4();
    let to_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(from_id.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    // NO KYC VERIFICATION for `from_id`

    let token = create_token(from_id);
    let req = TransferRequest {
        to_actor_id: to_id,
        amount: 300,
    };

    let res = server
        .post("/wallet/transfer")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .await;

    // Expecting PolicyViolation to map to some error status (e.g. 403 or 400)
    assert!(!res.status_code().is_success());

    let err_txt = res.text();
    assert!(err_txt.contains("eKYC") || err_txt.contains("AML"));
}

#[tokio::test]
#[serial]
async fn test_transfer_api_insufficient_funds() {
    let (server, pool, _tdir) = setup_test_app().await;

    let from_id = Uuid::new_v4();
    let to_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(from_id.to_string())
        .bind(100i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
        .bind(from_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let token = create_token(from_id);
    let req = TransferRequest {
        to_actor_id: to_id,
        amount: 300,
    };

    let res = server
        .post("/wallet/transfer")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .await;

    assert!(!res.status_code().is_success());
    let err_txt = res.text();
    assert!(err_txt.contains("Insufficient"));
}

#[tokio::test]
#[serial]
async fn test_transfer_api_self_rejected() {
    let (server, pool, _tdir) = setup_test_app().await;

    let from_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(from_id.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
        .bind(from_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let token = create_token(from_id);
    let req = TransferRequest {
        to_actor_id: from_id, // Same ID!
        amount: 300,
    };

    let res = server
        .post("/wallet/transfer")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .await;

    assert!(!res.status_code().is_success());
    let err_txt = res.text();
    assert!(err_txt.contains("Self"));
}
