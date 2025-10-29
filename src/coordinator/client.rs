//! Coordinator Client Library
//!
//! Provides a high-level async API for LSP and ACP processes to communicate
//! with the MCP Coordinator daemon via Unix socket IPC.
//!
//! ## Usage
//! ```rust,ignore
//! use universal_lsp::coordinator::CoordinatorClient;
//!
//! // Connect to coordinator daemon
//! let client = CoordinatorClient::connect().await?;
//!
//! // Connect to an MCP server
//! let connection_id = client.connect_to_server("smart-tree").await?;
//!
//! // Query MCP server
//! let request = McpRequest {
//!     request_type: "completion".to_string(),
//!     uri: "file:///test.rs".to_string(),
//!     position: Position { line: 10, character: 5 },
//!     context: Some("fn main()".to_string()),
//! };
//! let response = client.query("smart-tree", request).await?;
//!
//! // Get metrics
//! let metrics = client.get_metrics().await?;
//! ```

use crate::coordinator::{
    CoordinatorMetrics, CoordinatorRequest, CoordinatorResponse, IpcMessage, IpcPayload,
};
use crate::mcp::{McpRequest, McpResponse};
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// Default path to coordinator Unix socket
pub const DEFAULT_COORDINATOR_SOCKET: &str = "/tmp/universal-mcp.sock";

/// Coordinator client for IPC communication
pub struct CoordinatorClient {
    socket_path: String,
    next_request_id: AtomicU64,
}

impl CoordinatorClient {
    /// Create a new coordinator client with default socket path
    pub fn new() -> Self {
        Self {
            socket_path: DEFAULT_COORDINATOR_SOCKET.to_string(),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Create a new coordinator client with custom socket path
    pub fn with_socket_path(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Connect to the coordinator and verify availability
    pub async fn connect() -> Result<Self> {
        let client = Self::new();

        // Test connection by getting metrics
        client.get_metrics().await?;

        Ok(client)
    }

    /// Connect to the coordinator with custom socket path
    pub async fn connect_with_path(socket_path: impl Into<String>) -> Result<Self> {
        let client = Self::with_socket_path(socket_path);

        // Test connection
        client.get_metrics().await?;

        Ok(client)
    }

    /// Generate next request ID
    fn next_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a request and receive response
    async fn send_request(&self, request: CoordinatorRequest) -> Result<CoordinatorResponse> {
        // Connect to Unix socket
        let mut stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            anyhow!(
                "Failed to connect to coordinator at {}: {}",
                self.socket_path,
                e
            )
        })?;

        // Create IPC message
        let message = IpcMessage::request(self.next_id(), request);

        // Send message
        let bytes = message.to_bytes()?;
        stream.write_all(&bytes).await?;
        stream.flush().await?;

        // Read response
        let response_message = self.read_message(&mut stream).await?;

        // Extract response payload
        match response_message.payload {
            IpcPayload::Response(response) => Ok(response),
            IpcPayload::Request(_) => Err(anyhow!("Unexpected request from coordinator")),
        }
    }

    /// Read IPC message from stream
    async fn read_message(&self, stream: &mut UnixStream) -> Result<IpcMessage> {
        // Read Content-Length header
        let mut header = String::new();
        let mut buf = [0u8; 1];
        loop {
            stream.read_exact(&mut buf).await?;
            header.push(buf[0] as char);
            if header.ends_with("\r\n\r\n") {
                break;
            }
        }

        // Parse Content-Length
        let content_length = header
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.trim().parse::<usize>().ok())
            .ok_or_else(|| anyhow!("Invalid Content-Length header"))?;

        // Read message body
        let mut body = vec![0u8; content_length];
        stream.read_exact(&mut body).await?;

        let message_str = String::from_utf8(body)?;
        Ok(IpcMessage::from_str(&message_str)?)
    }

    /// Connect to an MCP server through the coordinator
    ///
    /// Returns the connection ID if successful, or reuses existing connection
    pub async fn connect_to_server(&self, server_name: impl Into<String>) -> Result<u64> {
        let request = CoordinatorRequest::Connect {
            server_name: server_name.into(),
        };

        match self.send_request(request).await? {
            CoordinatorResponse::Connected { connection_id } => Ok(connection_id),
            CoordinatorResponse::Error { message } => Err(anyhow!("Connection failed: {}", message)),
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Query an MCP server through the coordinator
    pub async fn query(
        &self,
        server_name: impl Into<String>,
        request: McpRequest,
    ) -> Result<McpResponse> {
        let coord_request = CoordinatorRequest::Query {
            server_name: server_name.into(),
            request,
        };

        match self.send_request(coord_request).await? {
            CoordinatorResponse::QueryResult(response) => Ok(response),
            CoordinatorResponse::Error { message } => Err(anyhow!("Query failed: {}", message)),
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Get cached response from coordinator
    pub async fn get_cache(&self, key: impl Into<String>) -> Result<Option<McpResponse>> {
        let request = CoordinatorRequest::GetCache {
            key: key.into(),
        };

        match self.send_request(request).await? {
            CoordinatorResponse::CacheHit(response) => Ok(Some(response)),
            CoordinatorResponse::CacheMiss => Ok(None),
            CoordinatorResponse::Error { message } => Err(anyhow!("Cache lookup failed: {}", message)),
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Set cached response in coordinator
    pub async fn set_cache(
        &self,
        key: impl Into<String>,
        value: McpResponse,
        ttl_seconds: u64,
    ) -> Result<()> {
        let request = CoordinatorRequest::SetCache {
            key: key.into(),
            value,
            ttl_seconds,
        };

        match self.send_request(request).await? {
            CoordinatorResponse::Ok => Ok(()),
            CoordinatorResponse::Error { message } => Err(anyhow!("Cache set failed: {}", message)),
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    /// Get coordinator metrics
    pub async fn get_metrics(&self) -> Result<CoordinatorMetrics> {
        let request = CoordinatorRequest::GetMetrics;

        match self.send_request(request).await? {
            CoordinatorResponse::Metrics(metrics) => Ok(metrics),
            CoordinatorResponse::Error { message } => Err(anyhow!("Metrics failed: {}", message)),
            other => Err(anyhow!("Unexpected response: {:?}", other)),
        }
    }

    // Note: Disconnect and Shutdown methods will be added once protocol support is implemented
    //
    // /// Disconnect from an MCP server (reduces reference count)
    // pub async fn disconnect(&self, server_name: impl Into<String>) -> Result<()> {
    //     let request = CoordinatorRequest::Disconnect {
    //         server_name: server_name.into(),
    //     };
    //
    //     match self.send_request(request).await? {
    //         CoordinatorResponse::Ok => Ok(()),
    //         CoordinatorResponse::Error { message } => Err(anyhow!("Disconnect failed: {}", message)),
    //         other => Err(anyhow!("Unexpected response: {:?}", other)),
    //     }
    // }
    //
    // /// Shutdown the coordinator daemon gracefully
    // pub async fn shutdown(&self) -> Result<()> {
    //     let request = CoordinatorRequest::Shutdown;
    //
    //     match self.send_request(request).await? {
    //         CoordinatorResponse::Ok => Ok(()),
    //         CoordinatorResponse::Error { message } => Err(anyhow!("Shutdown failed: {}", message)),
    //         other => Err(anyhow!("Unexpected response: {:?}", other)),
    //     }
    // }
}

impl Default for CoordinatorClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CoordinatorClient::new();
        assert_eq!(client.socket_path, DEFAULT_COORDINATOR_SOCKET);
        assert_eq!(client.next_id(), 1);
        assert_eq!(client.next_id(), 2);
    }

    #[test]
    fn test_custom_socket_path() {
        let client = CoordinatorClient::with_socket_path("/tmp/custom.sock");
        assert_eq!(client.socket_path, "/tmp/custom.sock");
    }

    #[tokio::test]
    async fn test_connect_fails_when_coordinator_not_running() {
        let result = CoordinatorClient::connect().await;
        // Should fail since coordinator is not running in test
        assert!(result.is_err());
    }
}
