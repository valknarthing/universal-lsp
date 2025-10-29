# Dual-Protocol Architecture: LSP + ACP with Shared MCP

## Overview

UniversalLSP will support both **LSP** (Language Server Protocol) and **ACP** (Agent Client Protocol) simultaneously, sharing MCP server connections and their memory state between both protocols.

## Challenge

- IDEs spawn **separate processes** for LSP and ACP communication
- Both processes need to access the **same MCP server instances**
- MCP servers maintain stateful connections and conversation history
- We cannot run a single process for both protocols

## Solution: Shared Connection Pool Architecture

### Component Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         IDE (Zed/VSCode)                         │
├────────────────────────┬───────────────────────────────────────┤
│  LSP Client (stdio)    │         ACP Client (stdio)            │
└────────────┬───────────┴──────────────────┬────────────────────┘
             │                              │
             │  JSON-RPC 2.0                │  JSON-RPC 2.0
             │  over stdio                  │  over stdio
             │                              │
    ┌────────▼────────┐            ┌────────▼────────┐
    │  LSP Process    │            │  ACP Process    │
    │ universal-lsp   │            │ universal-acp   │
    │                 │            │                 │
    │ Features:       │            │ Features:       │
    │ • Hover         │            │ • Agent tasks   │
    │ • Completion    │            │ • Diffs         │
    │ • Diagnostics   │            │ • Code mods     │
    │ • Symbols       │            │ • Markdown UI   │
    └────────┬────────┘            └────────┬────────┘
             │                              │
             │  IPC: Unix Socket            │
             │  /tmp/universal-mcp.sock     │
             │                              │
             └──────────────┬───────────────┘
                            │
                   ┌────────▼────────┐
                   │  MCP Coordinator │
                   │   (Daemon)       │
                   │                  │
                   │  Manages:        │
                   │  • Connections   │
                   │  • State cache   │
                   │  • Load balance  │
                   └────────┬─────────┘
                            │
           ┌────────────────┼────────────────┐
           │                │                │
      ┌────▼───┐       ┌────▼───┐      ┌────▼───┐
      │smart-  │       │in-     │      │File    │
      │tree    │       │memoria │      │ScopeMCP│
      │(stdio) │       │(stdio) │      │(http)  │
      └────────┘       └────────┘      └────────┘
```

### Process Responsibilities

#### 1. MCP Coordinator (Daemon)

**Purpose**: Single source of truth for all MCP connections

**Lifecycle**:
- Starts when first client (LSP or ACP) connects
- Runs in background
- Shuts down when idle for >5 minutes

**Responsibilities**:
- Spawn and manage MCP server subprocesses
- Maintain connection pool with reference counting
- Cache MCP responses with TTL
- Load balance requests across multiple clients
- Handle reconnection on MCP server crashes

**API** (via Unix Domain Socket):
```rust
// Request types
enum CoordinatorRequest {
    Connect { server_name: String },
    Query { server_name: String, request: McpRequest },
    GetCache { key: String },
    SetCache { key: String, value: McpResponse, ttl: u64 },
    Shutdown,
}

// Response types
enum CoordinatorResponse {
    Connected { connection_id: u64 },
    QueryResult(McpResponse),
    CacheHit(McpResponse),
    CacheMiss,
    Error(String),
}
```

#### 2. LSP Process (`universal-lsp`)

**Current Features** (already implemented):
- Hover information (with MCP context)
- Code completion (tree-sitter + grammar + AI)
- Diagnostics
- Document symbols
- Go to definition/references
- Formatting

**MCP Integration**:
- Connects to coordinator via `/tmp/universal-mcp.sock`
- Queries MCP servers for additional context
- Enhances LSP responses with MCP data

**Example Flow**:
```
IDE → LSP: textDocument/hover
LSP → Coordinator: Query(smart-tree, "get_symbol_docs")
Coordinator → smart-tree MCP
smart-tree → Coordinator: Response
Coordinator → LSP: CachedResponse
LSP → IDE: Hover with MCP-enhanced docs
```

#### 3. ACP Process (`universal-acp`)

**Purpose**: Handle AI agent requests

**Features**:
- Execute agent tasks (code generation, refactoring)
- Display diffs in Markdown format
- Apply code modifications
- Interactive multi-step workflows
- Progress reporting

**MCP Integration**:
- Same coordinator connection as LSP
- Can request MCP context for agent tasks
- Shares cached responses with LSP

**ACP Methods** (from spec):
```
- acp/getTask - Get current agent task
- acp/sendMessage - Send message to agent
- acp/applyDiff - Apply code changes
- acp/getProgress - Get task progress
- acp/cancel - Cancel running task
```

**Example Flow**:
```
IDE → ACP: acp/sendMessage("Refactor this function")
ACP → Coordinator: Query(in-memoria, "get_refactoring_context")
ACP → Claude API: Generate refactoring with MCP context
ACP → IDE: Diff in Markdown format
IDE → ACP: acp/applyDiff
ACP → IDE: Applied successfully
```

### Shared State Management

#### Cache Strategy

```rust
// In MCP Coordinator
struct ResponseCache {
    // Key: (server_name, request_hash)
    // Value: (response, expiry_timestamp)
    cache: DashMap<String, (McpResponse, Instant)>,
    ttl: Duration,
}

impl ResponseCache {
    fn get(&self, key: &str) -> Option<McpResponse> {
        self.cache.get(key)
            .filter(|(_, expiry)| expiry > &Instant::now())
            .map(|(response, _)| response.clone())
    }

    fn set(&self, key: String, response: McpResponse) {
        let expiry = Instant::now() + self.ttl;
        self.cache.insert(key, (response, expiry));
    }

    // Periodic cleanup
    fn cleanup_expired(&self) {
        let now = Instant::now();
        self.cache.retain(|_, (_, expiry)| expiry > &now);
    }
}
```

#### Connection Pool

```rust
// In MCP Coordinator
struct ConnectionPool {
    connections: DashMap<String, Arc<McpClient>>,
    ref_counts: DashMap<String, AtomicUsize>,
}

impl ConnectionPool {
    async fn get_or_create(&self, server_name: &str) -> Arc<McpClient> {
        if let Some(client) = self.connections.get(server_name) {
            self.ref_counts.get(server_name).unwrap().fetch_add(1, Ordering::Relaxed);
            return client.clone();
        }

        // Create new connection
        let client = Arc::new(self.spawn_mcp_server(server_name).await);
        self.connections.insert(server_name.to_string(), client.clone());
        self.ref_counts.insert(server_name.to_string(), AtomicUsize::new(1));
        client
    }

    async fn release(&self, server_name: &str) {
        if let Some(count) = self.ref_counts.get(server_name) {
            if count.fetch_sub(1, Ordering::Relaxed) == 1 {
                // Last reference, can close connection
                self.connections.remove(server_name);
                self.ref_counts.remove(server_name);
            }
        }
    }
}
```

### IPC Protocol

#### Unix Domain Socket Communication

```rust
// Message format over socket
#[derive(Serialize, Deserialize)]
struct IpcMessage {
    id: u64,
    payload: IpcPayload,
}

#[derive(Serialize, Deserialize)]
enum IpcPayload {
    Request(CoordinatorRequest),
    Response(CoordinatorResponse),
}

// Framing: Content-Length header (same as LSP)
// Example:
// Content-Length: 123\r\n
// \r\n
// {"id": 1, "payload": {...}}
```

#### Client Implementation

```rust
// In LSP and ACP processes
struct CoordinatorClient {
    socket: UnixStream,
    next_id: AtomicU64,
}

impl CoordinatorClient {
    async fn connect() -> Result<Self> {
        let socket = UnixStream::connect("/tmp/universal-mcp.sock").await?;
        Ok(Self { socket, next_id: AtomicU64::new(1) })
    }

    async fn query(&self, server: &str, request: McpRequest) -> Result<McpResponse> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = IpcMessage {
            id,
            payload: IpcPayload::Request(CoordinatorRequest::Query {
                server_name: server.to_string(),
                request,
            }),
        };

        self.send_message(&msg).await?;
        let response = self.receive_message().await?;

        match response.payload {
            IpcPayload::Response(CoordinatorResponse::QueryResult(r)) => Ok(r),
            _ => Err(anyhow!("Unexpected response")),
        }
    }
}
```

### Configuration

#### CLI Arguments

```bash
# Start LSP server (connects to coordinator)
universal-lsp --mcp-server=smart-tree=smart-tree,in-memoria=npx -y @pi22by7/in-memoria

# Start ACP server (connects to same coordinator)
universal-acp --mcp-server=smart-tree=smart-tree,in-memoria=npx -y @pi22by7/in-memoria

# Start coordinator manually (usually auto-started)
universal-mcp-coordinator --socket=/tmp/universal-mcp.sock
```

#### Shared Configuration File

```toml
# ~/.config/universal-lsp/config.toml

[coordinator]
socket_path = "/tmp/universal-mcp.sock"
cache_ttl_seconds = 300
idle_shutdown_seconds = 300

[[mcp.servers]]
name = "smart-tree"
command = "smart-tree"
args = []
transport = "stdio"

[[mcp.servers]]
name = "in-memoria"
command = "npx"
args = ["-y", "@pi22by7/in-memoria"]
transport = "stdio"

[[mcp.servers]]
name = "remote"
url = "http://localhost:3000"
transport = "http"
```

### Benefits

1. **Single MCP Connection**: Each MCP server runs only once
2. **Shared Memory**: Both LSP and ACP see same conversation history
3. **Performance**: Cached responses, no duplicate requests
4. **Reliability**: Coordinator handles reconnection, error recovery
5. **Scalability**: Easy to add more protocol handlers
6. **Debugging**: Single point to monitor all MCP traffic

### Implementation Phases

**Phase 4.1**: MCP Coordinator (Current)
- Create `src/coordinator/` module
- Implement Unix socket server
- Connection pool management
- Response caching

**Phase 4.2**: Refactor LSP to use Coordinator
- Replace direct MCP calls with IPC
- Connect to coordinator socket
- Maintain backward compatibility

**Phase 4.3**: Implement ACP Protocol
- Create `universal-acp` binary
- ACP protocol handler
- Agent task execution
- Diff rendering

**Phase 4.4**: Integration Testing
- Test LSP + ACP simultaneously
- Verify state sharing
- Performance benchmarks

### Testing Strategy

```bash
# Terminal 1: Start coordinator
universal-mcp-coordinator --log-level=debug

# Terminal 2: Start LSP
universal-lsp --log-level=debug

# Terminal 3: Start ACP
universal-acp --log-level=debug

# Terminal 4: Test IDE interaction
zed . # Opens with both LSP and ACP active

# Verify shared state:
# 1. Hover in LSP (triggers MCP query, cached)
# 2. Agent task in ACP (uses same cached MCP response)
# 3. Monitor coordinator logs for cache hits
```

### Security Considerations

1. **Unix Socket Permissions**: 0600 (owner only)
2. **Socket Location**: `/tmp/universal-mcp-$UID.sock` (per-user)
3. **Authentication**: Check connecting process UID
4. **Rate Limiting**: Prevent DOS on coordinator
5. **Resource Limits**: Max connections, max cache size

### Monitoring & Observability

```rust
// Coordinator exposes metrics endpoint
struct CoordinatorMetrics {
    active_connections: AtomicUsize,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    mcp_queries: HashMap<String, AtomicU64>,
    errors: AtomicU64,
}

// Query via special IPC message
// CoordinatorRequest::GetMetrics -> CoordinatorResponse::Metrics(...)
```

## Next Steps

1. **Design Review**: Get feedback on architecture
2. **Prototype Coordinator**: Basic Unix socket server
3. **Refactor LSP**: Extract MCP logic to coordinator client
4. **Implement ACP**: New binary with agent protocol
5. **Integration Test**: Both protocols active simultaneously
6. **Documentation**: API docs, deployment guide
