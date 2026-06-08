/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use super::types::{
    HistoryQuery, InstantRefundRequest, LoraTrainRequest, LoraTrainResponse, PurchaseS2SRequest,
    PurchaseS2SResponse, TransferRequest, TransferResponse, ValidateActivityRequest,
    WithdrawPointsRequest,
};
use crate::state::SharedState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use nurture_bridge::commerce::CommerceEngine;
use secrecy::ExposeSecret;
use tracing::error;
use uuid::Uuid;

pub async fn upload_handler(
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

pub async fn get_oxilean_status(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    // 環境変数はリクエストパスで読み込むが、gRPC 接続は connect_lazy で効率化
    let host = state.shadow_clone_grpc_host.clone();
    let port = state.shadow_clone_grpc_port.clone();
    let addr = format!("http://{}:{}", host, port);
    let auth_token = state
        .a2a_auth_token
        .clone()
        .map(|s| s.expose_secret().to_string())
        .unwrap_or_default();

    let endpoint = match tonic::transport::Endpoint::from_shared(addr) {
        Ok(ep) => ep,
        Err(e) => {
            tracing::error!(error = %e, "❌ [OxiLean] Invalid gRPC endpoint configuration");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Internal configuration error" })),
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
                Json(serde_json::json!({ "current_oxp": next_oxp })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "❌ [OxiLean] Failed to fetch status from Shadow Worker");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "Shadow Worker is temporarily unavailable" })),
            )
                .into_response()
        }
    }
}

pub async fn transfer_coins(
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

pub async fn instant_refund(
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

pub async fn withdraw_points(
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

pub async fn get_points(
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

pub async fn get_transaction_history(
    Path(actor_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
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

pub async fn internal_purchase(
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
pub fn map_commerce_error(e: nurture_bridge::error::AiomeError) -> Response {
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

pub async fn internal_lora_train(
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

pub async fn internal_validate_activity(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<ValidateActivityRequest>,
) -> impl IntoResponse {
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
