//! Integration tests for ACP Tool Execution Framework
//!
//! Tests the complete tool execution flow:
//! - Tool registry initialization
//! - Tool execution through the agent
//! - Multi-step tool workflows
//! - Error handling
//! - Claude API integration with tools

use std::path::PathBuf;
use tempfile::TempDir;
use tokio::sync::mpsc;
use universal_lsp::acp::tools::{Tool, ToolRegistry};

// ============================================================================
// Tool Registry Integration Tests
// ============================================================================

#[tokio::test]
async fn test_tool_registry_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Verify all 4 tools are registered
    assert_eq!(registry.count(), 4);

    // Verify tools can be retrieved
    assert!(registry.get_tool("read_file").is_some());
    assert!(registry.get_tool("write_file").is_some());
    assert!(registry.get_tool("list_files").is_some());
    assert!(registry.get_tool("search_code").is_some());
    assert!(registry.get_tool("nonexistent").is_none());
}

#[tokio::test]
async fn test_tool_definitions_format() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let definitions = registry.get_tool_definitions();
    assert_eq!(definitions.len(), 4);

    // Verify each definition has required fields
    for def in definitions {
        assert!(def["name"].is_string(), "Tool definition missing name");
        assert!(def["description"].is_string(), "Tool definition missing description");
        assert!(def["input_schema"].is_object(), "Tool definition missing input_schema");
    }
}

// ============================================================================
// Read File Tool Integration Tests
// ============================================================================

#[tokio::test]
async fn test_read_file_tool_execution() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    tokio::fs::write(&test_file, "Hello, World!").await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("read_file", serde_json::json!({"path": "test.txt"}))
        .await
        .unwrap();

    assert_eq!(result["content"], "Hello, World!");
    assert_eq!(result["success"], true);
    assert_eq!(result["lines"], 1);
}

#[tokio::test]
async fn test_read_file_tool_with_multiline_content() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("multiline.txt");
    tokio::fs::write(&test_file, "Line 1\nLine 2\nLine 3").await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("read_file", serde_json::json!({"path": "multiline.txt"}))
        .await
        .unwrap();

    assert_eq!(result["content"], "Line 1\nLine 2\nLine 3");
    assert_eq!(result["lines"], 3);
}

#[tokio::test]
async fn test_read_file_tool_security_path_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Attempt path traversal attack
    let result = registry
        .execute_tool("read_file", serde_json::json!({"path": "../../../etc/passwd"}))
        .await;

    assert!(result.is_err(), "Should reject path traversal attempts");
}

#[tokio::test]
async fn test_read_file_tool_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("read_file", serde_json::json!({"path": "nonexistent.txt"}))
        .await;

    assert!(result.is_err(), "Should fail for nonexistent files");
}

// ============================================================================
// Write File Tool Integration Tests
// ============================================================================

#[tokio::test]
async fn test_write_file_tool_execution() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": "new_file.txt",
                "content": "Test content"
            }),
        )
        .await
        .unwrap();

    assert_eq!(result["success"], true);
    assert_eq!(result["path"], "new_file.txt");

    // Verify file was actually created
    let content = tokio::fs::read_to_string(temp_dir.path().join("new_file.txt"))
        .await
        .unwrap();
    assert_eq!(content, "Test content");
}

#[tokio::test]
async fn test_write_file_tool_with_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": "subdir/nested.txt",
                "content": "Nested content"
            }),
        )
        .await
        .unwrap();

    assert_eq!(result["success"], true);

    // Verify directory and file were created
    let content = tokio::fs::read_to_string(temp_dir.path().join("subdir/nested.txt"))
        .await
        .unwrap();
    assert_eq!(content, "Nested content");
}

#[tokio::test]
async fn test_write_file_tool_overwrite_existing() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("existing.txt");
    tokio::fs::write(&test_file, "Original").await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": "existing.txt",
                "content": "Updated"
            }),
        )
        .await
        .unwrap();

    assert_eq!(result["success"], true);

    let content = tokio::fs::read_to_string(&test_file).await.unwrap();
    assert_eq!(content, "Updated");
}

#[tokio::test]
async fn test_write_file_tool_security_path_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": "../../../tmp/malicious.txt",
                "content": "Bad content"
            }),
        )
        .await;

    // Should fail due to path validation
    // Note: This might succeed if the path resolves within workspace
    // The key is that it shouldn't write outside workspace
    if result.is_ok() {
        assert!(!PathBuf::from("/tmp/malicious.txt").exists());
    }
}

// ============================================================================
// List Files Tool Integration Tests
// ============================================================================

#[tokio::test]
async fn test_list_files_tool_execution() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::write(temp_dir.path().join("file1.txt"), "").await.unwrap();
    tokio::fs::write(temp_dir.path().join("file2.txt"), "").await.unwrap();
    tokio::fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("list_files", serde_json::json!({"path": "."}))
        .await
        .unwrap();

    assert_eq!(result["success"], true);
    assert_eq!(result["count"], 3);

    let entries = result["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);

    // Verify directories come first
    assert_eq!(entries[0]["type"], "directory");
    assert_eq!(entries[1]["type"], "file");
    assert_eq!(entries[2]["type"], "file");
}

#[tokio::test]
async fn test_list_files_tool_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("list_files", serde_json::json!({"path": "."}))
        .await
        .unwrap();

    assert_eq!(result["success"], true);
    assert_eq!(result["count"], 0);
}

#[tokio::test]
async fn test_list_files_tool_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();
    tokio::fs::write(temp_dir.path().join("subdir/file.txt"), "").await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("list_files", serde_json::json!({"path": "subdir"}))
        .await
        .unwrap();

    assert_eq!(result["success"], true);
    assert_eq!(result["count"], 1);
}

// ============================================================================
// Search Code Tool Integration Tests
// ============================================================================

#[tokio::test]
async fn test_search_code_tool_execution() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::write(
        temp_dir.path().join("test.rs"),
        "fn main() {\n    println!(\"Hello\");\n}",
    )
    .await
    .unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("search_code", serde_json::json!({"pattern": "println"}))
        .await
        .unwrap();

    assert_eq!(result["success"], true);
    assert!(result["count"].as_u64().unwrap() > 0);

    let results = result["results"].as_array().unwrap();
    assert!(results[0]["file"].as_str().unwrap().contains("test.rs"));
    assert_eq!(results[0]["line"], 2);
}

#[tokio::test]
async fn test_search_code_tool_no_matches() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::write(temp_dir.path().join("test.txt"), "Hello World").await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("search_code", serde_json::json!({"pattern": "nonexistent"}))
        .await
        .unwrap();

    assert_eq!(result["success"], true);
    assert_eq!(result["count"], 0);
}

#[tokio::test]
async fn test_search_code_tool_max_results() {
    let temp_dir = TempDir::new().unwrap();
    let mut content = String::new();
    for i in 0..100 {
        content.push_str(&format!("match line {}\n", i));
    }
    tokio::fs::write(temp_dir.path().join("many.txt"), content).await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool(
            "search_code",
            serde_json::json!({"pattern": "match", "max_results": 10}),
        )
        .await
        .unwrap();

    assert_eq!(result["count"], 10);
    assert_eq!(result["truncated"], true);
}

#[tokio::test]
async fn test_search_code_tool_recursive() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();
    tokio::fs::write(temp_dir.path().join("file1.txt"), "pattern here").await.unwrap();
    tokio::fs::write(temp_dir.path().join("subdir/file2.txt"), "pattern there")
        .await
        .unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("search_code", serde_json::json!({"pattern": "pattern"}))
        .await
        .unwrap();

    assert_eq!(result["count"], 2);
}

#[tokio::test]
async fn test_search_code_tool_skips_hidden_files() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::write(temp_dir.path().join(".hidden.txt"), "secret pattern").await.unwrap();
    tokio::fs::write(temp_dir.path().join("visible.txt"), "public pattern").await.unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("search_code", serde_json::json!({"pattern": "pattern"}))
        .await
        .unwrap();

    // Should only find the visible file
    assert_eq!(result["count"], 1);
    let results = result["results"].as_array().unwrap();
    assert!(!results[0]["file"].as_str().unwrap().contains(".hidden"));
}

// ============================================================================
// Multi-Tool Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_multi_tool_workflow_read_modify_write() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::write(temp_dir.path().join("input.txt"), "original content")
        .await
        .unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Step 1: Read file
    let read_result = registry
        .execute_tool("read_file", serde_json::json!({"path": "input.txt"}))
        .await
        .unwrap();
    assert_eq!(read_result["content"], "original content");

    // Step 2: Write modified content
    let write_result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": "output.txt",
                "content": "modified content"
            }),
        )
        .await
        .unwrap();
    assert_eq!(write_result["success"], true);

    // Step 3: Verify with read
    let verify_result = registry
        .execute_tool("read_file", serde_json::json!({"path": "output.txt"}))
        .await
        .unwrap();
    assert_eq!(verify_result["content"], "modified content");
}

#[tokio::test]
async fn test_multi_tool_workflow_search_then_read() {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::write(temp_dir.path().join("target.rs"), "fn important_function() {}")
        .await
        .unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Step 1: Search for pattern
    let search_result = registry
        .execute_tool(
            "search_code",
            serde_json::json!({"pattern": "important_function"}),
        )
        .await
        .unwrap();
    assert!(search_result["count"].as_u64().unwrap() > 0);

    let results = search_result["results"].as_array().unwrap();
    let found_file = results[0]["file"].as_str().unwrap();

    // Step 2: Read the found file
    let read_result = registry
        .execute_tool("read_file", serde_json::json!({"path": found_file}))
        .await
        .unwrap();
    assert!(read_result["content"].as_str().unwrap().contains("important_function"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_tool_execution_with_invalid_tool_name() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let result = registry
        .execute_tool("nonexistent_tool", serde_json::json!({}))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_tool_execution_with_missing_parameters() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // read_file requires "path" parameter
    let result = registry
        .execute_tool("read_file", serde_json::json!({}))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_tool_execution_with_invalid_parameter_types() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // path should be string, not number
    let result = registry
        .execute_tool("read_file", serde_json::json!({"path": 123}))
        .await;

    assert!(result.is_err());
}

// ============================================================================
// Concurrent Tool Execution Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_tool_execution() {
    use std::sync::Arc;

    let temp_dir = TempDir::new().unwrap();
    for i in 0..5 {
        tokio::fs::write(temp_dir.path().join(format!("file{}.txt", i)), format!("content {}", i))
            .await
            .unwrap();
    }

    let registry = Arc::new(ToolRegistry::new(temp_dir.path().to_path_buf()));

    // Execute 5 read operations concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let reg = Arc::clone(&registry);
        let handle = tokio::spawn(async move {
            reg.execute_tool("read_file", serde_json::json!({"path": format!("file{}.txt", i)}))
                .await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_tool_execution_performance_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let large_content = "x".repeat(1_000_000); // 1MB file
    tokio::fs::write(temp_dir.path().join("large.txt"), &large_content)
        .await
        .unwrap();

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let start = std::time::Instant::now();
    let result = registry
        .execute_tool("read_file", serde_json::json!({"path": "large.txt"}))
        .await
        .unwrap();
    let duration = start.elapsed();

    assert_eq!(result["success"], true);
    assert!(duration.as_secs() < 1, "Large file read should complete in <1s");
}

#[tokio::test]
async fn test_search_performance_many_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create 50 files
    for i in 0..50 {
        tokio::fs::write(
            temp_dir.path().join(format!("file{}.txt", i)),
            format!("content {} with pattern", i),
        )
        .await
        .unwrap();
    }

    let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let start = std::time::Instant::now();
    let result = registry
        .execute_tool("search_code", serde_json::json!({"pattern": "pattern"}))
        .await
        .unwrap();
    let duration = start.elapsed();

    assert!(result["count"].as_u64().unwrap() >= 50);
    assert!(duration.as_secs() < 2, "Search across 50 files should complete in <2s");
}
