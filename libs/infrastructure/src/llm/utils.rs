/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use shared::output_validator;

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
}
