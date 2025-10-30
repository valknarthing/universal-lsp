//! Phase 4: MCP Integration Tests
//!
//! Tests for Model Context Protocol integration across LSP handlers

use universal_lsp::mcp::{McpClient, McpConfig, McpRequest, McpResponse, Position, TransportType};

#[test]
fn test_mcp_request_structure() {
    let request = McpRequest {
        request_type: "hover".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 10, character: 5 },
        context: Some("fn main() {".to_string()),
    };

    assert_eq!(request.request_type, "hover");
    assert_eq!(request.uri, "file:///test.rs");
    assert_eq!(request.position.line, 10);
    assert_eq!(request.position.character, 5);
    assert_eq!(request.context, Some("fn main() {".to_string()));
}

#[test]
fn test_mcp_response_structure() {
    let response = McpResponse {
        suggestions: vec!["suggestion1".to_string(), "suggestion2".to_string()],
        documentation: Some("This is documentation".to_string()),
        confidence: Some(0.95),
    };

    assert_eq!(response.suggestions.len(), 2);
    assert_eq!(response.suggestions[0], "suggestion1");
    assert_eq!(response.documentation, Some("This is documentation".to_string()));
    assert_eq!(response.confidence, Some(0.95));
}

#[test]
fn test_mcp_client_creation_http() {
    let config = McpConfig {
        server_url: "http://localhost:3000".to_string(),
        transport: TransportType::Http,
        timeout_ms: 5000,
    };

    let client = McpClient::new(config);
    // Client should be created successfully
    // Note: actual connection testing requires a running MCP server
    drop(client);
}

#[test]
fn test_mcp_client_creation_stdio() {
    let config = McpConfig {
        server_url: "echo test".to_string(),
        transport: TransportType::Stdio,
        timeout_ms: 5000,
    };

    let client = McpClient::new(config);
    // Client should be created successfully
    drop(client);
}

#[test]
fn test_mcp_request_types() {
    let request_types = vec!["hover", "completion", "diagnostics", "definition"];

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

#[test]
fn test_mcp_response_empty_suggestions() {
    let response = McpResponse {
        suggestions: Vec::new(),
        documentation: None,
        confidence: None,
    };

    assert!(response.suggestions.is_empty());
    assert_eq!(response.documentation, None);
    assert_eq!(response.confidence, None);
}

#[test]
fn test_mcp_response_with_confidence_scores() {
    let scores = vec![0.0, 0.5, 0.95, 1.0];

    for score in scores {
        let response = McpResponse {
            suggestions: vec!["test".to_string()],
            documentation: None,
            confidence: Some(score),
        };

        assert_eq!(response.confidence, Some(score));
        assert!(score >= 0.0 && score <= 1.0);
    }
}

#[test]
fn test_mcp_position_coordinates() {
    let positions = vec![
        (0, 0),
        (10, 5),
        (100, 50),
        (1000, 500),
    ];

    for (line, character) in positions {
        let pos = Position { line, character };
        assert_eq!(pos.line, line);
        assert_eq!(pos.character, character);
    }
}

#[test]
fn test_mcp_request_with_context() {
    let contexts = vec![
        "fn main() {}",
        "class MyClass:",
        "import sys",
        "struct MyStruct { value: i32 }",
    ];

    for context in contexts {
        let request = McpRequest {
            request_type: "hover".to_string(),
            uri: "file:///test.rs".to_string(),
            position: Position { line: 0, character: 0 },
            context: Some(context.to_string()),
        };

        assert_eq!(request.context, Some(context.to_string()));
    }
}

#[test]
fn test_mcp_response_multiple_suggestions() {
    let suggestions = vec![
        "suggestion1".to_string(),
        "suggestion2".to_string(),
        "suggestion3".to_string(),
        "suggestion4".to_string(),
        "suggestion5".to_string(),
    ];

    let response = McpResponse {
        suggestions: suggestions.clone(),
        documentation: None,
        confidence: Some(0.8),
    };

    assert_eq!(response.suggestions.len(), 5);
    for (i, suggestion) in suggestions.iter().enumerate() {
        assert_eq!(&response.suggestions[i], suggestion);
    }
}

#[test]
fn test_mcp_request_for_diagnostics() {
    let request = McpRequest {
        request_type: "diagnostics".to_string(),
        uri: "file:///src/main.rs".to_string(),
        position: Position { line: 0, character: 0 },
        context: Some("fn main() { let x = 5; }".to_string()),
    };

    assert_eq!(request.request_type, "diagnostics");
    assert!(request.context.is_some());
}

#[test]
fn test_mcp_request_for_completion() {
    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///src/lib.rs".to_string(),
        position: Position { line: 42, character: 15 },
        context: Some("use std::".to_string()),
    };

    assert_eq!(request.request_type, "completion");
    assert_eq!(request.position.line, 42);
    assert_eq!(request.position.character, 15);
}

#[test]
fn test_mcp_request_for_hover() {
    let request = McpRequest {
        request_type: "hover".to_string(),
        uri: "file:///test.py".to_string(),
        position: Position { line: 5, character: 10 },
        context: Some("def my_function(param1, param2):".to_string()),
    };

    assert_eq!(request.request_type, "hover");
    assert_eq!(request.uri, "file:///test.py");
}

#[test]
fn test_mcp_response_with_long_documentation() {
    let long_doc = "This is a very long documentation string that spans multiple lines\
        and provides comprehensive information about the symbol being hovered over.\
        It includes examples, parameter descriptions, return values, and usage notes.\
        This is important for providing rich context to developers.".to_string();

    let response = McpResponse {
        suggestions: vec![],
        documentation: Some(long_doc.clone()),
        confidence: Some(0.99),
    };

    assert_eq!(response.documentation, Some(long_doc));
    assert!(response.suggestions.is_empty());
}

#[test]
fn test_mcp_config_default_values() {
    let config = McpConfig::default();

    assert_eq!(config.server_url, "http://localhost:3000");
    assert!(matches!(config.transport, TransportType::Http));
    assert_eq!(config.timeout_ms, 5000);
}

#[test]
fn test_mcp_config_custom_timeout() {
    let timeouts = vec![1000, 5000, 10000, 30000];

    for timeout in timeouts {
        let config = McpConfig {
            server_url: "http://localhost:3000".to_string(),
            transport: TransportType::Http,
            timeout_ms: timeout,
        };

        assert_eq!(config.timeout_ms, timeout);
    }
}

#[test]
fn test_mcp_transport_types() {
    let config_http = McpConfig {
        server_url: "http://localhost:3000".to_string(),
        transport: TransportType::Http,
        timeout_ms: 5000,
    };

    let config_stdio = McpConfig {
        server_url: "npx mcp-server".to_string(),
        transport: TransportType::Stdio,
        timeout_ms: 5000,
    };

    let config_ws = McpConfig {
        server_url: "ws://localhost:3000".to_string(),
        transport: TransportType::WebSocket,
        timeout_ms: 5000,
    };

    assert!(matches!(config_http.transport, TransportType::Http));
    assert!(matches!(config_stdio.transport, TransportType::Stdio));
    assert!(matches!(config_ws.transport, TransportType::WebSocket));
}

#[test]
fn test_mcp_request_serialization() {
    let request = McpRequest {
        request_type: "hover".to_string(),
        uri: "file:///test.rs".to_string(),
        position: Position { line: 10, character: 5 },
        context: Some("test context".to_string()),
    };

    // Test that request can be serialized to JSON
    let json = serde_json::to_string(&request);
    assert!(json.is_ok());

    // Test that it can be deserialized back
    let json_str = json.unwrap();
    let deserialized: Result<McpRequest, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok());

    let deserialized_request = deserialized.unwrap();
    assert_eq!(deserialized_request.request_type, request.request_type);
    assert_eq!(deserialized_request.uri, request.uri);
    assert_eq!(deserialized_request.position.line, request.position.line);
}

#[test]
fn test_mcp_response_serialization() {
    let response = McpResponse {
        suggestions: vec!["test1".to_string(), "test2".to_string()],
        documentation: Some("docs".to_string()),
        confidence: Some(0.95),
    };

    // Test that response can be serialized to JSON
    let json = serde_json::to_string(&response);
    assert!(json.is_ok());

    // Test that it can be deserialized back
    let json_str = json.unwrap();
    let deserialized: Result<McpResponse, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok());

    let deserialized_response = deserialized.unwrap();
    assert_eq!(deserialized_response.suggestions, response.suggestions);
    assert_eq!(deserialized_response.documentation, response.documentation);
    assert_eq!(deserialized_response.confidence, response.confidence);
}

#[test]
fn test_mcp_request_without_context() {
    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///test.js".to_string(),
        position: Position { line: 20, character: 10 },
        context: None,
    };

    assert_eq!(request.context, None);
}

#[test]
fn test_mcp_response_high_confidence() {
    let response = McpResponse {
        suggestions: vec!["highly_confident_suggestion".to_string()],
        documentation: Some("Very reliable information".to_string()),
        confidence: Some(0.99),
    };

    assert!(response.confidence.unwrap() > 0.9);
}

#[test]
fn test_mcp_response_low_confidence() {
    let response = McpResponse {
        suggestions: vec!["uncertain_suggestion".to_string()],
        documentation: None,
        confidence: Some(0.3),
    };

    assert!(response.confidence.unwrap() < 0.5);
}

#[test]
fn test_mcp_integration_hover_request_format() {
    // Simulates what main.rs would send for hover
    let request = McpRequest {
        request_type: "hover".to_string(),
        uri: "file:///src/main.rs".to_string(),
        position: Position { line: 15, character: 20 },
        context: Some("let value = calculate_sum(a, b);".to_string()),
    };

    assert_eq!(request.request_type, "hover");
    assert!(request.context.is_some());
}

#[test]
fn test_mcp_integration_completion_request_format() {
    // Simulates what main.rs would send for completion
    let request = McpRequest {
        request_type: "completion".to_string(),
        uri: "file:///src/lib.rs".to_string(),
        position: Position { line: 50, character: 12 },
        context: None,
    };

    assert_eq!(request.request_type, "completion");
}

#[test]
fn test_mcp_integration_diagnostics_request_format() {
    // Simulates what main.rs would send for diagnostics
    let request = McpRequest {
        request_type: "diagnostics".to_string(),
        uri: "file:///src/main.rs".to_string(),
        position: Position { line: 0, character: 0 },
        context: Some("full file content here".to_string()),
    };

    assert_eq!(request.request_type, "diagnostics");
    assert!(request.context.is_some());
}
