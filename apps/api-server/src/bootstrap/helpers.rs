/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]

use tower_http::cors::{AllowOrigin, CorsLayer};
#[allow(unused_imports)] // error! は #[cfg(not(debug_assertions))] 内で使用
use tracing::{error, info, warn};

pub fn init_cors() -> anyhow::Result<CorsLayer> {
    use axum::http::HeaderValue;

    let mut layer = CorsLayer::new()
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    match std::env::var("ALLOWED_ORIGINS") {
        Ok(origins) if !origins.is_empty() => {
            let list: Vec<HeaderValue> = origins
                .split(',')
                .map(|s| {
                    HeaderValue::from_str(s.trim()).map_err(|e| {
                        anyhow::anyhow!("🚨 Invalid origin in ALLOWED_ORIGINS '{}': {}", s, e)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            layer = layer.allow_origin(AllowOrigin::list(list));
            info!("🌐 [CORS] Allowed origins: {}", origins);
        }
        _ => {
            #[cfg(debug_assertions)]
            {
                warn!("⚠️ [CORS] ALLOWED_ORIGINS not set. All origins allowed in dev mode.");
                layer = layer.allow_origin(AllowOrigin::any());
            }
            #[cfg(not(debug_assertions))]
            {
                error!("🚨 [FATAL SECURITY ERROR] ALLOWED_ORIGINS MUST be set in production!");
                std::process::exit(1);
            }
        }
    }

    Ok(layer)
}

/// 本番環境で API_SERVER_SECRET がセキュアな要件を満たしているか判定する
pub fn is_secure_production_secret(secret: &str) -> bool {
    let s = secret.trim();
    if s.len() < 16 {
        return false;
    }

    let blocklist = [
        "dev_secret_donotuseinprod",
        "dev_secret_change_me_immediately",
        "quickstart_secret_change_in_production",
        "mock_valid_token_tester",
    ];
    !blocklist.contains(&s)
}

pub fn backup_sqlite_db_before_migration(db_path: &str) {
    if !db_path.starts_with("postgres://")
        && !db_path.starts_with("postgresql://")
        && !db_path.contains(":memory:")
    {
        let db_file = db_path
            .strip_prefix("sqlite://")
            .or_else(|| db_path.strip_prefix("sqlite:"))
            .unwrap_or(db_path);

        let bak_path = format!("{}.pre_migration.bak", db_file);

        if std::path::Path::new(db_file).exists() {
            match std::fs::copy(db_file, &bak_path) {
                Ok(bytes) => {
                    tracing::info!(
                        "📸 [Backup] Pre-migration snapshot: {} ({} bytes)",
                        bak_path,
                        bytes
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠️ [Backup] Pre-migration snapshot failed (non-fatal): {}",
                        e
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn test_init_cors() {
        std::env::set_var("ALLOWED_ORIGINS", "http://localhost:1420");
        let result = init_cors();
        assert!(result.is_ok(), "init_cors failed: {:?}", result.err());

        // Negative test (invalid origin)
        std::env::set_var(
            "ALLOWED_ORIGINS",
            "invalid
uri",
        );
        let result = init_cors();
        assert!(result.is_err(), "init_cors should fail with invalid URI");
    }

    #[test]
    fn test_is_secure_production_secret() {
        assert!(!is_secure_production_secret("")); // Too short
        assert!(!is_secure_production_secret("short")); // Too short (5)
        assert!(!is_secure_production_secret("123456789012345")); // Too short (15)

        assert!(!is_secure_production_secret("dev_secret_donotuseinprod")); // Blocklist
        assert!(!is_secure_production_secret(
            "dev_secret_change_me_immediately"
        )); // Blocklist
        assert!(!is_secure_production_secret(
            "quickstart_secret_change_in_production"
        )); // Blocklist
        assert!(!is_secure_production_secret("mock_valid_token_tester")); // Blocklist

        assert!(is_secure_production_secret("my_super_secret_key_123!@#")); // Valid (26 chars)
        assert!(is_secure_production_secret("1234567890123456")); // Valid (16 chars)
    }

    #[test]
    fn test_backup_sqlite_db_before_migration() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_file = dir.path().join("test.db");
        let mut file = std::fs::File::create(&db_file).expect("Failed to create test DB");
        file.write_all(b"dummy db content")
            .expect("Failed to write test data");

        let db_path = format!("sqlite://{}", db_file.to_string_lossy());
        backup_sqlite_db_before_migration(&db_path);

        let bak_path = format!("{}.pre_migration.bak", db_file.to_string_lossy());
        assert!(
            std::path::Path::new(&bak_path).exists(),
            "Backup file should be created"
        );

        let content = std::fs::read_to_string(&bak_path).expect("Failed to read backup");
        assert_eq!(content, "dummy db content");
    }

    #[test]
    fn test_backup_skips_memory_db() {
        backup_sqlite_db_before_migration(":memory:");
        // No file should be created — :memory: is an in-memory DB.
        // If it tried to copy, it would panic. Success = no panic.
    }

    #[test]
    fn test_backup_skips_postgres() {
        backup_sqlite_db_before_migration("postgres://user:pass@localhost/aiome");
        backup_sqlite_db_before_migration("postgresql://user:pass@localhost/aiome");
        // No file operations should occur for PostgreSQL URLs.
    }

    #[test]
    fn test_backup_skips_nonexistent_file() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_file = dir.path().join("nonexistent.db");
        let db_path = format!("sqlite://{}", db_file.to_string_lossy());

        backup_sqlite_db_before_migration(&db_path);

        let bak_path = format!("{}.pre_migration.bak", db_file.to_string_lossy());
        assert!(
            !std::path::Path::new(&bak_path).exists(),
            "Backup file should NOT be created for nonexistent DB"
        );
    }

    #[test]
    fn test_backup_with_bare_path_no_prefix() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_file = dir.path().join("bare.db");
        let mut file = std::fs::File::create(&db_file).expect("Failed to create test DB");
        file.write_all(b"bare path content")
            .expect("Failed to write");

        // Test with raw path (no sqlite:// prefix)
        backup_sqlite_db_before_migration(&db_file.to_string_lossy());

        let bak_path = format!("{}.pre_migration.bak", db_file.to_string_lossy());
        assert!(
            std::path::Path::new(&bak_path).exists(),
            "Backup should work with bare file paths too"
        );
    }
}
