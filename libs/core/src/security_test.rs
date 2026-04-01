/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {
    use crate::security::ConstitutionalValidator;
    use aiome_contracts::security::PermissionManifest;

    #[tokio::test]
    async fn test_constitutional_violation_rejection() {
        let validator = ConstitutionalValidator::new();

        // 1. Harmful directive
        let harmful_manifest = PermissionManifest {
            allow_network: true,
            allow_filesystem_write: true,
            allow_shell_execution: true,
            allowed_domains: vec!["evil.com".to_string()],
        };

        let res = validator.validate_manifest(&harmful_manifest).await;

        // Should fail if it violates the "Axiomatic Safety" policy
        assert!(
            res.is_err(),
            "Harmful manifest should be rejected by ConstitutionalValidator"
        );
    }

    #[tokio::test]
    async fn test_safe_manifest_approval() {
        let validator = ConstitutionalValidator::new();

        let safe_manifest = PermissionManifest {
            allow_network: false,
            allow_filesystem_write: false,
            allow_shell_execution: false,
            allowed_domains: vec![],
        };

        let res = validator.validate_manifest(&safe_manifest).await;
        assert!(res.is_ok(), "Safe manifest should be approved");
    }
}
