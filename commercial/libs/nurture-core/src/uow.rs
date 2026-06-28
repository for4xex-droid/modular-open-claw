/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use uuid::Uuid;

use crate::ledger::LedgerEntry;
use crate::license::AssetLicense;

/// トランザクション管理を行う抽象レイヤー
#[async_trait]
pub trait UowManager: Send + Sync {
    /// 新しいクロスドメイントランザクションを開始する
    async fn begin_uow(&self) -> Result<Box<dyn CommerceUow>, NurtureError>;
}

/// 複数のドメイン（LicenseStore, EconomyLedger）にまたがる原子的な操作を提供する Unit of Work。
/// 内部にデータベーストランザクションなどを保持し、`commit` または `rollback` が呼ばれるまでロックを管理する。
#[async_trait]
pub trait CommerceUow: Send {
    /// ライセンスの移転（revoke -> issue）をアトミックにスケジュールする
    async fn transfer_license(
        &mut self,
        old_license_id: &Uuid,
        new_license: &AssetLicense,
    ) -> Result<(), NurtureError>;

    /// 経済台帳への記録をアトミックにスケジュールする
    async fn record_batch(&mut self, entries: &[LedgerEntry]) -> Result<(), NurtureError>;

    /// トランザクションを確定（コミット）する
    async fn commit(self: Box<Self>) -> Result<(), NurtureError>;

    /// トランザクションを破棄（ロールバック）する
    async fn rollback(self: Box<Self>) -> Result<(), NurtureError>;
}
