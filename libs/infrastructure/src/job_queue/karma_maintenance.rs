/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use tracing::info;

/// Karma Tiering Maintenance ジョブ (Adaptive Intelligence)
/// 7日/30日/90日のディケイ・サイクルに基づき tier を自動遷移させる。
pub async fn run_karma_tier_maintenance(pool: &DatabasePool) -> Result<(), AiomeError> {
    info!("🧬 [KarmaMaintenance] Starting Karma tiering cycle...");

    // 1. HOT 昇格: 直近7日間に3回以上適用された WARM なカルマ
    let hot_records = match pool {
        DatabasePool::Sqlite(p) => sqlx::query("UPDATE karma_logs SET tier = 'HOT' WHERE tier = 'WARM' AND apply_count >= 3 AND last_applied_at > datetime('now', '-7 days') AND is_archived = 0").execute(p).await.map(|r| r.rows_affected()),
        DatabasePool::Postgres(p) => sqlx::query("UPDATE karma_logs SET tier = 'HOT' WHERE tier = 'WARM' AND apply_count >= 3 AND last_applied_at > NOW() - INTERVAL '7 days' AND is_archived = 0").execute(p).await.map(|r| r.rows_affected()),
    }
    .map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to promote to HOT: {}", e),
    })?;

    // 2. WARM 降格: 30日以上未使用の HOT なカルマ
    let warm_records = match pool {
        DatabasePool::Sqlite(p) => sqlx::query("UPDATE karma_logs SET tier = 'WARM' WHERE tier = 'HOT' AND (last_applied_at IS NULL OR last_applied_at < datetime('now', '-30 days'))").execute(p).await.map(|r| r.rows_affected()),
        DatabasePool::Postgres(p) => sqlx::query("UPDATE karma_logs SET tier = 'WARM' WHERE tier = 'HOT' AND (last_applied_at IS NULL OR last_applied_at < NOW() - INTERVAL '30 days')").execute(p).await.map(|r| r.rows_affected()),
    }
    .map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to demote to WARM: {}", e),
    })?;

    // 3. COLD 降格 (アーカイブ): 90日以上未使用の WARM なカルマ
    let cold_records = match pool {
        DatabasePool::Sqlite(p) => sqlx::query("UPDATE karma_logs SET tier = 'COLD', is_archived = 1 WHERE tier = 'WARM' AND (last_applied_at IS NULL OR last_applied_at < datetime('now', '-90 days'))").execute(p).await.map(|r| r.rows_affected()),
        DatabasePool::Postgres(p) => sqlx::query("UPDATE karma_logs SET tier = 'COLD', is_archived = 1 WHERE tier = 'WARM' AND (last_applied_at IS NULL OR last_applied_at < NOW() - INTERVAL '90 days')").execute(p).await.map(|r| r.rows_affected()),
    }
    .map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to demote to COLD: {}", e),
    })?;

    if hot_records > 0 || warm_records > 0 || cold_records > 0 {
        info!(
            "✅ [KarmaMaintenance] Cycle completed: HOT promoted={}, WARM demoted={}, COLD archived={}",
            hot_records, warm_records, cold_records
        );
    }

    Ok(())
}
