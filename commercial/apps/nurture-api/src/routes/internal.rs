use crate::state::SharedState;
use axum::{
    extract::{Path, Request},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::Utc;
use commerce_protocol::identity::ActorId;
use nurture_bridge::commerce::CommerceEngine;
use nurture_bridge::oxilean::OxiLeanProofCertificate;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

pub fn internal_routes() -> Router {
    Router::new()
        .route("/balance/:actor_id", get(get_balance))
        .route("/daily-stats/:actor_id", get(get_daily_stats))
        .route("/coin-charge", post(charge_coins))
        .route("/escrow-create", post(create_escrow))
        .route("/escrow-release", post(release_escrow))
        .route("/escrow-refund", post(refund_escrow))
        .route("/escrow-list/:actor_id", get(list_escrows))
        .route("/deduct", post(deduct_cost))
        .route("/upload", post(upload_handler))
        .route("/forget/:actor_id", post(forget_actor))
        .route("/oxilean/status", get(get_oxilean_status))
        .route("/purchase", post(internal_purchase))
        .route("/transfer", post(transfer_coins))
        .route("/points/:actor_id", get(get_points))
        .route("/withdraw-points", post(withdraw_points))
        .route(
            "/transaction-history/:actor_id",
            get(get_transaction_history),
        )
        .route("/instant-refund", post(instant_refund))
        .route("/lora-train", post(internal_lora_train))
        .route("/validate-activity", post(internal_validate_activity))
        .nest("/asset", crate::routes::asset::asset_routes())
        .layer(middleware::from_fn(require_oxp_certificate))
}

async fn require_oxp_certificate(
    Extension(state): Extension<SharedState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = req.headers();

    // 1. Extract Header
    let cert_b64 = headers
        .get("x-oxilean-proof-certificate")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;

    // 2. Decode Base64
    let cert_json = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cert_b64)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 3. Deserialize JSON
    let cert: OxiLeanProofCertificate =
        serde_json::from_slice(&cert_json).map_err(|_| StatusCode::BAD_REQUEST)?;

    // 4. Verify Signature with Nurture Secret
    if !cert.verify(state.internal_secret.expose_secret()) {
        tracing::warn!(
            "OxiLean Certificate verification failed for {}",
            cert.subject_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // 5. Check OXP Score Threshold
    if cert.oxp_score < 900 {
        tracing::warn!(
            "OxiLean Certificate OXP score too low: {} < 900",
            cert.oxp_score
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // 6. Check Timestamp Freshness (prevent replay attacks)
    let cert_time = cert
        .timestamp
        .parse::<chrono::DateTime<Utc>>()
        .map_err(|_| {
            tracing::warn!("Invalid timestamp format in OxiLean Certificate");
            StatusCode::BAD_REQUEST
        })?;

    let now = Utc::now();
    let age = now.signed_duration_since(cert_time).num_seconds();

    // 証明書は5分(300秒)以内に発行されたものでなければならない
    if !(-60..=300).contains(&age) {
        tracing::warn!(
            "OxiLean Certificate is stale or from the future (age: {}s)",
            age
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: u64,
}

#[derive(Serialize)]
pub struct DailyStatsResponse {
    pub spent_today: u64,
    pub daily_limit: u64,
}

#[derive(Deserialize)]
pub struct CoinChargeRequest {
    pub actor_id: Uuid,
    pub amount: u64,
    pub currency: String,
    pub stripe_event_id: String,
    pub idempotency_key: String,
}

#[derive(Deserialize)]
pub struct EscrowCreateRequest {
    pub actor_id: Uuid,
    pub amount: u64,
}

#[derive(Serialize)]
pub struct EscrowCreateResponse {
    pub escrow_id: String,
}

// UploadResponse is imported from crate::mcp_tools

#[derive(Deserialize)]
pub struct EscrowReleaseRequest {
    pub escrow_id: String,
    pub recipient_id: Uuid,
}

#[derive(Deserialize)]
pub struct EscrowRefundRequest {
    pub escrow_id: String,
}

#[derive(Deserialize)]
pub struct DeductCostRequest {
    pub actor_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub amount: u64,
    pub generation_type: String,
}

// UploadRequest & UploadResponse are imported from crate::mcp_tools

async fn get_balance(
    Path(actor_id): Path<Uuid>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    match state.ledger.get_balance(&ActorId(actor_id)).await {
        Ok(wallet) => (
            StatusCode::OK,
            Json(BalanceResponse {
                balance: wallet.coin.balance,
            }),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to get balance for {}: {}", actor_id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn get_daily_stats(
    Path(actor_id): Path<Uuid>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    let policy = state.policy.read().await;
    match state.ledger.get_balance(&ActorId(actor_id)).await {
        Ok(wallet) => (
            StatusCode::OK,
            Json(DailyStatsResponse {
                spent_today: wallet.spent_today,
                daily_limit: policy.daily_spend_limit,
            }),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to get daily stats for {}: {}", actor_id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn charge_coins(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<CoinChargeRequest>,
) -> impl IntoResponse {
    // R-3: currency バリデーション — 現状は "coin" のみサポート
    if payload.currency != "coin" {
        error!(
            "❌ [Internal/CoinCharge] Unsupported currency: '{}'",
            payload.currency
        );
        return (
            StatusCode::BAD_REQUEST,
            format!("Unsupported currency: {}", payload.currency),
        );
    }

    // S-1: ゼロ金額チャージの拒否 — Idempotency キー消費と Ledger 汚染を防止
    if payload.amount == 0 {
        error!(
            "❌ [Internal/CoinCharge] Rejected zero-amount charge for {}",
            payload.actor_id
        );
        return (
            StatusCode::BAD_REQUEST,
            "Amount must be greater than zero".to_string(),
        );
    }

    info!(
        "🪙 [Internal/CoinCharge] Processing request for agent: {}, amount: {}, key: {}",
        payload.actor_id, payload.amount, payload.idempotency_key
    );

    // 1. Idempotency チェック (reserve → process → save のアトミックパターン)
    let store = state.idempotency.clone();
    match store.get_response(&payload.idempotency_key).await {
        Ok(Some(_)) => {
            info!(
                "ℹ️ [Internal/CoinCharge] Idempotency key {} already processed",
                payload.idempotency_key
            );
            return (StatusCode::OK, "Already processed".to_string());
        }
        Ok(None) => {} // proceed
        Err(e) => {
            error!("❌ [Internal/CoinCharge] Idempotency check failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Idempotency error".to_string(),
            );
        }
    }

    if let Err(e) = store
        .reserve_key(&payload.idempotency_key, chrono::Duration::hours(24))
        .await
    {
        // IdempotencyConflict = 既に処理中 or 完了済み
        info!(
            "ℹ️ [Internal/CoinCharge] Key {} already reserved (in progress): {}",
            payload.idempotency_key, e
        );
        return (StatusCode::OK, "Already in progress".to_string());
    }

    // 2. Stripe event ID から追跡可能な Namespace UUID v5 を生成 (C-8 fix)
    let entry_id = Uuid::new_v4();
    let tx_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, payload.stripe_event_id.as_bytes());

    let entry = nurture_core::ledger::LedgerEntry {
        id: entry_id,
        transaction_id: tx_id,
        asset_id: None,
        debit_account: state.system_actor_id,
        credit_account: ActorId(payload.actor_id),
        coin_amount: payload.amount,
        points_amount: 0,
        entry_type: nurture_core::ledger::EntryType::Charge,
        created_at: Utc::now(),
        debit_account_version: None,
    };

    match state.ledger.record_entry(&entry).await {
        Ok(_) => {
            info!(
                "✅ [Internal/CoinCharge] Added {} coins to {} (tx={})",
                payload.amount, payload.actor_id, tx_id
            );
            // 成功確定後のみ save_response — 失敗時はキーが InProgress のまま TTL 切れで再試行可能
            if let Err(e) = store
                .save_response(&payload.idempotency_key, 200, "Success".to_string())
                .await
            {
                error!(
                    "⚠️ [Internal/CoinCharge] save_response failed (non-fatal): {}",
                    e
                );
            }
            (StatusCode::OK, "Success".to_string())
        }
        Err(e) => {
            // レジャー失敗 = reserve は TTL 後に自動解放されるのでログのみ
            error!("❌ [Internal/CoinCharge] record_entry failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Ledger error".to_string(),
            )
        }
    }
}

async fn create_escrow(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<EscrowCreateRequest>,
) -> impl IntoResponse {
    // S-3: ゼロ金額エスクローの拒否
    if payload.amount == 0 {
        error!(
            "❌ [Internal/Escrow] Rejected zero-amount escrow for {}",
            payload.actor_id
        );
        return (
            StatusCode::BAD_REQUEST,
            "Escrow amount must be greater than zero",
        )
            .into_response();
    }

    // F-1/B-2: KYC AML Policy Check before allowing escrow
    let actor = commerce_protocol::identity::ActorId(payload.actor_id);
    match state.ekyc_store.is_verified(&actor).await {
        Ok(true) => { /* AML Passed */ }
        Ok(false) => {
            error!(
                "🚨 [Internal/Escrow] Escrow rejected: User {} has not completed KYC verification (AML Policy)",
                payload.actor_id
            );
            return (
                StatusCode::FORBIDDEN,
                "KYC verification required for escrow operations",
            )
                .into_response();
        }
        Err(e) => {
            error!("❌ [Internal/Escrow] Failed to verify KYC status: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Identity verification service unavailable",
            )
                .into_response();
        }
    }

    use nurture_bridge::commerce::CommerceEngine;
    match state
        .commerce_engine
        .escrow_create(payload.actor_id, payload.amount)
        .await
    {
        Ok(escrow_id) => (StatusCode::OK, Json(EscrowCreateResponse { escrow_id })).into_response(),
        Err(e) => {
            error!("❌ [Internal/Escrow] Create failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Escrow creation failed").into_response()
        }
    }
}

async fn release_escrow(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<EscrowReleaseRequest>,
) -> impl IntoResponse {
    if payload.escrow_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "escrow_id must not be empty").into_response();
    }

    use nurture_bridge::commerce::CommerceEngine;
    match state
        .commerce_engine
        .escrow_release(&payload.escrow_id, payload.recipient_id)
        .await
    {
        Ok(_) => (StatusCode::OK, "Success").into_response(),
        Err(e) => {
            let msg = e.to_string();
            error!("❌ [Internal/Escrow] Release failed: {}", msg);
            if msg.contains("not found") || msg.contains("already") || msg.contains("invalid") {
                (StatusCode::BAD_REQUEST, "Escrow release rejected").into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Escrow release failed").into_response()
            }
        }
    }
}

async fn deduct_cost(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<DeductCostRequest>,
) -> impl IntoResponse {
    // Defense-in-Depth: Fail-fast on invalid input before hitting the engine
    if payload.amount == 0 {
        error!(
            "❌ [Internal/Deduct] Rejected zero-amount deduction for {}",
            payload.actor_id
        );
        return (
            StatusCode::BAD_REQUEST,
            "Deduction amount must be greater than zero",
        )
            .into_response();
    }

    if payload.generation_type.is_empty() {
        return (StatusCode::BAD_REQUEST, "generation_type must not be empty").into_response();
    }

    use nurture_bridge::commerce::CommerceEngine;
    match state
        .commerce_engine
        .deduct_generation_cost(
            payload.actor_id,
            payload.asset_id,
            payload.amount,
            &payload.generation_type,
        )
        .await
    {
        Ok(_) => (StatusCode::OK, "Success").into_response(),
        Err(e) => {
            let msg = e.to_string();
            error!("❌ [Internal/Deduct] Failed: {}", msg);
            // Distinguish client-caused errors (4xx) from infrastructure errors (5xx)
            if msg.contains("Insufficient funds")
                || msg.contains("daily spend limit")
                || msg.contains("greater than zero")
            {
                (StatusCode::BAD_REQUEST, "Deduction rejected").into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Deduction failed").into_response()
            }
        }
    }
}

async fn upload_handler(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<crate::mcp_tools::UploadRequest>,
) -> impl IntoResponse {
    match crate::mcp_tools::handle_upload(state, payload).await {
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err(commerce_protocol::error::NurtureError::IdempotencyConflict { .. }) => (
            StatusCode::CONFLICT,
            "Concurrent request is processing this idempotency key.",
        )
            .into_response(),
        Err(commerce_protocol::error::NurtureError::PolicyViolation(msg)) => {
            (StatusCode::BAD_REQUEST, msg).into_response()
        }
        Err(commerce_protocol::error::NurtureError::CsamRejected { reason, .. }) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("CSAM violation: {}", reason),
        )
            .into_response(),
        Err(e) => {
            error!("❌ [Internal/Upload] Failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn forget_actor(
    Path(actor_id): Path<Uuid>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    info!("🗑️ [Internal/Forget] Request to forget actor: {}", actor_id);

    let actor_id_str = actor_id.to_string();

    // GDPR Right to be forgotten (Article 17)
    // 全 PII 関連テーブルをアトミックにパージする。
    // 物理アセット削除は DB commit 後に実行（外部 I/O は rollback 不可能なため）。
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("❌ [Internal/Forget] Failed to begin transaction: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to begin PII scrub transaction",
            )
                .into_response();
        }
    };

    // 1. KYC ステータスのパージ
    if let Err(e) = sqlx::query("DELETE FROM nurture_kyc_status WHERE actor_id = ?")
        .bind(&actor_id_str)
        .execute(&mut *tx)
        .await
    {
        error!(
            "❌ [Internal/Forget] Failed to scrub KYC for actor {}: {}",
            actor_id, e
        );
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to scrub KYC").into_response();
    }

    // 2. DRM ライセンスのパージ（暗号化された復号キーを含む PII）
    if let Err(e) = sqlx::query("DELETE FROM nurture_licenses WHERE owner_id = ?")
        .bind(&actor_id_str)
        .execute(&mut *tx)
        .await
    {
        error!(
            "❌ [Internal/Forget] Failed to scrub licenses for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrub licenses",
        )
            .into_response();
    }

    // 3. エスクロー処理:
    //    - pending 状態のエスクローは wallet に返金してから削除
    //    - released/refunded 状態のエスクローはそのまま削除
    let pending_escrows: Vec<(String, i64)> = match sqlx::query_as(
        "SELECT escrow_id, amount FROM nurture_escrows WHERE agent_id = ? AND status = 'pending'",
    )
    .bind(&actor_id_str)
    .fetch_all(&mut *tx)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            error!(
                "❌ [Internal/Forget] Failed to query pending escrows for actor {}: {}",
                actor_id, e
            );
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to query escrows").into_response();
        }
    };

    // pending エスクローの返金（wallet が存在しない場合は無視 — どうせ wallet も削除される）
    for (escrow_id, amount) in &pending_escrows {
        if let Err(e) =
            sqlx::query("UPDATE nurture_wallets SET balance = balance + ? WHERE actor_id = ?")
                .bind(amount)
                .bind(&actor_id_str)
                .execute(&mut *tx)
                .await
        {
            error!(
                "⚠️ [Internal/Forget] Failed to refund escrow {} (non-fatal, wallet will be deleted): {}",
                escrow_id, e
            );
            // 続行: wallet 自体も削除されるため、返金失敗は致命的ではない
        }

        // 🚨 WARNING-3 修正: Merkle 監査ハッシュチェーンの連続性を保持するため、
        //   Ledger に返金エントリを挿入する。
        //   これにより GDPR 忘却処理でも nurture_ledger の金融記録が完全になる。
        //   amount が 0 の場合は insert_ledger_refund_entry が Err を返すのでスキップする。
        if *amount > 0 {
            use nurture_infra::economy::bridge::NurtureCommerceBridge;
            if let Err(e) = NurtureCommerceBridge::insert_ledger_refund_entry_pub(
                &mut tx,
                &actor_id_str,
                *amount,
                Utc::now(),
            )
            .await
            {
                error!(
                    "⚠️ [Internal/Forget] Ledger refund entry failed for escrow {} (non-fatal, wallet will be deleted): {}",
                    escrow_id, e
                );
                // 返金の Ledger 記録失敗は非致命的: wallet はどうせ削除されるため導き続く
            }
        }
    }

    // 全エスクロー削除（pending/released/refunded すべて）
    if let Err(e) = sqlx::query("DELETE FROM nurture_escrows WHERE agent_id = ?")
        .bind(&actor_id_str)
        .execute(&mut *tx)
        .await
    {
        error!(
            "❌ [Internal/Forget] Failed to scrub escrows for actor {}: {}",
            actor_id, e
        );
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to scrub escrows").into_response();
    }

    // 4. ウォレットのパージ（残高・支出履歴を含む経済 PII）
    if let Err(e) = sqlx::query("DELETE FROM nurture_wallets WHERE actor_id = ?")
        .bind(&actor_id_str)
        .execute(&mut *tx)
        .await
    {
        error!(
            "❌ [Internal/Forget] Failed to scrub wallet for actor {}: {}",
            actor_id, e
        );
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to scrub wallet").into_response();
    }

    // 5. サブスクリプションと出金申請のパージ
    if let Err(e) = sqlx::query("DELETE FROM nurture_subscriptions WHERE actor_id = ?")
        .bind(&actor_id_str)
        .execute(&mut *tx)
        .await
    {
        error!(
            "❌ [Internal/Forget] Failed to scrub subscriptions for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrub subscriptions",
        )
            .into_response();
    }

    if let Err(e) = sqlx::query("DELETE FROM nurture_payout_requests WHERE actor_id = ?")
        .bind(&actor_id_str)
        .execute(&mut *tx)
        .await
    {
        error!(
            "❌ [Internal/Forget] Failed to scrub payout requests for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrub payout requests",
        )
            .into_response();
    }

    // 6. Customer PII の難読化 (emailはNULL化、stripe_customer_idは難読化)
    // レコード自体は購入履歴等の参照完全性のために残すが、PIIは消す
    // stripe_customer_id は NOT NULL 制約があるため、ダミー値（'purged_' + actor_id等）に置き換える
    if let Err(e) = sqlx::query("UPDATE nurture_customers SET email = NULL, stripe_customer_id = 'purged_' || actor_id WHERE actor_id = ?")
        .bind(&actor_id_str)
        .execute(&mut *tx)
        .await
    {
        error!("❌ [Internal/Forget] Failed to scrub customer PII for actor {}: {}", actor_id, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to scrub customer PII").into_response();
    }

    // 5. DB トランザクションのコミット
    if let Err(e) = tx.commit().await {
        error!(
            "❌ [Internal/Forget] Failed to commit PII scrub for actor {}: {}",
            actor_id, e
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to commit PII scrub",
        )
            .into_response();
    }

    info!(
        "✅ [Internal/Forget] DB PII scrub committed for actor {}. Starting physical asset purge...",
        actor_id
    );

    // 6. 物理アセットの非同期パージ（DB commit 後 — ロールバック不可能な外部 I/O）
    //    失敗しても DB 側は既にクリーン。孤立アセットは後日の GC バッチで回収可能。
    if let Err(e) = state
        .asset_storage
        .delete_assets_for_actor(&commerce_protocol::identity::ActorId(actor_id))
        .await
    {
        error!(
            "⚠️ [Internal/Forget] Physical asset purge failed for actor {} (DB already clean): {}",
            actor_id, e
        );
        // HTTP 200 を返す: DB はクリーン済み。物理アセットはバックグラウンド GC で回収。
        return (
            StatusCode::OK,
            "PII scrubbed from DB. Physical asset purge deferred due to storage error.",
        )
            .into_response();
    }

    info!(
        "✅ [Internal/Forget] Complete GDPR purge finished for actor {}",
        actor_id
    );
    (StatusCode::OK, "PII and assets successfully purged").into_response()
}

async fn list_escrows(
    Path(actor_id): Path<Uuid>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    match state.commerce_engine.list_escrows(actor_id).await {
        Ok(escrows) => (StatusCode::OK, Json(escrows)).into_response(),
        Err(e) => {
            error!(
                "❌ [Internal/EscrowList] Failed to list escrows for {}: {:?}",
                actor_id, e
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch escrows").into_response()
        }
    }
}

async fn refund_escrow(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<EscrowRefundRequest>,
) -> impl IntoResponse {
    match state
        .commerce_engine
        .escrow_refund(&payload.escrow_id)
        .await
    {
        Ok(_) => {
            info!(
                "✅ [Internal/EscrowRefund] Successfully refunded escrow {}",
                payload.escrow_id
            );
            (StatusCode::OK, "Escrow successfully refunded").into_response()
        }
        Err(e) => {
            error!(
                "❌ [Internal/EscrowRefund] Failed to refund escrow {}: {:?}",
                payload.escrow_id, e
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to refund escrow").into_response()
        }
    }
}

async fn get_oxilean_status(
    axum::extract::Extension(state): axum::extract::Extension<crate::state::SharedState>,
) -> impl IntoResponse {
    // 環境変数はリクエストパスで読み込むが、gRPC 接続は connect_lazy で効率化
    let host = state.shadow_clone_grpc_host.clone();
    let port = state.shadow_clone_grpc_port.clone();
    let addr = format!("http://{}:{}", host, port);
    let auth_token = state
        .a2a_auth_token
        .clone()
        .map(|s| {
            use secrecy::ExposeSecret;
            s.expose_secret().to_string()
        })
        .unwrap_or_default();

    let endpoint = match tonic::transport::Endpoint::from_shared(addr) {
        Ok(ep) => ep,
        Err(e) => {
            tracing::error!(error = %e, "❌ [OxiLean] Invalid gRPC endpoint configuration");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "Internal configuration error" })),
            )
                .into_response();
        }
    };

    // connect_lazy: TCP ハンドシェイクをリクエスト到着まで遅延し、コネクションプーリングを活用
    let channel = endpoint.connect_lazy();
    let mut client =
        aiome_core_contracts::a2a::internal::proof_verifier_client::ProofVerifierClient::new(
            channel,
        );

    let mut request =
        tonic::Request::new(aiome_core_contracts::a2a::internal::GetOxiLeanStatusRequest {});
    if !auth_token.is_empty() {
        if let Ok(metadata_val) = tonic::metadata::MetadataValue::try_from(&auth_token) {
            request.metadata_mut().insert("authorization", metadata_val);
        }
    }

    match client.get_oxi_lean_status(request).await {
        Ok(response) => {
            let next_oxp = response.into_inner().current_oxp;
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({ "current_oxp": next_oxp })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "❌ [OxiLean] Failed to fetch status from Shadow Worker");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(
                    serde_json::json!({ "error": "Shadow Worker is temporarily unavailable" }),
                ),
            )
                .into_response()
        }
    }
}

// ==========================================
// Proxy Endpoints for Aiome v1.1
// ==========================================

#[derive(Deserialize)]
pub struct TransferRequest {
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub amount: u64,
    /// Aiome 側で生成された冪等性キー（将来の重複防止用に予約）
    #[serde(default, rename = "idempotency_key")]
    pub _idempotency_key: Option<String>,
}

#[derive(Serialize)]
pub struct TransferResponse {
    pub transaction_id: String,
}

async fn transfer_coins(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<TransferRequest>,
) -> impl IntoResponse {
    match state
        .commerce_engine
        .transfer(payload.from_id, payload.to_id, payload.amount)
        .await
    {
        Ok(tx_id) => (
            StatusCode::OK,
            Json(TransferResponse {
                transaction_id: tx_id,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Transfer failed");
            map_commerce_error(e)
        }
    }
}

#[derive(Deserialize)]
pub struct InstantRefundRequest {
    pub transaction_id: String,
    pub actor_id: Uuid,
    #[serde(default, rename = "idempotency_key")]
    pub _idempotency_key: Option<String>,
}

async fn instant_refund(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<InstantRefundRequest>,
) -> impl IntoResponse {
    match state
        .commerce_engine
        .instant_refund(&payload.transaction_id, payload.actor_id)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "success"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Instant refund failed");
            map_commerce_error(e)
        }
    }
}

#[derive(Deserialize)]
pub struct WithdrawPointsRequest {
    pub actor_id: Uuid,
    pub points: u64,
    #[serde(default, rename = "idempotency_key")]
    pub _idempotency_key: Option<String>,
}

async fn withdraw_points(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<WithdrawPointsRequest>,
) -> impl IntoResponse {
    match state
        .commerce_engine
        .withdraw_points(payload.actor_id, payload.points)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "success"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Withdraw points failed");
            map_commerce_error(e)
        }
    }
}

async fn get_points(
    Path(actor_id): Path<Uuid>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    match state.commerce_engine.get_points(actor_id).await {
        Ok(points) => (StatusCode::OK, Json(points)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, actor_id = %actor_id, "Get points failed");
            map_commerce_error(e)
        }
    }
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<u32>,
}

async fn get_transaction_history(
    Path(actor_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<HistoryQuery>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50).min(200);
    match state
        .commerce_engine
        .get_transaction_history(actor_id, limit)
        .await
    {
        Ok(history) => (StatusCode::OK, Json(history)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, actor_id = %actor_id, "Get transaction history failed");
            map_commerce_error(e)
        }
    }
}

#[derive(Deserialize)]
pub struct PurchaseS2SRequest {
    pub buyer: Uuid,
    pub item_id: Uuid,
    pub idempotency_key: Option<String>,
}

#[derive(Serialize)]
pub struct PurchaseS2SResponse {
    pub transaction_id: String,
}

async fn internal_purchase(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<PurchaseS2SRequest>,
) -> impl IntoResponse {
    let req = commerce_protocol::mcp_commerce::BuyRequest {
        buyer: commerce_protocol::identity::ActorId(payload.buyer),
        item_id: payload.item_id,
        idempotency_key: payload.idempotency_key,
        use_escrow: Some(false),
    };

    match crate::mcp_tools::buy::handle_buy(state, req).await {
        Ok(res) => (
            StatusCode::OK,
            Json(PurchaseS2SResponse {
                transaction_id: res.transaction_id.to_string(),
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Internal purchase failed");
            let (status, msg) = match e {
                commerce_protocol::error::NurtureError::OptimisticLockConflict { .. } => {
                    (StatusCode::CONFLICT, e.to_string())
                }
                commerce_protocol::error::NurtureError::IdempotencyConflict { .. } => {
                    (StatusCode::CONFLICT, e.to_string())
                }
                commerce_protocol::error::NurtureError::PolicyViolation(ref r) => {
                    (StatusCode::BAD_REQUEST, r.clone())
                }
                commerce_protocol::error::NurtureError::CsamRejected { ref reason, .. } => {
                    (StatusCode::UNPROCESSABLE_ENTITY, reason.clone())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            };
            (status, Json(serde_json::json!({"error": msg}))).into_response()
        }
    }
}

/// AiomeError を適切な HTTP ステータスコードにマッピングするヘルパー。
/// **注意**: この関数は `/internal/*` ルート（OXP 認証済みの内部 API）でのみ使用される。
/// 外部公開 API には使用しないこと（内部エラーメッセージの漏洩リスク）。
fn map_commerce_error(e: nurture_bridge::error::AiomeError) -> Response {
    use nurture_bridge::error::AiomeError;
    let (status, msg) = match &e {
        AiomeError::Validation { reason } => (StatusCode::BAD_REQUEST, reason.clone()),
        AiomeError::Infrastructure { reason } => {
            (StatusCode::INTERNAL_SERVER_ERROR, reason.clone())
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

#[derive(Deserialize)]
pub struct LoraTrainRequest {
    pub base_model: String,
    pub dataset_id: String,
    pub params: serde_json::Value,
}

#[derive(Serialize)]
pub struct LoraTrainResponse {
    pub job_id: String,
}

async fn internal_lora_train(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<LoraTrainRequest>,
) -> impl IntoResponse {
    let params_str = payload.params.to_string();
    match state
        .job_queue
        .enqueue(
            "lora-train",
            &payload.base_model,
            &payload.dataset_id,
            Some(&params_str),
            None,
            None,
            1,
        )
        .await
    {
        Ok(job_id) => (StatusCode::ACCEPTED, Json(LoraTrainResponse { job_id })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to enqueue lora-train job");
            map_commerce_error(e)
        }
    }
}

#[derive(Deserialize)]
pub struct ValidateActivityRequest {
    pub actor_id: Uuid,
    pub activity_type: String,
    pub amount: u64,
}

async fn internal_validate_activity(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<ValidateActivityRequest>,
) -> impl IntoResponse {
    use nurture_bridge::commerce::CommerceEngine;
    match state
        .commerce_engine
        .validate_activity(payload.actor_id, &payload.activity_type, payload.amount)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "success"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Activity validation failed");
            map_commerce_error(e)
        }
    }
}
