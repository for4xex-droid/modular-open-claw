use axum::http::header;
use axum::http::StatusCode;
use axum::{Extension, Router};
use axum_test::TestServer;
use base64::Engine;
use nurture_api::routes::internal::internal_routes;
use nurture_api::state::AppState;
use nurture_bridge::oxilean::OxiLeanProofCertificate;
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

async fn setup_test_server() -> (TestServer, tempfile::TempDir) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
    let tdir = tempfile::tempdir().unwrap();
    let db_path = tdir.path().join("test_nurture.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // NurtureDB スキーマをセットアップ
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let system_actor = Uuid::new_v4();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let trajectory_store =
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&format!(
            "sqlite:{}?mode=rwc",
            db_path.to_str().unwrap()
        ))
        .await
        .unwrap();

    let jobs_db_path = tdir.path().join("test_jobs.db");
    let jobs_pool = SqlitePool::connect(&format!(
        "sqlite:{}?mode=rwc",
        jobs_db_path.to_str().unwrap()
    ))
    .await
    .unwrap();

    let job_queue: std::sync::Arc<dyn nurture_bridge::traits::JobQueue> = std::sync::Arc::new(
        nurture_bridge::job_queue::UniversalJobQueue::new(
            nurture_bridge::db::DatabasePool::Sqlite(jobs_pool),
            None,
            std::sync::Arc::new(trajectory_store),
        )
        .await
        .unwrap(),
    );

    let state = AppState::init(
        pool,
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        commerce_protocol::identity::ActorId(system_actor),
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

    let app = Router::new()
        .nest("/internal", internal_routes())
        .layer(Extension(state));

    (TestServer::new(app).unwrap(), tdir)
}

#[tokio::test]
async fn test_internal_api_requires_oxp_certificate() {
    let (server, _tdir) = setup_test_server().await;

    let payload = json!({
        "escrow_id": "test_escrow_1",
        "recipient_id": Uuid::new_v4().to_string()
    });

    // 証明書なし (Missing Header) -> 403 Forbidden
    let res = server.post("/internal/escrow-release").json(&payload).await;
    assert_eq!(res.status_code(), 403);
}

#[tokio::test]
async fn test_internal_api_valid_certificate() {
    let (server, _tdir) = setup_test_server().await;

    let payload = json!({
        "escrow_id": "test_escrow_2",
        "recipient_id": Uuid::new_v4().to_string()
    });

    // 正しい証明書を生成
    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950, // OXP > 900 (Threshold)
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // 証明書あり -> 200 OK (またはエスクローが存在しないというビジネスエラーで 400系)
    // ミドルウェアで弾かれなければ OK
    let res = server
        .post("/internal/escrow-release")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    // Authは通るが、エスクローがDBにないので 400 Bad Request になるのが正解
    assert_eq!(res.status_code(), 400);
}

#[tokio::test]
async fn test_internal_api_stale_timestamp_rejected() {
    let (server, _tdir) = setup_test_server().await;
    let secret = "test_secret_key"; // Same as state.internal_secret

    // Generate a stale certificate (10 minutes old)
    let stale_time = chrono::Utc::now() - chrono::Duration::minutes(10);
    let cert = OxiLeanProofCertificate::generate(
        "aiome-edge-node".to_string(),
        950,
        stale_time.to_rfc3339(),
        secret,
    );
    let cert_json = serde_json::to_vec(&cert).unwrap();
    let cert_b64 = base64::engine::general_purpose::STANDARD.encode(cert_json);

    let payload = serde_json::json!({
        "escrow_id": uuid::Uuid::new_v4().to_string(),
        "resolution": "completed"
    });

    let res = server
        .post("/internal/escrow-release")
        .add_header(
            axum::http::header::HeaderName::from_static("x-oxilean-proof-certificate"),
            cert_b64,
        )
        .json(&payload)
        .await;

    assert_eq!(res.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_internal_api_low_oxp_score_rejected() {
    let (server, _tdir) = setup_test_server().await;

    let payload = json!({
        "escrow_id": "test_escrow_3",
        "recipient_id": Uuid::new_v4().to_string()
    });

    // スコアが閾値未満 (800 < 900) の証明書
    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        800, // Low OXP
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // スコア不足 -> 403 Forbidden
    let res = server
        .post("/internal/escrow-release")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    assert_eq!(res.status_code(), 403);
}

#[tokio::test]
async fn test_forget_actor_purges_pii_and_physical_assets() {
    let tdir = tempfile::tempdir().unwrap();
    let db_path = tdir.path().join("test_nurture_forget.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // NurtureDB スキーマをセットアップ
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let actor_id = Uuid::new_v4();
    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'Verified')")
        .bind(actor_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_subscriptions (id, actor_id, stripe_subscription_id, plan_id, status, current_period_end) VALUES (?, ?, 'sub_123', 'plan_premium', 'active', CURRENT_TIMESTAMP)")
        .bind(Uuid::new_v4().to_string())
        .bind(actor_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_payout_requests (id, actor_id, amount_usd, points_burned, status) VALUES (?, ?, 10.0, 1000, 'pending')")
        .bind(Uuid::new_v4().to_string())
        .bind(actor_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO nurture_customers (actor_id, stripe_customer_id, email) VALUES (?, 'cus_123', 'test@example.com')")
        .bind(actor_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let system_actor = Uuid::new_v4();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let trajectory_store =
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&format!(
            "sqlite:{}?mode=rwc",
            db_path.to_str().unwrap()
        ))
        .await
        .unwrap();
    let job_queue: std::sync::Arc<dyn nurture_bridge::traits::JobQueue> =
        std::sync::Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            std::sync::Arc::new(trajectory_store),
        ));

    let mock_storage = std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new());

    let state = AppState::init(
        pool.clone(),
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        commerce_protocol::identity::ActorId(system_actor),
        cancel_token,
        "test_secret_key".to_string().into(),
        None,
        None,
        std::sync::Arc::new(nurture_bridge::auth::MockAuthManager::new()),
        "test_drm_master_key".to_string().into(),
        mock_storage.clone(),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();

    let app = Router::new()
        .nest("/internal", internal_routes())
        .layer(Extension(state));

    let server = TestServer::new(app).unwrap();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    let res = server
        .post(&format!("/internal/forget/{}", actor_id))
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .await;

    assert_eq!(res.status_code(), 200);

    // Verify KYC is scrubbed
    let row: Option<(String,)> =
        sqlx::query_as("SELECT actor_id FROM nurture_kyc_status WHERE actor_id = ?")
            .bind(actor_id.to_string())
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(row.is_none(), "KYC record should be scrubbed");

    // Verify Subscriptions scrubbed
    let sub_row: Option<(String,)> =
        sqlx::query_as("SELECT actor_id FROM nurture_subscriptions WHERE actor_id = ?")
            .bind(actor_id.to_string())
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(sub_row.is_none(), "Subscription record should be scrubbed");

    // Verify Payout Requests scrubbed
    let payout_row: Option<(String,)> =
        sqlx::query_as("SELECT actor_id FROM nurture_payout_requests WHERE actor_id = ?")
            .bind(actor_id.to_string())
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(
        payout_row.is_none(),
        "Payout request record should be scrubbed"
    );

    // Verify Customer PII scrubbed (email & stripe_customer_id)
    let customer_row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT email, stripe_customer_id FROM nurture_customers WHERE actor_id = ?",
    )
    .bind(actor_id.to_string())
    .fetch_optional(&pool)
    .await
    .unwrap();
    if let Some((email, stripe_id)) = customer_row {
        assert!(email.is_none(), "Customer email should be NULL");
        assert!(
            stripe_id.unwrap().starts_with("purged_"),
            "Customer stripe_id should be obfuscated"
        );
    }

    // Verify AssetStorage delete_assets_for_actor was called
    let called_actor = *mock_storage.called_actor.lock().unwrap();
    assert_eq!(
        called_actor,
        Some(commerce_protocol::identity::ActorId(actor_id)),
        "AssetStorage should be called to purge physical assets"
    );
}

#[tokio::test]
async fn test_internal_api_escrow_list() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    let res = server
        .get(&format!("/internal/escrow-list/{}", actor_id))
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .await;

    // We expect it to be routed properly and return 200 with an empty list initially
    assert_eq!(res.status_code(), 200);
    let records: Vec<nurture_bridge::commerce::EscrowRecord> = res.json();
    assert_eq!(records.len(), 0);
}

#[tokio::test]
async fn test_internal_api_escrow_refund() {
    let (server, _tdir) = setup_test_server().await;

    let payload = json!({
        "escrow_id": "non_existent_escrow",
    });

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    let res = server
        .post("/internal/escrow-refund")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    // We expect it to be routed properly, but return 400 or 500 because escrow doesn't exist.
    // At minimum, it shouldn't be 404 (Not Found) which would mean route is missing.
    assert_ne!(res.status_code(), 404);
}

#[tokio::test]
async fn test_internal_api_upload_asset() {
    let tdir = tempfile::tempdir().unwrap();
    let db_path = tdir.path().join("test_upload.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // Setup required tables
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let creator_id = Uuid::new_v4();
    sqlx::query("INSERT INTO nurture_kyc_status (actor_id, status) VALUES (?, 'verified')")
        .bind(creator_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let system_actor = Uuid::new_v4();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let trajectory_store =
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&format!(
            "sqlite:{}?mode=rwc",
            db_path.to_str().unwrap()
        ))
        .await
        .unwrap();
    let job_queue = std::sync::Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
        nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
        std::sync::Arc::new(trajectory_store),
    ));

    let mock_storage = std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new());

    let state = AppState::init(
        pool,
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        commerce_protocol::identity::ActorId(system_actor),
        cancel_token,
        "test_secret_key".to_string().into(),
        None,
        None,
        std::sync::Arc::new(nurture_bridge::auth::MockAuthManager::new()),
        "test_drm_master_key".to_string().into(),
        mock_storage.clone(),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();

    let app = Router::new()
        .nest("/internal", internal_routes())
        .layer(Extension(state));
    let server = TestServer::new(app).unwrap();

    let payload = json!({
        "idempotency_key": Uuid::new_v4().to_string(),
        "creator_id": creator_id.to_string(),
        "kind": "VrmAvatar",
        "name": "Test Avatar",
        "description": "A test avatar",
        "price_coins": 100,
        "content": "{\"content\": \"test_base64_data\", \"head_to_body_ratio\": 0.14}"
    });

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    let res = server
        .post("/internal/upload")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    let status = res.status_code();
    assert_eq!(status, StatusCode::CREATED);

    // Verify AssetStorage put_asset was called and data was stored
    let assets = &mock_storage.assets;
    assert_eq!(assets.len(), 1, "Asset should be stored in AssetStorage");
    let entry = assets.iter().next().unwrap();
    assert_eq!(
        entry.value(),
        b"{\"content\": \"test_base64_data\", \"head_to_body_ratio\": 0.14}"
    );
}

#[tokio::test]
async fn test_internal_api_download_asset() {
    let tdir = tempfile::tempdir().unwrap();
    let db_path = tdir.path().join("test_download.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // Setup required tables
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let system_actor = Uuid::new_v4();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let trajectory_store =
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&format!(
            "sqlite:{}?mode=rwc",
            db_path.to_str().unwrap()
        ))
        .await
        .unwrap();
    let job_queue = std::sync::Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
        nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
        std::sync::Arc::new(trajectory_store),
    ));

    let mock_storage = std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new());

    let state = AppState::init(
        pool.clone(),
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        commerce_protocol::identity::ActorId(system_actor),
        cancel_token,
        "test_secret_key".to_string().into(),
        None,
        None,
        std::sync::Arc::new(nurture_bridge::auth::MockAuthManager::new()),
        "test_drm_master_key_123456789012345".to_string().into(),
        mock_storage.clone(),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();

    let app = Router::new()
        .nest("/internal", internal_routes())
        .layer(Extension(state.clone()));
    let server = TestServer::new(app).unwrap();

    let creator_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();
    let buyer_id = Uuid::new_v4();
    let no_license_buyer_id = Uuid::new_v4();

    // Directly insert item
    sqlx::query(
        "INSERT INTO nurture_items (id, kind, name, description, price_coins, creator_id, created_at, metadata, sale_mode, drm_enabled)
         VALUES (?, 'VrmAvatar', 'Test Avatar', 'Desc', 100, ?, CURRENT_TIMESTAMP, '{}', 'Instant', 0)"
    ).bind(asset_id.to_string()).bind(creator_id.to_string()).execute(&pool).await.unwrap();

    // Store physical asset
    state
        .asset_storage
        .put_asset(
            &commerce_protocol::identity::ActorId(creator_id),
            &asset_id,
            b"secure_asset_data",
        )
        .await
        .unwrap();

    // Issue license to buyer_id
    let license = nurture_core::license::AssetLicense {
        id: Uuid::new_v4(),
        transaction_id: Uuid::new_v4(),
        asset_id,
        owner_id: commerce_protocol::identity::ActorId(buyer_id),
        decryption_key: "secure_key".to_string(),
        issued_at: chrono::Utc::now(),
        expires_at: None,
        revoked_at: None,
    };
    state.license_store.issue_license(&license).await.unwrap();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // 1. Success Download
    let res = server
        .get(&format!(
            "/internal/asset/{}/download/{}",
            asset_id, buyer_id
        ))
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .await;

    assert_eq!(res.status_code(), StatusCode::OK);
    assert_eq!(res.as_bytes().as_ref(), b"secure_asset_data");
    let headers = res.headers();
    assert_eq!(
        headers.get("x-nurture-drm-key").unwrap().to_str().unwrap(),
        "secure_key"
    );

    // 2. Forbidden Download (No License)
    let res = server
        .get(&format!(
            "/internal/asset/{}/download/{}",
            asset_id, no_license_buyer_id
        ))
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .await;

    assert_eq!(res.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_internal_api_get_balance_and_stats() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // Test get_balance
    let res_balance = server
        .get(&format!("/internal/balance/{}", actor_id))
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .await;
    assert_eq!(res_balance.status_code(), StatusCode::OK);
    let balance_data: serde_json::Value = res_balance.json();
    assert_eq!(balance_data["balance"], 0);

    // Test get_daily_stats
    let res_stats = server
        .get(&format!("/internal/daily-stats/{}", actor_id))
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .await;
    assert_eq!(res_stats.status_code(), StatusCode::OK);
    let stats_data: serde_json::Value = res_stats.json();
    assert_eq!(stats_data["spent_today"], 0);
    assert_eq!(stats_data["daily_limit"], 10000); // from EconomyPolicy::default()
}

#[tokio::test]
async fn test_internal_api_charge_coins() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // S-1: ゼロ金額チャージの拒否
    let payload_zero = json!({
        "actor_id": actor_id,
        "amount": 0,
        "currency": "coin",
        "stripe_event_id": "evt_123",
        "idempotency_key": "idemp_zero"
    });
    let res_zero = server
        .post("/internal/coin-charge")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload_zero)
        .await;
    assert_eq!(res_zero.status_code(), StatusCode::BAD_REQUEST);

    // Valid Charge
    let payload = json!({
        "actor_id": actor_id,
        "amount": 500,
        "currency": "coin",
        "stripe_event_id": "evt_456",
        "idempotency_key": "idemp_valid"
    });
    let res = server
        .post("/internal/coin-charge")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;
    assert_eq!(res.status_code(), StatusCode::OK);

    // Verify balance was updated
    let res_balance = server
        .get(&format!("/internal/balance/{}", actor_id))
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .await;
    let balance_data: serde_json::Value = res_balance.json();
    assert_eq!(balance_data["balance"], 500);
}

#[tokio::test]
async fn test_internal_api_create_escrow() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // Zero amount escrow
    let payload_zero = json!({
        "actor_id": actor_id,
        "amount": 0
    });
    let res_zero = server
        .post("/internal/escrow-create")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload_zero)
        .await;
    assert_eq!(res_zero.status_code(), StatusCode::BAD_REQUEST);

    // KYC missing escrow (should be forbidden or internal error if mock fails)
    let payload = json!({
        "actor_id": actor_id,
        "amount": 100
    });
    let res = server
        .post("/internal/escrow-create")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;
    // Without KYC, it should be 403 or 500 (since Mock is used in state)
    // The handler returns FORBIDDEN if is_verified is Ok(false)
    assert_eq!(res.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_internal_api_deduct_cost() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // Zero amount deduction
    let payload_zero = json!({
        "actor_id": actor_id,
        "asset_id": null,
        "amount": 0,
        "generation_type": "text"
    });
    let res_zero = server
        .post("/internal/deduct")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload_zero)
        .await;
    assert_eq!(res_zero.status_code(), StatusCode::BAD_REQUEST);

    // Valid deduction (will fail with 400 Insufficient funds since balance is 0)
    let payload = json!({
        "actor_id": actor_id,
        "asset_id": null,
        "amount": 50,
        "generation_type": "text"
    });
    let res = server
        .post("/internal/deduct")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;
    assert_eq!(res.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_internal_api_purchase_functional() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    let payload = json!({
        "buyer": actor_id,
        "item_id": Uuid::new_v4(),
        "idempotency_key": "test_idemp_purchase"
    });

    let res = server
        .post("/internal/purchase")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    // Endpoint exists, business logic fails due to missing item -> 500 INTERNAL_SERVER_ERROR
    assert_eq!(res.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_internal_api_lora_train() {
    let (server, _tdir) = setup_test_server().await;

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    let payload = json!({
        "base_model": "base_model_x",
        "dataset_id": "dataset_y",
        "params": {}
    });

    let res = server
        .post("/internal/lora-train")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    // Initially fails with 404 Not Found since endpoint is missing
    assert_eq!(res.status_code(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_internal_api_validate_activity() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    // 先にコインをチャージして残高を増やす
    let charge_payload = json!({
        "actor_id": actor_id,
        "amount": 500,
        "currency": "coin",
        "stripe_event_id": "evt_validate_activity_test",
        "idempotency_key": "idemp_validate_activity_test"
    });
    let charge_res = server
        .post("/internal/coin-charge")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&charge_payload)
        .await;
    assert_eq!(charge_res.status_code(), StatusCode::OK);

    let payload = json!({
        "actor_id": actor_id,
        "activity_type": "generation",
        "amount": 100
    });

    let res = server
        .post("/internal/validate-activity")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    // GREENフェーズで 200 OK (MockCommerceEngineが常にOk(())を返すため) が返ることを期待。
    assert_eq!(res.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_internal_api_validate_activity_missing_cert() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let payload = json!({
        "actor_id": actor_id,
        "activity_type": "generation",
        "amount": 100
    });

    // 証明書なし (Missing Header) -> 403 Forbidden
    let res = server
        .post("/internal/validate-activity")
        .json(&payload)
        .await;

    assert_eq!(res.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_internal_api_validate_activity_invalid_type() {
    let (server, _tdir) = setup_test_server().await;
    let actor_id = Uuid::new_v4();

    let cert = OxiLeanProofCertificate::generate(
        "test-edge-node".to_string(),
        950,
        chrono::Utc::now().to_rfc3339(),
        "test_secret_key",
    );
    let cert_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        serde_json::to_string(&cert).unwrap(),
    );

    let payload = json!({
        "actor_id": actor_id,
        "activity_type": "invalid_type_xyz",
        "amount": 100
    });

    let res = server
        .post("/internal/validate-activity")
        .add_header(
            header::HeaderName::from_static("x-oxilean-proof-certificate"),
            header::HeaderValue::from_str(&cert_b64).unwrap(),
        )
        .json(&payload)
        .await;

    // 不正なactivity_type -> 400 or 500 (200 OK にはならない)
    assert_ne!(res.status_code(), StatusCode::OK);
}
