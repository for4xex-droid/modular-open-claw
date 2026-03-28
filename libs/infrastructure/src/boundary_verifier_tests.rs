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
            vec![
                "ls".to_string(),
                "cat".to_string(),
                "cargo".to_string(),
                "echo".to_string(),
            ],
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

    #[test]
    fn test_binary_whitelist_dynamic_pass() {
        let verifier = BoundaryVerifier::from_global_config();
        // 'docker' は GLOBAL_SECURITY_CONFIG にあるが、現在の BoundaryVerifier にはハードコードされていない
        let result = verifier.verify_command("docker ps", false);
        assert!(
            result.is_ok(),
            "Expected 'docker' to pass via GLOBAL_SECURITY_CONFIG"
        );
    }

    #[test]
    fn test_payload_size_limit_reject() {
        let verifier = setup_verifier();
        // 64KB を超える巨大な引数
        let large_arg = "a".repeat(70000);
        let cmd = format!("echo {}", large_arg);
        let result = verifier.verify_command(&cmd, false);
        assert!(
            matches!(result, Err(AiomeError::Infrastructure { .. })),
            "Expected rejection for large payload"
        );
    }
}
