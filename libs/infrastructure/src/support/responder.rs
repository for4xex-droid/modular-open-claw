/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::escalator::{SupportIntent, SupportSeverity};
use aiome_core_contracts::traits::KarmaEntry;
use serde_json::Value;

/// サポート問い合わせ対応用のプロンプト構築管理機
pub struct SupportResponder;

impl SupportResponder {
    /// 不具合の性質、FAQ、および自己診断データを統合し、
    /// AI（AgentEngine）に与える高品質なサポート専用システムプロンプトを構築する。
    pub fn build_support_prompt(
        intent: &SupportIntent,
        karma_entries: &[KarmaEntry],
        diagnoses: &[Value],
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str("You are Aiome's Autonomous Support Assistant, a self-healing and proactive AI-first customer support agent.\n");
        prompt.push_str("Your goal is to understand the user's issue, provide highly accurate troubleshooting advice, utilize local FAQ/Knowledge base, and suggest resolutions.\n\n");

        // 1. User Intent (不具合報告・一般会話等)
        prompt.push_str("=== USER INTENT ===\n");
        match intent {
            SupportIntent::BugReport { summary, severity } => {
                prompt.push_str(&format!(
                    "Category: Bug Report\nSeverity: {:?}\nSummary: {}\n",
                    severity, summary
                ));
            }
            SupportIntent::GeneralChat => {
                prompt.push_str("Category: General Support Chat\n");
            }
            SupportIntent::FeatureRequest(req) => {
                prompt.push_str(&format!("Category: Feature Request\nDetails: {}\n", req));
            }
            SupportIntent::AccountIssue(issue) => {
                prompt.push_str(&format!(
                    "Category: Account / Billing Issue\nDetails: {}\n",
                    issue
                ));
            }
            SupportIntent::Unknown(raw) => {
                prompt.push_str(&format!("Category: General Inquiries\nDetails: {}\n", raw));
            }
        }
        prompt.push('\n');

        // 2. FAQ/Knowledge base (Karma)
        prompt.push_str("=== RELEVANT KNOWLEDGE (FAQ) ===\n");
        if karma_entries.is_empty() {
            prompt.push_str("No direct knowledge matches found.\n");
        } else {
            for (i, entry) in karma_entries.iter().enumerate() {
                prompt.push_str(&format!(
                    "{}. [{}] {}\n",
                    i + 1,
                    entry.related_skill,
                    entry.lesson
                ));
            }
        }
        prompt.push('\n');

        // 3. System Diagnostics (自己診断)
        prompt.push_str("=== SYSTEM DIAGNOSTICS ===\n");
        if diagnoses.is_empty() {
            prompt.push_str("No critical system diagnostics failures reported.\n");
        } else {
            for (i, diag) in diagnoses.iter().enumerate() {
                let root_cause = diag["root_cause"].as_str().unwrap_or("Unknown");
                let evidence = diag["evidence"].as_str().unwrap_or("None");
                let self_repair_hint = diag["self_repair_hint"].as_str().unwrap_or("None");
                prompt.push_str(&format!(
                    "{}. Cause: {}\n   Evidence: {}\n   Suggested Fix: {}\n",
                    i + 1,
                    root_cause,
                    evidence,
                    self_repair_hint
                ));
            }
        }
        prompt.push('\n');

        prompt.push_str("Please craft a professional, polite, and helpful response in Japanese (です・ます調).\n");
        prompt.push_str("If the issue is Critical/High severity, assure the user that their issue has been escalated to human engineers automatically.\n");

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_support_prompt_formatting() {
        // Arrange
        let intent = SupportIntent::BugReport {
            summary: "Failed to connect to database".to_string(),
            severity: SupportSeverity::Critical,
        };

        let mut faq = KarmaEntry::default();
        faq.related_skill = "database_connection".to_string();
        faq.lesson = "Verify database connection credentials and pool health.".to_string();
        let karma_entries = vec![faq];

        let diag_val = serde_json::json!({
            "root_cause": "TLS handshake timeout",
            "evidence": "port 5432 unreachable",
            "self_repair_hint": "Restart DB Proxy"
        });
        let diagnoses = vec![diag_val];

        // Act
        let prompt = SupportResponder::build_support_prompt(&intent, &karma_entries, &diagnoses);

        // Assert
        assert!(prompt.contains("You are Aiome's Autonomous Support Assistant"));
        assert!(prompt.contains("Category: Bug Report"));
        assert!(prompt.contains("Severity: Critical"));
        assert!(prompt.contains("Summary: Failed to connect to database"));

        assert!(prompt.contains(
            "[database_connection] Verify database connection credentials and pool health."
        ));

        assert!(prompt.contains("Cause: TLS handshake timeout"));
        assert!(prompt.contains("Evidence: port 5432 unreachable"));
        assert!(prompt.contains("Suggested Fix: Restart DB Proxy"));

        assert!(prompt.contains("Japanese (です・ます調)"));
        assert!(prompt.contains("escalated to human engineers automatically"));
    }
}
