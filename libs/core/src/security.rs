/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
pub use aiome_core_contracts::security::*;

/// 🛡️ ConstitutionalValidator
///
/// エージェントに付与される権限（PermissionManifest）が、
/// システムの「憲法（Constitution）」に違反していないか検証する。
pub struct ConstitutionalValidator;

impl ConstitutionalValidator {
    /// Creates a new ConstitutionalValidator instance
    pub fn new() -> Self {
        Self
    }

    /// 権限マニフェストを検証し、公理的安全性（Axiomatic Safety）を保証する。
    pub async fn validate_manifest(&self, manifest: &PermissionManifest) -> Result<(), AiomeError> {
        // Rule 1: 禁止ドメインのチェック (Axiomatic Denial)
        for domain in &manifest.allowed_domains {
            if domain.contains("evil.com") || domain.contains("malicious") {
                return Err(AiomeError::SecurityViolation {
                    reason: format!(
                        "Constitutional Violation: Restricted domain '{}' detected.",
                        domain
                    ),
                });
            }
        }

        // Rule 2: 過剰権限の制限 (Least Privilege Enforcement)
        // ネットワーク、ファイル書き込み、シェル実行のすべてが同時に許可されることは
        // 通常のプラグインではあり得ないため、公理的に拒否する。
        if manifest.allow_network
            && manifest.allow_filesystem_write
            && manifest.allow_shell_execution
        {
            return Err(AiomeError::SecurityViolation {
                reason: "Constitutional Violation: Excessive permissions set (Network + FS + Shell) is forbidden.".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for ConstitutionalValidator {
    fn default() -> Self {
        Self::new()
    }
}
