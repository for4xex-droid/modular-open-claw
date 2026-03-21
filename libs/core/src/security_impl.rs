/// LLM出力の軽量サニタイズ（HTMLエスケープ + 禁止パターン除去）
pub fn sanitize_llm_output(raw: &str) -> String {
    let sanitized = raw
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace("javascript:", "[blocked]")
        .replace("data:", "[blocked]");
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_html_tags() {
        let input = "Hello <script>alert(1)</script>";
        let expected = "Hello &lt;script&gt;alert(1)&lt;/script&gt;";
        assert_eq!(sanitize_llm_output(input), expected);
    }

    #[test]
    fn test_sanitize_blocked_patterns() {
        let input =
            "Click [here](javascript:void(0)) and see this [image](data:image/png;base64,...)";
        let expected =
            "Click [here]([blocked]void(0)) and see this [image]([blocked]image/png;base64,...)";
        assert_eq!(sanitize_llm_output(input), expected);
    }

    #[test]
    fn test_sanitize_no_change() {
        let input = "Hello, world! 123";
        assert_eq!(sanitize_llm_output(input), input);
    }
}
