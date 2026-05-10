/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

static SECRET_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        # Standard API Keys
        (sk-[a-zA-Z0-9]{48}) |
        (ghp_[a-zA-Z0-9]{36}) |
        (AIza[0-9A-Za-z\-_]{35}) |
        (AKIA[0-9A-Z]{16}) |
        (hf_[a-zA-Z0-9]{34}) |
        # JWT
        (eyJ[A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_=]+\.?[A-Za-z0-9\-_.+/=]*) |
        # Connection Strings
        ((?i)postgres(?:ql)?://[^:]+:[^@]+@[^:]+:\d+/[^\s]+) |
        ((?i)mysql://[^:]+:[^@]+@[^:]+:\d+/[^\s]+) |
        # Legacy SpecProvider pattern
        ((?i)(API_KEY|SECRET|TOKEN|PASSWORD|PRIVATE_KEY|ACCESS_KEY|CLIENT_SECRET)\s*[=:]\s*\S+)
        ",
    )
    .expect("Invalid regex for secret redactor") // allow-anti-pattern: Regex is static and verified
});

#[derive(Default, Clone, Debug)]
pub struct SecretRedactor;

impl SecretRedactor {
    pub fn new() -> Self {
        Self
    }

    pub fn redact<'a>(&self, input: &'a str) -> Cow<'a, str> {
        SECRET_PATTERN.replace_all(input, "[REDACTED]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_keys() {
        let redactor = SecretRedactor::new();

        let input = "My openAI key is sk-123456789012345678901234567890123456789012345678 and github is ghp_1234567890abcdef1234567890abcdef1234"; // gitleaks:allow
        let result = redactor.redact(input);

        assert_eq!(
            result,
            "My openAI key is [REDACTED] and github is [REDACTED]"
        );
    }

    #[test]
    fn test_redact_jwt() {
        let redactor = SecretRedactor::new();
        let input = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"; // gitleaks:allow
        let result = redactor.redact(input);
        assert_eq!(result, "Bearer [REDACTED]");
    }

    #[test]
    fn test_redact_connection_strings() {
        let redactor = SecretRedactor::new();
        let input = "DATABASE_URL=postgresql://user:pass123!@localhost:5432/db_name";
        let result = redactor.redact(input);
        // regex matched the url part
        assert_eq!(result, "DATABASE_URL=[REDACTED]");
    }

    #[test]
    fn test_redact_spec_provider_legacy() {
        let redactor = SecretRedactor::new();
        let input = "Here is my API_KEY=super_secret_value";
        let result = redactor.redact(input);
        // spec provider pattern matches 'API_KEY=super_secret_value'
        assert_eq!(result, "Here is my [REDACTED]");
    }

    #[test]
    fn test_false_positives() {
        let redactor = SecretRedactor::new();
        let input =
            "This is a normal sentence. The word mask-12345 is not a secret. asking is fine.";
        let result = redactor.redact(input);
        assert_eq!(result, input);
    }
}
