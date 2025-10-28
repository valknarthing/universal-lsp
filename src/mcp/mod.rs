//! Model Context Protocol (MCP) Client Support
//!
//! This module provides integration with MCP servers for AI-powered features.
//! MCP is a protocol for communication between AI models and external context providers.
//!
//! ## Features
//! - Connect to MCP servers via HTTP
//! - Query context from external sources
//! - Provide codebase context to AI models
//! - Timeout handling with configurable duration
//!
//! ## Usage
//! ```rust,ignore
//! use universal_lsp::mcp::McpClient;
//!
//! let client = McpClient::new("http://localhost:3000");
//! let context = client.query(&request).await?;
//! ```

use anyhow::{Result, Context as AnyhowContext};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub server_url: String,
    pub transport: TransportType,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportType {
    Stdio,
    Http,
    WebSocket,
}

/// MCP Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// Type of request (completion, hover, definition, etc.)
    pub request_type: String,
    /// File path or URI
    pub uri: String,
    /// Cursor position
    pub position: Position,
    /// Surrounding context (e.g., current function, class)
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// MCP Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// Enhanced suggestions from MCP server
    pub suggestions: Vec<String>,
    /// Additional context or documentation
    pub documentation: Option<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: Option<f32>,
}

#[derive(Debug)]
pub struct McpClient {
    config: McpConfig,
    http_client: reqwest::Client,
}

impl McpClient {
    /// Create a new MCP client with the given configuration
    pub fn new(config: McpConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, http_client }
    }

    /// Query MCP server with a request
    pub async fn query(&self, request: &McpRequest) -> Result<McpResponse> {
        match self.config.transport {
            TransportType::Http => self.query_http(request).await,
            TransportType::Stdio => {
                // TODO: Implement stdio transport
                Err(anyhow::anyhow!("Stdio transport not yet implemented"))
            }
            TransportType::WebSocket => {
                // TODO: Implement WebSocket transport
                Err(anyhow::anyhow!("WebSocket transport not yet implemented"))
            }
        }
    }

    /// Query via HTTP
    async fn query_http(&self, request: &McpRequest) -> Result<McpResponse> {
        let response = self
            .http_client
            .post(&self.config.server_url)
            .json(request)
            .send()
            .await
            .context("Failed to send MCP request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MCP server returned error status: {}",
                response.status()
            ));
        }

        let mcp_response: McpResponse = response
            .json()
            .await
            .context("Failed to parse MCP response")?;

        Ok(mcp_response)
    }

    /// Check if MCP server is available
    pub async fn is_available(&self) -> bool {
        match self.config.transport {
            TransportType::Http => self.check_http_health().await,
            _ => false,
        }
    }

    /// HTTP health check
    async fn check_http_health(&self) -> bool {
        let health_url = format!("{}/health", self.config.server_url);
        self.http_client
            .get(&health_url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Get context from MCP server (convenience method)
    pub async fn get_context(&self, query: &str) -> Result<String> {
        let request = McpRequest {
            request_type: "context".to_string(),
            uri: String::new(),
            position: Position { line: 0, character: 0 },
            context: Some(query.to_string()),
        };

        let response = self.query(&request).await?;
        Ok(response.suggestions.join("\n"))
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".to_string(),
            transport: TransportType::Http,
            timeout_ms: 5000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_creation() {
        let config = McpConfig::default();
        let client = McpClient::new(config);
        assert!(!client.is_available().await);
    }

    #[tokio::test]
    async fn test_get_context_placeholder() {
        let config = McpConfig::default();
        let client = McpClient::new(config);
        // This should fail since there's no actual MCP server running
        let result = client.get_context("test query").await;
        assert!(result.is_err(), "Expected error when no MCP server is available");
    }
}
