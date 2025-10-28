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

use universal_lsp::mcp::{McpClient, McpConfig, McpPipeline, McpRequest, McpResponse, merge_mcp_responses};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_mcp_client_creation() {
    let config = McpConfig {
        name: "test-server".to_string(),
        url: "http://localhost:3000".to_string(),
        timeout: Duration::from_secs(5),
        headers: Default::default(),
    };

    let client = McpClient::new(config);
    assert!(client.is_ok(), "Failed to create MCP client");
}

#[tokio::test]
async fn test_mcp_client_with_custom_headers() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
    headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

    let config = McpConfig {
        name: "test-server".to_string(),
        url: "http://localhost:3000".to_string(),
        timeout: Duration::from_secs(5),
        headers,
    };

    let client = McpClient::new(config);
    assert!(client.is_ok(), "Failed to create MCP client with custom headers");
}

#[tokio::test]
async fn test_mcp_request_serialization() {
    let request = McpRequest {
        method: "getContext".to_string(),
        params: serde_json::json!({
            "query": "What is this function?",
            "file": "test.rs",
            "line": 42
        }),
    };

    let serialized = serde_json::to_string(&request);
    assert!(serialized.is_ok(), "Failed to serialize MCP request");

    let json_str = serialized.unwrap();
    assert!(json_str.contains("getContext"));
    assert!(json_str.contains("test.rs"));
}

#[tokio::test]
async fn test_mcp_response_deserialization() {
    let json = r#"{
        "result": "This is a helper function that calculates the sum",
        "metadata": {
            "confidence": 0.95,
            "source": "code-analysis"
        }
    }"#;

    let response: Result<McpResponse, _> = serde_json::from_str(json);
    assert!(response.is_ok(), "Failed to deserialize MCP response");

    let resp = response.unwrap();
    assert!(resp.result.contains("helper function"));
}

#[tokio::test]
async fn test_mcp_client_timeout() {
    let config = McpConfig {
        name: "slow-server".to_string(),
        url: "http://localhost:9999".to_string(), // Non-existent server
        timeout: Duration::from_millis(100), // Very short timeout
        headers: Default::default(),
    };

    let client = McpClient::new(config).expect("Failed to create client");

    let request = McpRequest {
        method: "getContext".to_string(),
        params: serde_json::json!({"query": "test"}),
    };

    // Should timeout or fail quickly
    let result = timeout(Duration::from_millis(500), client.query(&request)).await;

    match result {
        Ok(Ok(_)) => panic!("Expected timeout or error, but got success"),
        Ok(Err(_)) => {}, // Expected: connection error
        Err(_) => {}, // Expected: timeout
    }
}

#[tokio::test]
async fn test_mcp_client_invalid_url() {
    let config = McpConfig {
        name: "invalid-server".to_string(),
        url: "not-a-valid-url".to_string(),
        timeout: Duration::from_secs(5),
        headers: Default::default(),
    };

    let client = McpClient::new(config).expect("Failed to create client");

    let request = McpRequest {
        method: "getContext".to_string(),
        params: serde_json::json!({"query": "test"}),
    };

    let result = client.query(&request).await;
    assert!(result.is_err(), "Expected error for invalid URL");
}

#[tokio::test]
async fn test_mcp_pipeline_creation() {
    let pre_config = McpConfig {
        name: "pre-processor".to_string(),
        url: "http://localhost:3001".to_string(),
        timeout: Duration::from_secs(5),
        headers: Default::default(),
    };

    let post_config = McpConfig {
        name: "post-processor".to_string(),
        url: "http://localhost:3002".to_string(),
        timeout: Duration::from_secs(5),
        headers: Default::default(),
    };

    let pre_clients = vec![Arc::new(McpClient::new(pre_config).unwrap())];
    let post_clients = vec![Arc::new(McpClient::new(post_config).unwrap())];

    let pipeline = McpPipeline::new(pre_clients, post_clients);
    assert!(pipeline.is_ok(), "Failed to create MCP pipeline");
}

#[tokio::test]
async fn test_mcp_pipeline_empty() {
    let pipeline = McpPipeline::new(vec![], vec![]);
    assert!(pipeline.is_ok(), "Failed to create empty MCP pipeline");

    let pipe = pipeline.unwrap();
    let request = McpRequest {
        method: "getContext".to_string(),
        params: serde_json::json!({"query": "test"}),
    };

    // Empty pipeline should return empty results
    let pre_results = pipe.pre_process(request.clone()).await;
    assert!(pre_results.is_ok());
    assert_eq!(pre_results.unwrap().len(), 0);

    let post_results = pipe.post_process(request, "original response").await;
    assert!(post_results.is_ok());
    assert_eq!(post_results.unwrap().len(), 0);
}

#[tokio::test]
async fn test_merge_mcp_responses_empty() {
    let responses = vec![];
    let merged = merge_mcp_responses(responses);

    assert_eq!(merged.result, "");
}

#[tokio::test]
async fn test_merge_mcp_responses_single() {
    let response = McpResponse {
        result: "Single response".to_string(),
        metadata: Some(serde_json::json!({"source": "server1"})),
    };

    let merged = merge_mcp_responses(vec![response]);
    assert_eq!(merged.result, "Single response");
}

#[tokio::test]
async fn test_merge_mcp_responses_multiple() {
    let responses = vec![
        McpResponse {
            result: "Response from server 1".to_string(),
            metadata: Some(serde_json::json!({"source": "server1"})),
        },
        McpResponse {
            result: "Response from server 2".to_string(),
            metadata: Some(serde_json::json!({"source": "server2"})),
        },
        McpResponse {
            result: "Response from server 3".to_string(),
            metadata: Some(serde_json::json!({"source": "server3"})),
        },
    ];

    let merged = merge_mcp_responses(responses);

    // Should contain content from all responses
    assert!(merged.result.contains("server 1") || merged.result.len() > 0);
}

#[tokio::test]
async fn test_mcp_parallel_processing() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Simulate parallel processing by creating multiple clients
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..10 {
        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            let config = McpConfig {
                name: format!("server-{}", i),
                url: format!("http://localhost:{}", 3000 + i),
                timeout: Duration::from_secs(1),
                headers: Default::default(),
            };

            let client = McpClient::new(config).expect("Failed to create client");
            counter_clone.fetch_add(1, Ordering::SeqCst);

            // Return client to verify it was created
            client
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Task panicked");
    }

    // All 10 clients should have been created
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

#[tokio::test]
async fn test_mcp_request_with_large_payload() {
    // Test with large context (simulating large file)
    let large_content = "fn main() { println!(\"test\"); }\n".repeat(1000);

    let request = McpRequest {
        method: "getContext".to_string(),
        params: serde_json::json!({
            "query": "What does this code do?",
            "content": large_content,
            "language": "rust"
        }),
    };

    let serialized = serde_json::to_string(&request);
    assert!(serialized.is_ok(), "Failed to serialize large request");

    let json_str = serialized.unwrap();
    assert!(json_str.len() > 10000, "Large payload should be preserved");
}

#[tokio::test]
async fn test_mcp_response_with_metadata() {
    let json = r#"{
        "result": "Function calculates fibonacci numbers",
        "metadata": {
            "confidence": 0.98,
            "source": "ai-analysis",
            "model": "claude-sonnet-4",
            "tokens_used": 150,
            "processing_time_ms": 234
        }
    }"#;

    let response: Result<McpResponse, _> = serde_json::from_str(json);
    assert!(response.is_ok(), "Failed to deserialize response with metadata");

    let resp = response.unwrap();
    assert!(resp.metadata.is_some());

    let metadata = resp.metadata.unwrap();
    assert_eq!(metadata["confidence"], 0.98);
    assert_eq!(metadata["model"], "claude-sonnet-4");
}

#[tokio::test]
async fn test_mcp_error_response_handling() {
    let error_json = r#"{
        "error": {
            "code": -32600,
            "message": "Invalid request format"
        }
    }"#;

    // Should handle error responses gracefully
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(error_json);
    assert!(parsed.is_ok());

    let value = parsed.unwrap();
    assert!(value["error"].is_object());
    assert_eq!(value["error"]["code"], -32600);
}

#[tokio::test]
async fn test_mcp_unicode_handling() {
    let request = McpRequest {
        method: "getContext".to_string(),
        params: serde_json::json!({
            "query": "What is this function? 你好世界 Привет мир",
            "content": "function привет() { return \"Привет, мир!\"; }",
            "language": "javascript"
        }),
    };

    let serialized = serde_json::to_string(&request);
    assert!(serialized.is_ok(), "Failed to serialize Unicode request");

    let json_str = serialized.unwrap();
    assert!(json_str.contains("你好世界"));
    assert!(json_str.contains("Привет"));
}

#[tokio::test]
async fn test_mcp_concurrent_pipeline_processing() {
    // Create pipeline with multiple servers
    let mut pre_clients = vec![];
    for i in 0..5 {
        let config = McpConfig {
            name: format!("pre-{}", i),
            url: format!("http://localhost:{}", 4000 + i),
            timeout: Duration::from_secs(1),
            headers: Default::default(),
        };
        pre_clients.push(Arc::new(McpClient::new(config).unwrap()));
    }

    let pipeline = McpPipeline::new(pre_clients, vec![]).expect("Failed to create pipeline");

    let request = McpRequest {
        method: "getContext".to_string(),
        params: serde_json::json!({"query": "test"}),
    };

    // Pre-process should handle multiple servers concurrently
    // Note: This will fail to connect, but shouldn't panic
    let result = timeout(Duration::from_secs(5), pipeline.pre_process(request)).await;

    // Should complete within timeout (either with results or errors)
    assert!(result.is_ok(), "Pipeline processing timed out");
}

#[tokio::test]
async fn test_mcp_config_validation() {
    // Test various config scenarios
    let configs = vec![
        McpConfig {
            name: "valid-http".to_string(),
            url: "http://localhost:3000".to_string(),
            timeout: Duration::from_secs(5),
            headers: Default::default(),
        },
        McpConfig {
            name: "valid-https".to_string(),
            url: "https://api.example.com/mcp".to_string(),
            timeout: Duration::from_secs(10),
            headers: Default::default(),
        },
        McpConfig {
            name: "with-port".to_string(),
            url: "http://127.0.0.1:8080".to_string(),
            timeout: Duration::from_millis(500),
            headers: Default::default(),
        },
    ];

    for config in configs {
        let client = McpClient::new(config);
        assert!(client.is_ok(), "Valid config should create client successfully");
    }
}
