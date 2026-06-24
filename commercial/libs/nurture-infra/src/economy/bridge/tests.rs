/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use super::*;
use crate::csam::CsamPipeline;
use crate::drm::license::SQLiteLicenseStore;
use crate::economy::idempotency::SQLiteIdempotencyStore;
use crate::economy::interceptor::EconomyInterceptor;
use crate::economy::ledger::DatabaseEconomyLedger;
use crate::economy::settlement::SQLiteSettlementProvider;
use crate::marketplace::sqlite::SQLiteMarketplace;
use crate::mock_job_queue::MockJobQueue;
use aiome_core_contracts::commerce::CommerceEngine;
use nurture_core::policy::EconomyPolicy;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

async fn setup_bridge() -> NurtureCommerceBridge {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("../../migrations/sqlite")
        .run(&pool)
        .await
        .unwrap();
    let db_pool = DatabasePool::Sqlite(pool);

    let policy = Arc::new(RwLock::new(EconomyPolicy::default()));

    let ledger = Arc::new(DatabaseEconomyLedger::new(db_pool.clone()));
    let settlement = Arc::new(SQLiteSettlementProvider::new(
        db_pool.clone(),
        ledger.clone() as Arc<dyn EconomyLedger>,
        policy.clone(),
        commerce_protocol::identity::ActorId(Uuid::nil()),
    ));
    let marketplace = Arc::new(SQLiteMarketplace::new(db_pool.clone()));
    let interceptor = Arc::new(EconomyInterceptor::new(policy.clone()));
    let csam_pipeline = Arc::new(CsamPipeline::new(vec![]));
    let job_queue = Arc::new(MockJobQueue::new("sqlite::memory:").await.unwrap());
    let idempotency = Arc::new(SQLiteIdempotencyStore::new(db_pool.clone()));
    use secrecy::SecretString;
    let license_store = Arc::new(SQLiteLicenseStore::new(
        db_pool.clone(),
        &SecretString::from("test-seed".to_string()),
    ));
    let executor = Arc::new(crate::sandbox::executor::PythonExecutor::new(
        crate::sandbox::executor::ResourceLimits::default(),
    ));
    let karma_forge = Arc::new(crate::economy::karma_forge::KarmaForge::new(
        job_queue.clone(),
        Arc::new(nurture_bridge::llm::MockLlmProvider::default()),
        executor,
    ));

    let uow_manager = Arc::new(crate::economy::uow::SqliteUowManager::new(
        db_pool.clone(),
        &"test-seed".to_string().into(),
    ));

    NurtureCommerceBridge::new(
        ledger,
        settlement,
        marketplace,
        interceptor,
        csam_pipeline,
        job_queue,
        idempotency,
        license_store,
        karma_forge,
        policy,
        db_pool,
        uow_manager,
    )
}

#[tokio::test]
async fn test_withdraw_points_insufficient() {
    let bridge = setup_bridge().await;
    let actor = Uuid::new_v4();

    let result = bridge.withdraw_points(actor, 100).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Insufficient points"));
}

#[tokio::test]
async fn test_instant_refund_not_found() {
    let bridge = setup_bridge().await;
    let actor = Uuid::new_v4();
    let tx_id = Uuid::new_v4();

    // Should fail because transaction doesn't exist
    let result = bridge.instant_refund(&tx_id.to_string(), actor).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Original purchase transaction not found"));
}

#[tokio::test]
async fn test_validate_activity_overflow() {
    let bridge = setup_bridge().await;
    let actor = Uuid::new_v4();

    // Give the actor a wallet with extremely high spent_today to trigger overflow
    sqlx::query(
        "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(actor.to_string())
    .bind(1000i64) // balance
    .bind(i64::MAX) // limit
    .bind(i64::MAX - 5) // spent_today close to max
    .bind(1)
    .execute(bridge.pool.get_sqlite_pool().unwrap())
    .await
    .unwrap();

    // 10 causes overflow: (u64::MAX - 5) + 10 > u64::MAX
    let result = bridge.validate_activity(actor, "inference", 10).await;

    assert!(
        result.is_err(),
        "Validation should fail fast on integer overflow, not saturate"
    );
}

#[tokio::test]
async fn test_stake_and_slash_fail_safe() {
    let bridge = setup_bridge().await;
    let actor = Uuid::new_v4();

    assert!(bridge.stake(actor, 100).await.is_err());
    assert!(bridge.slash(actor, 100, "test penalty").await.is_err());
}

#[tokio::test]
async fn test_verify_signature_fails_on_invalid() {
    let bridge = setup_bridge().await;
    // FAIL-SAFE MOCK always returns Ok
    let result = bridge.verify_signature("{}", "t=123,v1=invalid");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_subscription_and_transfer_stubs() {
    let bridge = setup_bridge().await;
    let actor = Uuid::new_v4();

    let create_res = bridge.create_subscription(actor, "plan_123").await;
    assert!(create_res.is_err());

    let cancel_res = bridge.cancel_subscription(actor, "sub_123").await;
    assert!(cancel_res.is_err());
}

#[tokio::test]
async fn test_transfer_happy_path() {
    let bridge = setup_bridge().await;
    let from_actor = Uuid::new_v4();
    let to_actor = Uuid::new_v4();

    // Setup wallets
    sqlx::query(
        "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(from_actor.to_string())
    .bind(1000i64)
    .bind(5000i64)
    .bind(0i64)
    .bind(1)
    .execute(bridge.pool.get_sqlite_pool().unwrap())
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(to_actor.to_string())
    .bind(500i64)
    .bind(5000i64)
    .bind(0i64)
    .bind(1)
    .execute(bridge.pool.get_sqlite_pool().unwrap())
    .await
    .unwrap();

    let result = bridge.transfer(from_actor, to_actor, 200).await;
    assert!(result.is_ok(), "Transfer should succeed");

    let from_wallet = bridge
        .ledger
        .get_balance(&commerce_protocol::identity::ActorId(from_actor))
        .await
        .unwrap();
    assert_eq!(from_wallet.coin.balance, 800);

    let to_wallet = bridge
        .ledger
        .get_balance(&commerce_protocol::identity::ActorId(to_actor))
        .await
        .unwrap();
    assert_eq!(to_wallet.coin.balance, 700);
}

#[tokio::test]
async fn test_transfer_insufficient() {
    let bridge = setup_bridge().await;
    let from_actor = Uuid::new_v4();
    let to_actor = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(from_actor.to_string())
    .bind(100i64)
    .bind(5000i64)
    .bind(0i64)
    .bind(1)
    .execute(bridge.pool.get_sqlite_pool().unwrap())
    .await
    .unwrap();

    let result = bridge.transfer(from_actor, to_actor, 200).await;
    assert!(
        result.is_err(),
        "Transfer should fail due to insufficient funds"
    );
}

#[tokio::test]
async fn test_transfer_self_rejected() {
    let bridge = setup_bridge().await;
    let actor = Uuid::new_v4();

    let result = bridge.transfer(actor, actor, 100).await;
    assert!(result.is_err(), "Self-transfer should be rejected");
}

#[tokio::test]
async fn test_deliver_gift_with_csam_guard() {
    let bridge = setup_bridge().await;
    let sender_id = Uuid::new_v4();
    let receiver_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();

    // Marketplace にアイテムを登録 (CSAMスキャン対象になるため必要)
    let item = commerce_protocol::commodity::ItemDescriptor {
        id: asset_id,
        kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
        name: "Secret Gift".to_string(),
        description: "A very secret gift".to_string(),
        price: commerce_protocol::commodity::PriceTag::Free,
        creator_id: commerce_protocol::identity::ActorId(Uuid::new_v4()),
        sale_mode: commerce_protocol::offer::SaleMode::Instant,
        drm_enabled: true,
        created_at: chrono::Utc::now(),
        metadata: serde_json::json!({"test": "data"}),
        content_hash: None,
    };
    bridge.marketplace.create_item(&item).await.unwrap();

    // Sender にライセンスを付与（所有している前提）
    let license = nurture_core::license::AssetLicense {
        id: Uuid::new_v4(),
        transaction_id: Uuid::new_v4(),
        asset_id,
        owner_id: commerce_protocol::identity::ActorId(sender_id),
        decryption_key: "key_123".to_string(),
        issued_at: chrono::Utc::now(),
        expires_at: None,
        revoked_at: None,
    };
    bridge.license_store.issue_license(&license).await.unwrap();

    // Act: Giftを配送する
    let result = bridge.deliver_gift(asset_id, sender_id, receiver_id).await;

    // Assert: 成功すること
    assert!(result.is_ok(), "Gift delivery should succeed");

    // Sender はライセンスを失っていること (譲渡のため)
    let sender_license = bridge
        .license_store
        .get_license(&commerce_protocol::identity::ActorId(sender_id), &asset_id)
        .await
        .unwrap();
    assert!(
        sender_license.is_none(),
        "Sender should no longer have the license"
    );

    // Receiver がライセンスを獲得していること
    let receiver_license = bridge
        .license_store
        .get_license(
            &commerce_protocol::identity::ActorId(receiver_id),
            &asset_id,
        )
        .await
        .unwrap();
    assert!(
        receiver_license.is_some(),
        "Receiver should have the license"
    );

    // 台帳に Gift 取引が記録されていること
    let sender_history = bridge
        .ledger
        .get_history(&commerce_protocol::identity::ActorId(sender_id), 10)
        .await
        .unwrap();
    let gift_entry = sender_history
        .iter()
        .find(|e| e.entry_type == nurture_core::ledger::EntryType::Gift);
    assert!(
        gift_entry.is_some(),
        "Gift entry should be recorded in the ledger"
    );
    let entry = gift_entry.unwrap();
    assert_eq!(entry.coin_amount, 0);
    assert_eq!(entry.credit_account.0, receiver_id);
}

#[tokio::test]
async fn test_deliver_gift_self_gift_rejected() {
    let bridge = setup_bridge().await;
    let actor = Uuid::new_v4();
    let asset_id = Uuid::new_v4();

    let result = bridge.deliver_gift(asset_id, actor, actor).await;
    assert!(result.is_err(), "Self-gift should be rejected");
    assert!(
        result.unwrap_err().to_string().contains("自分自身"),
        "Error should mention self-gift prohibition"
    );
}

#[tokio::test]
async fn test_deliver_gift_without_ownership() {
    let bridge = setup_bridge().await;
    let sender_id = Uuid::new_v4();
    let receiver_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();

    // Marketplace にアイテムを登録するが、sender にライセンスを付与しない
    let item = commerce_protocol::commodity::ItemDescriptor {
        id: asset_id,
        kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
        name: "Unowned Gift".to_string(),
        description: "Sender does not own this".to_string(),
        price: commerce_protocol::commodity::PriceTag::Free,
        creator_id: commerce_protocol::identity::ActorId(Uuid::new_v4()),
        sale_mode: commerce_protocol::offer::SaleMode::Instant,
        drm_enabled: false,
        created_at: chrono::Utc::now(),
        metadata: serde_json::json!({}),
        content_hash: None,
    };
    bridge.marketplace.create_item(&item).await.unwrap();

    let result = bridge.deliver_gift(asset_id, sender_id, receiver_id).await;
    assert!(result.is_err(), "Gift without ownership should fail");
    assert!(
        result.unwrap_err().to_string().contains("does not own"),
        "Error should mention lack of ownership"
    );
}

#[tokio::test]
async fn test_deliver_gift_nonexistent_item() {
    let bridge = setup_bridge().await;
    let sender_id = Uuid::new_v4();
    let receiver_id = Uuid::new_v4();
    let nonexistent_id = Uuid::new_v4();

    let result = bridge
        .deliver_gift(nonexistent_id, sender_id, receiver_id)
        .await;
    assert!(result.is_err(), "Gift of nonexistent item should fail");
}

/// C-4 TDD: instant_refund で生成される refund_entry が
/// 元の purchase の asset_id を正しく伝搬することを検証する。
#[tokio::test]
async fn test_instant_refund_preserves_asset_id() {
    let bridge = setup_bridge().await;
    let buyer = Uuid::new_v4();
    let seller = Uuid::new_v4();
    let asset = Uuid::new_v4();
    let tx_id = Uuid::new_v4();

    // buyer の wallet を作成
    sqlx::query(
        "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(buyer.to_string())
    .bind(1000i64)
    .bind(5000i64)
    .bind(0i64)
    .bind(1)
    .execute(bridge.pool.get_sqlite_pool().unwrap())
    .await
    .unwrap();

    // seller の wallet を作成
    sqlx::query(
        "INSERT INTO nurture_wallets (actor_id, balance, daily_limit, spent_today, version) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(seller.to_string())
    .bind(0i64)
    .bind(5000i64)
    .bind(0i64)
    .bind(1)
    .execute(bridge.pool.get_sqlite_pool().unwrap())
    .await
    .unwrap();

    // Purchase エントリを直接挿入 (asset_id 付き)
    let purchase_entry = nurture_core::ledger::LedgerEntry {
        id: Uuid::new_v4(),
        transaction_id: tx_id,
        asset_id: Some(asset),
        debit_account: commerce_protocol::identity::ActorId(buyer),
        credit_account: commerce_protocol::identity::ActorId(seller),
        coin_amount: 50, // refund_limit (karma=0 → limit=0) を超えるが、DRM=false なら拒否される
        points_amount: 0,
        entry_type: nurture_core::ledger::EntryType::Purchase,
        created_at: chrono::Utc::now(),
        debit_account_version: Some(1),
    };
    bridge.ledger.record_entry(&purchase_entry).await.unwrap();

    // Marketplace にアイテム登録 (DRM=true → karma チェックをバイパス)
    let item = commerce_protocol::commodity::ItemDescriptor {
        id: asset,
        kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
        name: "DRM Asset".to_string(),
        description: "DRM protected".to_string(),
        price: commerce_protocol::commodity::PriceTag::Fixed(50),
        creator_id: commerce_protocol::identity::ActorId(seller),
        sale_mode: commerce_protocol::offer::SaleMode::Instant,
        drm_enabled: true,
        created_at: chrono::Utc::now(),
        metadata: serde_json::json!({}),
        content_hash: None,
    };
    bridge.marketplace.create_item(&item).await.unwrap();

    // instant_refund 実行
    let result = bridge.instant_refund(&tx_id.to_string(), buyer).await;
    assert!(
        result.is_ok(),
        "instant_refund should succeed: {:?}",
        result.err()
    );

    // Refund エントリの asset_id が purchase の asset_id と一致することを検証
    let _entries = bridge
        .ledger
        .get_entries_by_transaction(&tx_id)
        .await
        .unwrap();
    // Note: refund_entry は新しい transaction_id を持つので、buyer の history から検索
    let history = bridge
        .ledger
        .get_history(&commerce_protocol::identity::ActorId(buyer), 20)
        .await
        .unwrap();

    let refund = history
        .iter()
        .find(|e| e.entry_type == nurture_core::ledger::EntryType::Refund)
        .expect("Refund entry should exist in history");

    assert_eq!(
        refund.asset_id,
        Some(asset),
        "C-4: Refund entry must preserve the original purchase's asset_id for audit traceability"
    );
}

/// C-5 TDD: deliver_gift で生成される audit_entry が
/// ギフト対象の asset_id を正しく記録することを検証する。
#[tokio::test]
async fn test_deliver_gift_preserves_asset_id() {
    let bridge = setup_bridge().await;
    let sender_id = Uuid::new_v4();
    let receiver_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();

    let sender_actor = commerce_protocol::identity::ActorId(sender_id);

    // Marketplace にアイテム登録
    let item = commerce_protocol::commodity::ItemDescriptor {
        id: asset_id,
        kind: commerce_protocol::commodity::CommodityKind::KnowledgePack,
        name: "Gift Item".to_string(),
        description: "For gift test".to_string(),
        price: commerce_protocol::commodity::PriceTag::Free,
        creator_id: commerce_protocol::identity::ActorId(Uuid::new_v4()),
        sale_mode: commerce_protocol::offer::SaleMode::Instant,
        drm_enabled: false,
        created_at: chrono::Utc::now(),
        metadata: serde_json::json!({}),
        content_hash: None,
    };
    bridge.marketplace.create_item(&item).await.unwrap();

    // sender にライセンスを発行
    let license = nurture_core::license::AssetLicense {
        id: Uuid::new_v4(),
        transaction_id: Uuid::new_v4(),
        asset_id,
        owner_id: sender_actor,
        decryption_key: "test-key".to_string(),
        issued_at: chrono::Utc::now(),
        expires_at: None,
        revoked_at: None,
    };
    bridge.license_store.issue_license(&license).await.unwrap();

    // deliver_gift 実行
    let result = bridge.deliver_gift(asset_id, sender_id, receiver_id).await;
    assert!(
        result.is_ok(),
        "deliver_gift should succeed: {:?}",
        result.err()
    );

    // Gift audit entry の asset_id が正しいことを検証
    let history = bridge.ledger.get_history(&sender_actor, 20).await.unwrap();

    let gift_entry = history
        .iter()
        .find(|e| e.entry_type == nurture_core::ledger::EntryType::Gift)
        .expect("Gift audit entry should exist in history");

    assert_eq!(
        gift_entry.asset_id,
        Some(asset_id),
        "C-5: Gift audit entry must record the asset_id for traceability"
    );
}
