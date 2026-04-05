/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::syndicate::{Guild, GuildMember, SyndicateOps};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use tracing::error;
use uuid::Uuid;

pub struct SqliteSyndicateStore {
    pool: SqlitePool,
}

impl SqliteSyndicateStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SyndicateOps for SqliteSyndicateStore {
    async fn create_guild(
        &self,
        name: String,
        description: Option<String>,
        owner_id: Uuid,
    ) -> Result<Uuid, AiomeError> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let owner_str = owner_id.to_string();

        sqlx::query("INSERT INTO guilds (id, name, description, owner_id) VALUES (?, ?, ?, ?)")
            .bind(&id_str)
            .bind(name)
            .bind(description)
            .bind(&owner_str)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to create guild: {}", e);
                AiomeError::Infrastructure {
                    reason: e.to_string(),
                }
            })?;

        // Add owner as a member with 'admin' role
        self.add_member(id, owner_id, "admin".to_string(), owner_id)
            .await?;

        Ok(id)
    }

    async fn delete_guild(&self, guild_id: Uuid, requester_id: Uuid) -> Result<(), AiomeError> {
        let guild_id_str = guild_id.to_string();
        let requester_str = requester_id.to_string();

        // Security check: Only owner can delete
        let row = sqlx::query("SELECT owner_id FROM guilds WHERE id = ?")
            .bind(&guild_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if let Some(row) = row {
            let owner_id: String = row.get("owner_id");
            if owner_id != requester_str {
                return Err(AiomeError::Unauthorized {
                    reason: "Only owner can delete guild".into(),
                });
            }
        } else {
            return Err(AiomeError::NotFound {
                reason: "Guild not found".into(),
            });
        }

        sqlx::query("DELETE FROM guilds WHERE id = ?")
            .bind(&guild_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        Ok(())
    }

    async fn add_member(
        &self,
        guild_id: Uuid,
        agent_id: Uuid,
        role: String,
        requester_id: Uuid,
    ) -> Result<(), AiomeError> {
        let guild_id_str = guild_id.to_string();
        let agent_id_str = agent_id.to_string();
        let requester_str = requester_id.to_string();

        // Validation: Requester must be a member or owner?
        // For MVP, if adding ANYONE, let's check if requester is owner of the guild
        // Skip for initial owner addition (where agent_id == requester_id == owner_id)
        let owner_check = sqlx::query("SELECT owner_id FROM guilds WHERE id = ?")
            .bind(&guild_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if let Some(row) = owner_check {
            let owner_id: String = row.get("owner_id");
            // If it's not the initial owner addition, check permission
            if owner_id != requester_str && agent_id_str != owner_id {
                // For now, only owner can add members
                return Err(AiomeError::Unauthorized {
                    reason: "Only owner can add members".into(),
                });
            }
        }

        sqlx::query(
            "INSERT OR REPLACE INTO guild_members (guild_id, agent_id, role) VALUES (?, ?, ?)",
        )
        .bind(&guild_id_str)
        .bind(&agent_id_str)
        .bind(role)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(())
    }

    async fn remove_member(
        &self,
        guild_id: Uuid,
        agent_id: Uuid,
        requester_id: Uuid,
    ) -> Result<(), AiomeError> {
        let guild_id_str = guild_id.to_string();
        let agent_id_str = agent_id.to_string();
        let requester_str = requester_id.to_string();

        // Only owner can remove someone, OR you can remove yourself
        let owner_check = sqlx::query("SELECT owner_id FROM guilds WHERE id = ?")
            .bind(&guild_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if let Some(row) = owner_check {
            let owner_id: String = row.get("owner_id");
            if owner_id != requester_str && agent_id_str != requester_str {
                return Err(AiomeError::Unauthorized {
                    reason: "Insufficient permissions to remove member".into(),
                });
            }
            // Cannot remove owner?
            if agent_id_str == owner_id && requester_str != owner_id {
                return Err(AiomeError::Unauthorized {
                    reason: "Cannot remove guild owner".into(),
                });
            }
        }

        sqlx::query("DELETE FROM guild_members WHERE guild_id = ? AND agent_id = ?")
            .bind(&guild_id_str)
            .bind(&agent_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        Ok(())
    }

    async fn fetch_guilds(&self) -> Result<Vec<Guild>, AiomeError> {
        let rows = sqlx::query("SELECT id, name, description, owner_id, created_at FROM guilds")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        let mut guilds = Vec::new();
        for row in rows {
            guilds.push(Guild {
                id: Uuid::parse_str(row.get("id")).unwrap(), // allow-anti-pattern
                name: row.get("name"),
                description: row.get("description"),
                owner_id: Uuid::parse_str(row.get("owner_id")).unwrap(), // allow-anti-pattern
                created_at: row.get("created_at"),
            });
        }
        Ok(guilds)
    }

    async fn fetch_members(&self, guild_id: Uuid) -> Result<Vec<GuildMember>, AiomeError> {
        let guild_id_str = guild_id.to_string();
        let rows = sqlx::query(
            "SELECT guild_id, agent_id, role, joined_at FROM guild_members WHERE guild_id = ?",
        )
        .bind(&guild_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        let mut members = Vec::new();
        for row in rows {
            members.push(GuildMember {
                guild_id: Uuid::parse_str(row.get("guild_id")).unwrap(), // allow-anti-pattern
                agent_id: Uuid::parse_str(row.get("agent_id")).unwrap(), // allow-anti-pattern
                role: row.get("role"),
                joined_at: row.get("joined_at"),
            });
        }
        Ok(members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create memory DB"); // allow-anti-pattern

        sqlx::query("CREATE TABLE guilds (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, owner_id TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)")
            .execute(&pool).await.unwrap(); // allow-anti-pattern
        sqlx::query("CREATE TABLE guild_members (guild_id TEXT NOT NULL, agent_id TEXT NOT NULL, role TEXT NOT NULL, joined_at DATETIME DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY (guild_id, agent_id))")
            .execute(&pool).await.unwrap(); // allow-anti-pattern

        pool
    }

    #[tokio::test]
    async fn test_create_and_fetch_guilds() {
        let pool = setup_test_db().await;
        let store = SqliteSyndicateStore::new(pool);

        let owner_id = Uuid::new_v4();
        let guild_id = store
            .create_guild("Test Guild".into(), Some("Desc".into()), owner_id)
            .await
            .unwrap(); // allow-anti-pattern

        let guilds = store.fetch_guilds().await.unwrap(); // allow-anti-pattern
        assert_eq!(guilds.len(), 1);
        assert_eq!(guilds[0].name, "Test Guild");
        assert_eq!(guilds[0].id, guild_id);

        // Check initial member
        let members = store.fetch_members(guild_id).await.unwrap(); // allow-anti-pattern
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].agent_id, owner_id);
        assert_eq!(members[0].role, "admin");
    }

    #[tokio::test]
    async fn test_guild_management_cycle() {
        let pool = setup_test_db().await;
        let store = SqliteSyndicateStore::new(pool);

        let owner_id = Uuid::new_v4();
        let other_agent_id = Uuid::new_v4();

        let guild_id = store
            .create_guild("Syndicate Alpha".into(), None, owner_id)
            .await
            .unwrap(); // allow-anti-pattern

        // Add member
        store
            .add_member(guild_id, other_agent_id, "worker".into(), owner_id)
            .await
            .unwrap(); // allow-anti-pattern
        let members = store.fetch_members(guild_id).await.unwrap(); // allow-anti-pattern
        assert_eq!(members.len(), 2);

        // Unauthorized add
        let result = store
            .add_member(guild_id, Uuid::new_v4(), "spy".into(), other_agent_id)
            .await;
        assert!(result.is_err());

        // Remove member
        store
            .remove_member(guild_id, other_agent_id, other_agent_id)
            .await
            .unwrap(); // allow-anti-pattern
        let members = store.fetch_members(guild_id).await.unwrap(); // allow-anti-pattern
        assert_eq!(members.len(), 1);

        // Delete guild (unauthorized)
        let result = store.delete_guild(guild_id, other_agent_id).await;
        assert!(result.is_err());

        // Delete guild (authorized)
        store.delete_guild(guild_id, owner_id).await.unwrap(); // allow-anti-pattern
        let guilds = store.fetch_guilds().await.unwrap(); // allow-anti-pattern
        assert_eq!(guilds.len(), 0);
    }
}
