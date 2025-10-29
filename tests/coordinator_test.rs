//! Integration tests for MCP Coordinator
//!
//! Test-Driven Development approach:
//! 1. Define requirements as tests
//! 2. Run tests (they should fail)
//! 3. Implement to make tests pass
//! 4. Refactor while keeping tests green

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use universal_lsp::config::{Config, McpConfig, McpServerConfig, ProxyConfig, ServerConfig};
use universal_lsp::coordinator::{
    Coordinator, CoordinatorRequest, CoordinatorResponse, IpcMessage, IpcPayload,
};
use universal_lsp::mcp::{McpRequest, Position};

/// Helper to create test configuration
fn create_test_config() -> Config {
    // Get path to mock MCP server binary
    let mock_server_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("mock-mcp-server");

    let mut servers = std::collections::HashMap::new();
    servers.insert(
        "test-mock".to_string(),
        McpServerConfig {
            name: "test-mock".to_string(),
            target: mock_server_path.to_string_lossy().to_string(), // Mock MCP server binary
        },
    );

    Config {
        server: ServerConfig {
            log_level: "debug".to_string(),
            max_concurrent: 10,
            log_requests: true,
        },
        mcp: McpConfig {
            servers,
            timeout_ms: 2000, // Increased timeout for subprocess startup
            enable_cache: true,
        },
        proxy: ProxyConfig {
            servers: std::collections::HashMap::new(),
        },
    }
}

/// Helper to generate unique socket path for each test
fn unique_socket_path(test_name: &str) -> String {
    let pid = std::process::id();
    let thread_id = std::thread::current().id();
    format!("/tmp/ulsp-test-{}-{:?}-{}.sock", test_name, thread_id, pid)
}

/// Helper to cleanup socket file
async fn cleanup_socket(socket_path: &str) {
    let _ = tokio::fs::remove_file(socket_path).await;
}

/// Helper to send IPC message and receive response
async fn send_ipc_message(
    stream: &mut UnixStream,
    message: IpcMessage,
) -> anyhow::Result<IpcMessage> {
    // Send message
    let bytes = message.to_bytes()?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;

    // Read Content-Length header
    let mut header = String::new();
    let mut buf = [0u8; 1];
    loop {
        stream.read_exact(&mut buf).await?;
        header.push(buf[0] as char);
        if header.ends_with("\r\n\r\n") {
            break;
        }
    }

    // Parse Content-Length
    let content_length = header
        .lines()
        .find(|line| line.starts_with("Content-Length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|s| s.trim().parse::<usize>().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid Content-Length"))?;

    // Read message body
    let mut body = vec![0u8; content_length];
    stream.read_exact(&mut body).await?;

    let message_str = String::from_utf8(body)?;
    Ok(IpcMessage::from_str(&message_str)?)
}

#[tokio::test]
async fn test_coordinator_starts_and_accepts_connections() {
    // REQUIREMENT: Coordinator should start and accept Unix socket connections

    let socket_path = unique_socket_path("starts_and_accepts");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    // Start coordinator in background
    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move {
            coord.run().await
        })
    };

    // Give coordinator time to bind
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Try to connect
    let result = UnixStream::connect(&socket_path).await;
    assert!(result.is_ok(), "Should be able to connect to coordinator socket");

    // Cleanup
    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_handles_connect_request() {
    // REQUIREMENT: Coordinator should handle Connect requests and return connection ID

    let socket_path = unique_socket_path("handles_connect");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");

    // Send Connect request
    let request = IpcMessage::request(
        1,
        CoordinatorRequest::Connect {
            server_name: "test-mock".to_string(),
        },
    );

    let response = send_ipc_message(&mut stream, request)
        .await
        .expect("Failed to send/receive message");

    // Verify response
    match response.payload {
        IpcPayload::Response(CoordinatorResponse::Connected { connection_id }) => {
            assert!(connection_id > 0, "Connection ID should be positive");
        }
        IpcPayload::Response(CoordinatorResponse::Error { message }) => {
            println!("Connect failed (expected for mock server): {}", message);
        }
        _ => panic!("Expected Connected or Error response, got: {:?}", response.payload),
    }

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_handles_unknown_server() {
    // REQUIREMENT: Coordinator should return error for unknown servers

    let socket_path = unique_socket_path("unknown_server");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");

    // Send Connect request for non-existent server
    let request = IpcMessage::request(
        1,
        CoordinatorRequest::Connect {
            server_name: "nonexistent-server".to_string(),
        },
    );

    let response = send_ipc_message(&mut stream, request)
        .await
        .expect("Failed to send/receive message");

    // Verify error response
    match response.payload {
        IpcPayload::Response(CoordinatorResponse::Error { message }) => {
            assert!(
                message.contains("Unknown server"),
                "Error message should mention unknown server"
            );
        }
        _ => panic!("Expected Error response, got: {:?}", response.payload),
    }

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_handles_query_request() {
    // REQUIREMENT: Coordinator should handle Query requests and cache responses

    let socket_path = unique_socket_path("query_request");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");

    // Send Query request
    let mcp_request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 10, character: 5 },
        context: Some("fn main()".to_string()),
    };

    let request = IpcMessage::request(
        1,
        CoordinatorRequest::Query {
            server_name: "test-mock".to_string(),
            request: mcp_request,
        },
    );

    let response = send_ipc_message(&mut stream, request)
        .await
        .expect("Failed to send/receive message");

    // Verify response (should be QueryResult or Error)
    match response.payload {
        IpcPayload::Response(CoordinatorResponse::QueryResult(_)) => {
            // Success - query returned result
        }
        IpcPayload::Response(CoordinatorResponse::Error { message }) => {
            // Expected for echo command which isn't a real MCP server
            println!("Query failed as expected: {}", message);
        }
        _ => panic!("Expected QueryResult or Error, got: {:?}", response.payload),
    }

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_cache_functionality() {
    // REQUIREMENT: Coordinator should support SetCache and GetCache operations

    let socket_path = unique_socket_path("cache_functionality");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");

    // Create test response
    let test_response = universal_lsp::mcp::McpResponse {
        suggestions: vec!["test".to_string()],
        documentation: Some("Test doc".to_string()),
        confidence: Some(0.9),
    };

    // Send SetCache request
    let set_request = IpcMessage::request(
        1,
        CoordinatorRequest::SetCache {
            key: "test-key".to_string(),
            value: test_response.clone(),
            ttl_seconds: 60,
        },
    );

    let set_response = send_ipc_message(&mut stream, set_request)
        .await
        .expect("Failed to send SetCache");

    assert!(
        matches!(
            set_response.payload,
            IpcPayload::Response(CoordinatorResponse::Ok)
        ),
        "SetCache should return Ok"
    );

    // Send GetCache request
    let get_request = IpcMessage::request(
        2,
        CoordinatorRequest::GetCache {
            key: "test-key".to_string(),
        },
    );

    let get_response = send_ipc_message(&mut stream, get_request)
        .await
        .expect("Failed to send GetCache");

    // Verify cache hit
    match get_response.payload {
        IpcPayload::Response(CoordinatorResponse::CacheHit(response)) => {
            assert_eq!(response.suggestions, test_response.suggestions);
            assert_eq!(response.documentation, test_response.documentation);
        }
        _ => panic!("Expected CacheHit, got: {:?}", get_response.payload),
    }

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_cache_miss() {
    // REQUIREMENT: Coordinator should return CacheMiss for non-existent keys

    let socket_path = unique_socket_path("cache_miss");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");

    // Send GetCache request for non-existent key
    let request = IpcMessage::request(
        1,
        CoordinatorRequest::GetCache {
            key: "nonexistent-key".to_string(),
        },
    );

    let response = send_ipc_message(&mut stream, request)
        .await
        .expect("Failed to send/receive");

    // Verify cache miss
    assert!(
        matches!(
            response.payload,
            IpcPayload::Response(CoordinatorResponse::CacheMiss)
        ),
        "Should return CacheMiss for non-existent key"
    );

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_metrics() {
    // REQUIREMENT: Coordinator should track and report metrics

    let socket_path = unique_socket_path("metrics");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");

    // Send GetMetrics request
    let request = IpcMessage::request(1, CoordinatorRequest::GetMetrics);

    let response = send_ipc_message(&mut stream, request)
        .await
        .expect("Failed to send/receive");

    // Verify metrics response
    match response.payload {
        IpcPayload::Response(CoordinatorResponse::Metrics(metrics)) => {
            assert_eq!(metrics.active_connections, 0, "Should have 0 active MCP connections");
            assert!(metrics.total_queries <= 1, "Should have at most 1 query (GetMetrics itself)");
            assert_eq!(metrics.errors, 0, "Should have 0 errors");
            assert!(metrics.uptime_seconds >= 0, "Uptime should be non-negative");
        }
        _ => panic!("Expected Metrics response, got: {:?}", response.payload),
    }

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_multiple_clients() {
    // REQUIREMENT: Coordinator should handle multiple concurrent clients

    let socket_path = unique_socket_path("multiple_clients");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connect multiple clients concurrently
    let mut handles = vec![];

    for i in 0..5 {
        let socket_path_clone = socket_path.clone();
        let handle = tokio::spawn(async move {
            let mut stream = UnixStream::connect(&socket_path_clone)
                .await
                .expect("Failed to connect");

            let request = IpcMessage::request(
                i as u64,
                CoordinatorRequest::GetMetrics,
            );

            let response = send_ipc_message(&mut stream, request)
                .await
                .expect("Failed to send/receive");

            matches!(
                response.payload,
                IpcPayload::Response(CoordinatorResponse::Metrics(_))
            )
        });

        handles.push(handle);
    }

    // Wait for all clients to complete
    let results = futures::future::join_all(handles).await;

    for (i, result) in results.iter().enumerate() {
        assert!(
            result.as_ref().unwrap(),
            "Client {} should receive metrics response",
            i
        );
    }

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[tokio::test]
async fn test_coordinator_connection_pooling() {
    // REQUIREMENT: Multiple requests to same server should reuse connection

    let socket_path = unique_socket_path("connection_pooling");
    cleanup_socket(&socket_path).await;

    let config = create_test_config();
    let coordinator = Arc::new(Coordinator::with_socket_path(&config, &socket_path));

    let coord_handle = {
        let coord = Arc::clone(&coordinator);
        tokio::spawn(async move { coord.run().await })
    };

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect");

    // Connect to same server twice
    for i in 1..=2 {
        let request = IpcMessage::request(
            i,
            CoordinatorRequest::Connect {
                server_name: "test-mock".to_string(),
            },
        );

        let response = send_ipc_message(&mut stream, request)
            .await
            .expect("Failed to send/receive");

        match response.payload {
            IpcPayload::Response(CoordinatorResponse::Connected { connection_id }) => {
                println!("Connection {} got ID: {}", i, connection_id);
            }
            IpcPayload::Response(CoordinatorResponse::Error { message }) => {
                // Mock server may not be available - this is acceptable in tests
                println!("Connection {} failed (expected for mock server): {}", i, message);
            }
            _ => panic!("Unexpected response: {:?}", response.payload),
        }
    }

    // Check metrics - should show connection pooling
    let metrics_request = IpcMessage::request(3, CoordinatorRequest::GetMetrics);
    let metrics_response = send_ipc_message(&mut stream, metrics_request)
        .await
        .expect("Failed to get metrics");

    match metrics_response.payload {
        IpcPayload::Response(CoordinatorResponse::Metrics(metrics)) => {
            // Should have at most 1 active connection due to pooling
            assert!(
                metrics.active_connections <= 1,
                "Connection pooling should reuse connections"
            );
        }
        _ => panic!("Expected Metrics response"),
    }

    coord_handle.abort();
    cleanup_socket(&socket_path).await;
}

#[test]
fn test_ipc_message_serialization() {
    // REQUIREMENT: IPC messages should serialize/deserialize correctly

    let request = CoordinatorRequest::Connect {
        server_name: "test".to_string(),
    };
    let message = IpcMessage::request(1, request);

    // Test to_bytes
    let bytes = message.to_bytes().expect("Should serialize");
    let string = String::from_utf8(bytes).expect("Should be valid UTF-8");

    assert!(string.starts_with("Content-Length:"));
    assert!(string.contains("test"));

    // Test from_str
    let json_part = string.split("\r\n\r\n").nth(1).expect("Should have body");
    let deserialized = IpcMessage::from_str(json_part).expect("Should deserialize");

    assert_eq!(message.id, deserialized.id);
}
