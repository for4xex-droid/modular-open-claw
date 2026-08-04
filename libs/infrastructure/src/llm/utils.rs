/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmRequest, CACHE_SCOPE_CHANNEL_KEY};
use sha2::{Digest, Sha256};
use shared::output_validator;

const CACHE_KEY_SOME_TAG: u8 = 0x00;
const CACHE_KEY_NONE_TAG: u8 = 0xFF;

fn update_framed(hasher: &mut Sha256, part: &str) {
    hasher.update((part.len() as u64).to_le_bytes());
    hasher.update(part.as_bytes());
}

fn update_opt_str(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(s) => {
            hasher.update([CACHE_KEY_SOME_TAG]);
            update_framed(hasher, s);
        }
        None => {
            hasher.update([CACHE_KEY_NONE_TAG]);
        }
    }
}

/// Prompt + optional system の SHA-256（長さプレフィクス framing）。
/// eval logger / SemanticCache::get|set 共通。
pub fn compute_prompt_hash(prompt: &str, system: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    update_framed(&mut hasher, prompt);
    if let Some(sys) = system {
        update_framed(&mut hasher, sys);
    }
    hex::encode(hasher.finalize())
}

/// Channel scope from request metadata (chat cache isolation).
pub fn cache_scope_channel(request: &LlmRequest) -> Option<&str> {
    request
        .metadata
        .as_ref()
        .and_then(|m| m.get(CACHE_SCOPE_CHANNEL_KEY))
        .map(String::as_str)
        .filter(|s| !s.is_empty())
}

/// LlmRequest 全体（route_* metadata / stop_sequences 除外）からチャットキャッシュキーを算出。
/// `channel_id` のみスコープとしてキーに含める（必須。欠落時は呼び出し側で bypass）。
pub fn compute_request_cache_key(request: &LlmRequest) -> String {
    let mut hasher = Sha256::new();
    // Scope first so missing channel cannot collide with scoped keys if a caller forgets bypass.
    update_opt_str(&mut hasher, cache_scope_channel(request));
    for m in &request.messages {
        update_framed(&mut hasher, &m.role);
        update_framed(&mut hasher, &m.content);
    }
    let format_norm = request
        .format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());
    update_opt_str(&mut hasher, format_norm.as_deref());
    match request.temperature {
        Some(t) => {
            // Canonicalize -0.0 → +0.0 so keys do not split on signed zero.
            let bits = if t == 0.0 {
                0.0f32.to_bits()
            } else {
                t.to_bits()
            };
            hasher.update([CACHE_KEY_SOME_TAG]);
            hasher.update(bits.to_le_bytes());
        }
        None => {
            hasher.update([CACHE_KEY_NONE_TAG]);
        }
    }
    match request.max_tokens {
        Some(v) => {
            hasher.update([CACHE_KEY_SOME_TAG]);
            hasher.update(v.to_le_bytes());
        }
        None => {
            hasher.update([CACHE_KEY_NONE_TAG]);
        }
    }
    hex::encode(hasher.finalize())
}

/// LLM レスポンスからソースコードブロックを抽出する
pub fn extract_code_block(response: &str) -> String {
    if let Some(start) = response.find("```rust") {
        let snippet = &response[start + 7..];
        if let Some(end) = snippet.find("```") {
            let inner = snippet[..end].trim();
            if !inner.is_empty() {
                return inner.to_string();
            }
        }
    }
    if let Some(start) = response.find("```") {
        let snippet = &response[start + 3..];
        if let Some(end) = snippet.find("```") {
            let inner = snippet[..end].trim();
            if !inner.is_empty() {
                return inner.to_string();
            }
        }
    }
    response.trim().to_string()
}

/// LLMレスポンスからJSONブロックを抽出・検証する
pub fn extract_json(text: &str) -> Result<String, AiomeError> {
    let block = output_validator::extract_json_block(text);
    if block.trim().is_empty() || (!block.contains('{') && !block.contains('[')) {
        return Err(AiomeError::Infrastructure {
            reason: "No JSON block detected in LLM output".into(),
        });
    }
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::llm::LlmMessage;
    use std::collections::HashMap;

    fn scoped(mut req: LlmRequest, channel: &str) -> LlmRequest {
        let mut meta = req.metadata.unwrap_or_default();
        meta.insert(CACHE_SCOPE_CHANNEL_KEY.to_string(), channel.to_string());
        req.metadata = Some(meta);
        req
    }

    #[test]
    fn test_compute_prompt_hash_stable() {
        let h1 = compute_prompt_hash("hello", Some("sys"));
        let h2 = compute_prompt_hash("hello", Some("sys"));
        assert_eq!(h1, h2);
        assert_ne!(h1, compute_prompt_hash("hello", None));
    }

    #[test]
    fn test_prompt_hash_no_boundary_collision() {
        assert_ne!(
            compute_prompt_hash("ab", Some("c")),
            compute_prompt_hash("a", Some("bc"))
        );
        assert_ne!(
            compute_prompt_hash("abc", None),
            compute_prompt_hash("ab", Some("c"))
        );
    }

    #[test]
    fn test_request_cache_key_sensitive_to_history_and_params() {
        let base = scoped(
            LlmRequest {
                messages: vec![LlmMessage {
                    role: "user".into(),
                    content: "hello".into(),
                    cache: false,
                }],
                ..Default::default()
            },
            "ch-a",
        );
        let with_history = scoped(
            LlmRequest {
                messages: vec![
                    LlmMessage {
                        role: "user".into(),
                        content: "prev".into(),
                        cache: false,
                    },
                    LlmMessage {
                        role: "assistant".into(),
                        content: "ok".into(),
                        cache: false,
                    },
                    LlmMessage {
                        role: "user".into(),
                        content: "hello".into(),
                        cache: false,
                    },
                ],
                ..Default::default()
            },
            "ch-a",
        );
        let with_format = {
            let mut r = base.clone();
            r.format = Some("json".into());
            r
        };
        let with_temp = {
            let mut r = base.clone();
            r.temperature = Some(0.7);
            r
        };
        let with_max = {
            let mut r = base.clone();
            r.max_tokens = Some(1);
            r
        };
        assert_ne!(
            compute_request_cache_key(&base),
            compute_request_cache_key(&with_history)
        );
        assert_ne!(
            compute_request_cache_key(&base),
            compute_request_cache_key(&with_format)
        );
        assert_ne!(
            compute_request_cache_key(&base),
            compute_request_cache_key(&with_temp)
        );
        assert_ne!(
            compute_request_cache_key(&base),
            compute_request_cache_key(&with_max)
        );
    }

    #[test]
    fn test_request_cache_key_channel_scope_isolates() {
        let msg = vec![LlmMessage {
            role: "user".into(),
            content: "hello".into(),
            cache: false,
        }];
        let a = scoped(
            LlmRequest {
                messages: msg.clone(),
                ..Default::default()
            },
            "channel-a",
        );
        let b = scoped(
            LlmRequest {
                messages: msg,
                ..Default::default()
            },
            "channel-b",
        );
        assert_ne!(compute_request_cache_key(&a), compute_request_cache_key(&b));
    }

    #[test]
    fn test_format_normalization_in_cache_key() {
        let base = scoped(
            LlmRequest {
                messages: vec![LlmMessage {
                    role: "user".into(),
                    content: "hi".into(),
                    cache: false,
                }],
                format: Some("json".into()),
                ..Default::default()
            },
            "ch",
        );
        let upper = {
            let mut r = base.clone();
            r.format = Some("JSON".into());
            r
        };
        assert_eq!(
            compute_request_cache_key(&base),
            compute_request_cache_key(&upper)
        );
    }

    #[test]
    fn test_extract_rust_block_with_text() {
        let input = "Here is the code:\n```rust\nfn main() {}\n```\nHope it helps!";
        let result = extract_code_block(input);
        assert_eq!(result, "fn main() {}");
    }

    #[test]
    fn test_extract_generic_block() {
        let input = "Snippet:\n```\nhello world\n```";
        let result = extract_code_block(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_extract_raw_text() {
        let input = "fn raw() {}";
        let result = extract_code_block(input);
        assert_eq!(result, "fn raw() {}");
    }

    #[test]
    fn test_extract_json_valid_object() {
        let input = "Here is the result:\n```json\n{\"key\":\"val\"}\n```";
        let result = extract_json(input).unwrap();
        assert_eq!(result, "{\"key\":\"val\"}");
    }

    #[test]
    fn test_extract_json_valid_array() {
        let input = "Result: [1, 2, 3]";
        let result = extract_json(input).unwrap();
        assert_eq!(result, "[1, 2, 3]");
    }

    #[test]
    fn test_extract_json_invalid() {
        let input = "There is no json here";
        assert!(extract_json(input).is_err());
    }

    #[test]
    fn test_cache_scope_channel_reads_metadata() {
        let mut meta = HashMap::new();
        meta.insert(CACHE_SCOPE_CHANNEL_KEY.to_string(), "c1".into());
        let req = LlmRequest {
            metadata: Some(meta),
            ..Default::default()
        };
        assert_eq!(cache_scope_channel(&req), Some("c1"));
        assert!(cache_scope_channel(&LlmRequest::default()).is_none());
    }
}
