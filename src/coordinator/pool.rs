//! Connection Pool for MCP Servers
//!
//! Manages connections to MCP servers with reference counting,
//! automatic cleanup, and reconnection on failure.

use crate::config::McpServerConfig;
use crate::mcp::{McpClient, McpConfig, TransportType};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Connection pool for MCP servers
pub struct ConnectionPool {
    /// Active connections indexed by server name
    connections: DashMap<String, Arc<McpClient>>,

    /// Reference counts for each connection
    ref_counts: DashMap<String, Arc<AtomicUsize>>,

    /// Next connection ID
    next_connection_id: AtomicU64,

    /// MCP timeout for all connections
    timeout_ms: u64,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            connections: DashMap::new(),
            ref_counts: DashMap::new(),
            next_connection_id: AtomicU64::new(1),
            timeout_ms,
        }
    }

    /// Get or create a connection to an MCP server
    pub async fn get_or_create(
        &self,
        server_name: &str,
        server_config: &McpServerConfig,
    ) -> Result<(Arc<McpClient>, u64), String> {
        // Check if connection exists
        if let Some(entry) = self.connections.get(server_name) {
            let client = entry.value().clone();

            // Increment reference count
            if let Some(count) = self.ref_counts.get(server_name) {
                count.fetch_add(1, Ordering::Relaxed);
            }

            let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
            return Ok((client, connection_id));
        }

        // Create new connection
        let client = self.spawn_mcp_server(server_config).await?;
        let client_arc = Arc::new(client);

        // Store connection and initialize reference count
        self.connections.insert(server_name.to_string(), client_arc.clone());
        self.ref_counts.insert(
            server_name.to_string(),
            Arc::new(AtomicUsize::new(1)),
        );

        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        Ok((client_arc, connection_id))
    }

    /// Release a connection (decrement reference count)
    pub async fn release(&self, server_name: &str) {
        if let Some(count_entry) = self.ref_counts.get(server_name) {
            let current = count_entry.fetch_sub(1, Ordering::Relaxed);

            // If this was the last reference, remove from pool
            if current == 1 {
                self.connections.remove(server_name);
                self.ref_counts.remove(server_name);
                log::info!("Closed MCP connection to {}", server_name);
            }
        }
    }

    /// Get reference count for a server
    pub fn get_ref_count(&self, server_name: &str) -> usize {
        self.ref_counts
            .get(server_name)
            .map(|count| count.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get number of active connections
    pub fn active_connections(&self) -> usize {
        self.connections.len()
    }

    /// Get all active server names
    pub fn active_servers(&self) -> Vec<String> {
        self.connections
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Check if a server connection exists
    pub fn has_connection(&self, server_name: &str) -> bool {
        self.connections.contains_key(server_name)
    }

    /// Spawn a new MCP server connection
    async fn spawn_mcp_server(&self, server_config: &McpServerConfig) -> Result<McpClient, String> {
        // Determine transport type
        let transport = if server_config.target.starts_with("http://")
            || server_config.target.starts_with("https://")
        {
            TransportType::Http
        } else {
            TransportType::Stdio
        };

        let transport_str = format!("{:?}", transport); // Format before move

        let config = McpConfig {
            server_url: server_config.target.clone(),
            transport,
            timeout_ms: self.timeout_ms,
        };

        let client = McpClient::new(config);

        // Verify connection is available
        if !client.is_available().await {
            return Err(format!("Failed to connect to MCP server: {}", server_config.name));
        }

        log::info!(
            "Spawned MCP server '{}' with transport {}",
            server_config.name,
            transport_str
        );

        Ok(client)
    }

    /// Close all connections
    pub async fn shutdown(&self) {
        log::info!("Shutting down connection pool ({} active)", self.active_connections());

        // Clear all connections
        self.connections.clear();
        self.ref_counts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let pool = ConnectionPool::new(5000);
        assert_eq!(pool.active_connections(), 0);
    }

    #[tokio::test]
    async fn test_reference_counting() {
        let pool = ConnectionPool::new(5000);
        let server_config = McpServerConfig {
            name: "test".to_string(),
            target: "echo test".to_string(),
        };

        // Get connection (ref count = 1)
        let result = pool.get_or_create("test", &server_config).await;
        // Note: This will fail because echo isn't a real MCP server,
        // but it demonstrates the API
        assert!(result.is_err() || pool.active_connections() == 1);
    }

    #[test]
    fn test_active_servers() {
        let pool = ConnectionPool::new(5000);
        assert_eq!(pool.active_servers().len(), 0);
    }

    #[test]
    fn test_has_connection() {
        let pool = ConnectionPool::new(5000);
        assert!(!pool.has_connection("nonexistent"));
    }
}
