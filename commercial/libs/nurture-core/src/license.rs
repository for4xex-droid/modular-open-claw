/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// アセット利用ライセンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLicense {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub asset_id: Uuid,
    pub owner_id: ActorId,
    /// 復号キー (Base64 または暗号化された状態を想定)
    pub decryption_key: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait LicenseStore: Send + Sync {
    /// 新規ライセンスを発行する
    async fn issue_license(&self, license: &AssetLicense) -> Result<(), NurtureError>;

    /// 指定したオーナーのアセットの有効なライセンスを取得する
    async fn get_license(
        &self,
        owner: &ActorId,
        asset_id: &Uuid,
    ) -> Result<Option<AssetLicense>, NurtureError>;

    /// ライセンスを無効化する
    async fn revoke_license(&self, license_id: &Uuid) -> Result<(), NurtureError>;

    /// 古いライセンスを無効化し、新しいライセンスをアトミックに発行する（所有権移転等）
    async fn transfer_license(
        &self,
        old_license_id: &Uuid,
        new_license: &AssetLicense,
    ) -> Result<(), NurtureError>;

    /// 期限切れ、または無効化されたライセンスをクリーンアップする (GC)
    async fn purge_expired_licenses(&self) -> Result<u64, NurtureError>;
}
