/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::mcp::types::{
    CallToolResult, JsonRpcRequest, JsonRpcResponse, ListToolsResult, McpTool,
};
use anyhow::{anyhow, Result};
use reqwest::Client;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicI64, Ordering};
use tracing::{error, info, warn};
use url::Url;

/// [A-4] SSRF Protection (CVE-2026-40933 related DNS Rebind Defense)
/// Validates that the URL is safe to connect to.
/// Blocks private IP ranges in production mode unless explicitly allowed.
async fn is_safe_url(url_str: &str) -> Result<()> {
    let url = Url::parse(url_str).map_err(|e| anyhow!("Invalid URL: {}", e))?;

    let localhost_allowed = is_localhost_allowed();

    if let Some(host) = url.host_str() {
        if host == "localhost" || host == "127.0.0.1" || host == "::1" {
            #[cfg(not(debug_assertions))]
            if !localhost_allowed {
                return Err(anyhow!("🚨 [SECURITY] localhost MCP blocked in production. Set MCP_ALLOW_LOCALHOST=true to allow."));
            }
            return Ok(());
        }
    }

    let host = url.host_str().unwrap_or("");
    let port = url.port_or_known_default().unwrap_or(80);

    // DNS Rebinding prevention: Resolve ALL IPs for this host
    if let Ok(addrs) = tokio::net::lookup_host(format!("{}:{}", host, port)).await {
        let mut resolved = false;
        for addr in addrs {
            resolved = true;
            // Normalize IPv4-mapped IPv6 (e.g. ::ffff:127.0.0.1 → 127.0.0.1)
            // to prevent SSRF bypass via dual-stack addressing
            let ip = normalize_ip(addr.ip());
            if ip.is_loopback() {
                #[cfg(not(debug_assertions))]
                if !localhost_allowed {
                    return Err(anyhow!("🚨 [SECURITY] Loopback IP blocked: {}", ip));
                }
            } else {
                #[cfg(not(debug_assertions))]
                if is_private_ip(ip) {
                    return Err(anyhow!("🚨 [SECURITY] Private IP blocked: {}", ip));
                }
            }
        }

        if !resolved {
            return Err(anyhow!("🚨 [SECURITY] Could not resolve host: {}", host));
        }
    } else {
        return Err(anyhow!(
            "🚨 [SECURITY] DNS resolution failed for host: {}",
            host
        ));
    }

    Ok(())
}

/// Checks whether localhost MCP connections are explicitly allowed.
/// Only `"true"` (case-insensitive) is treated as enabled.
fn is_localhost_allowed() -> bool {
    std::env::var("MCP_ALLOW_LOCALHOST")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Normalize IPv4-mapped IPv6 addresses (e.g., `::ffff:127.0.0.1`) to their
/// IPv4 equivalents. This prevents SSRF bypass via dual-stack addressing
/// where `::ffff:10.0.0.1` would evade IPv4 private-range checks.
fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            // ::ffff:x.x.x.x (IPv4-mapped IPv6)
            // ::x.x.x.x (IPv4-compatible IPv6, deprecated but defense-in-depth)
            if segs[0] == 0
                && segs[1] == 0
                && segs[2] == 0
                && segs[3] == 0
                && segs[4] == 0
                && (segs[5] == 0xffff || segs[5] == 0)
                && (segs[5] != 0 || segs[6] != 0 || segs[7] != 0)
            // exclude ::0.0.0.0
            {
                IpAddr::V4(std::net::Ipv4Addr::new(
                    (segs[6] >> 8) as u8,
                    segs[6] as u8,
                    (segs[7] >> 8) as u8,
                    segs[7] as u8,
                ))
            } else {
                ip
            }
        }
        _ => ip,
    }
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private() || v4.is_link_local() // 169.254.0.0/16 — blocks cloud metadata (169.254.169.254)
        }
        IpAddr::V6(v6) => {
            let seg0 = v6.segments()[0];
            // Unique Local Address (fc00::/7)
            (seg0 & 0xfe00) == 0xfc00
            // Link-local (fe80::/10)
            || (seg0 & 0xffc0) == 0xfe80
        }
    }
}

/// [A-3] MCP HTTP Client
/// Implementation of MCP over HTTP (JSON-RPC).
/// Used for connecting to external services like AiToEarn.
pub struct McpHttpClient {
    pub id: String,
    url: String,
    client: Client,
    headers: HashMap<String, String>,
    request_counter: AtomicI64,
}

impl McpHttpClient {
    pub fn new(id: String, url: String, headers: HashMap<String, String>) -> Self {
        // KI mandate: Use global HTTP client for SSRF redirect protection & connection pooling
        Self {
            id,
            url,
            client: aiome_core::http::get_http_client().clone(),
            headers,
            request_counter: AtomicI64::new(1),
        }
    }

    pub async fn call(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // [Gate: SSRF]
        is_safe_url(&self.url).await?;

        let id_val = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::json!(id_val)),
        };

        let mut rb = self.client.post(&self.url);
        for (k, v) in &self.headers {
            rb = rb.header(k, v);
        }

        let response = rb.json(&request).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP error: {}", response.status()));
        }

        let rpc_res: JsonRpcResponse = response.json().await?;

        if let Some(error) = rpc_res.error {
            return Err(anyhow!("MCP Error ({}): {}", error.code, error.message));
        }

        rpc_res
            .result
            .ok_or_else(|| anyhow!("Empty result from MCP"))
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let res = self.call("tools/list", None).await?;
        let list: ListToolsResult = serde_json::from_value(res)?;
        Ok(list.tools)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        let res = self.call("tools/call", Some(params)).await?;
        Ok(serde_json::from_value(res)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_mcp_http_client_list_tools() {
        let _server = MockServer::start().await;
        let client = McpHttpClient::new(
            "test".to_string(),
            _server.uri(),
            [("x-api-key".to_string(), "test-key".to_string())].into(),
        );

        Mock::given(method("POST"))
            .and(header("x-api-key", "test-key"))
            .and(body_partial_json(json!({"method": "tools/list"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "tools": [
                        {
                            "name": "test_tool",
                            "description": "A test tool",
                            "inputSchema": { "type": "object" }
                        }
                    ]
                }
            })))
            .mount(&_server)
            .await;

        let tools = client.list_tools().await.unwrap(); // allow-anti-pattern
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_mcp_http_client_ssrf_protection() {
        // Should allow localhost in debug mode
        assert!(is_safe_url("http://localhost:8080/mcp").await.is_ok()); // allow-anti-pattern
        assert!(is_safe_url("http://127.0.0.1:8080/mcp").await.is_ok()); // allow-anti-pattern

        // Should block private IPs if in release mode (simulated)
        assert!(is_private_ip("10.0.0.1".parse().unwrap())); // allow-anti-pattern
        assert!(is_private_ip("192.168.1.1".parse().unwrap())); // allow-anti-pattern
        assert!(!is_private_ip("8.8.8.8".parse().unwrap())); // allow-anti-pattern

        // [Gate: IPv4 link-local / cloud metadata]
        assert!(is_private_ip("169.254.169.254".parse().unwrap())); // allow-anti-pattern
        assert!(is_private_ip("169.254.1.1".parse().unwrap())); // allow-anti-pattern

        // [Gate: IPv4-mapped IPv6 normalization]
        let mapped_loopback = normalize_ip("::ffff:127.0.0.1".parse().unwrap()); // allow-anti-pattern
        assert!(
            mapped_loopback.is_loopback(),
            "::ffff:127.0.0.1 must normalize to loopback"
        );

        let mapped_private = normalize_ip("::ffff:10.0.0.1".parse().unwrap()); // allow-anti-pattern
        assert!(
            is_private_ip(mapped_private),
            "::ffff:10.0.0.1 must normalize to private"
        );

        let mapped_meta = normalize_ip("::ffff:169.254.169.254".parse().unwrap()); // allow-anti-pattern
        assert!(
            is_private_ip(mapped_meta),
            "::ffff:169.254.169.254 must be blocked (cloud metadata)"
        );

        // [Gate: IPv6 link-local]
        assert!(is_private_ip("fe80::1".parse().unwrap())); // allow-anti-pattern

        // [Gate: IPv6 ULA]
        assert!(is_private_ip("fc00::1".parse().unwrap())); // allow-anti-pattern
        assert!(is_private_ip("fd12::1".parse().unwrap())); // allow-anti-pattern

        // Public IPs should pass
        assert!(!is_private_ip("2001:db8::1".parse().unwrap())); // allow-anti-pattern

        // [Gate: IPv4-compatible IPv6 (deprecated ::x.x.x.x)]
        let compat_private = normalize_ip("::10.0.0.1".parse().unwrap()); // allow-anti-pattern
        assert!(
            is_private_ip(compat_private),
            "::10.0.0.1 (IPv4-compatible) must normalize to private"
        );

        let compat_meta = normalize_ip("::169.254.169.254".parse().unwrap()); // allow-anti-pattern
        assert!(
            is_private_ip(compat_meta),
            "::169.254.169.254 (IPv4-compatible) must be blocked"
        );

        // ::0.0.0.0 should NOT be normalized (it's the unspecified address)
        let unspecified = normalize_ip("::".parse().unwrap()); // allow-anti-pattern
        assert!(
            unspecified.is_unspecified(),
            ":: must remain as unspecified, not normalized"
        );
    }
}
