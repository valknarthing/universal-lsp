//! Stdio Transport for MCP
//!
//! Implements communication with subprocess-based MCP servers via stdin/stdout.
//! This is the transport used by smart-tree and In-Memoria.

use crate::mcp::error::{McpError, McpResult};
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::transport::McpTransport;
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::time::Duration;

/// Stdio Transport for subprocess-based MCP servers
pub struct StdioTransport {
    process: Mutex<Option<Child>>,
    command: String,
    args: Vec<String>,
    timeout: Duration,
    /// Request ID counter
    next_id: Mutex<i64>,
}

impl StdioTransport {
    /// Create a new Stdio transport
    pub fn new(command: String, args: Vec<String>, timeout: Duration) -> Self {
        Self {
            process: Mutex::new(None),
            command,
            args,
            timeout,
            next_id: Mutex::new(1),
        }
    }

    /// Start the MCP server process
    pub async fn start(&self) -> McpResult<()> {
        let mut process_guard = self.process.lock().await;

        if process_guard.is_some() {
            return Ok(()); // Already running
        }

        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                McpError::Transport(format!(
                    "Failed to spawn MCP server '{}': {}",
                    self.command, e
                ))
            })?;

        // Verify process started successfully
        tokio::time::sleep(Duration::from_millis(100)).await;

        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(McpError::Transport(format!(
                    "MCP server exited immediately with status: {}",
                    status
                )));
            }
            Ok(None) => {
                // Still running, good
            }
            Err(e) => {
                return Err(McpError::Transport(format!(
                    "Failed to check process status: {}",
                    e
                )));
            }
        }

        *process_guard = Some(child);
        Ok(())
    }

    /// Get next request ID
    async fn next_request_id(&self) -> i64 {
        let mut id = self.next_id.lock().await;
        let current = *id;
        *id += 1;
        current
    }

    /// Send JSON-RPC message to stdin
    async fn write_message(&self, message: &JsonRpcRequest) -> McpResult<()> {
        let mut process_guard = self.process.lock().await;

        let process = process_guard
            .as_mut()
            .ok_or_else(|| McpError::Transport("Process not started".to_string()))?;

        let stdin = process
            .stdin
            .as_mut()
            .ok_or_else(|| McpError::Transport("No stdin available".to_string()))?;

        let json_str = serde_json::to_string(message)
            .map_err(|e| McpError::Transport(format!("Failed to serialize request: {}", e)))?;

        let content_length = json_str.len();
        let lsp_message = format!("Content-Length: {}\r\n\r\n{}", content_length, json_str);

        stdin
            .write_all(lsp_message.as_bytes())
            .await
            .map_err(|e| McpError::Transport(format!("Failed to write to stdin: {}", e)))?;

        stdin
            .flush()
            .await
            .map_err(|e| McpError::Transport(format!("Failed to flush stdin: {}", e)))?;

        Ok(())
    }

    /// Read JSON-RPC message from stdout
    async fn read_message(&self) -> McpResult<JsonRpcResponse> {
        let mut process_guard = self.process.lock().await;

        let process = process_guard
            .as_mut()
            .ok_or_else(|| McpError::Transport("Process not started".to_string()))?;

        let stdout = process
            .stdout
            .as_mut()
            .ok_or_else(|| McpError::Transport("No stdout available".to_string()))?;

        let mut reader = BufReader::new(stdout);
        let mut content_length = 0;

        // Read headers
        loop {
            let mut line = String::new();
            let bytes_read = reader
                .read_line(&mut line)
                .await
                .map_err(|e| McpError::Transport(format!("Failed to read from stdout: {}", e)))?;

            if bytes_read == 0 {
                return Err(McpError::Transport("Process closed stdout".to_string()));
            }

            if line == "\r\n" || line == "\n" {
                break; // End of headers
            }

            if line.starts_with("Content-Length:") {
                let len_str = line.trim_start_matches("Content-Length:").trim();
                content_length = len_str
                    .parse()
                    .map_err(|e| McpError::Protocol(format!("Invalid Content-Length: {}", e)))?;
            }
        }

        if content_length == 0 {
            return Err(McpError::Protocol("Missing Content-Length header".to_string()));
        }

        // Read body
        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut reader, &mut body)
            .await
            .map_err(|e| McpError::Transport(format!("Failed to read response body: {}", e)))?;

        let response: JsonRpcResponse = serde_json::from_slice(&body)
            .map_err(|e| McpError::Protocol(format!("Failed to parse JSON-RPC response: {}", e)))?;

        if let Some(error) = &response.error {
            return Err(McpError::JsonRpc(error.code, error.message.clone()));
        }

        Ok(response)
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&mut self, mut request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        // Ensure process is started
        self.start().await?;

        // Assign ID if not present
        if request.id.is_none() {
            request.id = Some(Value::Number(self.next_request_id().await.into()));
        }

        // Send request
        self.write_message(&request).await?;

        // Read response with timeout
        let response = tokio::time::timeout(self.timeout, self.read_message())
            .await
            .map_err(|_| McpError::Timeout(self.timeout.as_millis() as u64))??;

        Ok(response)
    }

    async fn send_notification(&mut self, notification: JsonRpcRequest) -> McpResult<()> {
        // Ensure process is started
        self.start().await?;

        // Write notification (no response expected)
        self.write_message(&notification).await?;

        Ok(())
    }

    async fn is_available(&self) -> bool {
        let process_guard = self.process.lock().await;

        if let Some(process) = process_guard.as_ref() {
            // Check if process is still running
            match process.id() {
                Some(_) => true,
                None => false,
            }
        } else {
            false
        }
    }

    async fn close(&mut self) -> McpResult<()> {
        let mut process_guard = self.process.lock().await;

        if let Some(mut child) = process_guard.take() {
            // Send graceful shutdown
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(b"exit\n").await;
                let _ = stdin.flush().await;
            }

            // Wait for process to exit
            tokio::select! {
                _ = child.wait() => {}
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    // Force kill if it doesn't exit
                    let _ = child.kill().await;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stdio_transport_creation() {
        let transport = StdioTransport::new(
            "echo".to_string(),
            vec!["test".to_string()],
            Duration::from_secs(5),
        );

        assert!(!transport.is_available().await);
    }

    #[tokio::test]
    async fn test_stdio_transport_invalid_command() {
        let transport = StdioTransport::new(
            "non_existent_command_12345".to_string(),
            vec![],
            Duration::from_secs(5),
        );

        let result = transport.start().await;
        assert!(result.is_err());
    }
}
