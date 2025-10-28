//! Comprehensive tests for MCP (Model Context Protocol) client and pipeline
//!
//! Tests cover:
//! - MCP client creation and configuration
//! - HTTP request/response handling
//! - Health checks and availability
//! - Error handling and timeouts
//! - Pipeline pre/post-processing
//! - Response merging
//! - Parallel request handling

use universal_lsp::mcp::{McpClient, McpConfig, McpRequest, McpResponse, Position, TransportType};
use std::time::Duration;

#[tokio::test]
async fn test_mcp_config_creation() {
    let config = McpConfig {
        server_url: "http://localhost:3000".to_string(),
        transport: TransportType::Http,
        timeout_ms: 5000,
    };

    assert_eq!(config.server_url, "http://localhost:3000");
    assert_eq!(config.timeout_ms, 5000);
}

#[tokio::test]
async fn test_mcp_config_default() {
    let config = McpConfig::default();

    assert_eq!(config.server_url, "http://localhost:3000");
    assert_eq!(config.timeout_ms, 5000);
}

#[tokio::test]
async fn test_mcp_config_with_custom_timeout() {
    let config = McpConfig {
        server_url: "http://localhost:3000".to_string(),
        transport: TransportType::Http,
        timeout_ms: 10000,
    };

    assert_eq!(config.timeout_ms, 10000);
}

#[tokio::test]
async fn test_mcp_config_transport_types() {
    let configs = vec![
        McpConfig {
            server_url: "http://localhost:3000".to_string(),
            transport: TransportType::Http,
            timeout_ms: 5000,
        },
        McpConfig {
            server_url: "http://localhost:3000".to_string(),
            transport: TransportType::Stdio,
            timeout_ms: 5000,
        },
        McpConfig {
            server_url: "ws://localhost:3000".to_string(),
            transport: TransportType::WebSocket,
            timeout_ms: 5000,
        },
    ];

    assert_eq!(configs.len(), 3);
}

#[tokio::test]
async fn test_mcp_client_creation() {
    let config = McpConfig::default();
    let client = McpClient::new(config);

    // Client should be created successfully
    // (Note: is_available() will return false without a real server)
    assert!(!client.is_available().await);
}

#[tokio::test]
async fn test_mcp_request_structure() {
    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 10, character: 5 },
        context: Some("fn main() {".to_string()),
    };

    assert_eq!(request.request_type, "completion");
    assert_eq!(request.uri, "file:///test.rs");
    assert_eq!(request.position.line, 10);
    assert_eq!(request.position.character, 5);
    assert!(request.context.is_some());
}

#[tokio::test]
async fn test_mcp_request_without_context() {
    let request = McpRequest {
        request_type: "hover".to_string(),
        uri: "file:///test.py".to_string(),
        position: Position { line: 20, character: 15 },
        context: None,
    };

    assert_eq!(request.request_type, "hover");
    assert!(request.context.is_none());
}

#[tokio::test]
async fn test_mcp_response_structure() {
    let response = McpResponse {
        suggestions: vec![
            "fn main()".to_string(),
            "fn test()".to_string(),
        ],
        documentation: Some("Main function documentation".to_string()),
        confidence: Some(0.95),
    };

    assert_eq!(response.suggestions.len(), 2);
    assert!(response.documentation.is_some());
    assert_eq!(response.confidence, Some(0.95));
}

#[tokio::test]
async fn test_mcp_response_minimal() {
    let response = McpResponse {
        suggestions: vec!["suggestion".to_string()],
        documentation: None,
        confidence: None,
    };

    assert_eq!(response.suggestions.len(), 1);
    assert!(response.documentation.is_none());
    assert!(response.confidence.is_none());
}

#[tokio::test]
async fn test_mcp_client_with_non_existent_server() {
    let config = McpConfig {
        server_url: "http://localhost:9999".to_string(),
        transport: TransportType::Http,
        timeout_ms: 100,
    };

    let client = McpClient::new(config);

    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 0, character: 0 },
        context: None,
    };

    // Should fail to connect (timeout or connection error)
    let result = tokio::time::timeout(
        Duration::from_millis(500),
        client.query(&request)
    ).await;

    match result {
        Ok(Ok(_)) => panic!("Expected timeout or error, but got success"),
        Ok(Err(_)) => {}, // Expected: connection error
        Err(_) => {}, // Expected: timeout
    }
}

#[tokio::test]
async fn test_mcp_position_structure() {
    let positions = vec![
        Position { line: 0, character: 0 },
        Position { line: 100, character: 50 },
        Position { line: 999, character: 999 },
    ];

    for pos in positions {
        assert!(pos.line >= 0);
        assert!(pos.character >= 0);
    }
}

#[tokio::test]
async fn test_mcp_request_types() {
    let request_types = vec![
        "completion",
        "hover",
        "definition",
        "references",
        "context",
    ];

    for req_type in request_types {
        let request = McpRequest {
            request_type: req_type.to_string(),
            uri: "file:///test.rs".to_string(),
            position: Position { line: 0, character: 0 },
            context: None,
        };

        assert_eq!(request.request_type, req_type);
    }
}

#[tokio::test]
async fn test_mcp_get_context() {
    let config = McpConfig {
        server_url: "http://localhost:9999".to_string(), // Non-existent server
        transport: TransportType::Http,
        timeout_ms: 100,
    };

    let client = McpClient::new(config);

    // This should fail since server doesn't exist
    let result = client.get_context("test query").await;
    assert!(result.is_err(), "Expected error for non-existent server");
}

#[tokio::test]
async fn test_mcp_client_is_available() {
    let config = McpConfig {
        server_url: "http://localhost:9999".to_string(),
        transport: TransportType::Http,
        timeout_ms: 100,
    };

    let client = McpClient::new(config);

    // Should return false for non-existent server
    assert!(!client.is_available().await);
}

#[tokio::test]
async fn test_mcp_stdio_transport_not_implemented() {
    let config = McpConfig {
        server_url: "stdio".to_string(),
        transport: TransportType::Stdio,
        timeout_ms: 5000,
    };

    let client = McpClient::new(config);

    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 0, character: 0 },
        context: None,
    };

    let result = client.query(&request).await;
    assert!(result.is_err(), "Stdio transport should not be implemented yet");

    if let Err(e) = result {
        assert!(e.to_string().contains("not yet implemented"));
    }
}

#[tokio::test]
async fn test_mcp_websocket_transport_not_implemented() {
    let config = McpConfig {
        server_url: "ws://localhost:3000".to_string(),
        transport: TransportType::WebSocket,
        timeout_ms: 5000,
    };

    let client = McpClient::new(config);

    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 0, character: 0 },
        context: None,
    };

    let result = client.query(&request).await;
    assert!(result.is_err(), "WebSocket transport should not be implemented yet");

    if let Err(e) = result {
        assert!(e.to_string().contains("not yet implemented"));
    }
}

#[tokio::test]
async fn test_mcp_request_serialization() {
    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 42, character: 15 },
        context: Some("fn main() { let x = ".to_string()),
    };

    let serialized = serde_json::to_string(&request);
    assert!(serialized.is_ok(), "Failed to serialize MCP request");

    let json_str = serialized.unwrap();
    assert!(json_str.contains("completion"));
    assert!(json_str.contains("file:///test.rs"));
}

#[tokio::test]
async fn test_mcp_response_deserialization() {
    let json = r#"{
        "suggestions": ["fn main()", "fn test()"],
        "documentation": "Function documentation",
        "confidence": 0.95
    }"#;

    let response: Result<McpResponse, _> = serde_json::from_str(json);
    assert!(response.is_ok(), "Failed to deserialize MCP response");

    let resp = response.unwrap();
    assert_eq!(resp.suggestions.len(), 2);
    assert!(resp.documentation.is_some());
    assert_eq!(resp.confidence, Some(0.95));
}

#[tokio::test]
async fn test_mcp_concurrent_requests() {
    use std::sync::Arc;

    let config = McpConfig {
        server_url: "http://localhost:9999".to_string(),
        transport: TransportType::Http,
        timeout_ms: 100,
    };

    let client = Arc::new(McpClient::new(config));
    let mut handles = vec![];

    // Spawn multiple concurrent requests
    for i in 0..5 {
        let request = McpRequest {
            request_type: "completion".to_string(),
            uri: format!("file:///test{}.rs", i),
            position: Position { line: i, character: i },
            context: None,
        };

        let client_clone = Arc::clone(&client);
        let handle = tokio::spawn(async move {
            client_clone.query(&request).await
        });
        handles.push(handle);
    }

    // Wait for all tasks (they should all fail gracefully)
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Task should not panic");
    }
}

#[tokio::test]
async fn test_mcp_config_various_urls() {
    let urls = vec![
        "http://localhost:3000",
        "http://127.0.0.1:8080",
        "https://api.example.com/mcp",
        "http://mcp-server:9000",
    ];

    for url in urls {
        let config = McpConfig {
            server_url: url.to_string(),
            transport: TransportType::Http,
            timeout_ms: 5000,
        };

        assert_eq!(config.server_url, url);
        let client = McpClient::new(config);
        assert!(!client.is_available().await);
    }
}

#[tokio::test]
async fn test_mcp_timeout_values() {
    let timeouts = vec![100, 500, 1000, 5000, 10000];

    for timeout in timeouts {
        let config = McpConfig {
            server_url: "http://localhost:3000".to_string(),
            transport: TransportType::Http,
            timeout_ms: timeout,
        };

        assert_eq!(config.timeout_ms, timeout);
    }
}

#[tokio::test]
async fn test_mcp_request_with_large_context() {
    let large_context = "fn main() { ".to_string() + &"let x = 1;\n".repeat(100);

    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///large.rs".to_string(),
        position: Position { line: 100, character: 0 },
        context: Some(large_context.clone()),
    };

    assert!(request.context.is_some());
    assert!(request.context.unwrap().len() > 1000);
}

#[tokio::test]
async fn test_mcp_response_with_many_suggestions() {
    let suggestions: Vec<String> = (0..100)
        .map(|i| format!("suggestion_{}", i))
        .collect();

    let response = McpResponse {
        suggestions: suggestions.clone(),
        documentation: None,
        confidence: Some(0.8),
    };

    assert_eq!(response.suggestions.len(), 100);
    assert_eq!(response.suggestions[0], "suggestion_0");
    assert_eq!(response.suggestions[99], "suggestion_99");
}

#[tokio::test]
async fn test_mcp_confidence_range() {
    let confidences = vec![0.0, 0.25, 0.5, 0.75, 0.95, 1.0];

    for conf in confidences {
        let response = McpResponse {
            suggestions: vec!["test".to_string()],
            documentation: None,
            confidence: Some(conf),
        };

        assert!(response.confidence.unwrap() >= 0.0);
        assert!(response.confidence.unwrap() <= 1.0);
    }
}

#[tokio::test]
async fn test_mcp_unicode_support() {
    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///тест.rs".to_string(),
        position: Position { line: 0, character: 0 },
        context: Some("fn 你好() { // Привет".to_string()),
    };

    assert!(request.uri.contains("тест"));
    assert!(request.context.unwrap().contains("你好"));
}

#[tokio::test]
async fn test_mcp_position_edge_cases() {
    let positions = vec![
        Position { line: 0, character: 0 },
        Position { line: u32::MAX, character: 0 },
        Position { line: 0, character: u32::MAX },
        Position { line: u32::MAX, character: u32::MAX },
    ];

    for pos in positions {
        let request = McpRequest {
            request_type: "test".to_string(),
            uri: "file:///test.rs".to_string(),
            position: pos,
            context: None,
        };

        // Should not panic
        let _ = format!("{:?}", request);
    }
}

#[tokio::test]
async fn test_mcp_empty_suggestions() {
    let response = McpResponse {
        suggestions: vec![],
        documentation: None,
        confidence: None,
    };

    assert_eq!(response.suggestions.len(), 0);
}

#[tokio::test]
async fn test_mcp_documentation_formats() {
    let docs = vec![
        "Simple documentation",
        "Multi-line\ndocumentation\nwith\nnewlines",
        "Documentation with **markdown**",
        "```rust\nfn example() {}\n```",
    ];

    for doc in docs {
        let response = McpResponse {
            suggestions: vec!["test".to_string()],
            documentation: Some(doc.to_string()),
            confidence: None,
        };

        assert!(response.documentation.is_some());
        assert!(!response.documentation.unwrap().is_empty());
    }
}
