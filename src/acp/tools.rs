//! Tool Execution Framework for ACP Agent
//!
//! This module provides tools that Claude can use to interact with the workspace:
//! - read_file: Read file contents
//! - write_file: Write/update files
//! - list_files: Browse workspace
//! - search_code: Find code patterns
//!
//! Security: All tools are sandboxed to the workspace root.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Tool trait for Claude-executable actions
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name (used by Claude to invoke it)
    fn name(&self) -> &str;

    /// Get human-readable description for Claude
    fn description(&self) -> &str;

    /// Get JSON schema for tool parameters
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with given parameters
    async fn execute(&self, args: Value) -> Result<Value>;
}

// ============================================================================
// ReadFileTool - Read file contents from workspace
// ============================================================================

pub struct ReadFileTool {
    workspace_root: PathBuf,
}

impl ReadFileTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Validate path is within workspace (security)
    fn validate_path(&self, path: &str) -> Result<PathBuf> {
        let full_path = self.workspace_root.join(path);
        let canonical_path = full_path.canonicalize()
            .context("Failed to resolve path")?;

        if !canonical_path.starts_with(&self.workspace_root) {
            anyhow::bail!("Path outside workspace: {}", path);
        }

        Ok(canonical_path)
    }
}

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file from the workspace. Use this to examine code, \
         configuration files, or any text-based content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file from workspace root (e.g., 'src/main.rs', 'README.md')"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args["path"]
            .as_str()
            .context("Missing 'path' parameter")?;

        tracing::info!("read_file: {}", path);

        let full_path = self.validate_path(path)?;

        let content = fs::read_to_string(&full_path)
            .await
            .context(format!("Failed to read file: {}", path))?;

        let metadata = fs::metadata(&full_path).await?;

        Ok(json!({
            "path": path,
            "content": content,
            "size": metadata.len(),
            "lines": content.lines().count(),
            "success": true
        }))
    }
}

// ============================================================================
// WriteFileTool - Write or update files in workspace
// ============================================================================

pub struct WriteFileTool {
    workspace_root: PathBuf,
}

impl WriteFileTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf> {
        let full_path = self.workspace_root.join(path);

        // For new files, validate parent directory
        if let Some(parent) = full_path.parent() {
            if parent.exists() {
                let canonical_parent = parent.canonicalize()
                    .context("Failed to resolve parent path")?;
                if !canonical_parent.starts_with(&self.workspace_root) {
                    anyhow::bail!("Path outside workspace: {}", path);
                }
            }
        }

        Ok(full_path)
    }
}

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write or update a file in the workspace. Creates parent directories if needed. \
         Use this to create new files or modify existing ones."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file from workspace root"
                },
                "content": {
                    "type": "string",
                    "description": "Complete content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args["path"]
            .as_str()
            .context("Missing 'path' parameter")?;
        let content = args["content"]
            .as_str()
            .context("Missing 'content' parameter")?;

        tracing::info!("write_file: {} ({} bytes)", path, content.len());

        let full_path = self.validate_path(path)?;

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create parent directories")?;
        }

        fs::write(&full_path, content)
            .await
            .context(format!("Failed to write file: {}", path))?;

        Ok(json!({
            "path": path,
            "size": content.len(),
            "lines": content.lines().count(),
            "success": true,
            "created": !full_path.exists()
        }))
    }
}

// ============================================================================
// ListFilesTool - List files and directories in workspace
// ============================================================================

pub struct ListFilesTool {
    workspace_root: PathBuf,
}

impl ListFilesTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf> {
        let full_path = if path.is_empty() || path == "." {
            self.workspace_root.clone()
        } else {
            self.workspace_root.join(path)
        };

        if full_path.exists() {
            let canonical = full_path.canonicalize()
                .context("Failed to resolve path")?;
            if !canonical.starts_with(&self.workspace_root) {
                anyhow::bail!("Path outside workspace: {}", path);
            }
            Ok(canonical)
        } else {
            anyhow::bail!("Path does not exist: {}", path);
        }
    }
}

#[async_trait::async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files and directories in a directory. Use this to explore the workspace \
         structure and find files you need to work with."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to directory (defaults to workspace root if empty)",
                    "default": "."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args["path"]
            .as_str()
            .unwrap_or(".");

        tracing::info!("list_files: {}", path);

        let full_path = self.validate_path(path)?;

        let mut entries = Vec::new();
        let mut dir_reader = fs::read_dir(&full_path)
            .await
            .context(format!("Failed to read directory: {}", path))?;

        while let Some(entry) = dir_reader.next_entry().await? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().await?;
            let metadata = entry.metadata().await?;

            entries.push(json!({
                "name": file_name,
                "type": if file_type.is_dir() { "directory" } else { "file" },
                "size": metadata.len(),
                "modified": metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
            }));
        }

        // Sort: directories first, then files alphabetically
        entries.sort_by(|a, b| {
            let a_type = a["type"].as_str().unwrap_or("");
            let b_type = b["type"].as_str().unwrap_or("");
            let a_name = a["name"].as_str().unwrap_or("");
            let b_name = b["name"].as_str().unwrap_or("");

            match (a_type, b_type) {
                ("directory", "file") => std::cmp::Ordering::Less,
                ("file", "directory") => std::cmp::Ordering::Greater,
                _ => a_name.cmp(b_name),
            }
        });

        Ok(json!({
            "path": path,
            "entries": entries,
            "count": entries.len(),
            "success": true
        }))
    }
}

// ============================================================================
// SearchCodeTool - Search for code patterns in workspace
// ============================================================================

pub struct SearchCodeTool {
    workspace_root: PathBuf,
}

impl SearchCodeTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Search files recursively for pattern
    fn search_in_dir<'a>(&'a self, dir: &'a Path, pattern: &'a str, max_results: usize) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Value>>> + Send + 'a>> {
        Box::pin(async move {
        let mut results = Vec::new();
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip hidden files and common exclusions
            if file_name_str.starts_with('.')
                || file_name_str == "target"
                || file_name_str == "node_modules"
                || file_name_str == "__pycache__" {
                continue;
            }

            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                // Recursively search subdirectories
                if let Ok(sub_results) = self.search_in_dir(&path, pattern, max_results - results.len()).await {
                    results.extend(sub_results);
                }
            } else if file_type.is_file() {
                // Search in text files only
                if let Ok(content) = fs::read_to_string(&path).await {
                    let relative_path = path.strip_prefix(&self.workspace_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    // Find matching lines
                    for (line_num, line) in content.lines().enumerate() {
                        if line.contains(pattern) {
                            results.push(json!({
                                "file": relative_path,
                                "line": line_num + 1,
                                "content": line.trim(),
                            }));

                            if results.len() >= max_results {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
        })
    }
}

#[async_trait::async_trait]
impl Tool for SearchCodeTool {
    fn name(&self) -> &str {
        "search_code"
    }

    fn description(&self) -> &str {
        "Search for code patterns across all files in the workspace. Use this to find \
         function definitions, variable usages, or any text pattern."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Text pattern to search for (case-sensitive substring match)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 50
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let pattern = args["pattern"]
            .as_str()
            .context("Missing 'pattern' parameter")?;
        let max_results = args["max_results"]
            .as_u64()
            .unwrap_or(50) as usize;

        tracing::info!("search_code: '{}' (max: {})", pattern, max_results);

        let results = self.search_in_dir(&self.workspace_root, pattern, max_results).await?;

        Ok(json!({
            "pattern": pattern,
            "results": results,
            "count": results.len(),
            "truncated": results.len() >= max_results,
            "success": true
        }))
    }
}

// ============================================================================
// Tool Registry - Manages all available tools
// ============================================================================

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry with all standard tools
    pub fn new(workspace_root: PathBuf) -> Self {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(ReadFileTool::new(workspace_root.clone())),
            Box::new(WriteFileTool::new(workspace_root.clone())),
            Box::new(ListFilesTool::new(workspace_root.clone())),
            Box::new(SearchCodeTool::new(workspace_root)),
        ];

        Self { tools }
    }

    /// Get all tool definitions for Claude (name, description, schema)
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.parameters_schema()
                })
            })
            .collect()
    }

    /// Find a tool by name
    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|tool| tool.name() == name)
            .map(|tool| tool.as_ref())
    }

    /// Execute a tool by name with parameters
    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        let tool = self
            .get_tool(name)
            .context(format!("Unknown tool: {}", name))?;

        tool.execute(args).await
    }

    /// Get number of available tools
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").await.unwrap();

        let tool = ReadFileTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "test.txt" })).await.unwrap();

        assert_eq!(result["content"], "Hello, World!");
        assert_eq!(result["success"], true);
    }

    #[tokio::test]
    async fn test_write_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteFileTool::new(temp_dir.path().to_path_buf());

        let result = tool.execute(json!({
            "path": "new_file.txt",
            "content": "Test content"
        })).await.unwrap();

        assert_eq!(result["success"], true);

        let content = fs::read_to_string(temp_dir.path().join("new_file.txt"))
            .await
            .unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_list_files_tool() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "").await.unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "").await.unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();

        let tool = ListFilesTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "." })).await.unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["count"], 3);
    }

    #[tokio::test]
    async fn test_search_code_tool() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "fn main() {\n    println!(\"Hello\");\n}")
            .await
            .unwrap();

        let tool = SearchCodeTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "pattern": "println" })).await.unwrap();

        assert_eq!(result["success"], true);
        assert!(result["count"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let temp_dir = TempDir::new().unwrap();
        let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        assert_eq!(registry.count(), 4);

        let definitions = registry.get_tool_definitions();
        assert_eq!(definitions.len(), 4);

        assert!(registry.get_tool("read_file").is_some());
        assert!(registry.get_tool("write_file").is_some());
        assert!(registry.get_tool("list_files").is_some());
        assert!(registry.get_tool("search_code").is_some());
        assert!(registry.get_tool("nonexistent").is_none());
    }
}
