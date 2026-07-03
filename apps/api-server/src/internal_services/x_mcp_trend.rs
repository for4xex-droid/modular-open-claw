/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::mcp::client::McpProcessManager;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::TrendItem;
use async_trait::async_trait;
use infrastructure::trend_sonar::TrendAdapter;
use std::sync::Arc;

pub struct XMcpTrendAdapter {
    mcp_manager: Arc<McpProcessManager>,
}

impl XMcpTrendAdapter {
    pub fn new(mcp_manager: Arc<McpProcessManager>) -> Self {
        Self { mcp_manager }
    }

    /// Strips control characters and normalizes whitespace
    fn sanitize_text(raw: &str) -> String {
        let no_newlines = raw.replace('\n', " ");
        let cleaned: String = no_newlines.chars().filter(|c| !c.is_control()).collect();
        // Truncate to maximum 280 characters for prompt safety
        if cleaned.chars().count() > 280 {
            let mut truncated: String = cleaned.chars().take(277).collect();
            truncated.push_str("...");
            truncated
        } else {
            cleaned
        }
    }

    /// Extracts text fields from JSON strings if the output is structured, otherwise returns the original text.
    fn extract_text_from_json(json_str: &str) -> Vec<String> {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            let mut extracted = Vec::new();
            // Case 1: X API response format with "data" array containing objects with "text"
            if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                for item in data {
                    if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                        extracted.push(t.to_string());
                    }
                }
            }
            // Case 2: Direct array of items
            else if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(t) = item.as_str() {
                        extracted.push(t.to_string());
                    } else if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                        extracted.push(t.to_string());
                    }
                }
            }
            // Case 3: Single object with "text"
            else if let Some(t) = val.get("text").and_then(|t| t.as_str()) {
                extracted.push(t.to_string());
            }

            if !extracted.is_empty() {
                return extracted;
            }
        }
        vec![json_str.to_string()]
    }
}

#[async_trait]
impl TrendAdapter for XMcpTrendAdapter {
    fn name(&self) -> &str {
        "XMcpTrend"
    }

    async fn fetch(&self, query: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // x_twitter MCP クライアントの取得を試みる
        let Some(client) = self.mcp_manager.get_client("x_twitter").await else {
            tracing::warn!(
                "X MCP client 'x_twitter' is not active or registered. Skipping XMcpTrend."
            );
            return Ok(vec![]);
        };

        // ツール一覧を取得し、検索ツール名 (search_posts / search_tweets) を特定
        let tools = match client.list_tools().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to list tools from X MCP: {}. Skipping.", e);
                return Ok(vec![]);
            }
        };

        let search_tool = tools
            .iter()
            .find(|t| t.name == "search_posts" || t.name == "search_tweets");
        let Some(tool) = search_tool else {
            tracing::warn!(
                "X MCP does not expose a search tool. Available: {:?}",
                tools.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
            return Ok(vec![]);
        };

        // 検索ツールの実行
        let args = serde_json::json!({
            "query": query,
            "limit": 10
        });

        let result = match client.call_tool(&tool.name, args).await {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("Failed to call X MCP search tool {}: {}", tool.name, e);
                return Ok(vec![]);
            }
        };

        if result.is_error {
            tracing::error!("X MCP search tool returned error: {:?}", result.content);
            return Ok(vec![]);
        }

        // 結果の TrendItem へのマッピング (最大 5 件)
        let mut trends = Vec::new();
        for content in result.content {
            if let crate::mcp::types::McpContent::Text { text } = content {
                let raw_texts = Self::extract_text_from_json(&text);
                for raw_text in raw_texts {
                    let cleaned = Self::sanitize_text(&raw_text);
                    if !cleaned.trim().is_empty() {
                        trends.push(TrendItem {
                            keyword: cleaned,
                            source: "X".to_string(), // Align source name with legacy adapter
                            score: 1.0,
                        });
                    }
                }
            }
        }

        // 5件までに制限
        trends.truncate(5);

        Ok(trends)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::client::McpEndpoint;
    use crate::mcp::http_client::McpHttpClient;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_x_mcp_trend_adapter_fetch_success() {
        // 1. Mock JSON-RPC over HTTP server to simulate X MCP
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_url = format!("http://{}", addr);

        tokio::spawn(async move {
            let mut request_count = 0;
            while let Ok((mut stream, _)) = listener.accept().await {
                request_count += 1;
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap();
                let request_str = String::from_utf8_lossy(&buf[..n]);

                let response_body = if request_str.contains("tools/list") {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "tools": [
                                {
                                    "name": "search_posts",
                                    "description": "Search posts",
                                    "inputSchema": {}
                                }
                            ]
                        },
                        "id": request_count
                    })
                } else if request_str.contains("tools/call") {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": "Rust 2027 edition\nreleased"
                                },
                                {
                                    "type": "text",
                                    "text": "AI OS scalability milestones"
                                }
                            ],
                            "isError": false
                        },
                        "id": request_count
                    })
                } else {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32601,
                            "message": "Method not found"
                        },
                        "id": request_count
                    })
                };

                let response_body_str = response_body.to_string();
                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body_str.len(),
                    response_body_str
                );
                stream.write_all(http_response.as_bytes()).await.unwrap();
                stream.flush().await.unwrap();
            }
        });

        // 2. Setup McpProcessManager and insert Mock client directly into clients map
        let mcp_manager = Arc::new(McpProcessManager::new());
        let http_client = Arc::new(McpHttpClient::new(
            "x_twitter".to_string(),
            server_url,
            HashMap::new(),
        ));
        let endpoint = Arc::new(McpEndpoint::Http(http_client));

        {
            let mut clients = mcp_manager.clients.lock().await;
            clients.insert("x_twitter".to_string(), endpoint);
        }

        // 3. Execute adapter
        let adapter = XMcpTrendAdapter::new(mcp_manager);
        let result = adapter.fetch("test_query").await;

        // 4. Assertions
        assert!(result.is_ok());
        let trends = result.unwrap();
        assert_eq!(trends.len(), 2);
        // Assert that newlines are correctly replaced by spaces
        assert_eq!(trends[0].keyword, "Rust 2027 edition released");
        assert_eq!(trends[0].source, "X");
        assert_eq!(trends[1].keyword, "AI OS scalability milestones");
    }

    #[tokio::test]
    async fn test_x_mcp_trend_adapter_fetch_json_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_url = format!("http://{}", addr);

        tokio::spawn(async move {
            let mut request_count = 0;
            while let Ok((mut stream, _)) = listener.accept().await {
                request_count += 1;
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap();
                let request_str = String::from_utf8_lossy(&buf[..n]);

                let response_body = if request_str.contains("tools/list") {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "tools": [
                                {
                                    "name": "search_posts",
                                    "description": "Search posts",
                                    "inputSchema": {}
                                }
                            ]
                        },
                        "id": request_count
                    })
                } else if request_str.contains("tools/call") {
                    // Simulate JSON-formatted response containing X API search data
                    let mock_x_api_json = serde_json::json!({
                        "data": [
                            {"id": "1", "text": "Structured JSON Tweet 1"},
                            {"id": "2", "text": "Structured JSON Tweet 2"}
                        ]
                    });
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": mock_x_api_json.to_string()
                                }
                            ],
                            "isError": false
                        },
                        "id": request_count
                    })
                } else {
                    serde_json::json!({"jsonrpc": "2.0", "result": {}, "id": request_count})
                };

                let response_body_str = response_body.to_string();
                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body_str.len(),
                    response_body_str
                );
                stream.write_all(http_response.as_bytes()).await.unwrap();
                stream.flush().await.unwrap();
            }
        });

        let mcp_manager = Arc::new(McpProcessManager::new());
        let http_client = Arc::new(McpHttpClient::new(
            "x_twitter".to_string(),
            server_url,
            HashMap::new(),
        ));
        let endpoint = Arc::new(McpEndpoint::Http(http_client));

        {
            let mut clients = mcp_manager.clients.lock().await;
            clients.insert("x_twitter".to_string(), endpoint);
        }

        let adapter = XMcpTrendAdapter::new(mcp_manager);
        let result = adapter.fetch("test_query").await;

        assert!(result.is_ok());
        let trends = result.unwrap();
        assert_eq!(trends.len(), 2);
        assert_eq!(trends[0].keyword, "Structured JSON Tweet 1");
        assert_eq!(trends[0].source, "X");
        assert_eq!(trends[1].keyword, "Structured JSON Tweet 2");
    }

    #[tokio::test]
    async fn test_x_mcp_trend_adapter_no_client_graceful_empty() {
        let mcp_manager = Arc::new(McpProcessManager::new());
        let adapter = XMcpTrendAdapter::new(mcp_manager);

        let result = adapter.fetch("rustlang").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_sanitize_text_truncation() {
        let long_text = "a".repeat(300);
        let sanitized = XMcpTrendAdapter::sanitize_text(&long_text);
        assert_eq!(sanitized.chars().count(), 280);
        assert!(sanitized.ends_with("..."));
    }
}
