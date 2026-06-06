/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use commerce_protocol::identity::ActorId;
use nurture_api::state::AppState;
use nurture_bridge::commerce::CommerceEngine;
use nurture_infra::economy::bridge::NurtureCommerceBridge;
use serial_test::serial;
use sqlx::SqlitePool;
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

async fn setup_test_engine() -> (
    Arc<NurtureCommerceBridge>,
    SqlitePool,
    ActorId,
    tempfile::TempDir,
) {
    let tdir = tempdir().unwrap();
    let db_path = tdir.path().join("test_ce.db");

    let connect_opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(connect_opts.clone())
        .await
        .unwrap();

    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let system_id = Uuid::new_v4();
    let cancel_token = tokio_util::sync::CancellationToken::new();

    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());
    let store = Arc::new(
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&db_url)
            .await
            .unwrap(),
    ) as Arc<dyn nurture_bridge::trajectory::TrajectoryStore>;

    let job_queue: Arc<dyn nurture_bridge::traits::JobQueue> =
        Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            store,
        ));

    let state = AppState::init(
        pool.clone(),
        job_queue,
        nurture_core::policy::EconomyPolicy::default(), // daily limit defaults to 10000
        ActorId(system_id),
        cancel_token,
        "test_secret_key".to_string().into(),
        None,
        None,
        std::sync::Arc::new(nurture_bridge::auth::MockAuthManager::new()),
        "test_drm_master_key".to_string().into(),
        std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new()),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();

    let uow_manager = Arc::new(nurture_infra::economy::uow::SqliteUowManager::new(
        pool.clone(),
        &"test_drm_master_key".to_string().into(),
    ));

    let bridge = Arc::new(NurtureCommerceBridge::new(
        state.ledger.clone(),
        state.settlement.clone(),
        state.marketplace.clone(),
        state.interceptor.clone(),
        state.csam_pipeline.clone(),
        state.job_queue.clone(),
        state.idempotency.clone(),
        state.license_store.clone(),
        state.karma_forge.clone(),
        state.policy.clone(),
        pool.clone(),
        uow_manager,
    ));

    (bridge, pool, ActorId(system_id), tdir)
}

#[tokio::test]
#[serial]
async fn test_escrow_lifecycle() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();
    let recipient_id = Uuid::new_v4();

    // 1. Give agent some money
    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    // 2. Create Escrow
    let escrow_id = engine
        .escrow_create(agent_id, 200)
        .await
        .expect("Failed to create escrow");
    assert!(escrow_id.starts_with("escrow-"));

    // Check balance (should be 800)
    let balance: i64 = sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
        .bind(agent_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(balance, 800);

    // Check spent_today (should be 200)
    let spent: i64 =
        sqlx::query_scalar("SELECT spent_today FROM nurture_wallets WHERE actor_id = ?")
            .bind(agent_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(spent, 200);

    // 3. Release Escrow
    engine
        .escrow_release(&escrow_id, recipient_id)
        .await
        .expect("Failed to release escrow");

    // Recipient should have 200
    let rec_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(recipient_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rec_balance, 200);

    // Escrow status should be released
    let status: String =
        sqlx::query_scalar("SELECT status FROM nurture_escrows WHERE escrow_id = ?")
            .bind(&escrow_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "released");

    // Ledger should have a Purchase entry for the release
    let expected_purchase_type =
        serde_json::to_string(&nurture_core::ledger::EntryType::Purchase).unwrap();
    let release_ledger: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM nurture_ledger WHERE debit_account = ? AND credit_account = ? AND entry_type = ?"
    )
    .bind(agent_id.to_string())
    .bind(recipient_id.to_string())
    .bind(&expected_purchase_type)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        release_ledger, 1,
        "Ledger should record the escrow release as Purchase"
    );
}

#[tokio::test]
#[serial]
async fn test_escrow_refund_logic() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    let escrow_id = engine.escrow_create(agent_id, 300).await.unwrap();

    // Reflexion 期待する防御挙動のテスト: 返金時に spent_today を回復させているか？
    engine
        .escrow_refund(&escrow_id)
        .await
        .expect("Failed to refund escrow");

    let agent_row =
        sqlx::query("SELECT balance, spent_today FROM nurture_wallets WHERE actor_id = ?")
            .bind(agent_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

    let balance: i64 = sqlx::Row::get(&agent_row, "balance");
    let spent: i64 = sqlx::Row::get(&agent_row, "spent_today");

    assert_eq!(balance, 1000); // 戻っている
    assert_eq!(spent, 0); // スパム防止のため枠も回復している

    // entry_type は serde_json::to_string(&EntryType::Refund) の出力に一致すること
    let expected_entry_type =
        serde_json::to_string(&nurture_core::ledger::EntryType::Refund).unwrap();
    let ledger_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM nurture_ledger WHERE credit_account = ? AND entry_type = ?",
    )
    .bind(agent_id.to_string())
    .bind(&expected_entry_type)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(ledger_entries, 1, "Ledger should record the escrow refund");
}

#[tokio::test]
#[serial]
async fn test_deduct_generation_cost_flow() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(500i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    engine
        .deduct_generation_cost(agent_id, None, 50, "image_gen")
        .await
        .expect("Failed to deduct");

    let agent_row =
        sqlx::query("SELECT balance, spent_today FROM nurture_wallets WHERE actor_id = ?")
            .bind(agent_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

    let balance: i64 = sqlx::Row::get(&agent_row, "balance");
    let spent: i64 = sqlx::Row::get(&agent_row, "spent_today");

    assert_eq!(balance, 450);
    assert_eq!(spent, 50);

    // Ledger should have SystemFee entry (credit is nil)
    let system_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(Uuid::nil().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(system_balance, 50); // System got the money
}

#[tokio::test]
#[serial]
async fn test_edge_case_daily_limit_exceeded() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();

    // Wallet with 20k to avoid Insufficient funds error
    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(20000i64)
        .bind(200i64) // Has no effect, policy is used
        .execute(&pool)
        .await
        .unwrap();

    // The test engine policy has daily_spend_limit = 10000 by default (EconomyPolicy::default()).
    // Let's try to spend 10001, which should be rejected.
    let res = engine.escrow_create(agent_id, 10001).await;
    assert!(res.is_err());

    // Check error message contains daily spend limit
    if let Err(nurture_bridge::error::AiomeError::Infrastructure { reason }) = res {
        assert!(reason.contains("daily spend limit"));
    } else {
        panic!("Wrong error type");
    }
}

#[tokio::test]
#[serial]
async fn test_consecutive_escrows_accumulate_daily_spend() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(20000i64)
        .bind(10000i64)
        .execute(&pool)
        .await
        .unwrap();

    // Create 5 escrows of 1500 each (total: 7500, under 10000 limit)
    for i in 0..5 {
        let res = engine.escrow_create(agent_id, 1500).await;
        assert!(res.is_ok(), "Escrow {} should succeed", i);
    }

    // 6th should exceed daily limit (7500 + 1500 = 9000, still under)
    let res = engine.escrow_create(agent_id, 1500).await;
    assert!(res.is_ok(), "6th escrow should succeed (total 9000)");

    // 7th should exceed (9000 + 1500 = 10500 > 10000)
    let res = engine.escrow_create(agent_id, 1500).await;
    assert!(res.is_err(), "7th escrow should fail due to daily limit");

    // Verify spent_today is 9000
    let spent: i64 =
        sqlx::query_scalar("SELECT spent_today FROM nurture_wallets WHERE actor_id = ?")
            .bind(agent_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(spent, 9000);
}

#[tokio::test]
#[serial]
async fn test_double_release_rejected() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();
    let recipient_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    let escrow_id = engine.escrow_create(agent_id, 200).await.unwrap();

    // First release should succeed
    engine
        .escrow_release(&escrow_id, recipient_id)
        .await
        .unwrap();

    // Second release should fail (already resolved)
    let res = engine.escrow_release(&escrow_id, recipient_id).await;
    assert!(res.is_err());
    if let Err(nurture_bridge::error::AiomeError::Infrastructure { reason }) = res {
        assert!(reason.contains("not found or already resolved"));
    }

    // Recipient balance should be 200, not 400
    let rec_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM nurture_wallets WHERE actor_id = ?")
            .bind(recipient_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rec_balance, 200);
}

#[tokio::test]
#[serial]
async fn test_zero_amount_escrow_rejected() {
    let (engine, _pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();

    // amount = 0 は Fail-fast で拒否される
    let res = engine.escrow_create(agent_id, 0).await;
    assert!(res.is_err());
    if let Err(nurture_bridge::error::AiomeError::Infrastructure { reason }) = res {
        assert!(reason.contains("greater than zero"));
    } else {
        panic!("Expected Infrastructure error for zero escrow");
    }
}

#[tokio::test]
#[serial]
async fn test_zero_amount_deduct_rejected() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(1000i64)
        .bind(5000i64)
        .execute(&pool)
        .await
        .unwrap();

    let res = engine
        .deduct_generation_cost(agent_id, None, 0, "test")
        .await;
    assert!(res.is_err());
    if let Err(nurture_bridge::error::AiomeError::Infrastructure { reason }) = res {
        assert!(reason.contains("greater than zero"));
    } else {
        panic!("Expected Infrastructure error for zero deduction");
    }
}

#[tokio::test]
#[serial]
async fn test_register_license() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();
    let creator_id = Uuid::new_v4();
    let transaction_id = Uuid::new_v4().to_string();
    let license_type = "subscription";

    // Insert an item with subscription mode
    sqlx::query(
        "INSERT INTO nurture_items (id, kind, creator_id, name, description, price_coins, metadata, sale_mode, drm_enabled, subscription_interval_days, subscription_price_coins) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(asset_id.to_string())
    .bind("WasmSkill")
    .bind(creator_id.to_string())
    .bind("Test Item")
    .bind("Description")
    .bind(100i64)
    .bind(r#"{"drm_key":"test_specific_key_base64"}"#)
    .bind(serde_json::to_string(&commerce_protocol::offer::SaleMode::Subscription { interval_days: 30, price_coins: 100 }).unwrap())
    .bind(1)
    .bind(30)
    .bind(100)
    .execute(&pool)
    .await
    .unwrap();

    // Call register_license
    let license_id = engine
        .register_license(agent_id, asset_id, &transaction_id, license_type)
        .await
        .expect("Failed to register license");

    assert!(!license_id.is_empty(), "License ID should not be empty");

    // Check DB
    let (count, decryption_key): (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), decryption_key FROM nurture_licenses WHERE owner_id = ? AND asset_id = ? AND transaction_id = ?",
    )
    .bind(agent_id.to_string())
    .bind(asset_id.to_string())
    .bind(&transaction_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1, "License should be stored in DB");
    assert_ne!(
        decryption_key, "test_specific_key_base64",
        "Decryption key should be encrypted, not plaintext"
    );
    assert!(
        decryption_key.contains(':'),
        "Encrypted key should be in b64_nonce:b64_ciphertext format"
    );
}

#[tokio::test]
#[serial]
async fn test_validate_activity_security_policies() {
    let (engine, pool, _, _tdir) = setup_test_engine().await;

    let agent_id = Uuid::new_v4();

    sqlx::query("INSERT INTO nurture_wallets (actor_id, balance, daily_limit) VALUES (?, ?, ?)")
        .bind(agent_id.to_string())
        .bind(100i64) // balance: 100
        .bind(50i64) // daily_limit: 50 (厳しい上限)
        .execute(&pool)
        .await
        .unwrap();

    // 1. activity_type が未知・不正な場合はエラー
    let res = engine.validate_activity(agent_id, "unknown_hack", 10).await;
    assert!(
        matches!(
            res,
            Err(nurture_bridge::error::AiomeError::Infrastructure { .. })
        ),
        "Unknown activity type MUST be rejected"
    );

    // 2. 空文字列もエラー
    let res = engine.validate_activity(agent_id, "", 10).await;
    assert!(res.is_err(), "Empty activity type MUST be rejected");

    // 3. 残高不足チェック (balance: 100, 要求: 150)
    let res = engine.validate_activity(agent_id, "generation", 150).await;
    assert!(res.is_err(), "Insufficient balance MUST be rejected");

    // 4. 日次上限チェック (balance: 100 は足りているが、daily_limit: 50 なので 要求: 60 は上限超過)
    let res = engine.validate_activity(agent_id, "inference", 60).await;
    assert!(res.is_err(), "Daily limit exceeded MUST be rejected");

    // 5. 正常系: 条件を満たす場合は Ok(())
    let res = engine.validate_activity(agent_id, "clone_fork", 30).await;
    assert!(res.is_ok(), "Valid activity MUST be accepted");

    // 6. amount = 0 (ゼロコスト活動) は残高チェック等をバイパスして成功
    let res = engine
        .validate_activity(agent_id, "knowledge_query", 0)
        .await;
    assert!(res.is_ok(), "Zero amount activity MUST be accepted");
}
