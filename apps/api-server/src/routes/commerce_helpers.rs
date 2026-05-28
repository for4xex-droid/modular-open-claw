/// URL リダイレクト先のバリデーション
///
/// `is_dev_mode`: ブート時に評価済みの開発モードフラグ
/// `allowed_origins`: ブート時にパース済みのホワイトリスト (None = 未設定)
///
/// # Security
/// - 本番モードでは https スキームのみ許可
/// - 開発モードでは http://localhost と http://127.0.0.1 を追加許可
/// - ホワイトリスト未設定 + 開発モード → 全 https URL を許可（開発便宜）
/// - ホワイトリスト未設定 + 本番モード → 全 URL を拒否（fail-closed）
pub fn validate_redirect_url_with_config(
    raw_url: &str,
    is_dev_mode: bool,
    allowed_origins: Option<&[String]>,
) -> bool {
    let parsed = match url::Url::parse(raw_url) {
        Ok(u) => u,
        Err(_) => return false,
    };

    // 1. Scheme Validation
    if parsed.scheme() != "https" {
        if is_dev_mode && parsed.scheme() == "http" {
            if let Some(host) = parsed.host_str() {
                if host == "localhost" || host == "127.0.0.1" {
                    return true;
                }
            }
        }
        return false;
    }

    // 2. Whitelist Domain Validation (ALLOWED_ORIGINS)
    if let Some(allowed_hosts) = allowed_origins {
        if let Some(host) = parsed.host_str() {
            let host_lower = host.to_lowercase();
            return allowed_hosts.iter().any(|allowed| {
                host_lower == *allowed || host_lower.ends_with(&format!(".{}", allowed))
            });
        }
        return false;
    }

    // No whitelist configured
    if is_dev_mode {
        return true;
    }

    // Fail-closed: no whitelist + production = reject all
    tracing::warn!(
        "🚨 [Commerce] ALLOWED_ORIGINS not configured in production. Rejecting redirect URL: {}",
        raw_url
    );
    false
}

/// 後方互換のラッパー — 既存の呼び出し元から AppState 経由で設定を渡せない場合に使用。
/// ※ 新規コードでは `validate_redirect_url_with_config()` を直接使用すること。
pub fn validate_redirect_url(raw_url: &str) -> bool {
    let is_dev = std::env::var("AIOME_DEV_MODE").unwrap_or_default() == "1";
    let allowed_origins_str = std::env::var("ALLOWED_ORIGINS").ok();

    let allowed_hosts: Option<Vec<String>> = allowed_origins_str.map(|origins_str| {
        origins_str
            .split(',')
            .map(|s| s.trim())
            .filter_map(|s| url::Url::parse(s).ok())
            .filter_map(|u| u.host_str().map(|h| h.to_lowercase()))
            .collect()
    });

    validate_redirect_url_with_config(raw_url, is_dev, allowed_hosts.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_with_config_whitelisted_https() {
        let allowed = vec!["example.com".to_string(), "api.aiome.network".to_string()];
        assert!(validate_redirect_url_with_config(
            "https://example.com",
            false,
            Some(&allowed)
        ));
        assert!(validate_redirect_url_with_config(
            "https://api.aiome.network",
            false,
            Some(&allowed)
        ));
    }

    #[test]
    fn test_validate_with_config_subdomain_match() {
        let allowed = vec!["aiome.network".to_string()];
        assert!(validate_redirect_url_with_config(
            "https://sub.aiome.network/callback",
            false,
            Some(&allowed)
        ));
        // Negative: suffix attack must NOT match
        assert!(!validate_redirect_url_with_config(
            "https://evil-aiome.network",
            false,
            Some(&allowed)
        ));
    }

    #[test]
    fn test_validate_with_config_localhost_dev_mode() {
        let allowed = vec!["example.com".to_string()];
        assert!(validate_redirect_url_with_config(
            "http://localhost:3000",
            true,
            Some(&allowed)
        ));
        assert!(validate_redirect_url_with_config(
            "http://127.0.0.1:1420",
            true,
            Some(&allowed)
        ));
    }

    #[test]
    fn test_validate_with_config_localhost_prod_mode_rejected() {
        let allowed = vec!["example.com".to_string()];
        assert!(!validate_redirect_url_with_config(
            "http://localhost:3000",
            false,
            Some(&allowed)
        ));
    }

    #[test]
    fn test_validate_with_config_malicious_domain() {
        let allowed = vec!["example.com".to_string()];
        assert!(!validate_redirect_url_with_config(
            "https://malicious-phishing.com",
            false,
            Some(&allowed)
        ));
    }

    #[test]
    fn test_validate_with_config_invalid_url() {
        assert!(!validate_redirect_url_with_config(
            "not-a-valid-url",
            false,
            None
        ));
    }

    #[test]
    fn test_validate_with_config_no_whitelist_dev_mode() {
        // Dev mode + no whitelist -> allow any https
        assert!(validate_redirect_url_with_config(
            "https://anything.example.com",
            true,
            None
        ));
    }

    #[test]
    fn test_validate_with_config_no_whitelist_prod_mode() {
        // Prod mode + no whitelist -> fail-closed
        assert!(!validate_redirect_url_with_config(
            "https://anything.example.com",
            false,
            None
        ));
    }
}
