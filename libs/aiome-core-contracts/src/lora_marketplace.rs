/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// LoRA 出品ステータス
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
pub enum ListingStatus {
    /// 出品中（購入可能）
    Open,
    /// 売却済み
    Sold,
    /// 出品取り下げ
    Delisted,
}

/// LoRA 購入ステータス
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
pub enum PurchaseStatus {
    /// エスクロー中（資金一時保留）
    Escrowed,
    /// 完了（ハッシュ検証通過・転送済み）
    Completed,
    /// 返金済み（ハッシュ不一致等）
    Refunded,
}

/// LoRA 出品情報
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoraListing {
    /// 出品 ID
    #[schema(value_type = String)]
    pub id: Uuid,
    /// 出品者（Agent ID）
    #[schema(value_type = String)]
    pub seller_id: Uuid,
    /// ArtifactStore 上の成果物パス
    pub adapter_path: String,
    /// モデルファミリー ("gemma4", "qwen3.5" 等)
    pub model_family: String,
    /// ベースモデル名 ("gemma4:26b" 等)
    pub base_model: String,
    /// 出品タイトル
    pub title: String,
    /// 説明
    pub description: String,
    /// コイン価格
    pub price_coins: u64,
    /// SHA-256 完全性ハッシュ
    pub adapter_hash: String,
    /// ファイルサイズ（バイト）
    pub adapter_size_bytes: u64,
    /// タグ (例: ["japanese", "creative-writing"])
    pub tags: Vec<String>,
    /// ステータス
    pub status: ListingStatus,
    /// 出品日時
    #[schema(value_type = String)]
    pub created_at: DateTime<Utc>,
}

/// LoRA 購入記録
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoraPurchase {
    /// 購入 ID
    #[schema(value_type = String)]
    pub id: Uuid,
    /// 出品 ID
    #[schema(value_type = String)]
    pub listing_id: Uuid,
    /// 購入者（Agent ID）
    #[schema(value_type = String)]
    pub buyer_id: Uuid,
    /// エスクロー ID
    pub escrow_id: String,
    /// ステータス
    pub status: PurchaseStatus,
    /// 購入日時
    #[schema(value_type = String)]
    pub purchased_at: DateTime<Utc>,
}

/// 出品フィルター
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListingFilter {
    /// モデルファミリーで絞り込み
    pub model_family: Option<String>,
    /// ステータスで絞り込み
    pub status: Option<ListingStatus>,
    /// 出品者で絞り込み
    #[schema(value_type = Option<String>)]
    pub seller_id: Option<Uuid>,
    /// 購入者で絞り込み（購入履歴用）
    #[schema(value_type = Option<String>)]
    pub buyer_id: Option<Uuid>,
    /// 最大取得件数
    pub limit: Option<u32>,
}

/// LoRA マーケットプレイス・トレイト
///
/// AI 間の LoRA アダプター（人格 IP）取引を安全に仲介する。
/// - エスクロー決済による資金保護
/// - SHA-256 ハッシュによる改竄検知
/// - PathSandbox によるファイルパス検証
#[async_trait]
pub trait LoraMarketplace: Send + Sync {
    /// LoRA を出品する
    async fn publish_listing(&self, listing: LoraListing) -> Result<Uuid, AiomeError>;

    /// 出品一覧を取得する（フィルタ対応）
    async fn list_listings(&self, filter: ListingFilter) -> Result<Vec<LoraListing>, AiomeError>;

    /// 購入を開始する（エスクロー生成）
    async fn purchase(&self, listing_id: Uuid, buyer_id: Uuid) -> Result<LoraPurchase, AiomeError>;

    /// 購入を完了する（ハッシュ検証 → Vault コピー → エスクロー解放）
    async fn complete_purchase(&self, purchase_id: Uuid, caller_id: Uuid)
        -> Result<(), AiomeError>;

    /// 出品を取り下げる
    async fn delist(&self, listing_id: Uuid, seller_id: Uuid) -> Result<(), AiomeError>;
}
