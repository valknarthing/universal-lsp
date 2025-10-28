//! Integration tests for VSCode + Universal LSP + Claude
//!
//! This test suite mocks:
//! - Claude MCP Server (HTTP)
//! - Universal LSP Server (stdio)
//! - VSCode Extension Host
//! - TypeScript Language Server

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

/// Mock MCP Server for testing
struct MockMcpServer {
    port: u16,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MockMcpServer {
    /// Start a mock MCP server on localhost
    async fn start(port: u16) -> Self {
        use warp::Filter;

        let health_route = warp::path("health")
            .map(|| warp::reply::json(&json!({"status": "healthy"})));

        let mcp_route = warp::post()
            .and(warp::path::end())
            .and(warp::body::json())
            .map(|body: Value| {
                let request_type = body["request_type"].as_str().unwrap_or("unknown");

                let response = match request_type {
                    "completion" => json!({
                        "suggestions": [
                            "async function",
                            "const result =",
                            "import { useState }",
                            "export default",
                            "interface Props"
                        ],
                        "documentation": "AI-powered TypeScript completions",
                        "confidence": 0.92
                    }),
                    "hover" => json!({
                        "suggestions": [],
                        "documentation": "AI-enhanced documentation for this symbol",
                        "confidence": 0.88
                    }),
                    _ => json!({
                        "suggestions": [],
                        "documentation": null,
                        "confidence": null
                    })
                };

                warp::reply::json(&response)
            });

        let routes = health_route.or(mcp_route);

        let server_handle = tokio::spawn(async move {
            warp::serve(routes)
                .run(([127, 0, 0, 1], port))
                .await;
        });

        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(500)).await;

        Self {
            port,
            server_handle: Some(server_handle),
        }
    }

    /// Get the server URL
    fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Stop the mock server
    async fn stop(mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

/// Mock LSP Client for testing
struct MockLspClient {
    process: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    request_id: i64,
}

impl MockLspClient {
    /// Start Universal LSP with MCP configuration
    async fn start(mcp_url: &str) -> anyhow::Result<Self> {
        let mut process = Command::new("target/release/universal-lsp")
            .args(&[
                "--log-level=debug",
                &format!("--mcp-pre={}", mcp_url),
                "--mcp-timeout=3000",
                "--mcp-cache=true",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdin = process.stdin.take().unwrap();
        let stdout = BufReader::new(process.stdout.take().unwrap());

        Ok(Self {
            process,
            stdin,
            stdout,
            request_id: 1,
        })
    }

    /// Send LSP initialize request
    async fn initialize(&mut self) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": "file:///tmp/test",
                "capabilities": {
                    "textDocument": {
                        "completion": {
                            "completionItem": {
                                "snippetSupport": true
                            }
                        }
                    }
                }
            }
        });

        self.request_id += 1;
        self.send_request(request).await?;

        // Send initialized notification
        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });

        self.send_notification(initialized).await?;

        // Read initialize response
        self.read_response().await
    }

    /// Send completion request
    async fn completion(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "triggerKind": 1 }
            }
        });

        self.request_id += 1;
        self.send_request(request).await?;
        self.read_response().await
    }

    /// Send hover request
    async fn hover(&mut self, uri: &str, line: u32, character: u32) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        });

        self.request_id += 1;
        self.send_request(request).await?;
        self.read_response().await
    }

    /// Send LSP request
    async fn send_request(&mut self, request: Value) -> anyhow::Result<()> {
        let message = serde_json::to_string(&request)?;
        let header = format!("Content-Length: {}\r\n\r\n", message.len());

        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    /// Send LSP notification (no response expected)
    async fn send_notification(&mut self, notification: Value) -> anyhow::Result<()> {
        self.send_request(notification).await
    }

    /// Read LSP response
    async fn read_response(&mut self) -> anyhow::Result<Value> {
        let mut content_length = 0;

        // Read headers
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).await?;

            if line == "\r\n" || line == "\n" {
                break;
            }

            if line.starts_with("Content-Length:") {
                let len_str = line.trim_start_matches("Content-Length:").trim();
                content_length = len_str.parse()?;
            }
        }

        // Read body
        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut self.stdout, &mut body).await?;

        let response: Value = serde_json::from_slice(&body)?;
        Ok(response)
    }

    /// Shutdown the LSP server
    async fn shutdown(mut self) -> anyhow::Result<()> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "shutdown",
            "params": null
        });

        self.send_request(request).await?;
        self.read_response().await?;

        // Send exit notification
        let exit = json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        });

        self.send_notification(exit).await?;

        // Kill the process
        self.process.kill().await?;

        Ok(())
    }
}

#[tokio::test]
async fn test_vscode_initialization() {
    // Start mock MCP server
    let mcp_server = MockMcpServer::start(3001).await;

    // Start Universal LSP
    let mut lsp_client = MockLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    // Initialize
    let response = timeout(Duration::from_secs(5), lsp_client.initialize())
        .await
        .expect("Initialize timeout")
        .expect("Initialize failed");

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["capabilities"].is_object());
    assert!(response["result"]["serverInfo"]["name"].as_str().unwrap().contains("Universal"));

    // Cleanup
    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_vscode_completion_with_mcp() {
    let mcp_server = MockMcpServer::start(3002).await;
    let mut lsp_client = MockLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Request completion
    let response = timeout(
        Duration::from_secs(5),
        lsp_client.completion("file:///tmp/test.ts", 10, 5),
    )
    .await
    .expect("Completion timeout")
    .expect("Completion failed");

    // Verify MCP-enhanced completions
    assert_eq!(response["jsonrpc"], "2.0");

    let items = response["result"].as_array().expect("Expected array");
    assert!(!items.is_empty(), "Expected completion items");

    // Check for AI-powered suggestions
    let has_ai_suggestion = items.iter().any(|item| {
        item["detail"]
            .as_str()
            .map(|d| d.contains("AI") || d.contains("suggestion"))
            .unwrap_or(false)
    });

    assert!(has_ai_suggestion, "Expected AI-powered completions");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_vscode_hover_with_mcp() {
    let mcp_server = MockMcpServer::start(3003).await;
    let mut lsp_client = MockLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Request hover
    let response = timeout(
        Duration::from_secs(5),
        lsp_client.hover("file:///tmp/test.py", 5, 10),
    )
    .await
    .expect("Hover timeout")
    .expect("Hover failed");

    // Verify hover response
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["contents"].is_string() ||
            response["result"]["contents"].is_object());

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_vscode_mcp_fallback() {
    // Start LSP without MCP server (it should still work)
    let mut lsp_client = MockLspClient::start("http://localhost:9999")
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Request completion - should work with fallback
    let response = timeout(
        Duration::from_secs(5),
        lsp_client.completion("file:///tmp/test.js", 1, 0),
    )
    .await
    .expect("Completion timeout")
    .expect("Completion failed");

    // Should still get basic completions
    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected array");
    assert!(!items.is_empty(), "Expected fallback completions");

    lsp_client.shutdown().await.expect("Shutdown failed");
}

#[tokio::test]
async fn test_vscode_multiple_languages() {
    let mcp_server = MockMcpServer::start(3004).await;
    let mut lsp_client = MockLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Test Python
    let py_response = lsp_client
        .completion("file:///tmp/test.py", 1, 0)
        .await
        .expect("Python completion failed");
    assert_eq!(py_response["jsonrpc"], "2.0");

    // Test TypeScript
    let ts_response = lsp_client
        .completion("file:///tmp/test.ts", 1, 0)
        .await
        .expect("TypeScript completion failed");
    assert_eq!(ts_response["jsonrpc"], "2.0");

    // Test Rust
    let rs_response = lsp_client
        .completion("file:///tmp/test.rs", 1, 0)
        .await
        .expect("Rust completion failed");
    assert_eq!(rs_response["jsonrpc"], "2.0");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_vscode_performance() {
    let mcp_server = MockMcpServer::start(3005).await;
    let mut lsp_client = MockLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Measure completion latency
    let start = std::time::Instant::now();

    for i in 0..10 {
        let _ = lsp_client
            .completion("file:///tmp/test.ts", i, 0)
            .await
            .expect("Completion failed");
    }

    let elapsed = start.elapsed();
    let avg_latency = elapsed / 10;

    // Should be under 500ms per completion
    assert!(
        avg_latency < Duration::from_millis(500),
        "Average latency too high: {:?}",
        avg_latency
    );

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}
