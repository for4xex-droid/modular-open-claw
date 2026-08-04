#![allow(clippy::unwrap_used)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::{LlmRequest, LlmResponse};
use aiome_contracts::security::{host_permitted, AgentHook};
use async_trait::async_trait;

#[derive(Debug)]
struct MockSecurityHook;

#[async_trait]
impl AgentHook for MockSecurityHook {
    async fn on_pre_execute(&self, _request: &LlmRequest) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_post_execute(
        &self,
        _request: &LlmRequest,
        _response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_agent_hook_trait_exists() {
    let hook = MockSecurityHook;
    // This should fail to compile if AgentHook is not defined
    let _ = hook;
}

#[test]
fn test_host_permitted_empty_domains_deny() {
    assert!(!host_permitted("example.com", &[]));
}

#[test]
fn test_host_permitted_exact_and_suffix() {
    let domains = vec!["example.com".to_string()];
    assert!(host_permitted("example.com", &domains));
    assert!(host_permitted("api.example.com", &domains));
    assert!(!host_permitted("evil.com", &domains));
    assert!(!host_permitted("example.com.evil.com", &domains));
}

#[test]
fn test_host_permitted_bare_tld_does_not_suffix_match() {
    let domains = vec!["com".to_string()];
    assert!(host_permitted("com", &domains)); // exact still ok
    assert!(!host_permitted("evil.com", &domains));
    assert!(!host_permitted("api.example.com", &domains));
}

#[test]
fn test_host_permitted_wildcard() {
    let domains = vec!["*".to_string()];
    assert!(host_permitted("anywhere.example", &domains));
    assert!(!host_permitted("", &domains));
}

#[test]
fn test_host_permitted_ignores_empty_domain_entry() {
    let domains = vec!["".to_string(), "ok.dev".to_string()];
    assert!(host_permitted("ok.dev", &domains));
    assert!(!host_permitted("nope.dev", &domains));
}

#[test]
fn test_host_permitted_case_insensitive_allow() {
    let domains = vec!["evil.com".to_string()];
    assert!(host_permitted("EVIL.COM", &domains));
    assert!(host_permitted("Evil.Com", &domains));
}

#[test]
fn test_host_permitted_rejects_control_and_internal_whitespace() {
    let domains = vec!["example.com".to_string()];
    assert!(!host_permitted("exam\0ple.com", &domains));
    assert!(!host_permitted("exam ple.com", &domains));
}

#[test]
fn test_host_permitted_trailing_dot_fqdn_normalized() {
    let domains = vec!["example.com".to_string()];
    assert!(host_permitted("example.com.", &domains));
    assert!(host_permitted("api.example.com.", &domains));
}

#[test]
fn test_host_permitted_trailing_dot_does_not_widen_suffix() {
    assert!(!host_permitted(
        "evil.com.",
        &[".com".to_string(), "com.".to_string(), "com".to_string()]
    ));
}

#[test]
fn test_host_permitted_trims_and_rejects_dotted_junk_entries() {
    let domains = vec![
        "  example.com  ".to_string(),
        ".example.com".to_string(),
        "example.com.".to_string(),
    ];
    assert!(host_permitted("api.example.com", &domains));
    assert!(host_permitted("  example.com  ", &domains));
    // Junk entries alone must not grant
    assert!(!host_permitted(
        "evil.com",
        &[".com".to_string(), "com.".to_string()]
    ));
}
