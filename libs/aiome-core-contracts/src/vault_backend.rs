/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use std::fmt::Debug;
use uuid::Uuid;
use zeroize::Zeroizing;

/// ボイスアセットおよびクリプトグラフィックの DEK（Data Encryption Key）ストレージを抽象化するバックエンド。
/// Phase A: SqliteBackend, Phase C: HashiCorpVaultBackend / KmsBackend へ対応。
#[async_trait]
pub trait VaultBackend: Send + Sync + Debug {
    /// 指定されたアセット ID の DEK (Data Encryption Key) を取得し、復号状態で返す。
    async fn get_dek(&self, asset_id: Uuid) -> Result<Zeroizing<Vec<u8>>, AiomeError>;

    /// 新しいアセットの DEK (Data Encryption Key) を登録・保存する。
    async fn store_dek(&self, asset_id: Uuid, dek: &[u8]) -> Result<(), AiomeError>;

    /// バックエンド自体のヘルスチェックを行う。
    async fn health_check(&self) -> Result<(), AiomeError>;
}
