/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ギルド（エージェント間連携チーム）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Guild {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: String,
}

/// ギルドメンバー
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GuildMember {
    pub guild_id: Uuid,
    pub agent_id: Uuid,
    pub role: String,
    pub joined_at: String,
}

/// ギルド操作の抽象インターフェース
#[async_trait]
pub trait SyndicateOps: Send + Sync {
    /// ギルドを新規作成する
    async fn create_guild(
        &self,
        name: String,
        description: Option<String>,
        owner_id: Uuid,
    ) -> Result<Uuid, AiomeError>;

    /// ギルドを削除する
    async fn delete_guild(&self, guild_id: Uuid, requester_id: Uuid) -> Result<(), AiomeError>;

    /// メンバーを追加する
    async fn add_member(
        &self,
        guild_id: Uuid,
        agent_id: Uuid,
        role: String,
        requester_id: Uuid,
    ) -> Result<(), AiomeError>;

    /// メンバーを削除する
    async fn remove_member(
        &self,
        guild_id: Uuid,
        agent_id: Uuid,
        requester_id: Uuid,
    ) -> Result<(), AiomeError>;

    /// ギルド一覧を取得する
    async fn fetch_guilds(&self) -> Result<Vec<Guild>, AiomeError>;

    /// ギルドのメンバー一覧を取得する
    async fn fetch_members(&self, guild_id: Uuid) -> Result<Vec<GuildMember>, AiomeError>;
}
