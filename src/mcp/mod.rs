//! Model Context Protocol (MCP) Client Support
//!
//! This module provides integration with MCP servers for AI-powered features.
//! MCP is a protocol for communication between AI models and external context providers.
//!
//! ## Features
//! - Connect to MCP servers via Stdio, HTTP, or WebSocket
//! - Query context from external sources
//! - Provide codebase context to AI models
//! - Timeout handling with configurable duration
//! - Tool registry for extensible MCP tool support
//!
//! ## Usage
//! ```rust,ignore
//! use universal_lsp::mcp::{McpClient, McpConfig};
//! use universal_lsp::mcp::protocol::{TransportType, McpRequest, Position};
//!
//! // HTTP transport
//! let config = McpConfig {
//!     server_url: "http://localhost:3000".to_string(),
//!     transport: TransportType::Http,
//!     timeout_ms: 5000,
//! };
//! let client = McpClient::new(config);
//!
//! // Stdio transport (for subprocess-based MCP servers)
//! let client = McpClient::new_stdio("smart-tree", vec![], 5000)?;
//!
//! // Query for context
//! let request = McpRequest {
//!     request_type: "completion".to_string(),
//!     uri: "file:///test.rs".to_string(),
//!     position: Position { line: 10, character: 5 },
//!     context: Some("fn main() {".to_string()),
//! };
//! let response = client.query(&request).await?;
//! ```

// Re-export public types
pub mod error;
pub mod protocol;
pub mod transport;

pub use error::{McpError, McpResult};
pub use protocol::{
    JsonRpcRequest, JsonRpcResponse, McpConfig, McpRequest, McpResponse,
    Position, TransportType,
};
pub use transport::{HttpTransport, McpTransport};

use std::time::Duration;
use transport::stdio::StdioTransport;

/// MCP Client with unified transport interface
pub struct McpClient {
    transport: tokio::sync::Mutex<Box<dyn McpTransport>>,
}

impl McpClient {
    /// Create a new MCP client with the given configuration
    pub fn new(config: McpConfig) -> Self {
        let transport: Box<dyn McpTransport> = match config.transport {
            TransportType::Http => {
                Box::new(HttpTransport::new(
                    config.server_url.clone(),
                    Duration::from_millis(config.timeout_ms),
                ))
            }
            TransportType::Stdio => {
                // For Stdio, server_url contains command and args separated by space
                let parts: Vec<String> = config.server_url
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                let command = parts.first().cloned().unwrap_or_default();
                let args = parts.into_iter().skip(1).collect();

                Box::new(StdioTransport::new(
                    command,
                    args,
                    Duration::from_millis(config.timeout_ms),
                ))
            }
            TransportType::WebSocket => {
                Box::new(transport::websocket::WebSocketTransport::new(
                    config.server_url.clone(),
                ))
            }
        };

        Self {
            transport: tokio::sync::Mutex::new(transport),
        }
    }

    /// Create a new MCP client with Stdio transport (convenience constructor)
    pub fn new_stdio(command: impl Into<String>, args: Vec<String>, timeout_ms: u64) -> Self {
        let transport = Box::new(StdioTransport::new(
            command.into(),
            args,
            Duration::from_millis(timeout_ms),
        ));

        Self {
            transport: tokio::sync::Mutex::new(transport),
        }
    }

    /// Create a new MCP client with HTTP transport (convenience constructor)
    pub fn new_http(server_url: impl Into<String>, timeout_ms: u64) -> Self {
        let transport = Box::new(HttpTransport::new(
            server_url.into(),
            Duration::from_millis(timeout_ms),
        ));

        Self {
            transport: tokio::sync::Mutex::new(transport),
        }
    }

    /// Query MCP server with a request
    pub async fn query(&self, request: &McpRequest) -> McpResult<McpResponse> {
        // Convert application-level McpRequest to JSON-RPC request
        let json_rpc_request = JsonRpcRequest::new(
            "query",
            Some(serde_json::to_value(request)?),
        );

        let mut transport = self.transport.lock().await;
        let response = transport.send_request(json_rpc_request).await?;

        // Extract result from JSON-RPC response
        let result = response.result.ok_or_else(|| {
            McpError::Protocol("Missing result in response".to_string())
        })?;

        let mcp_response: McpResponse = serde_json::from_value(result)?;
        Ok(mcp_response)
    }

    /// Check if MCP server is available
    pub async fn is_available(&self) -> bool {
        self.transport.lock().await.is_available().await
    }

    /// Get context from MCP server (convenience method)
    pub async fn get_context(&self, query: &str) -> McpResult<String> {
        let request = McpRequest {
            request_type: "context".to_string(),
            uri: String::new(),
            position: Position { line: 0, character: 0 },
            context: Some(query.to_string()),
        };

        let response = self.query(&request).await?;
        Ok(response.suggestions.join("\n"))
    }

    /// Close the MCP client and clean up resources
    pub async fn close(self) -> McpResult<()> {
        self.transport.lock().await.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_http_creation() {
        let client = McpClient::new_http("http://localhost:3000", 5000);
        assert!(!client.is_available().await);
    }

    #[tokio::test]
    async fn test_mcp_client_stdio_creation() {
        let client = McpClient::new_stdio("echo", vec!["test".to_string()], 5000);
        // Stdio transport won't be available until started
        assert!(!client.is_available().await);
    }

    #[tokio::test]
    async fn test_get_context_placeholder() {
        let client = McpClient::new_http("http://localhost:3000", 5000);
        // This should fail since there's no actual MCP server running
        let result = client.get_context("test query").await;
        assert!(result.is_err(), "Expected error when no MCP server is available");
    }
}
