/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;

use super::config::McpDiscoveryFile;

/// Canonical list of OAuth-capable MCP providers.
/// Single source of truth — used by authorize, callback, and bootstrap credential loading.
pub const ALLOWED_OAUTH_PROVIDERS: &[&str] = &["github", "slack", "notion", "discord", "figma"];

pub(crate) fn provider_to_asset_id(provider: &str) -> uuid::Uuid {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("aiome:mcp:oauth:{}", provider).as_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    uuid::Uuid::from_bytes(bytes)
}

#[derive(Clone, Debug)]
pub struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: secrecy::SecretString,
}

/// Static mapping from provider name to OAuth token endpoint URL.
fn oauth_token_url(provider: &str) -> Option<String> {
    // S-1 fix: restrict test-only override to test builds to prevent CWE-350 in production
    #[cfg(test)]
    if let Ok(override_url) = std::env::var("TEST_OAUTH_TOKEN_URL_OVERRIDE") {
        return Some(override_url);
    }
    match provider {
        "github" => Some("https://github.com/login/oauth/access_token".to_string()),
        "slack" => Some("https://slack.com/api/oauth.v2.access".to_string()),
        "notion" => Some("https://api.notion.com/v1/oauth/token".to_string()),
        "discord" => Some("https://discord.com/api/oauth2/token".to_string()),
        "figma" => Some("https://api.figma.com/v1/oauth/token".to_string()),
        _ => None,
    }
}

/// The fixed redirect URI used for all OAuth providers.
/// This MUST match the redirect URI registered with each provider's OAuth App.
fn get_oauth_redirect_uri() -> String {
    std::env::var("OAUTH_REDIRECT_URI")
        .unwrap_or_else(|_| "http://api-server:3015/api/v1/mcp/oauth/callback".to_string())
}

/// Static mapping from provider name to OAuth authorization URL template.
/// Prevents Open Redirect (CWE-601) by never interpolating user input into the host.
/// Includes `response_type=code` (RFC 6749 §4.1.1) and a CSRF `state` parameter.
/// All query parameters are percent-encoded (CWE-116 / RFC 3986 §2.1).
fn oauth_auth_url(provider: &str, client_id: &str, state: &str) -> Option<String> {
    let ru = get_oauth_redirect_uri();
    // S-1 fix: percent-encode all dynamic query parameters to prevent injection
    let enc = |s: &str| -> String { url::form_urlencoded::byte_serialize(s.as_bytes()).collect() };
    let cid = enc(client_id);
    let redirect = enc(&ru);
    let st = enc(state);
    match provider {
        "github" => Some(format!("https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=repo&state={}", cid, redirect, st)),
        "slack" => Some(format!("https://slack.com/oauth/v2/authorize?client_id={}&redirect_uri={}&state={}", cid, redirect, st)),
        "notion" => Some(format!("https://api.notion.com/v1/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&state={}", cid, redirect, st)),
        "discord" => Some(format!("https://discord.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}", cid, redirect, st)),
        "figma" => Some(format!("https://www.figma.com/oauth?client_id={}&redirect_uri={}&response_type=code&state={}", cid, redirect, st)),
        _ => None,
    }
}

#[derive(Deserialize)]
pub struct OauthAuthQuery {
    pub provider: String,
}

/// [GET] /api/v1/mcp/oauth/authorize
/// Initiates the OAuth 2.0 Authorization Code flow for an MCP provider.
pub async fn oauth_authorize(
    State(state): State<crate::AppState>,
    Query(query): Query<OauthAuthQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // C-1 fix: validate provider against canonical allow-list
    if !ALLOWED_OAUTH_PROVIDERS.contains(&query.provider.as_str()) {
        return Err(crate::error::AppError::bad_request(
            "Invalid or unsupported OAuth provider",
        ));
    }

    // C-5 fix: early error instead of 'dummy' fallback
    let creds = state.mcp_oauth_secrets.get(&query.provider).ok_or_else(|| {
        crate::error::AppError::bad_request(
            "OAuth credentials not configured for this provider. Set {PROVIDER}_CLIENT_ID and {PROVIDER}_CLIENT_SECRET in .env",
        )
    })?;

    // E-2 fix: pkce_cache MUST be initialized — without it, CSRF protection is impossible
    let pkce_cache = state.pkce_cache.as_opt().ok_or_else(|| {
        tracing::error!("PKCE cache not initialized — OAuth flow cannot proceed safely");
        crate::error::AppError::internal("OAuth infrastructure not ready")
    })?;

    // C-4 fix: generate CSRF state token and cache it for validation in callback
    let csrf_state = uuid::Uuid::new_v4().to_string();
    // Store (None, csrf_state) — the first element is reserved for PKCE code_verifier
    pkce_cache
        .insert(csrf_state.clone(), (None, query.provider.clone()))
        .await;

    let auth_url =
        oauth_auth_url(&query.provider, &creds.client_id, &csrf_state).ok_or_else(|| {
            crate::error::AppError::bad_request("Invalid or unsupported OAuth provider")
        })?;

    Ok(Redirect::temporary(&auth_url))
}

/// Enables an OAuth provider in the MCP config file and injects the access token.
///
/// # Security Note (CWE-312)
/// Tokens are currently stored in plaintext in `mcp_servers.json`.
/// Production deployments MUST migrate to a secrets manager (e.g., OS Keychain,
/// HashiCorp Vault, or encrypted-at-rest storage) before public release.
///
/// # Concurrency Note (TOCTOU)
/// This function reads and writes the config file without file locking.
/// In a multi-process deployment, use `flock` or equivalent to prevent races.
pub async fn enable_oauth_provider(
    provider: &str,
    access_token: &str,
    override_path: Option<&std::path::Path>,
    vault_backend: Option<std::sync::Arc<dyn aiome_core_contracts::vault_backend::VaultBackend>>,
) -> anyhow::Result<()> {
    let config_path = if let Some(p) = override_path {
        p.to_path_buf()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".aiome/mcp_servers.json")
    };

    // S-2 fix: fail explicitly when config file doesn't exist instead of silently succeeding
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "MCP config file not found at {}. Cannot persist OAuth token.",
            config_path.display()
        ));
    }

    let content = std::fs::read_to_string(&config_path)?;
    let mut discovery: McpDiscoveryFile = serde_json::from_str(&content)?;

    if let Some(config) = discovery.mcp_servers.get_mut(provider) {
        config.disabled = Some(false);

        let safe_token = if let Some(vault) = &vault_backend {
            let asset_id = provider_to_asset_id(provider);
            vault.store_dek(asset_id, access_token.as_bytes()).await?;
            "$VAULT_TOKEN".to_string()
        } else {
            access_token.to_string()
        };

        // ⚠️ Plaintext token storage is migrated to Vault. Tokens are replaced with $VAULT_TOKEN placeholder.
        match provider {
            "github" => {
                config
                    .env
                    .insert("GITHUB_PERSONAL_ACCESS_TOKEN".to_string(), safe_token);
            }
            "notion" => {
                config.env.insert("NOTION_API_KEY".to_string(), safe_token);
            }
            "discord" => {
                config.env.insert("DISCORD_TOKEN".to_string(), safe_token);
            }
            "slack" | "figma" => {
                let header_val = if safe_token == "$VAULT_TOKEN" {
                    "Bearer $VAULT_TOKEN".to_string()
                } else {
                    format!("Bearer {}", safe_token)
                };
                config
                    .headers
                    .insert("Authorization".to_string(), header_val);
            }
            _ => {}
        }
    }

    std::fs::write(&config_path, serde_json::to_string_pretty(&discovery)?)?;
    Ok(())
}

#[derive(Deserialize)]
pub struct OauthCallbackQuery {
    pub provider: String,
    pub code: Option<String>,
    pub state: Option<String>,
}

/// Exchange an authorization code for an access token (RFC 6749 §4.1.3).
/// Sends `grant_type=authorization_code`, `redirect_uri`, and provider credentials.
async fn exchange_code_for_token(
    provider: &str,
    code: &str,
    creds: &OAuthCredentials,
) -> anyhow::Result<String> {
    let token_url = oauth_token_url(provider)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", provider))?;

    let client = aiome_core::http::get_http_client();
    let redirect_uri = get_oauth_redirect_uri();
    // C-2 fix: include grant_type and redirect_uri per RFC 6749 §4.1.3
    let resp = client
        .post(&token_url)
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", creds.client_id.as_str()),
            (
                "client_secret",
                secrecy::ExposeSecret::expose_secret(&creds.client_secret),
            ),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await?;

    // C-6 fix: check HTTP status before attempting JSON parse
    let status = resp.status();
    // E-3 fix: capture raw text first to provide useful diagnostics on non-JSON responses
    let raw_body = resp.text().await?;
    let body: serde_json::Value = serde_json::from_str(&raw_body).map_err(|e| {
        // E-1 fix: use char-safe truncation to prevent panic on multi-byte UTF-8 boundaries
        let truncated: String = raw_body.chars().take(500).collect();
        anyhow::anyhow!(
            "Token endpoint returned non-JSON response (HTTP {}): parse error={}, body={}",
            status,
            e,
            truncated
        )
    })?;
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Token endpoint returned HTTP {}: {}",
            status,
            body
        ));
    }

    body["access_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("No access_token in response: {}", body))
}

/// [GET] /api/v1/mcp/oauth/callback
/// Handles the OAuth callback after the user authorizes with the external provider.
pub async fn oauth_callback(
    State(state): State<crate::AppState>,
    Query(query): Query<OauthCallbackQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    if !ALLOWED_OAUTH_PROVIDERS.contains(&query.provider.as_str()) {
        return Err(crate::error::AppError::bad_request(
            "Invalid or unsupported OAuth provider",
        ));
    }

    // C-7 fix: use let-else pattern instead of is_none() + unwrap()
    let Some(code) = &query.code else {
        return Err(crate::error::AppError::bad_request(
            "Missing authorization code",
        ));
    };

    // E-1 fix: pkce_cache MUST be initialized — without it, CSRF validation is impossible
    let pkce_cache = state.pkce_cache.as_opt().ok_or_else(|| {
        tracing::error!("PKCE cache not initialized — OAuth callback cannot validate CSRF");
        crate::error::AppError::internal("OAuth infrastructure not ready")
    })?;

    // C-4 fix: validate CSRF state parameter against cached value
    match &query.state {
        Some(state_param) => {
            let cached = pkce_cache.get(state_param).await;
            if cached.is_none() {
                tracing::warn!("OAuth CSRF state mismatch for provider={}", query.provider);
                return Err(crate::error::AppError::bad_request(
                    "Invalid or expired OAuth state parameter (possible CSRF)",
                ));
            }
            // Consume the state token (one-time use)
            pkce_cache.invalidate(state_param).await;
        }
        None => {
            tracing::warn!(
                "Missing OAuth state parameter for provider={}",
                query.provider
            );
            return Err(crate::error::AppError::bad_request(
                "Missing state parameter",
            ));
        }
    }

    let creds = state
        .mcp_oauth_secrets
        .get(&query.provider)
        .ok_or_else(|| {
            crate::error::AppError::bad_request(
                "OAuth credentials not configured for this provider",
            )
        })?;

    let access_token = match exchange_code_for_token(&query.provider, code, creds).await {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Token exchange failed for {}: {}", query.provider, e);
            return Err(crate::error::AppError::internal(
                "OAuth token exchange failed",
            ));
        }
    };

    // S-2 fix: propagate enable_oauth_provider failure as HTTP error instead of silent success
    if let Err(e) = enable_oauth_provider(
        &query.provider,
        &access_token,
        None,
        Some(state.vault_backend.get_inner().clone()),
    )
    .await
    {
        tracing::error!("Failed to enable OAuth provider {}: {}", query.provider, e);
        return Err(crate::error::AppError::internal(
            "Failed to persist OAuth provider configuration",
        ));
    }

    // Reload MCP discovery to activate the newly enabled provider
    if let Err(e) = super::discovery::discover_and_connect(
        &state.mcp_manager,
        &state.registry,
        Some(state.vault_backend.get_inner().clone()),
        state.config.get_inner(),
    )
    .await
    {
        tracing::error!(
            "Failed to reload MCP manager after OAuth for {}: {}",
            query.provider,
            e
        );
        // Non-fatal: provider is saved but discovery reload failed. Continue to dashboard.
    }

    // SAFETY: provider is already validated against ALLOWED_OAUTH_PROVIDERS,
    // which are all ASCII-only identifiers. No encoding needed.
    let dashboard_url = format!(
        "https://localhost:3000/dashboard?provider={}&status=success",
        query.provider
    );

    Ok(Redirect::temporary(&dashboard_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enable_oauth_provider_injects_token() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(temp_dir.join(".aiome")).unwrap();
        let config_path = temp_dir.join(".aiome/mcp_servers.json");

        let initial_json = r#"{
            "mcp_servers": {
                "github": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-github"],
                    "disabled": true,
                    "env": {}
                }
            }
        }"#;
        std::fs::write(&config_path, initial_json).unwrap();

        let result =
            enable_oauth_provider("github", "ghp_dummy123", Some(&config_path), None).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        let discovery: McpDiscoveryFile = serde_json::from_str(&content).unwrap();

        let github = discovery.mcp_servers.get("github").unwrap();
        assert_eq!(github.disabled, Some(false));
        assert_eq!(
            github
                .env
                .get("GITHUB_PERSONAL_ACCESS_TOKEN")
                .map(|s| s.as_str()),
            Some("ghp_dummy123")
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_exchange_code_for_token_wiremock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        std::env::set_var(
            "TEST_OAUTH_TOKEN_URL_OVERRIDE",
            format!("{}/token", mock_server.uri()),
        );

        let creds = OAuthCredentials {
            client_id: "test_id".to_string(),
            client_secret: secrecy::SecretString::from("test_secret".to_string()),
        };

        // 1. 正常系
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "gho_mock_token_123"
            })))
            .mount(&mock_server)
            .await;

        let result = exchange_code_for_token("github", "auth_code_xyz", &creds).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "gho_mock_token_123");

        mock_server.reset().await;

        // 2. 異常系: トークンなし
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error": "invalid_grant"
            })))
            .mount(&mock_server)
            .await;

        let result2 = exchange_code_for_token("github", "auth_code_xyz", &creds).await;
        assert!(result2.is_err());
        assert!(result2
            .unwrap_err()
            .to_string()
            .contains("No access_token in response"));

        mock_server.reset().await;

        // 3. 異常系: HTTP 401
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "message": "Bad credentials"
            })))
            .mount(&mock_server)
            .await;

        let result3 = exchange_code_for_token("github", "auth_code_xyz", &creds).await;
        assert!(result3.is_err());
        assert!(result3
            .unwrap_err()
            .to_string()
            .contains("Token endpoint returned HTTP 401"));
    }
}
