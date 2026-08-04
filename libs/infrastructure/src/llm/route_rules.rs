/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::llm::{LlmRouteDecision, ROUTE_TIER_KEY, ROUTE_TIER_LOCKED_KEY};
use aiome_core_contracts::task_tier::TaskTier;
use std::collections::HashMap;

/// Configuration slice for synchronous route rules (no I/O).
#[derive(Debug, Clone, Copy)]
pub struct RouteRulesConfig {
    pub short_prompt_chars: usize,
}

const SMART_KEYWORDS: &[&str] = &[
    "security",
    "audit",
    "reasoning",
    "vulnerability",
    "要約",
    "推論",
    "監査",
    "セキュリティ",
];

/// Evaluate routing tier from prompt signals. Must remain synchronous (no I/O).
pub fn decide_route(
    prompt: &str,
    format: Option<&str>,
    metadata: Option<&HashMap<String, String>>,
    config: &RouteRulesConfig,
) -> LlmRouteDecision {
    if let Some(meta) = metadata {
        if let Some(locked) = meta.get(ROUTE_TIER_LOCKED_KEY) {
            return parse_tier_override(locked, "tier_locked", "Sticky tier from prior evaluation");
        }
        // External route_tier metadata is not trusted (OP-099 FIX-8).
    }

    if format
        .map(str::trim)
        .is_some_and(|s| s.eq_ignore_ascii_case("json"))
    {
        return LlmRouteDecision {
            tier: TaskTier::Smart,
            reason_code: "format_json".to_string(),
            reason_detail: "Structured JSON output requires Smart tier".to_string(),
        };
    }

    let prompt_lower = prompt.to_lowercase();
    for kw in SMART_KEYWORDS {
        if prompt_lower.contains(kw) {
            return LlmRouteDecision {
                tier: TaskTier::Smart,
                reason_code: "keyword_smart".to_string(),
                reason_detail: format!("Matched smart-tier keyword: {}", kw),
            };
        }
    }

    let char_count = prompt.chars().count();
    if char_count <= config.short_prompt_chars {
        return LlmRouteDecision {
            tier: TaskTier::Fast,
            reason_code: "short_prompt".to_string(),
            reason_detail: format!(
                "Prompt length {} <= threshold {}",
                char_count, config.short_prompt_chars
            ),
        };
    }

    LlmRouteDecision {
        tier: TaskTier::Smart,
        reason_code: "default_smart".to_string(),
        reason_detail: "Default to Smart tier for safety".to_string(),
    }
}

fn parse_tier_override(raw: &str, code: &str, detail: &str) -> LlmRouteDecision {
    let tier = match raw.trim().to_lowercase().as_str() {
        "fast" | "cheap" | "local" => TaskTier::Fast,
        "smart" | "standard" | "cloud" => TaskTier::Smart,
        _ => TaskTier::Smart,
    };
    LlmRouteDecision {
        tier,
        reason_code: code.to_string(),
        reason_detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: RouteRulesConfig = RouteRulesConfig {
        short_prompt_chars: 512,
    };

    #[test]
    fn test_short_prompt_fast() {
        let d = decide_route("hello", None, None, &CFG);
        assert_eq!(d.tier, TaskTier::Fast);
        assert_eq!(d.reason_code, "short_prompt");
    }

    #[test]
    fn test_json_format_smart() {
        let d = decide_route("hi", Some("json"), None, &CFG);
        assert_eq!(d.tier, TaskTier::Smart);
        assert_eq!(d.reason_code, "format_json");
        let d2 = decide_route("hi", Some("JSON"), None, &CFG);
        assert_eq!(d2.tier, TaskTier::Smart);
        assert_eq!(d2.reason_code, "format_json");
    }

    #[test]
    fn test_security_keyword_smart() {
        let d = decide_route("check security policy", None, None, &CFG);
        assert_eq!(d.tier, TaskTier::Smart);
        assert_eq!(d.reason_code, "keyword_smart");
    }

    #[test]
    fn test_metadata_override_fast() {
        // route_tier metadata では上書きできない（長文は default_smart 維持）
        let mut meta = HashMap::new();
        meta.insert(ROUTE_TIER_KEY.to_string(), "fast".to_string());
        let long = "x".repeat(600);
        let d = decide_route(&long, None, Some(&meta), &CFG);
        assert_eq!(d.tier, TaskTier::Smart);
        assert_eq!(d.reason_code, "default_smart");
    }

    #[test]
    fn test_tier_locked_sticky() {
        let mut meta = HashMap::new();
        meta.insert(ROUTE_TIER_LOCKED_KEY.to_string(), "fast".to_string());
        let long = "x".repeat(600);
        let d = decide_route(&long, None, Some(&meta), &CFG);
        assert_eq!(d.tier, TaskTier::Fast);
        assert_eq!(d.reason_code, "tier_locked");
    }

    #[test]
    fn test_long_prompt_default_smart() {
        let long = "x".repeat(600);
        let d = decide_route(&long, None, None, &CFG);
        assert_eq!(d.tier, TaskTier::Smart);
        assert_eq!(d.reason_code, "default_smart");
    }
}
