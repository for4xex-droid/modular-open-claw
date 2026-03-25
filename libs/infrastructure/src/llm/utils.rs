/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

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
}
