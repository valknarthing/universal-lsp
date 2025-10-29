//! IPC Protocol for MCP Coordinator
//!
//! Defines message types for communication between:
//! - LSP Process ↔ MCP Coordinator
//! - ACP Process ↔ MCP Coordinator

use crate::mcp::{McpRequest, McpResponse};
use serde::{Deserialize, Serialize};

/// Request types sent to the coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CoordinatorRequest {
    /// Connect to a specific MCP server
    Connect {
        server_name: String,
    },

    /// Query an MCP server
    Query {
        server_name: String,
        request: McpRequest,
    },

    /// Get cached response
    GetCache {
        key: String,
    },

    /// Set cached response with TTL
    SetCache {
        key: String,
        value: McpResponse,
        ttl_seconds: u64,
    },

    /// Get coordinator metrics
    GetMetrics,

    /// Shutdown the coordinator
    Shutdown,
}

/// Response types sent from the coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CoordinatorResponse {
    /// Successfully connected to MCP server
    Connected {
        connection_id: u64,
    },

    /// Query result from MCP server
    QueryResult(McpResponse),

    /// Cache hit with response
    CacheHit(McpResponse),

    /// Cache miss
    CacheMiss,

    /// Coordinator metrics
    Metrics(CoordinatorMetrics),

    /// Error occurred
    Error {
        message: String,
    },

    /// Generic success response
    Ok,
}

/// Metrics exposed by the coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorMetrics {
    pub active_connections: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_queries: u64,
    pub errors: u64,
    pub uptime_seconds: u64,
}

/// IPC Message wrapper with ID for request-response matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: u64,
    pub payload: IpcPayload,
}

/// IPC Payload (either request or response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IpcPayload {
    Request(CoordinatorRequest),
    Response(CoordinatorResponse),
}

impl IpcMessage {
    /// Create a new request message
    pub fn request(id: u64, request: CoordinatorRequest) -> Self {
        Self {
            id,
            payload: IpcPayload::Request(request),
        }
    }

    /// Create a new response message
    pub fn response(id: u64, response: CoordinatorResponse) -> Self {
        Self {
            id,
            payload: IpcPayload::Response(response),
        }
    }

    /// Serialize to JSON bytes with Content-Length header
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());
        Ok(format!("{}{}", header, json).into_bytes())
    }

    /// Deserialize from JSON string
    pub fn from_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::Position;

    #[test]
    fn test_request_serialization() {
        let request = CoordinatorRequest::Connect {
            server_name: "smart-tree".to_string(),
        };
        let msg = IpcMessage::request(1, request);

        let bytes = msg.to_bytes().unwrap();
        let string = String::from_utf8(bytes).unwrap();

        assert!(string.starts_with("Content-Length:"));
        assert!(string.contains("smart-tree"));
    }

    #[test]
    fn test_response_serialization() {
        let response = CoordinatorResponse::Connected { connection_id: 42 };
        let msg = IpcMessage::response(1, response);

        let bytes = msg.to_bytes().unwrap();
        let string = String::from_utf8(bytes).unwrap();

        assert!(string.contains("Connected"));
        assert!(string.contains("42"));
    }

    #[test]
    fn test_query_request() {
        let request = CoordinatorRequest::Query {
            server_name: "smart-tree".to_string(),
            request: McpRequest {
                request_type: "completion".to_string(),
                uri: "file:///test.rs".to_string(),
                position: Position { line: 10, character: 5 },
                context: Some("fn main()".to_string()),
            },
        };
        let msg = IpcMessage::request(2, request);

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(msg.id, deserialized.id);
    }

    #[test]
    fn test_metrics_response() {
        let metrics = CoordinatorMetrics {
            active_connections: 3,
            cache_hits: 100,
            cache_misses: 20,
            total_queries: 120,
            errors: 5,
            uptime_seconds: 3600,
        };
        let response = CoordinatorResponse::Metrics(metrics);
        let msg = IpcMessage::response(3, response);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"active_connections\":3"));
        assert!(json.contains("\"cache_hits\":100"));
    }
}
