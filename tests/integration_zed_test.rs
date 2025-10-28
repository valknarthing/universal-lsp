//! Integration tests for Zed + Universal LSP + Claude
//!
//! This test suite mocks:
//! - Claude MCP Server (HTTP)
//! - Universal LSP Server (stdio)
//! - Zed Editor Configuration
//! - Multiple language proxies (pyright, rust-analyzer, tsserver)

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;
use std::path::PathBuf;

/// Mock MCP Server with language-aware responses
struct MockMultiLanguageMcpServer {
    port: u16,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MockMultiLanguageMcpServer {
    async fn start(port: u16) -> Self {
        use warp::Filter;

        let health_route = warp::path("health")
            .map(|| warp::reply::json(&json!({"status": "healthy", "languages": ["python", "rust", "typescript", "svelte"]})));

        let mcp_route = warp::post()
            .and(warp::path::end())
            .and(warp::body::json())
            .map(|body: Value| {
                let request_type = body["request_type"].as_str().unwrap_or("unknown");
                let uri = body["uri"].as_str().unwrap_or("");

                // Detect language from URI
                let language = if uri.ends_with(".py") {
                    "python"
                } else if uri.ends_with(".rs") {
                    "rust"
                } else if uri.ends_with(".ts") || uri.ends_with(".tsx") {
                    "typescript"
                } else if uri.ends_with(".svelte") {
                    "svelte"
                } else {
                    "unknown"
                };

                let suggestions = match (request_type, language) {
                    ("completion", "python") => vec![
                        "async def",
                        "import asyncio",
                        "from typing import",
                        "class Meta:",
                        "@dataclass"
                    ],
                    ("completion", "rust") => vec![
                        "pub fn",
                        "impl Trait for",
                        "async fn",
                        "#[derive(Debug)]",
                        "match self"
                    ],
                    ("completion", "typescript") => vec![
                        "async function",
                        "interface Props",
                        "type Result =",
                        "const [state, setState]",
                        "export default"
                    ],
                    ("completion", "svelte") => vec![
                        "<script lang=\"ts\">",
                        "{#if condition}",
                        "on:click={handler}",
                        "bind:value",
                        "$: reactive"
                    ],
                    _ => vec![]
                };

                let documentation = format!(
                    "AI-enhanced {} documentation for {}",
                    request_type, language
                );

                warp::reply::json(&json!({
                    "suggestions": suggestions,
                    "documentation": documentation,
                    "confidence": 0.91,
                    "language": language
                }))
            });

        let routes = health_route.or(mcp_route);

        let server_handle = tokio::spawn(async move {
            warp::serve(routes)
                .run(([127, 0, 0, 1], port))
                .await;
        });

        tokio::time::sleep(Duration::from_millis(500)).await;

        Self {
            port,
            server_handle: Some(server_handle),
        }
    }

    fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    async fn stop(mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

/// Mock LSP Client with Zed-specific configuration
struct ZedLspClient {
    process: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    request_id: i64,
}

impl ZedLspClient {
    /// Start Universal LSP with Zed-style configuration
    async fn start(mcp_url: &str, with_proxies: bool) -> anyhow::Result<Self> {
        let mut args = vec![
            "--log-level=info".to_string(),
            format!("--mcp-pre={}", mcp_url),
            "--mcp-timeout=3000".to_string(),
            "--mcp-cache=true".to_string(),
            "--max-concurrent=200".to_string(),
        ];

        if with_proxies {
            args.extend(vec![
                "--lsp-proxy=python=pyright".to_string(),
                "--lsp-proxy=rust=rust-analyzer".to_string(),
                "--lsp-proxy=typescript=typescript-language-server".to_string(),
            ]);
        }

        let mut process = Command::new("target/release/universal-lsp")
            .args(&args)
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

    async fn initialize(&mut self) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": "file:///tmp/zed-test",
                "capabilities": {
                    "textDocument": {
                        "completion": {
                            "completionItem": {
                                "snippetSupport": true,
                                "documentationFormat": ["markdown", "plaintext"]
                            }
                        },
                        "hover": {
                            "contentFormat": ["markdown", "plaintext"]
                        }
                    },
                    "workspace": {
                        "configuration": true
                    }
                },
                "workspaceFolders": [{
                    "uri": "file:///tmp/zed-test",
                    "name": "zed-test"
                }]
            }
        });

        self.request_id += 1;
        self.send_request(request).await?;

        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        self.send_notification(initialized).await?;

        self.read_response().await
    }

    async fn completion(&mut self, uri: &str, line: u32, character: u32) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": {
                    "triggerKind": 1,
                    "triggerCharacter": null
                }
            }
        });

        self.request_id += 1;
        self.send_request(request).await?;
        self.read_response().await
    }

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

    async fn send_request(&mut self, request: Value) -> anyhow::Result<()> {
        let message = serde_json::to_string(&request)?;
        let header = format!("Content-Length: {}\r\n\r\n", message.len());

        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    async fn send_notification(&mut self, notification: Value) -> anyhow::Result<()> {
        self.send_request(notification).await
    }

    async fn read_response(&mut self) -> anyhow::Result<Value> {
        let mut content_length = 0;

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

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut self.stdout, &mut body).await?;

        let response: Value = serde_json::from_slice(&body)?;
        Ok(response)
    }

    async fn shutdown(mut self) -> anyhow::Result<()> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "shutdown",
            "params": null
        });

        self.send_request(request).await?;
        self.read_response().await?;

        let exit = json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        });
        self.send_notification(exit).await?;

        self.process.kill().await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_zed_basic_initialization() {
    let mcp_server = MockMultiLanguageMcpServer::start(4001).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    let response = timeout(Duration::from_secs(5), lsp_client.initialize())
        .await
        .expect("Initialize timeout")
        .expect("Initialize failed");

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["capabilities"].is_object());

    let caps = &response["result"]["capabilities"];
    assert!(caps["textDocumentSync"].is_number() || caps["textDocumentSync"].is_object());
    assert!(caps["completionProvider"].is_object());
    assert!(caps["hoverProvider"].as_bool().unwrap_or(false));

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_python_completion() {
    let mcp_server = MockMultiLanguageMcpServer::start(4002).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = timeout(
        Duration::from_secs(5),
        lsp_client.completion("file:///tmp/test.py", 10, 0),
    )
    .await
    .expect("Completion timeout")
    .expect("Completion failed");

    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected completion array");
    assert!(!items.is_empty());

    // Check for Python-specific suggestions
    let labels: Vec<String> = items
        .iter()
        .filter_map(|item| item["label"].as_str().map(String::from))
        .collect();

    let has_python_suggestion = labels.iter().any(|label|
        label.contains("async") || label.contains("def") || label.contains("import")
    );

    assert!(has_python_suggestion, "Expected Python-specific completions");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_rust_completion() {
    let mcp_server = MockMultiLanguageMcpServer::start(4003).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = lsp_client
        .completion("file:///tmp/test.rs", 5, 0)
        .await
        .expect("Completion failed");

    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected completion array");
    assert!(!items.is_empty());

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_svelte_completion() {
    let mcp_server = MockMultiLanguageMcpServer::start(4004).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = lsp_client
        .completion("file:///tmp/Component.svelte", 1, 0)
        .await
        .expect("Completion failed");

    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected completion array");
    assert!(!items.is_empty());

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_with_proxies() {
    let mcp_server = MockMultiLanguageMcpServer::start(4005).await;

    // Start with proxy configuration
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), true)
        .await
        .expect("Failed to start LSP with proxies");

    lsp_client.initialize().await.expect("Initialize failed");

    // Test that completions still work with proxy config
    let response = lsp_client
        .completion("file:///tmp/test.py", 1, 0)
        .await
        .expect("Completion with proxy failed");

    assert_eq!(response["jsonrpc"], "2.0");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_hover_with_mcp() {
    let mcp_server = MockMultiLanguageMcpServer::start(4006).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = timeout(
        Duration::from_secs(5),
        lsp_client.hover("file:///tmp/test.ts", 10, 5),
    )
    .await
    .expect("Hover timeout")
    .expect("Hover failed");

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["contents"].is_string()
            || response["result"]["contents"].is_object());

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_multi_language_session() {
    let mcp_server = MockMultiLanguageMcpServer::start(4007).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Test multiple languages in same session (like Zed would)
    let languages = vec![
        ("file:///tmp/app.py", "Python"),
        ("file:///tmp/main.rs", "Rust"),
        ("file:///tmp/App.svelte", "Svelte"),
        ("file:///tmp/index.ts", "TypeScript"),
    ];

    for (uri, lang) in languages {
        let response = lsp_client
            .completion(uri, 1, 0)
            .await
            .unwrap_or_else(|e| panic!("{} completion failed: {}", lang, e));

        assert_eq!(response["jsonrpc"], "2.0", "{} completion invalid", lang);

        let items = response["result"].as_array().expect("Expected array");
        assert!(!items.is_empty(), "{} returned no completions", lang);
    }

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_concurrent_requests() {
    let mcp_server = MockMultiLanguageMcpServer::start(4008).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Simulate rapid typing (like in Zed)
    for i in 0..20 {
        let response = lsp_client
            .completion("file:///tmp/test.ts", i, 0)
            .await
            .expect("Concurrent completion failed");

        assert_eq!(response["jsonrpc"], "2.0");
    }

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_zed_performance_target() {
    let mcp_server = MockMultiLanguageMcpServer::start(4009).await;
    let mut lsp_client = ZedLspClient::start(&mcp_server.url(), false)
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Measure p95 latency
    let mut latencies = Vec::new();

    for i in 0..20 {
        let start = std::time::Instant::now();

        let _ = lsp_client
            .completion("file:///tmp/test.py", i, 0)
            .await
            .expect("Completion failed");

        latencies.push(start.elapsed());
    }

    latencies.sort();
    let p95_index = (latencies.len() as f64 * 0.95) as usize;
    let p95_latency = latencies[p95_index];

    // Zed target: <100ms p95
    assert!(
        p95_latency < Duration::from_millis(100),
        "P95 latency too high: {:?}",
        p95_latency
    );

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}
