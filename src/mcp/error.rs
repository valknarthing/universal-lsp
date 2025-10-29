//! MCP-specific error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("MCP transport error: {0}")]
    Transport(String),

    #[error("MCP protocol error: {0}")]
    Protocol(String),

    #[error("MCP timeout: operation exceeded {0}ms")]
    Timeout(u64),

    #[error("MCP tool not found: {0}")]
    ToolNotFound(String),

    #[error("MCP tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Invalid MCP configuration: {0}")]
    InvalidConfig(String),

    #[error("JSON-RPC error: code={0}, message={1}")]
    JsonRpc(i32, String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type McpResult<T> = Result<T, McpError>;
