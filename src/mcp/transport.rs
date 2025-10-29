//! MCP Transport Layer
//!
//! Provides different transport mechanisms for MCP communication:
//! - Stdio: Process-based communication via stdin/stdout
//! - HTTP: REST API communication
//! - WebSocket: Real-time bidirectional communication

use crate::mcp::error::{McpError, McpResult};
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;
use std::time::Duration;

/// Transport trait for MCP communication
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a request and receive a response
    async fn send_request(&mut self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse>;

    /// Send a notification (no response expected)
    async fn send_notification(&mut self, notification: JsonRpcRequest) -> McpResult<()>;

    /// Check if the transport is available
    async fn is_available(&self) -> bool;

    /// Close the transport
    async fn close(&mut self) -> McpResult<()>;
}

/// HTTP Transport implementation
pub struct HttpTransport {
    server_url: String,
    http_client: reqwest::Client,
    timeout: Duration,
}

impl HttpTransport {
    pub fn new(server_url: String, timeout: Duration) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            server_url,
            http_client,
            timeout,
        }
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn send_request(&mut self, request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let response = self
            .http_client
            .post(&self.server_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| McpError::Transport(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(McpError::Transport(format!(
                "HTTP status error: {}",
                response.status()
            )));
        }

        let json_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| McpError::Transport(format!("Failed to parse response: {}", e)))?;

        if let Some(error) = &json_response.error {
            return Err(McpError::JsonRpc(error.code, error.message.clone()));
        }

        Ok(json_response)
    }

    async fn send_notification(&mut self, notification: JsonRpcRequest) -> McpResult<()> {
        self.http_client
            .post(&self.server_url)
            .json(&notification)
            .send()
            .await
            .map_err(|e| McpError::Transport(format!("HTTP notification failed: {}", e)))?;

        Ok(())
    }

    async fn is_available(&self) -> bool {
        let health_url = format!("{}/health", self.server_url);
        self.http_client
            .get(&health_url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn close(&mut self) -> McpResult<()> {
        // HTTP client doesn't need explicit cleanup
        Ok(())
    }
}

/// Stdio Transport implementation (for subprocess-based MCP servers)
pub mod stdio;

/// WebSocket Transport implementation
pub mod websocket;
