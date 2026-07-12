/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use super::types::{
    BalanceResponse, CoinChargeRequest, DailyStatsResponse, DeductCostRequest,
    UpdateMonthlySpendLimitRequest,
};
use crate::state::SharedState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use chrono::Utc;
use commerce_protocol::identity::ActorId;
use nurture_bridge::commerce::CommerceEngine;
use nurture_core::effective_daily_limit;
use tracing::{error, info};
use uuid::Uuid;

pub async fn get_balance(
    Path(actor_id): Path<Uuid>,
    Extension(state): axum::Extension<SharedState>,
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

pub async fn get_daily_stats(
    Path(actor_id): Path<Uuid>,
    Extension(state): axum::Extension<SharedState>,
) -> impl IntoResponse {
    let policy = state.policy.read().await;
    match state.ledger.get_balance(&ActorId(actor_id)).await {
        Ok(wallet) => (
            StatusCode::OK,
            Json(DailyStatsResponse {
                spent_today: wallet.spent_today,
                daily_limit: effective_daily_limit(wallet.daily_limit, policy.daily_spend_limit),
            }),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to get daily stats for {}: {}", actor_id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn charge_coins(
    Extension(state): axum::Extension<SharedState>,
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
        memo: None,
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

pub async fn deduct_cost(
    Extension(state): axum::Extension<SharedState>,
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
                || msg.contains("monthly spend limit")
                || msg.contains("greater than zero")
            {
                (StatusCode::BAD_REQUEST, "Deduction rejected").into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Deduction failed").into_response()
            }
        }
    }
}

pub async fn update_monthly_spend_limit(
    Extension(state): axum::Extension<SharedState>,
    Json(payload): Json<UpdateMonthlySpendLimitRequest>,
) -> impl IntoResponse {
    let mut policy = state.policy.read().await.clone();
    policy.monthly_spend_limit = payload.monthly_spend_limit;
    if let Err(e) = policy.validate() {
        error!("❌ [Internal/Policy] Invalid monthly_spend_limit: {}", e);
        return (StatusCode::BAD_REQUEST, "Invalid policy").into_response();
    }

    match state.commerce_engine.reload_policy(policy).await {
        Ok(_) => {
            info!(
                "✅ [Internal/Policy] monthly_spend_limit updated to {}",
                payload.monthly_spend_limit
            );
            (StatusCode::OK, "Success").into_response()
        }
        Err(e) => {
            error!("❌ [Internal/Policy] reload_policy failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
