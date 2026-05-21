/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! LoRA Marketplace API — 安心・安全なアダプター取引エンドポイント

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aiome_core_contracts::lora_marketplace::{
    ListingFilter, ListingStatus, LoraListing, LoraPurchase,
};

// --- Request / Response DTOs ---

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PublishListingRequest {
    /// アダプターファイルの Vault 相対パス
    pub adapter_path: String,
    /// モデルファミリー ("gemma4", "qwen3.5" 等)
    pub model_family: String,
    /// ベースモデル名 ("gemma4:26b" 等)
    pub base_model: String,
    /// 出品タイトル
    pub title: String,
    /// 説明
    #[serde(default)]
    pub description: String,
    /// コイン価格
    pub price_coins: u64,
    /// SHA-256 ハッシュ
    pub adapter_hash: String,
    /// ファイルサイズ（バイト）
    pub adapter_size_bytes: u64,
    /// タグ
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PurchaseRequest {
    /// 購入対象の出品 ID
    #[schema(value_type = String)]
    pub listing_id: Uuid,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PurchaseResponse {
    #[schema(value_type = String)]
    pub purchase_id: Uuid,
    pub escrow_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ListingQueryParams {
    pub family: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

// --- Handlers ---

/// [GET] /api/v1/lora/market — 出品一覧取得
#[utoipa::path(
    get,
    path = "/api/v1/lora/market",
    params(
        ("family" = Option<String>, Query, description = "Filter by model family (e.g. gemma4)"),
        ("status" = Option<String>, Query, description = "Filter by status (Open, Sold, Delisted)"),
        ("limit" = Option<u32>, Query, description = "Max records to return")
    ),
    responses(
        (status = 200, description = "Marketplace listings", body = Vec<LoraListing>)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn list_market(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Query(params): Query<ListingQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let marketplace = state.lora_marketplace.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "LoRA Marketplace not enabled".into(),
        }
    })?;

    let status_filter = params.status.as_deref().and_then(|s| match s {
        "Open" => Some(ListingStatus::Open),
        "Sold" => Some(ListingStatus::Sold),
        "Delisted" => Some(ListingStatus::Delisted),
        _ => None,
    });

    let filter = ListingFilter {
        model_family: params.family,
        status: status_filter,
        seller_id: None,
        buyer_id: None,
        limit: params.limit,
    };

    let listings = marketplace.list_listings(filter).await?;
    Ok(Json(listings))
}

/// [POST] /api/v1/lora/market/publish — LoRA を出品
#[utoipa::path(
    post,
    path = "/api/v1/lora/market/publish",
    request_body = PublishListingRequest,
    responses(
        (status = 201, description = "Listing published")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn publish_listing(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(mut req): Json<PublishListingRequest>,
) -> Result<impl IntoResponse, AppError> {
    let marketplace = state.lora_marketplace.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "LoRA Marketplace not enabled".into(),
        }
    })?;

    // 🛡️ [GlassWorm Shield] Sanitize text fields
    req.title = shared::guardrails::strip_invisible_unicode(&req.title).into_owned();
    req.description = shared::guardrails::strip_invisible_unicode(&req.description).into_owned();
    req.model_family = shared::guardrails::strip_invisible_unicode(&req.model_family).into_owned();
    req.base_model = shared::guardrails::strip_invisible_unicode(&req.base_model).into_owned();
    req.adapter_path = shared::guardrails::strip_invisible_unicode(&req.adapter_path).into_owned();

    // Input validation: price must be greater than zero
    if req.price_coins == 0 {
        return Err(AppError::bad_request(
            "price_coins must be greater than zero. Use a dedicated free-distribution flow for free adapters.",
        ));
    }

    let listing = LoraListing {
        id: Uuid::new_v4(),
        seller_id: auth.agent_id,
        adapter_path: req.adapter_path,
        model_family: req.model_family,
        base_model: req.base_model,
        title: req.title,
        description: req.description,
        price_coins: req.price_coins,
        adapter_hash: req.adapter_hash,
        adapter_size_bytes: req.adapter_size_bytes,
        tags: req.tags,
        status: ListingStatus::Open,
        created_at: chrono::Utc::now(),
    };

    let listing_id = marketplace.publish_listing(listing).await?;

    tracing::info!(
        "🏪 [LoraMarket] Agent {} published listing {}",
        auth.agent_id,
        listing_id
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "listing_id": listing_id.to_string() })),
    ))
}

/// [POST] /api/v1/lora/market/purchase — LoRA を購入（エスクロー開始）
#[utoipa::path(
    post,
    path = "/api/v1/lora/market/purchase",
    request_body = PurchaseRequest,
    responses(
        (status = 201, description = "Purchase escrowed", body = PurchaseResponse)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn purchase_listing(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<PurchaseRequest>,
) -> Result<impl IntoResponse, AppError> {
    let marketplace = state.lora_marketplace.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "LoRA Marketplace not enabled".into(),
        }
    })?;

    let purchase = marketplace.purchase(req.listing_id, auth.agent_id).await?;

    tracing::info!(
        "🛒 [LoraMarket] Agent {} purchased listing {}, escrow={}",
        auth.agent_id,
        req.listing_id,
        purchase.escrow_id
    );

    Ok((
        StatusCode::CREATED,
        Json(PurchaseResponse {
            purchase_id: purchase.id,
            escrow_id: purchase.escrow_id,
            status: "Escrowed".into(),
        }),
    ))
}

/// [POST] /api/v1/lora/market/complete/{purchase_id} — 購入完了（ハッシュ検証と資金移動）
#[utoipa::path(
    post,
    path = "/api/v1/lora/market/complete/{purchase_id}",
    params(
        ("purchase_id" = Uuid, Path, description = "The ID of the purchase to complete")
    ),
    responses(
        (status = 200, description = "Purchase completed successfully")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn complete_purchase(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(purchase_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let marketplace = state.lora_marketplace.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "LoRA Marketplace not enabled".into(),
        }
    })?;

    marketplace
        .complete_purchase(purchase_id, auth.agent_id)
        .await?;
    Ok(StatusCode::OK)
}

/// [DELETE] /api/v1/lora/market/{listing_id} — 出品取り下げ
#[utoipa::path(
    delete,
    path = "/api/v1/lora/market/{listing_id}",
    params(
        ("listing_id" = Uuid, Path, description = "The ID of the listing to delist")
    ),
    responses(
        (status = 200, description = "Listing removed")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn delist_listing(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(listing_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let marketplace = state.lora_marketplace.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "LoRA Marketplace not enabled".into(),
        }
    })?;

    marketplace.delist(listing_id, auth.agent_id).await?;
    Ok(StatusCode::OK)
}

/// [GET] /api/v1/lora/market/my-listings — 自分の出品一覧
#[utoipa::path(
    get,
    path = "/api/v1/lora/market/my-listings",
    responses(
        (status = 200, description = "Agent's own listings", body = Vec<LoraListing>)
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn my_listings(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<impl IntoResponse, AppError> {
    let marketplace = state.lora_marketplace.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "LoRA Marketplace not enabled".into(),
        }
    })?;

    let filter = ListingFilter {
        seller_id: Some(auth.agent_id),
        limit: Some(50),
        ..Default::default()
    };

    let listings = marketplace.list_listings(filter).await?;
    Ok(Json(listings))
}
