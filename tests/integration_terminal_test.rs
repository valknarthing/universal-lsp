//! Integration tests for Terminal/CLI + Universal LSP + Claude
//!
//! This test suite mocks:
//! - Claude MCP Server (HTTP)
//! - Universal LSP Server (stdio/CLI)
//! - Direct stdin/stdout communication
//! - curl/HTTP testing

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

/// Mock MCP Server for CLI testing
struct CliMcpServer {
    port: u16,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    request_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl CliMcpServer {
    async fn start(port: u16) -> Self {
        use warp::Filter;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU64, Ordering};

        let request_count = Arc::new(AtomicU64::new(0));
        let request_count_clone = Arc::clone(&request_count);

        let health_route = warp::path("health")
            .map(|| warp::reply::json(&json!({
                "status": "healthy",
                "uptime": 12345,
                "requests_served": 0
            })));

        let mcp_route = warp::post()
            .and(warp::path::end())
            .and(warp::body::json())
            .map(move |body: Value| {
                request_count_clone.fetch_add(1, Ordering::SeqCst);

                let request_type = body["request_type"].as_str().unwrap_or("unknown");
                let uri = body["uri"].as_str().unwrap_or("");
                let context = body["context"].as_str().unwrap_or("");

                let suggestions = match request_type {
                    "completion" => vec![
                        "// CLI-generated completion",
                        "fn main() {",
                        "async fn process() {",
                        "impl Default for",
                        "use std::collections::"
                    ],
                    "hover" => vec![],
                    _ => vec![]
                };

                let documentation = if request_type == "hover" {
                    Some(format!("Documentation for symbol at {}", uri))
                } else {
                    None
                };

                warp::reply::json(&json!({
                    "suggestions": suggestions,
                    "documentation": documentation,
                    "confidence": 0.89,
                    "metadata": {
                        "source": "cli-mcp-server",
                        "cached": false,
                        "latency_ms": 42
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
            request_count,
        }
    }

    fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    fn get_request_count(&self) -> u64 {
        self.request_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    async fn stop(mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

#[tokio::test]
async fn test_terminal_mcp_health_check() {
    let mcp_server = CliMcpServer::start(5001).await;

    // Test with curl
    let output = Command::new("curl")
        .args(&[
            "-s",
            &format!("{}/health", mcp_server.url())
        ])
        .output()
        .await
        .expect("curl failed");

    assert!(output.status.success());

    let response: Value = serde_json::from_slice(&output.stdout)
        .expect("Invalid JSON response");

    assert_eq!(response["status"], "healthy");
    assert!(response["uptime"].is_number());

    mcp_server.stop().await;
}

#[tokio::test]
async fn test_terminal_mcp_completion_request() {
    let mcp_server = CliMcpServer::start(5002).await;

    // Test direct MCP request with curl
    let request_json = json!({
        "request_type": "completion",
        "uri": "file:///tmp/test.rs",
        "position": {"line": 10, "character": 5},
        "context": "fn main() {"
    }).to_string();

    let output = Command::new("curl")
        .args(&[
            "-s",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", &request_json,
            &mcp_server.url()
        ])
        .output()
        .await
        .expect("curl failed");

    assert!(output.status.success());

    let response: Value = serde_json::from_slice(&output.stdout)
        .expect("Invalid JSON response");

    assert!(response["suggestions"].is_array());
    let suggestions = response["suggestions"].as_array().unwrap();
    assert!(!suggestions.is_empty());

    assert_eq!(mcp_server.get_request_count(), 1);

    mcp_server.stop().await;
}

#[tokio::test]
async fn test_terminal_lsp_stdio_protocol() {
    let mcp_server = CliMcpServer::start(5003).await;

    // Start LSP server as subprocess
    let mut child = Command::new("target/release/universal-lsp")
        .args(&[
            "--log-level=info",
            &format!("--mcp-pre={}", mcp_server.url()),
            "--mcp-timeout=5000",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start LSP server");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": "file:///tmp/terminal-test",
            "capabilities": {}
        }
    });

    let message = serde_json::to_string(&init_request).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", message.len());

    stdin.write_all(header.as_bytes()).await.unwrap();
    stdin.write_all(message.as_bytes()).await.unwrap();
    stdin.flush().await.unwrap();

    // Read response
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).await.unwrap();

        if line == "\r\n" || line == "\n" {
            break;
        }

        if line.starts_with("Content-Length:") {
            let len_str = line.trim_start_matches("Content-Length:").trim();
            content_length = len_str.parse().unwrap();
        }
    }

    let mut body = vec![0u8; content_length];
    tokio::io::AsyncReadExt::read_exact(&mut stdout, &mut body).await.unwrap();

    let response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["capabilities"].is_object());

    // Cleanup
    child.kill().await.ok();
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_terminal_batch_completions() {
    let mcp_server = CliMcpServer::start(5004).await;

    // Simulate batch processing from command line
    let files = vec![
        ("test.rs", 5),
        ("main.rs", 10),
        ("lib.rs", 15),
    ];

    for (filename, line) in files {
        let request_json = json!({
            "request_type": "completion",
            "uri": format!("file:///tmp/{}", filename),
            "position": {"line": line, "character": 0},
            "context": ""
        }).to_string();

        let output = Command::new("curl")
            .args(&[
                "-s",
                "-X", "POST",
                "-H", "Content-Type: application/json",
                "-d", &request_json,
                &mcp_server.url()
            ])
            .output()
            .await
            .expect("curl failed");

        assert!(output.status.success());

        let response: Value = serde_json::from_slice(&output.stdout)
            .expect("Invalid JSON response");

        assert!(response["suggestions"].is_array());
    }

    // Verify all requests were processed
    assert_eq!(mcp_server.get_request_count(), 3);

    mcp_server.stop().await;
}

#[tokio::test]
async fn test_terminal_performance_measurement() {
    let mcp_server = CliMcpServer::start(5005).await;

    let request_json = json!({
        "request_type": "completion",
        "uri": "file:///tmp/bench.rs",
        "position": {"line": 1, "character": 0},
        "context": ""
    }).to_string();

    // Measure 10 requests
    let mut total_duration = Duration::ZERO;

    for _ in 0..10 {
        let start = std::time::Instant::now();

        let output = Command::new("curl")
            .args(&[
                "-s",
                "-X", "POST",
                "-H", "Content-Type: application/json",
                "-d", &request_json,
                &mcp_server.url()
            ])
            .output()
            .await
            .expect("curl failed");

        total_duration += start.elapsed();
        assert!(output.status.success());
    }

    let avg_duration = total_duration / 10;

    // Should be under 200ms average for CLI usage
    assert!(
        avg_duration < Duration::from_millis(200),
        "Average latency too high: {:?}",
        avg_duration
    );

    mcp_server.stop().await;
}

#[tokio::test]
async fn test_terminal_error_handling() {
    // Start LSP without MCP server (should handle gracefully)
    let mut child = Command::new("target/release/universal-lsp")
        .args(&[
            "--log-level=info",
            "--mcp-pre=http://localhost:9999", // Non-existent server
            "--mcp-timeout=1000",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start LSP server");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Initialize should still work
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": "file:///tmp",
            "capabilities": {}
        }
    });

    let message = serde_json::to_string(&init_request).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", message.len());

    stdin.write_all(header.as_bytes()).await.unwrap();
    stdin.write_all(message.as_bytes()).await.unwrap();
    stdin.flush().await.unwrap();

    // Should still get response despite MCP failure
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        let result = timeout(Duration::from_secs(3), stdout.read_line(&mut line)).await;

        if result.is_err() {
            panic!("Timeout reading response");
        }

        if line == "\r\n" || line == "\n" {
            break;
        }

        if line.starts_with("Content-Length:") {
            let len_str = line.trim_start_matches("Content-Length:").trim();
            content_length = len_str.parse().unwrap();
        }
    }

    assert!(content_length > 0);

    child.kill().await.ok();
}

#[tokio::test]
async fn test_terminal_json_rpc_protocol() {
    let mcp_server = CliMcpServer::start(5006).await;

    let mut child = Command::new("target/release/universal-lsp")
        .args(&[
            "--log-level=debug",
            &format!("--mcp-pre={}", mcp_server.url()),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start LSP");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Test multiple requests in sequence
    let requests = vec![
        ("initialize", json!({"processId": null, "rootUri": "file:///tmp", "capabilities": {}})),
        ("textDocument/completion", json!({
            "textDocument": {"uri": "file:///tmp/test.rs"},
            "position": {"line": 1, "character": 0}
        })),
    ];

    for (i, (method, params)) in requests.iter().enumerate() {
        let request = json!({
            "jsonrpc": "2.0",
            "id": i + 1,
            "method": method,
            "params": params
        });

        let message = serde_json::to_string(&request).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", message.len());

        stdin.write_all(header.as_bytes()).await.unwrap();
        stdin.write_all(message.as_bytes()).await.unwrap();
        stdin.flush().await.unwrap();

        // Read response
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            stdout.read_line(&mut line).await.unwrap();

            if line == "\r\n" || line == "\n" {
                break;
            }

            if line.starts_with("Content-Length:") {
                content_length = line.trim_start_matches("Content-Length:").trim().parse().unwrap();
            }
        }

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut stdout, &mut body).await.unwrap();

        let response: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], i + 1);
    }

    child.kill().await.ok();
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_terminal_systemd_like_daemon() {
    let mcp_server = CliMcpServer::start(5007).await;

    // Start LSP as long-running daemon
    let mut child = Command::new("target/release/universal-lsp")
        .args(&[
            "--log-level=info",
            &format!("--mcp-pre={}", mcp_server.url()),
            "--mcp-cache",
            "--max-concurrent=50",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start LSP daemon");

    // Verify it stays alive
    tokio::time::sleep(Duration::from_secs(2)).await;

    let status = child.try_wait().expect("Failed to check status");
    assert!(status.is_none(), "Daemon exited unexpectedly");

    // Cleanup
    child.kill().await.ok();
    mcp_server.stop().await;
}

#[tokio::test]
async fn test_terminal_streaming_requests() {
    let mcp_server = CliMcpServer::start(5008).await;

    let mut child = Command::new("target/release/universal-lsp")
        .args(&[
            "--log-level=info",
            &format!("--mcp-pre={}", mcp_server.url()),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start LSP");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Initialize first
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"processId": null, "rootUri": "file:///tmp", "capabilities": {}}
    });

    let msg = serde_json::to_string(&init).unwrap();
    let hdr = format!("Content-Length: {}\r\n\r\n", msg.len());
    stdin.write_all(hdr.as_bytes()).await.unwrap();
    stdin.write_all(msg.as_bytes()).await.unwrap();
    stdin.flush().await.unwrap();

    // Read init response (and ignore)
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).await.unwrap();
        if line == "\r\n" || line == "\n" { break; }
        if line.starts_with("Content-Length:") {
            content_length = line.trim_start_matches("Content-Length:").trim().parse().unwrap();
        }
    }
    let mut body = vec![0u8; content_length];
    tokio::io::AsyncReadExt::read_exact(&mut stdout, &mut body).await.unwrap();

    // Stream multiple completion requests rapidly
    for i in 0..5 {
        let request = json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": "file:///tmp/stream.rs"},
                "position": {"line": i, "character": 0}
            }
        });

        let msg = serde_json::to_string(&request).unwrap();
        let hdr = format!("Content-Length: {}\r\n\r\n", msg.len());
        stdin.write_all(hdr.as_bytes()).await.unwrap();
        stdin.write_all(msg.as_bytes()).await.unwrap();
        stdin.flush().await.unwrap();
    }

    // Read all responses
    for _ in 0..5 {
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            stdout.read_line(&mut line).await.unwrap();
            if line == "\r\n" || line == "\n" { break; }
            if line.starts_with("Content-Length:") {
                content_length = line.trim_start_matches("Content-Length:").trim().parse().unwrap();
            }
        }

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut stdout, &mut body).await.unwrap();

        let response: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
    }

    child.kill().await.ok();
    mcp_server.stop().await;
}
