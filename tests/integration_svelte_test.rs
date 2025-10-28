//! Integration tests for SvelteKit + Tailwind + Universal LSP + Claude
//!
//! This test suite mocks:
//! - Claude MCP Server with multi-language support (HTTP)
//! - Universal LSP Server (stdio)
//! - Mock FastAPI backend
//! - SvelteKit frontend language detection
//! - Tailwind CSS class completion

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

/// Full-stack MCP Server with SvelteKit + Python support
struct FullStackMcpServer {
    port: u16,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    stats: std::sync::Arc<std::sync::Mutex<ServerStats>>,
}

#[derive(Debug, Clone, Default)]
struct ServerStats {
    typescript_requests: u32,
    svelte_requests: u32,
    python_requests: u32,
    css_requests: u32,
    total_requests: u32,
}

impl FullStackMcpServer {
    async fn start(port: u16) -> Self {
        use warp::Filter;
        use std::sync::{Arc, Mutex};

        let stats = Arc::new(Mutex::new(ServerStats::default()));
        let stats_clone = Arc::clone(&stats);

        let health_route = warp::path("health").map(move || {
            let s = stats_clone.lock().unwrap().clone();
            warp::reply::json(&json!({
                "status": "healthy",
                "stack": "sveltekit + tailwind + fastapi",
                "stats": {
                    "typescript": s.typescript_requests,
                    "svelte": s.svelte_requests,
                    "python": s.python_requests,
                    "css": s.css_requests,
                    "total": s.total_requests
                }
            }))
        });

        let stats_clone2 = Arc::clone(&stats);
        let mcp_route = warp::post()
            .and(warp::path::end())
            .and(warp::body::json())
            .map(move |body: Value| {
                let request_type = body["request_type"].as_str().unwrap_or("unknown");
                let uri = body["uri"].as_str().unwrap_or("");

                // Detect language and framework
                let (language, framework) = if uri.ends_with(".svelte") {
                    ("svelte", Some("sveltekit"))
                } else if uri.ends_with(".ts") || uri.ends_with(".tsx") {
                    ("typescript", Some("sveltekit"))
                } else if uri.ends_with(".py") {
                    ("python", Some("fastapi"))
                } else if uri.ends_with(".css") || uri.contains("tailwind") {
                    ("css", Some("tailwind"))
                } else {
                    ("unknown", None)
                };

                // Update stats
                {
                    let mut s = stats_clone2.lock().unwrap();
                    s.total_requests += 1;
                    match language {
                        "typescript" => s.typescript_requests += 1,
                        "svelte" => s.svelte_requests += 1,
                        "python" => s.python_requests += 1,
                        "css" => s.css_requests += 1,
                        _ => {}
                    }
                }

                let suggestions = match (language, request_type) {
                    ("svelte", "completion") => vec![
                        "<script lang=\"ts\">",
                        "{#if condition}",
                        "{#each items as item}",
                        "on:click={handleClick}",
                        "bind:value={inputValue}",
                        "$: reactiveStatement",
                        "export let prop",
                        "<style>",
                        "import { onMount }",
                        "{@html content}"
                    ],
                    ("typescript", "completion") => vec![
                        "async function",
                        "const [state, setState]",
                        "interface Props {",
                        "type Result<T> =",
                        "export default",
                        "import { writable }",
                        "import { goto }",
                        "fetch('/api/')",
                        "try { ... } catch",
                        "Promise<void>"
                    ],
                    ("python", "completion") => vec![
                        "from fastapi import",
                        "async def endpoint(",
                        "@app.get(\"/api/\")",
                        "@app.post(\"/api/\")",
                        "class BaseModel:",
                        "from pydantic import",
                        "async with session:",
                        "return JSONResponse(",
                        "raise HTTPException(",
                        "from typing import"
                    ],
                    ("css", "completion") => vec![
                        "class=\"flex items-center",
                        "class=\"grid grid-cols-",
                        "class=\"bg-gradient-to-r",
                        "class=\"text-brand-500",
                        "class=\"rounded-lg shadow",
                        "@apply",
                        "hover:bg-",
                        "dark:text-",
                        "md:flex-row",
                        "transition-all"
                    ],
                    _ => vec![]
                };

                let system_prompt = match (language, framework) {
                    ("svelte", Some("sveltekit")) => "Svelte 4/5 + SvelteKit expert with reactivity patterns",
                    ("typescript", Some("sveltekit")) => "TypeScript + SvelteKit expert with type safety",
                    ("python", Some("fastapi")) => "Python + FastAPI expert with async patterns",
                    ("css", Some("tailwind")) => "Tailwind CSS 4 expert with utility-first patterns",
                    _ => "Full-stack development expert"
                };

                warp::reply::json(&json!({
                    "suggestions": suggestions,
                    "documentation": format!("{} - {}", system_prompt, request_type),
                    "confidence": 0.94,
                    "metadata": {
                        "language": language,
                        "framework": framework,
                        "stack": "sveltekit-tailwind-fastapi"
                    }
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
            stats,
        }
    }

    fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    fn get_stats(&self) -> ServerStats {
        self.stats.lock().unwrap().clone()
    }

    async fn stop(mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

/// LSP Client for full-stack testing
struct FullStackLspClient {
    process: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    request_id: i64,
}

impl FullStackLspClient {
    async fn start(mcp_url: &str) -> anyhow::Result<Self> {
        let mut process = Command::new("target/release/universal-lsp")
            .args(&[
                "--log-level=info",
                &format!("--mcp-pre={}", mcp_url),
                "--lsp-proxy=typescript=typescript-language-server",
                "--lsp-proxy=python=pyright",
                "--lsp-proxy=svelte=svelteserver",
                "--mcp-timeout=3000",
                "--mcp-cache=true",
                "--max-concurrent=200",
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

    async fn initialize(&mut self) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": "file:///tmp/fullstack-app",
                "capabilities": {
                    "textDocument": {
                        "completion": {
                            "completionItem": {
                                "snippetSupport": true,
                                "documentationFormat": ["markdown"]
                            }
                        }
                    }
                },
                "initializationOptions": {
                    "stack": "sveltekit-tailwind-fastapi"
                }
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
async fn test_fullstack_initialization() {
    let mcp_server = FullStackMcpServer::start(6001).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    let response = timeout(Duration::from_secs(5), lsp_client.initialize())
        .await
        .expect("Initialize timeout")
        .expect("Initialize failed");

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["capabilities"].is_object());

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_svelte_component_completion() {
    let mcp_server = FullStackMcpServer::start(6002).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = lsp_client
        .completion("file:///tmp/app/src/routes/+page.svelte", 10, 0)
        .await
        .expect("Svelte completion failed");

    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected array");
    assert!(!items.is_empty());

    // Verify Svelte-specific suggestions
    let labels: Vec<String> = items
        .iter()
        .filter_map(|item| item["label"].as_str().map(String::from))
        .collect();

    let has_svelte = labels.iter().any(|l|
        l.contains("{#if") || l.contains("script") || l.contains("bind") || l.contains("on:")
    );

    assert!(has_svelte, "Expected Svelte-specific completions");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_typescript_sveltekit_completion() {
    let mcp_server = FullStackMcpServer::start(6003).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = lsp_client
        .completion("file:///tmp/app/src/lib/api.ts", 5, 0)
        .await
        .expect("TypeScript completion failed");

    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected array");
    assert!(!items.is_empty());

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_python_fastapi_completion() {
    let mcp_server = FullStackMcpServer::start(6004).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = lsp_client
        .completion("file:///tmp/backend/main.py", 10, 0)
        .await
        .expect("Python completion failed");

    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected array");
    assert!(!items.is_empty());

    // Check for FastAPI-specific suggestions
    let labels: Vec<String> = items
        .iter()
        .filter_map(|item| item["label"].as_str().map(String::from))
        .collect();

    let has_fastapi = labels.iter().any(|l|
        l.contains("@app") || l.contains("fastapi") || l.contains("async def")
    );

    assert!(has_fastapi, "Expected FastAPI-specific completions");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_tailwind_css_completion() {
    let mcp_server = FullStackMcpServer::start(6005).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    let response = lsp_client
        .completion("file:///tmp/app/src/app.css", 1, 0)
        .await
        .expect("CSS completion failed");

    assert_eq!(response["jsonrpc"], "2.0");
    let items = response["result"].as_array().expect("Expected array");
    assert!(!items.is_empty());

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_fullstack_multi_file_session() {
    let mcp_server = FullStackMcpServer::start(6006).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Simulate editing multiple files in a full-stack app
    let files = vec![
        ("file:///tmp/app/src/routes/+page.svelte", "Svelte"),
        ("file:///tmp/app/src/lib/stores.ts", "TypeScript"),
        ("file:///tmp/backend/main.py", "Python"),
        ("file:///tmp/app/tailwind.config.js", "JavaScript"),
    ];

    for (uri, lang) in files {
        let response = lsp_client
            .completion(uri, 1, 0)
            .await
            .unwrap_or_else(|e| panic!("{} completion failed: {}", lang, e));

        assert_eq!(response["jsonrpc"], "2.0");
        let items = response["result"].as_array().expect("Expected array");
        assert!(!items.is_empty(), "{} returned no completions", lang);
    }

    // Verify stats
    let stats = mcp_server.get_stats();
    assert!(stats.total_requests >= 4, "Expected at least 4 requests");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_fullstack_concurrent_edits() {
    let mcp_server = FullStackMcpServer::start(6007).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Simulate rapid editing across frontend and backend
    for i in 0..10 {
        let uri = if i % 2 == 0 {
            "file:///tmp/app/src/Component.svelte"
        } else {
            "file:///tmp/backend/api.py"
        };

        let response = lsp_client
            .completion(uri, i, 0)
            .await
            .expect("Concurrent completion failed");

        assert_eq!(response["jsonrpc"], "2.0");
    }

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_fullstack_performance() {
    let mcp_server = FullStackMcpServer::start(6008).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Measure latency across different file types
    let test_files = vec![
        "file:///tmp/app/src/App.svelte",
        "file:///tmp/backend/main.py",
        "file:///tmp/app/src/lib/utils.ts",
    ];

    let mut total_duration = Duration::ZERO;
    let iterations = 15;

    for _ in 0..iterations {
        for uri in &test_files {
            let start = std::time::Instant::now();

            let _ = lsp_client
                .completion(uri, 1, 0)
                .await
                .expect("Completion failed");

            total_duration += start.elapsed();
        }
    }

    let avg_duration = total_duration / (iterations * test_files.len() as u32);

    // Full-stack target: <150ms average
    assert!(
        avg_duration < Duration::from_millis(150),
        "Average latency too high: {:?}",
        avg_duration
    );

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_fullstack_mcp_stats() {
    let mcp_server = FullStackMcpServer::start(6009).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Make requests to different file types
    lsp_client.completion("file:///tmp/app/Component.svelte", 1, 0).await.ok();
    lsp_client.completion("file:///tmp/app/utils.ts", 1, 0).await.ok();
    lsp_client.completion("file:///tmp/backend/main.py", 1, 0).await.ok();
    lsp_client.completion("file:///tmp/app/app.css", 1, 0).await.ok();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let stats = mcp_server.get_stats();

    assert!(stats.svelte_requests > 0, "No Svelte requests tracked");
    assert!(stats.typescript_requests > 0, "No TypeScript requests tracked");
    assert!(stats.python_requests > 0, "No Python requests tracked");
    assert!(stats.total_requests >= 4, "Not all requests tracked");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_fullstack_error_resilience() {
    let mcp_server = FullStackMcpServer::start(6010).await;
    let mut lsp_client = FullStackLspClient::start(&mcp_server.url())
        .await
        .expect("Failed to start LSP");

    lsp_client.initialize().await.expect("Initialize failed");

    // Test invalid URIs (should not crash)
    let invalid_uris = vec![
        "file:///tmp/nonexistent.xyz",
        "file:///tmp/malformed",
        "",
    ];

    for uri in invalid_uris {
        let _ = lsp_client.completion(uri, 1, 0).await;
        // Should not panic or crash
    }

    // Verify server still works after errors
    let response = lsp_client
        .completion("file:///tmp/app/valid.svelte", 1, 0)
        .await
        .expect("Completion after errors failed");

    assert_eq!(response["jsonrpc"], "2.0");

    lsp_client.shutdown().await.expect("Shutdown failed");
    mcp_server.stop().await;
}
