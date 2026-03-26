/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

//! # Security Implementation — セキュリティ基盤実装
//!
//! [G-21] Unified Response Purger (Entity-Level Sanitization)
//! LLM出力や外部コンテンツのサニタイズを統括する。

use ammonia::Builder;
use html_escape::decode_html_entities;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://\S+").expect("Invalid regex"));
static HTML_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").expect("Invalid regex"));
static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("Invalid regex"));
static SCRIPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<script\b[^>]*>.*?</script>").expect("Invalid regex"));
static STYLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<style\b[^>]*>.*?</style>").expect("Invalid regex"));

/// [G-21] Unified Response Purger (Entity-Level Sanitization)
///
/// 以下の手順でコンテンツを徹底的に浄化する：
/// 1. URLの除去（不審なリンクへの誘導防止）
/// 2. HTML実体参照のデコード（隠されたタグの露出）
/// 3. script/styleタグとその中身の完全除去
/// 4. ammoniaによるHTML/XSSサニタイズ（全タグ禁止）
/// 5. 正規表現による残存タグの再帰的除去
/// 6. 最終的なデコードとブラケット強制除去
/// 7. 空白文字の集約
pub fn purge_entities(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    // 1. Strip URLs (to prevent phishing or resource exfiltration)
    let mut text = URL_RE.replace_all(input, "").to_string();

    // 2. Decode HTML entities first to expose hidden tags (e.g. &lt;script&gt;)
    text = decode_html_entities(&text).to_string();

    // 3. Specifically strip script and style tags AND their content
    loop {
        let next_text = SCRIPT_RE.replace_all(&text, "").to_string();
        let next_text = STYLE_RE.replace_all(&next_text, "").to_string();
        if next_text == text {
            break;
        }
        text = next_text;
    }

    // 4. Use ammonia for thorough HTML/XSS sanitization, ensuring NO tags are allowed
    text = Builder::default()
        .tags(HashSet::new()) // Allow zero tags
        .link_rel(None)
        .clean(&text)
        .to_string();

    // 5. Secondary defense: Recursive regex-based tag stripping to catch nested/malformed remnants
    loop {
        let next_text = HTML_RE.replace_all(&text, "").to_string();
        if next_text == text {
            break;
        }
        text = next_text;
    }

    // 6. Final safety: Purge any lone brackets or leftover entities
    text = decode_html_entities(&text).to_string();
    text = text.replace(['<', '>'], "");

    // 7. Collapse whitespace
    text = WS_RE.replace_all(&text, " ").to_string();

    text.trim().to_string()
}

/// LLM出力の軽量サニタイズ（後方互換性のために残すが、内部で purge_entities を呼ぶ）
pub fn sanitize_llm_output(raw: &str) -> String {
    purge_entities(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purge_entities_xss_bypass() {
        let inputs = vec![
            "Hello <script>alert(1)</script> world",
            "Nested <div onmouseover='alert(1)'>safe? <scr<script>ipt>alert(2)</script></div>",
            "Hidden &lt;img src=x onerror=alert(1)&gt;",
            "Recursive <<scr<script>ipt>ipt>alert(1)</script>",
            "Style bypass <style>body { background: url('javascript:alert(1)') }</style>",
            "Data URI [click](data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==)",
        ];

        for input in inputs {
            let result = purge_entities(input);
            assert!(
                !result.contains('<'),
                "Result should not contain '<': {}",
                result
            );
            assert!(
                !result.contains('>'),
                "Result should not contain '>': {}",
                result
            );
            assert!(
                !result.contains("script"),
                "Result should not contain 'script': {}",
                result
            );
            assert!(
                !result.contains("alert"),
                "Result should not contain 'alert': {}",
                result
            );
        }
    }

    #[test]
    fn test_purge_entities_url() {
        let input = "Check this: https://malicious.com/payload and this: http://evil.org";
        let result = purge_entities(input);
        assert!(
            !result.contains("http"),
            "Result should not contain URLs: {}",
            result
        );
    }
}
