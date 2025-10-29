//! MCP Coordinator Module
//!
//! Shared daemon for managing MCP server connections across LSP and ACP processes.
//! Provides Unix socket IPC with connection pooling and response caching.

pub mod cache;
pub mod client;
pub mod pool;
pub mod protocol;

pub use cache::ResponseCache;
pub use client::{CoordinatorClient, DEFAULT_COORDINATOR_SOCKET};
pub use pool::ConnectionPool;
pub use protocol::{
    CoordinatorMetrics, CoordinatorRequest, CoordinatorResponse, IpcMessage, IpcPayload,
};

use crate::config::{Config, McpServerConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

/// MCP Coordinator Server
pub struct Coordinator {
    /// Connection pool for MCP servers
    pool: Arc<ConnectionPool>,

    /// Response cache
    cache: Arc<ResponseCache>,

    /// Start time for uptime calculation
    start_time: Instant,

    /// Total query counter
    total_queries: std::sync::atomic::AtomicU64,

    /// Error counter
    errors: std::sync::atomic::AtomicU64,

    /// MCP server configurations
    server_configs: std::collections::HashMap<String, McpServerConfig>,

    /// Unix socket path
    socket_path: PathBuf,
}

impl Coordinator {
    /// Create a new coordinator from configuration
    pub fn new(config: &Config) -> Self {
        let cache_ttl = 300; // 5 minutes default
        let timeout_ms = config.mcp.timeout_ms;

        Self {
            pool: Arc::new(ConnectionPool::new(timeout_ms)),
            cache: Arc::new(ResponseCache::new(cache_ttl)),
            start_time: Instant::now(),
            total_queries: std::sync::atomic::AtomicU64::new(0),
            errors: std::sync::atomic::AtomicU64::new(0),
            server_configs: config.mcp.servers.clone(),
            socket_path: PathBuf::from("/tmp/universal-mcp.sock"),
        }
    }

    /// Run the coordinator server
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        // Remove existing socket if it exists
        if self.socket_path.exists() {
            tokio::fs::remove_file(&self.socket_path).await?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        log::info!("MCP Coordinator listening on {:?}", self.socket_path);

        // Spawn cache cleanup task
        let cache_clone = Arc::clone(&self.cache);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                cache_clone.cleanup_expired();
            }
        });

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let coord = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(e) = coord.handle_client(stream).await {
                            log::error!("Client handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Accept error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a client connection
    async fn handle_client(&self, mut stream: UnixStream) -> anyhow::Result<()> {
        loop {
            // Read Content-Length header
            let mut header = String::new();
            let mut buf = [0u8; 1];

            loop {
                let n = stream.read(&mut buf).await?;
                if n == 0 {
                    return Ok(()); // Connection closed
                }

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
                .ok_or_else(|| anyhow::anyhow!("Invalid Content-Length header"))?;

            // Read message body
            let mut body = vec![0u8; content_length];
            stream.read_exact(&mut body).await?;

            let message_str = String::from_utf8(body)?;
            let message: IpcMessage = serde_json::from_str(&message_str)?;

            // Handle request
            let response = self.handle_request(message.id, &message.payload).await;

            // Send response
            let response_bytes = response.to_bytes()?;
            stream.write_all(&response_bytes).await?;
            stream.flush().await?;
        }
    }

    /// Handle an IPC request
    async fn handle_request(&self, id: u64, payload: &IpcPayload) -> IpcMessage {
        match payload {
            IpcPayload::Request(req) => {
                let response = match req {
                    CoordinatorRequest::Connect { server_name } => {
                        self.handle_connect(server_name).await
                    }
                    CoordinatorRequest::Query { server_name, request } => {
                        self.handle_query(server_name, request).await
                    }
                    CoordinatorRequest::GetCache { key } => self.handle_get_cache(key),
                    CoordinatorRequest::SetCache { key, value, ttl_seconds } => {
                        self.handle_set_cache(key, value, *ttl_seconds)
                    }
                    CoordinatorRequest::GetMetrics => self.handle_get_metrics(),
                    CoordinatorRequest::Shutdown => {
                        log::info!("Shutdown requested");
                        CoordinatorResponse::Ok
                    }
                };

                IpcMessage::response(id, response)
            }
            IpcPayload::Response(_) => {
                // We shouldn't receive responses as a server
                IpcMessage::response(
                    id,
                    CoordinatorResponse::Error {
                        message: "Unexpected response from client".to_string(),
                    },
                )
            }
        }
    }

    /// Handle connect request
    async fn handle_connect(&self, server_name: &str) -> CoordinatorResponse {
        let server_config = match self.server_configs.get(server_name) {
            Some(config) => config,
            None => {
                return CoordinatorResponse::Error {
                    message: format!("Unknown server: {}", server_name),
                }
            }
        };

        match self.pool.get_or_create(server_name, server_config).await {
            Ok((_, connection_id)) => {
                log::info!("Connected to MCP server '{}' (ID: {})", server_name, connection_id);
                CoordinatorResponse::Connected { connection_id }
            }
            Err(e) => {
                self.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                CoordinatorResponse::Error { message: e }
            }
        }
    }

    /// Handle query request
    async fn handle_query(
        &self,
        server_name: &str,
        request: &crate::mcp::McpRequest,
    ) -> CoordinatorResponse {
        self.total_queries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Check cache first
        let cache_key = ResponseCache::make_key(server_name, request);
        if let Some(cached) = self.cache.get(&cache_key) {
            log::debug!("Cache hit for {}", cache_key);
            return CoordinatorResponse::QueryResult(cached);
        }

        // Get or create connection
        let server_config = match self.server_configs.get(server_name) {
            Some(config) => config,
            None => {
                self.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return CoordinatorResponse::Error {
                    message: format!("Unknown server: {}", server_name),
                };
            }
        };

        let (client, _) = match self.pool.get_or_create(server_name, server_config).await {
            Ok(result) => result,
            Err(e) => {
                self.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return CoordinatorResponse::Error { message: e };
            }
        };

        // Query MCP server
        match client.query(request).await {
            Ok(response) => {
                // Cache the response
                self.cache.set(cache_key, response.clone(), None);
                CoordinatorResponse::QueryResult(response)
            }
            Err(e) => {
                self.errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                CoordinatorResponse::Error {
                    message: format!("MCP query failed: {}", e),
                }
            }
        }
    }

    /// Handle get cache request
    fn handle_get_cache(&self, key: &str) -> CoordinatorResponse {
        match self.cache.get(key) {
            Some(response) => CoordinatorResponse::CacheHit(response),
            None => CoordinatorResponse::CacheMiss,
        }
    }

    /// Handle set cache request
    fn handle_set_cache(
        &self,
        key: &str,
        value: &crate::mcp::McpResponse,
        ttl_seconds: u64,
    ) -> CoordinatorResponse {
        self.cache.set(key.to_string(), value.clone(), Some(ttl_seconds));
        CoordinatorResponse::Ok
    }

    /// Handle get metrics request
    fn handle_get_metrics(&self) -> CoordinatorResponse {
        let metrics = CoordinatorMetrics {
            active_connections: self.pool.active_connections(),
            cache_hits: self.cache.hits(),
            cache_misses: self.cache.misses(),
            total_queries: self.total_queries.load(std::sync::atomic::Ordering::Relaxed),
            errors: self.errors.load(std::sync::atomic::Ordering::Relaxed),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        };

        CoordinatorResponse::Metrics(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let config = Config {
            server: crate::config::ServerConfig {
                log_level: "info".to_string(),
                max_concurrent: 100,
                log_requests: false,
            },
            mcp: crate::config::McpConfig {
                servers: std::collections::HashMap::new(),
                timeout_ms: 5000,
                enable_cache: true,
            },
            proxy: crate::config::ProxyConfig {
                servers: std::collections::HashMap::new(),
            },
        };

        let coordinator = Coordinator::new(&config);
        assert_eq!(coordinator.pool.active_connections(), 0);
    }
}
