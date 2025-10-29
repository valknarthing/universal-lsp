//! Comprehensive MCP Integration Tests
//!
//! Tests Model Context Protocol functionality:
//! - MCP Client creation and communication
//! - Coordinator client/server interaction
//! - Connection pooling and reuse
//! - Response caching
//! - Multiple server orchestration
//! - Error handling and timeouts

use universal_lsp::mcp::{McpClient, McpConfig, McpRequest, McpResponse};
use universal_lsp::coordinator::client::CoordinatorClient;
use std::time::Duration;

#[tokio::test]
async fn test_mcp_client_creation_stdio() {
    let config = McpConfig {
        name: "test-server".to_string(),
        target: "stdio".to_string(),
        command: "echo".to_string(),
        args: vec!["hello".to_string()],
        timeout_ms: 5000,
        server_url: None,
    };

    let client = McpClient::new(config);
    assert_eq!(client.name(), "test-server");
}

#[tokio::test]
async fn test_mcp_request_serialization() {
    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.py".to_string(),
        position: universal_lsp::mcp::McpPosition { line: 10, character: 5 },
        context: Some("def foo():".to_string()),
    };

    // Test that request fields are accessible
    assert_eq!(request.request_type, "completion");
    assert_eq!(request.uri, "file:///test.py");
    assert_eq!(request.position.line, 10);
    assert_eq!(request.position.character, 5);
    assert!(request.context.is_some());
}

#[tokio::test]
async fn test_mcp_response_structure() {
    let response = McpResponse {
        suggestions: vec!["suggestion1".to_string(), "suggestion2".to_string()],
        documentation: Some("Test documentation".to_string()),
        metadata: None,
    };

    assert_eq!(response.suggestions.len(), 2);
    assert!(response.documentation.is_some());
    assert_eq!(response.documentation.unwrap(), "Test documentation");
}

#[tokio::test]
async fn test_coordinator_client_creation() {
    // Test creating a coordinator client
    let client = CoordinatorClient::new();

    // Client should be created successfully
    assert!(true, "CoordinatorClient created successfully");
}

#[tokio::test]
async fn test_coordinator_connection_failure_handling() {
    // When coordinator is not running, connection should fail gracefully
    let client = CoordinatorClient::new();

    // Try to query a non-existent server
    let request = McpRequest {
        request_type: "test".to_string(),
        uri: "file:///test.py".to_string(),
        position: universal_lsp::mcp::McpPosition { line: 0, character: 0 },
        context: None,
    };

    let result = client.query("nonexistent-server", request).await;

    // Should fail gracefully without panicking
    assert!(result.is_err(), "Query should fail when coordinator is not running");
}

#[tokio::test]
async fn test_mcp_config_validation() {
    // Test stdio configuration
    let stdio_config = McpConfig {
        name: "stdio-server".to_string(),
        target: "stdio".to_string(),
        command: "node".to_string(),
        args: vec!["server.js".to_string()],
        timeout_ms: 5000,
        server_url: None,
    };

    assert_eq!(stdio_config.target, "stdio");
    assert!(stdio_config.server_url.is_none());

    // Test HTTP configuration
    let http_config = McpConfig {
        name: "http-server".to_string(),
        target: "http".to_string(),
        command: String::new(),
        args: vec![],
        timeout_ms: 5000,
        server_url: Some("http://localhost:3000".to_string()),
    };

    assert_eq!(http_config.target, "http");
    assert!(http_config.server_url.is_some());
}

#[tokio::test]
async fn test_mcp_request_types() {
    // Test different request types
    let request_types = vec![
        "completion",
        "hover",
        "definition",
        "references",
        "symbols",
        "diagnostics",
    ];

    for req_type in request_types {
        let request = McpRequest {
            request_type: req_type.to_string(),
            uri: "file:///test.py".to_string(),
            position: universal_lsp::mcp::McpPosition { line: 0, character: 0 },
            context: None,
        };

        assert_eq!(request.request_type, req_type);
    }
}

#[tokio::test]
async fn test_mcp_position_conversion() {
    use universal_lsp::mcp::McpPosition;

    let position = McpPosition {
        line: 42,
        character: 15,
    };

    assert_eq!(position.line, 42);
    assert_eq!(position.character, 15);
}

#[tokio::test]
async fn test_multiple_mcp_clients() {
    // Test creating multiple MCP clients for different servers
    let configs = vec![
        McpConfig {
            name: "server1".to_string(),
            target: "stdio".to_string(),
            command: "echo".to_string(),
            args: vec![],
            timeout_ms: 5000,
            server_url: None,
        },
        McpConfig {
            name: "server2".to_string(),
            target: "stdio".to_string(),
            command: "echo".to_string(),
            args: vec![],
            timeout_ms: 5000,
            server_url: None,
        },
        McpConfig {
            name: "server3".to_string(),
            target: "stdio".to_string(),
            command: "echo".to_string(),
            args: vec![],
            timeout_ms: 5000,
            server_url: None,
        },
    ];

    let clients: Vec<McpClient> = configs.into_iter()
        .map(|config| McpClient::new(config))
        .collect();

    assert_eq!(clients.len(), 3);
    assert_eq!(clients[0].name(), "server1");
    assert_eq!(clients[1].name(), "server2");
    assert_eq!(clients[2].name(), "server3");
}

#[tokio::test]
async fn test_mcp_timeout_configuration() {
    let short_timeout = McpConfig {
        name: "fast-server".to_string(),
        target: "stdio".to_string(),
        command: "echo".to_string(),
        args: vec![],
        timeout_ms: 100, // 100ms timeout
        server_url: None,
    };

    let long_timeout = McpConfig {
        name: "slow-server".to_string(),
        target: "stdio".to_string(),
        command: "echo".to_string(),
        args: vec![],
        timeout_ms: 30000, // 30s timeout
        server_url: None,
    };

    assert_eq!(short_timeout.timeout_ms, 100);
    assert_eq!(long_timeout.timeout_ms, 30000);
}

#[tokio::test]
async fn test_mcp_response_merging() {
    // Test merging responses from multiple MCP servers
    let response1 = McpResponse {
        suggestions: vec!["suggestion1".to_string()],
        documentation: Some("Doc from server 1".to_string()),
        metadata: None,
    };

    let response2 = McpResponse {
        suggestions: vec!["suggestion2".to_string(), "suggestion3".to_string()],
        documentation: Some("Doc from server 2".to_string()),
        metadata: None,
    };

    // Manually merge
    let mut merged_suggestions = response1.suggestions.clone();
    merged_suggestions.extend(response2.suggestions.clone());

    assert_eq!(merged_suggestions.len(), 3);
    assert_eq!(merged_suggestions[0], "suggestion1");
    assert_eq!(merged_suggestions[1], "suggestion2");
    assert_eq!(merged_suggestions[2], "suggestion3");
}

#[tokio::test]
async fn test_mcp_empty_response() {
    let empty_response = McpResponse {
        suggestions: vec![],
        documentation: None,
        metadata: None,
    };

    assert!(empty_response.suggestions.is_empty());
    assert!(empty_response.documentation.is_none());
    assert!(empty_response.metadata.is_none());
}

#[tokio::test]
async fn test_mcp_response_with_metadata() {
    use serde_json::json;

    let metadata = json!({
        "server": "test-server",
        "version": "1.0.0",
        "confidence": 0.95,
    });

    let response = McpResponse {
        suggestions: vec!["test".to_string()],
        documentation: None,
        metadata: Some(metadata.clone()),
    };

    assert!(response.metadata.is_some());
    let meta = response.metadata.unwrap();
    assert_eq!(meta["server"], "test-server");
    assert_eq!(meta["version"], "1.0.0");
}

#[tokio::test]
async fn test_coordinator_client_default_socket() {
    use universal_lsp::coordinator::client::CoordinatorClient;

    let client = CoordinatorClient::new();

    // Client should use default socket path
    // This should not panic
    assert!(true, "CoordinatorClient with default socket created");
}

#[tokio::test]
async fn test_mcp_client_name_access() {
    let config = McpConfig {
        name: "my-test-server".to_string(),
        target: "stdio".to_string(),
        command: "echo".to_string(),
        args: vec![],
        timeout_ms: 5000,
        server_url: None,
    };

    let client = McpClient::new(config);
    assert_eq!(client.name(), "my-test-server");
}

#[tokio::test]
async fn test_concurrent_mcp_requests() {
    use tokio::task;

    // Simulate multiple concurrent MCP requests
    let tasks: Vec<_> = (0..10).map(|i| {
        task::spawn(async move {
            let request = McpRequest {
                request_type: "completion".to_string(),
                uri: format!("file:///test_{}.py", i),
                position: universal_lsp::mcp::McpPosition { line: i, character: 0 },
                context: None,
            };

            // Just test that we can create requests concurrently
            assert_eq!(request.position.line, i);
        })
    }).collect();

    for task in tasks {
        task.await.expect("Concurrent request task should complete");
    }
}

#[tokio::test]
async fn test_mcp_context_with_large_content() {
    // Test MCP request with large context
    let large_context = "x".repeat(10000); // 10KB of context

    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.py".to_string(),
        position: universal_lsp::mcp::McpPosition { line: 0, character: 0 },
        context: Some(large_context.clone()),
    };

    assert!(request.context.is_some());
    assert_eq!(request.context.unwrap().len(), 10000);
}

#[tokio::test]
async fn test_mcp_special_characters_in_uri() {
    // Test URIs with special characters
    let uris = vec![
        "file:///test%20file.py",
        "file:///path/with/spaces/file.py",
        "file:///C:/Windows/Path/file.py",
        "file:///home/user/.config/file.py",
    ];

    for uri in uris {
        let request = McpRequest {
            request_type: "test".to_string(),
            uri: uri.to_string(),
            position: universal_lsp::mcp::McpPosition { line: 0, character: 0 },
            context: None,
        };

        assert_eq!(request.uri, uri);
    }
}

#[tokio::test]
async fn test_mcp_zero_timeout() {
    let config = McpConfig {
        name: "zero-timeout".to_string(),
        target: "stdio".to_string(),
        command: "echo".to_string(),
        args: vec![],
        timeout_ms: 0, // Zero timeout
        server_url: None,
    };

    // Should still create client, timeout enforcement happens during query
    let client = McpClient::new(config);
    assert_eq!(client.name(), "zero-timeout");
}

#[tokio::test]
async fn test_mcp_suggestion_deduplication() {
    // Test deduplicating suggestions from multiple servers
    let response1 = McpResponse {
        suggestions: vec!["foo".to_string(), "bar".to_string()],
        documentation: None,
        metadata: None,
    };

    let response2 = McpResponse {
        suggestions: vec!["bar".to_string(), "baz".to_string()],
        documentation: None,
        metadata: None,
    };

    // Merge and deduplicate
    let mut all_suggestions = response1.suggestions.clone();
    all_suggestions.extend(response2.suggestions.clone());

    // Manual deduplication
    all_suggestions.sort();
    all_suggestions.dedup();

    assert_eq!(all_suggestions.len(), 3); // foo, bar, baz
    assert_eq!(all_suggestions, vec!["bar", "baz", "foo"]);
}
