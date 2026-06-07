/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::syndicate::{Guild, GuildMember, SyndicateOps};
use async_trait::async_trait;
use shared::db::DatabasePool;
use shared::{sql_exec, sql_fetch_all_map, sql_fetch_optional_map};
use sqlx::Row;
use tracing::error;
use uuid::Uuid;

pub struct UniversalSyndicateStore {
    pool: DatabasePool,
}

impl UniversalSyndicateStore {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SyndicateOps for UniversalSyndicateStore {
    async fn create_guild(
        &self,
        name: String,
        description: Option<String>,
        owner_id: Uuid,
    ) -> Result<Uuid, AiomeError> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let owner_str = owner_id.to_string();

        sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO guilds (id, name, description, owner_id) VALUES (?, ?, ?, ?)",
            pg: "INSERT INTO guilds (id, name, description, owner_id) VALUES ($1, $2, $3, $4)",
            &id_str,
            &name,
            &description,
            &owner_str
        )
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
        let owner_id_str = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT owner_id FROM guilds WHERE id = ?",
            |row| Ok::<String, AiomeError>(row.get::<String, _>("owner_id")),
            pg: "SELECT owner_id FROM guilds WHERE id = $1",
            |row| Ok::<String, AiomeError>(row.get::<String, _>("owner_id")),
            &guild_id_str
        )?;

        if let Some(owner_id) = owner_id_str {
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

        sql_exec!(
            &self.pool,
            sqlite: "DELETE FROM guilds WHERE id = ?",
            pg: "DELETE FROM guilds WHERE id = $1",
            &guild_id_str
        )
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

        let owner_id_str = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT owner_id FROM guilds WHERE id = ?",
            |row| Ok::<String, AiomeError>(row.get::<String, _>("owner_id")),
            pg: "SELECT owner_id FROM guilds WHERE id = $1",
            |row| Ok::<String, AiomeError>(row.get::<String, _>("owner_id")),
            &guild_id_str
        )?;

        if let Some(owner_id) = owner_id_str {
            if owner_id != requester_str && agent_id_str != owner_id {
                return Err(AiomeError::Unauthorized {
                    reason: "Only owner can add members".into(),
                });
            }
        } else {
            return Err(AiomeError::NotFound {
                reason: "Guild not found".into(),
            });
        }

        sql_exec!(
            &self.pool,
            sqlite: "INSERT OR REPLACE INTO guild_members (guild_id, agent_id, role) VALUES (?, ?, ?)",
            pg: "INSERT INTO guild_members (guild_id, agent_id, role) VALUES ($1, $2, $3) ON CONFLICT (guild_id, agent_id) DO UPDATE SET role = EXCLUDED.role",
            &guild_id_str,
            &agent_id_str,
            &role
        )
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

        let owner_id_str = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT owner_id FROM guilds WHERE id = ?",
            |row| Ok::<String, AiomeError>(row.get::<String, _>("owner_id")),
            pg: "SELECT owner_id FROM guilds WHERE id = $1",
            |row| Ok::<String, AiomeError>(row.get::<String, _>("owner_id")),
            &guild_id_str
        )?;

        if let Some(owner_id) = owner_id_str {
            if owner_id != requester_str && agent_id_str != requester_str {
                return Err(AiomeError::Unauthorized {
                    reason: "Insufficient permissions to remove member".into(),
                });
            }
            if agent_id_str == owner_id && requester_str != owner_id {
                return Err(AiomeError::Unauthorized {
                    reason: "Cannot remove guild owner".into(),
                });
            }
        } else {
            return Err(AiomeError::NotFound {
                reason: "Guild not found".into(),
            });
        }

        sql_exec!(
            &self.pool,
            sqlite: "DELETE FROM guild_members WHERE guild_id = ? AND agent_id = ?",
            pg: "DELETE FROM guild_members WHERE guild_id = $1 AND agent_id = $2",
            &guild_id_str,
            &agent_id_str
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(())
    }

    async fn fetch_guilds(&self) -> Result<Vec<Guild>, AiomeError> {
        sql_fetch_all_map!(
            &self.pool,
            sqlite: "SELECT id, name, description, owner_id, created_at FROM guilds",
            |row| {
                let id = Uuid::parse_str(row.get("id")).map_err(|e| {
                    error!(guild_id_raw = %row.get::<String, _>("id"), "Corrupted guild id in DB");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid guild id: {}", e),
                    }
                })?;
                let owner_id = Uuid::parse_str(row.get("owner_id")).map_err(|e| {
                    error!(owner_id_raw = %row.get::<String, _>("owner_id"), "Corrupted owner_id in DB");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid owner_id: {}", e),
                    }
                })?;
                Ok::<_, AiomeError>(Guild {
                    id,
                    name: row.get("name"),
                    description: row.get("description"),
                    owner_id,
                    created_at: row.get("created_at"),
                })
            },
            pg: "SELECT id::TEXT, name::TEXT, description::TEXT, owner_id::TEXT, created_at::TEXT FROM guilds",
            |row| {
                let id = Uuid::parse_str(row.get("id")).map_err(|e| {
                    error!(guild_id_raw = %row.get::<String, _>("id"), "Corrupted guild id in DB (postgres)");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid guild id: {}", e),
                    }
                })?;
                let owner_id = Uuid::parse_str(row.get("owner_id")).map_err(|e| {
                    error!(owner_id_raw = %row.get::<String, _>("owner_id"), "Corrupted owner_id in DB (postgres)");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid owner_id: {}", e),
                    }
                })?;
                Ok::<_, AiomeError>(Guild {
                    id,
                    name: row.get("name"),
                    description: row.get("description"),
                    owner_id,
                    created_at: row.get("created_at"),
                })
            }
        )
    }

    async fn fetch_members(&self, guild_id: Uuid) -> Result<Vec<GuildMember>, AiomeError> {
        let guild_id_str = guild_id.to_string();
        sql_fetch_all_map!(
            &self.pool,
            sqlite: "SELECT guild_id, agent_id, role, joined_at FROM guild_members WHERE guild_id = ?",
            |row| {
                let parsed_guild_id = Uuid::parse_str(row.get("guild_id")).map_err(|e| {
                    error!(guild_id_raw = %row.get::<String, _>("guild_id"), "Corrupted guild_id in guild_members");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid guild_id: {}", e),
                    }
                })?;
                let parsed_agent_id = Uuid::parse_str(row.get("agent_id")).map_err(|e| {
                    error!(agent_id_raw = %row.get::<String, _>("agent_id"), "Corrupted agent_id in guild_members");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid agent_id: {}", e),
                    }
                })?;
                Ok::<_, AiomeError>(GuildMember {
                    guild_id: parsed_guild_id,
                    agent_id: parsed_agent_id,
                    role: row.get("role"),
                    joined_at: row.get("joined_at"),
                })
            },
            pg: "SELECT guild_id::TEXT, agent_id::TEXT, role::TEXT, joined_at::TEXT FROM guild_members WHERE guild_id = $1",
            |row| {
                let parsed_guild_id = Uuid::parse_str(row.get("guild_id")).map_err(|e| {
                    error!(guild_id_raw = %row.get::<String, _>("guild_id"), "Corrupted guild_id in guild_members (postgres)");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid guild_id: {}", e),
                    }
                })?;
                let parsed_agent_id = Uuid::parse_str(row.get("agent_id")).map_err(|e| {
                    error!(agent_id_raw = %row.get::<String, _>("agent_id"), "Corrupted agent_id in guild_members (postgres)");
                    AiomeError::Infrastructure {
                        reason: format!("Invalid agent_id: {}", e),
                    }
                })?;
                Ok::<_, AiomeError>(GuildMember {
                    guild_id: parsed_guild_id,
                    agent_id: parsed_agent_id,
                    role: row.get("role"),
                    joined_at: row.get("joined_at"),
                })
            },
            &guild_id_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> shared::db::DatabasePool {
        let pool = shared::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .expect("Failed to create memory DB");

        let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();

        sqlx::query("CREATE TABLE guilds (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, owner_id TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)")
            .execute(sqlite_pool).await.unwrap();
        sqlx::query("CREATE TABLE guild_members (guild_id TEXT NOT NULL, agent_id TEXT NOT NULL, role TEXT NOT NULL, joined_at DATETIME DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY (guild_id, agent_id))")
            .execute(sqlite_pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_and_fetch_guilds() {
        let pool = setup_test_db().await;
        let store = UniversalSyndicateStore::new(pool);

        let owner_id = Uuid::new_v4();
        let guild_id = store
            .create_guild("Test Guild".into(), Some("Desc".into()), owner_id)
            .await
            .unwrap();

        let guilds = store.fetch_guilds().await.unwrap();
        assert_eq!(guilds.len(), 1);
        assert_eq!(guilds[0].name, "Test Guild");
        assert_eq!(guilds[0].id, guild_id);

        // Check initial member
        let members = store.fetch_members(guild_id).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].agent_id, owner_id);
        assert_eq!(members[0].role, "admin");
    }

    #[tokio::test]
    async fn test_guild_management_cycle() {
        let pool = setup_test_db().await;
        let store = UniversalSyndicateStore::new(pool);

        let owner_id = Uuid::new_v4();
        let other_agent_id = Uuid::new_v4();

        let guild_id = store
            .create_guild("Syndicate Alpha".into(), None, owner_id)
            .await
            .unwrap();

        // Add member
        store
            .add_member(guild_id, other_agent_id, "worker".into(), owner_id)
            .await
            .unwrap();
        let members = store.fetch_members(guild_id).await.unwrap();
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
            .unwrap();
        let members = store.fetch_members(guild_id).await.unwrap();
        assert_eq!(members.len(), 1);

        // Delete guild (unauthorized)
        let result = store.delete_guild(guild_id, other_agent_id).await;
        assert!(result.is_err());

        // Delete guild (authorized)
        store.delete_guild(guild_id, owner_id).await.unwrap();
        let guilds = store.fetch_guilds().await.unwrap();
        assert_eq!(guilds.len(), 0);
    }

    #[tokio::test]
    async fn test_non_existent_guild_operations() {
        let pool = setup_test_db().await;
        let store = UniversalSyndicateStore::new(pool);
        let fake_guild_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();

        let res_add = store
            .add_member(fake_guild_id, agent_id, "admin".into(), agent_id)
            .await;
        assert!(matches!(res_add, Err(AiomeError::NotFound { .. })));

        let res_remove = store.remove_member(fake_guild_id, agent_id, agent_id).await;
        assert!(matches!(res_remove, Err(AiomeError::NotFound { .. })));

        let res_delete = store.delete_guild(fake_guild_id, agent_id).await;
        assert!(matches!(res_delete, Err(AiomeError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_fetch_guilds_invalid_uuid_graceful_error() {
        let pool = setup_test_db().await;
        let store = UniversalSyndicateStore::new(pool.clone());

        // Directly insert a bad UUID using raw SQL to bypass the create_guild logic
        let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();
        sqlx::query("INSERT INTO guilds (id, name, description, owner_id) VALUES ('invalid-uuid', 'Bad Guild', 'Desc', 'another-invalid')")
            .execute(sqlite_pool)
            .await
            .unwrap();

        let res = store.fetch_guilds().await;
        assert!(
            res.is_err(),
            "Expected an error when parsing invalid UUID, but got {:?}",
            res
        );
        assert!(matches!(res, Err(AiomeError::Infrastructure { .. })));
    }

    #[tokio::test]
    async fn test_fetch_members_invalid_uuid_graceful_error() {
        let pool = setup_test_db().await;
        let store = UniversalSyndicateStore::new(pool.clone());

        // Insert a valid guild first, then corrupt guild_members
        let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();
        let guild_id = Uuid::new_v4();
        let guild_id_str = guild_id.to_string();
        sqlx::query(
            "INSERT INTO guilds (id, name, description, owner_id) VALUES (?, 'TestGuild', NULL, ?)",
        )
        .bind(&guild_id_str)
        .bind(&guild_id_str)
        .execute(sqlite_pool)
        .await
        .unwrap();

        // Insert corrupted member record with invalid agent_id
        sqlx::query("INSERT INTO guild_members (guild_id, agent_id, role) VALUES (?, 'not-a-uuid', 'worker')")
            .bind(&guild_id_str)
            .execute(sqlite_pool)
            .await
            .unwrap();

        let res = store.fetch_members(guild_id).await;
        assert!(
            res.is_err(),
            "Expected an error when parsing invalid agent_id, but got {:?}",
            res
        );
        assert!(matches!(res, Err(AiomeError::Infrastructure { .. })));
    }
}
