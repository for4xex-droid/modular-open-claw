/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#[cfg(test)]
mod tests {
    use crate::boundary_verifier::BoundaryVerifier;
    use aiome_contracts::error::AiomeError;
    use std::path::PathBuf;

    fn setup_verifier() -> BoundaryVerifier {
        BoundaryVerifier::new(
            PathBuf::from("/tmp/aiome-workspace"),
            Some(PathBuf::from("/tmp/aiome-vault")),
        )
    }

    #[test]
    fn test_path_in_sandbox_pass() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("ls /tmp/aiome-workspace/docs", false);
        assert!(result.is_ok());
        assert!(result.unwrap().contains(&"path_in_sandbox".to_string()));
    }

    #[test]
    fn test_path_not_system_reject() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("cat /etc/passwd", false);
        assert!(matches!(result, Err(AiomeError::Infrastructure { .. })));
    }

    #[test]
    fn test_binary_whitelist_pass() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("cargo build", false);
        assert!(result.is_ok());
        assert!(result.unwrap().contains(&"binary_in_whitelist".to_string()));
    }

    #[test]
    fn test_binary_not_in_whitelist_reject() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("rm -rf /", false);
        assert!(matches!(result, Err(AiomeError::Infrastructure { .. })));
    }

    #[test]
    fn test_no_meta_characters_reject() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("ls; rm -rf /", false);
        assert!(matches!(result, Err(AiomeError::Infrastructure { .. })));
    }

    #[test]
    fn test_no_env_access_reject() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("cat .env", false);
        assert!(matches!(result, Err(AiomeError::Infrastructure { .. })));
    }

    #[test]
    fn test_vault_access_external_reject() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("cat /tmp/aiome-vault/secret", false);
        assert!(matches!(result, Err(AiomeError::Infrastructure { .. })));
    }

    #[test]
    fn test_vault_access_internal_pass() {
        let verifier = setup_verifier();
        let result = verifier.verify_command("cat /tmp/aiome-vault/secret", true);
        assert!(result.is_ok());
    }
}
