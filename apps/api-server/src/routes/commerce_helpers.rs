pub fn validate_redirect_url(raw_url: &str) -> bool {
    let is_dev = std::env::var("AIOME_DEV_MODE").unwrap_or_default() == "1";
    let parsed = match url::Url::parse(raw_url) {
        Ok(u) => u,
        Err(_) => return false,
    };

    // 1. Scheme Validation
    if parsed.scheme() != "https" {
        if is_dev && parsed.scheme() == "http" {
            if let Some(host) = parsed.host_str() {
                if host == "localhost" || host == "127.0.0.1" {
                    return true;
                }
            }
        }
        return false;
    }

    // 2. Whitelist Domain Validation (ALLOWED_ORIGINS)
    if let Ok(origins_str) = std::env::var("ALLOWED_ORIGINS") {
        let allowed_hosts: Vec<String> = origins_str
            .split(',')
            .map(|s| s.trim())
            .filter_map(|s| url::Url::parse(s).ok())
            .filter_map(|u| u.host_str().map(|h| h.to_lowercase()))
            .collect();

        if let Some(host) = parsed.host_str() {
            let host_lower = host.to_lowercase();
            return allowed_hosts.iter().any(|allowed| {
                host_lower == *allowed || host_lower.ends_with(&format!(".{}", allowed))
            });
        }
    } else {
        if is_dev {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_redirect_url_all_cases() {
        // We use a single test to avoid parallel execution race conditions on std::env::set_var

        // Scenario 1: Whitelisted HTTPS domain
        std::env::set_var(
            "ALLOWED_ORIGINS",
            "https://example.com,https://api.aiome.network",
        );
        std::env::set_var("AIOME_DEV_MODE", "0");
        assert!(validate_redirect_url("https://example.com"));
        assert!(validate_redirect_url("https://api.aiome.network"));

        // Scenario 2: Localhost in dev mode (Expected: true)
        std::env::set_var("ALLOWED_ORIGINS", "https://example.com");
        std::env::set_var("AIOME_DEV_MODE", "1");
        assert!(validate_redirect_url("http://localhost:3000"));
        assert!(validate_redirect_url("http://127.0.0.1:1420"));

        // Scenario 3: Localhost in non-dev mode (Expected: false)
        std::env::set_var("ALLOWED_ORIGINS", "https://example.com");
        std::env::set_var("AIOME_DEV_MODE", "0");
        assert!(!validate_redirect_url("http://localhost:3000"));

        // Scenario 4: Malicious/Non-whitelisted domain (Expected: false)
        std::env::set_var("ALLOWED_ORIGINS", "https://example.com");
        std::env::set_var("AIOME_DEV_MODE", "0");
        assert!(!validate_redirect_url("https://malicious-phishing.com"));

        // Scenario 5: Invalid URL format (Expected: false)
        std::env::set_var("ALLOWED_ORIGINS", "https://example.com");
        std::env::set_var("AIOME_DEV_MODE", "0");
        assert!(!validate_redirect_url("not-a-valid-url"));
    }
}
