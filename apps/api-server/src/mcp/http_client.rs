/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
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

/// [A-4] SSRF Protection
/// Validates that the URL is safe to connect to.
/// Blocks private IP ranges in production mode.
fn is_safe_url(url_str: &str) -> Result<()> {
    let url = Url::parse(url_str).map_err(|e| anyhow!("Invalid URL: {}", e))?;

    // Always allow localhost/127.0.0.1 for development (local Docker AiToEarn)
    if let Some(host) = url.host_str() {
        if host == "localhost" || host == "127.0.0.1" {
            return Ok(());
        }
    }

    if let Ok(ip) = url.host_str().unwrap_or("").parse::<IpAddr>() {
        if ip.is_loopback() {
            return Ok(());
        }

        #[cfg(not(debug_assertions))]
        {
            if is_private_ip(ip) {
                return Err(anyhow!(
                    "🚨 [SECURITY] Blocked connection to private IP: {}",
                    ip
                ));
            }
        }
    }

    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private(),
        IpAddr::V6(v6) => {
            (v6.segments()[0] & 0xff00) == 0xfc00 || (v6.segments()[0] & 0xff00) == 0xfd00
        } // Unique Local Address
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
        Self {
            id,
            url,
            client: Client::new(),
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
        is_safe_url(&self.url)?;

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

        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[test]
    fn test_mcp_http_client_ssrf_protection() {
        // Should allow localhost
        assert!(is_safe_url("http://localhost:8080/mcp").is_ok());
        assert!(is_safe_url("http://127.0.0.1:8080/mcp").is_ok());

        // Should block private IPs if in release mode (simulated)
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
    }
}
