# Universal LSP Server - Advanced Architecture

## Overview

The Universal LSP Server now supports three major advanced features:
1. **MCP Pipeline Integration** - AI-powered pre/post-processing of LSP requests/responses
2. **LSP Proxy System** - Forward requests to specialized LSP servers
3. **CLI-Based Configuration** - All settings configurable via command-line arguments

## Architecture Diagram

```
┌─────────────┐
│   Editor    │
│ (VS Code,   │
│  Neovim)    │
└──────┬──────┘
       │ LSP Protocol
       ▼
┌────────────────────────────────────────────────────────────┐
│          Universal LSP Server (universal-lsp)              │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐ │
│  │              Request Pipeline                         │ │
│  │                                                       │ │
│  │  1. Receive Request                                  │ │
│  │  2. MCP Pre-Processing (optional)                    │ │
│  │  3. LSP Proxy OR Local Handling                      │ │
│  │  4. MCP Post-Processing (optional)                   │ │
│  │  5. Return Response                                  │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │ MCP Pipeline │  │  LSP Proxy   │  │ Local Language │  │
│  │   Module     │  │   Module     │  │   Detection    │  │
│  └──────────────┘  └──────────────┘  └────────────────┘  │
└────────────────────────────────────────────────────────────┘
       │                    │                    │
       ▼                    ▼                    ▼
┌─────────────┐      ┌─────────────┐      ┌──────────────┐
│ MCP Server  │      │ Rust        │      │  242+ Lang   │
│ (AI Model)  │      │ Analyzer    │      │  Definitions │
│             │      │             │      │              │
│ - Claude    │      │ Pyright     │      │  - Python    │
│ - GPT-4     │      │             │      │  - JavaScript│
│ - Custom    │      │ tsserver    │      │  - Rust      │
└─────────────┘      └─────────────┘      │  - Go        │
                                           │  - ...       │
                                           └──────────────┘
```

## MCP Pipeline Integration

### Request Flow with MCP

```
Editor Request
     │
     ▼
┌────────────────────────────────────────────┐
│ 1. INCOMING REQUEST                        │
│    - textDocument/completion               │
│    - textDocument/hover                    │
│    - textDocument/definition               │
└────────────────┬───────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────┐
│ 2. MCP PRE-PROCESSING (Optional)           │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │  For each MCP pre-server:            │  │
│  │  - Send request context to MCP       │  │
│  │  - Get AI suggestions/enhancements   │  │
│  │  - Merge results                     │  │
│  └──────────────────────────────────────┘  │
│                                             │
│  Example: Claude adds context-aware hints  │
└────────────────┬───────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────┐
│ 3. REQUEST HANDLING                        │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │ IF LSP Proxy configured for language │  │
│  │ THEN: Forward to proxy LSP server    │  │
│  │ ELSE: Use local language detection   │  │
│  └──────────────────────────────────────┘  │
└────────────────┬───────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────┐
│ 4. MCP POST-PROCESSING (Optional)          │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │  For each MCP post-server:           │  │
│  │  - Send response to MCP              │  │
│  │  - Get AI enhancements               │  │
│  │  - Filter/rank/improve results       │  │
│  └──────────────────────────────────────┘  │
│                                             │
│  Example: GPT-4 ranks completion items    │
└────────────────┬───────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────┐
│ 5. RETURN RESPONSE TO EDITOR               │
└────────────────────────────────────────────┘
```

### MCP Pipeline Configuration

```bash
# Start with MCP pre-processing only
universal-lsp \
  --mcp-pre=http://localhost:3000,http://localhost:3001 \
  --mcp-timeout=5000

# Start with both pre and post-processing
universal-lsp \
  --mcp-pre=http://localhost:3000 \
  --mcp-post=http://localhost:4000 \
  --mcp-cache=true

# Disable MCP caching for development
universal-lsp \
  --mcp-pre=http://localhost:3000 \
  --mcp-cache=false
```

## LSP Proxy System

### How LSP Proxying Works

The LSP proxy system allows universal-lsp to forward requests to specialized LSP servers while adding MCP enhancements.

```
Editor (VS Code)
     │
     ▼
Universal LSP (universal-lsp)
     │
     ├─ Python file?  ──▶  Pyright LSP  ──▶  MCP Post-Process
     │
     ├─ Rust file?    ──▶  rust-analyzer  ──▶  MCP Post-Process
     │
     ├─ TypeScript?   ──▶  tsserver       ──▶  MCP Post-Process
     │
     └─ Other lang?   ──▶  Local handling ──▶  MCP Post-Process
```

### LSP Proxy Configuration

```bash
# Configure proxy servers for specific languages
universal-lsp \
  --lsp-proxy=python=pyright-langserver \
  --lsp-proxy=rust=rust-analyzer \
  --lsp-proxy=typescript=typescript-language-server \
  --lsp-proxy=go=gopls

# Combine with MCP pipeline
universal-lsp \
  --lsp-proxy=python=pyright \
  --lsp-proxy=rust=rust-analyzer \
  --mcp-post=http://localhost:4000 \
  --mcp-timeout=3000
```

### Proxy Fallback Strategy

1. **Check Proxy Configuration**: If language has configured proxy
2. **Forward to Proxy**: Send LSP request to proxy server
3. **Handle Response**: Process proxy server response
4. **MCP Enhancement**: Apply MCP post-processing
5. **Fallback**: If proxy fails, use local handling

## CLI Configuration

### Complete CLI Options

```bash
universal-lsp [OPTIONS]

OPTIONS:
  --log-level <LEVEL>              Log level [default: info]
                                   Values: error, warn, info, debug, trace

  --mcp-pre <URLS>                 MCP pre-processing servers (comma-separated)
                                   Example: --mcp-pre=http://localhost:3000,http://localhost:3001

  --mcp-post <URLS>                MCP post-processing servers (comma-separated)
                                   Example: --mcp-post=http://localhost:4000

  --mcp-timeout <MS>               MCP request timeout in milliseconds [default: 5000]

  --mcp-cache <BOOL>               Enable MCP response caching [default: true]

  --lsp-proxy <MAPPINGS>           LSP proxy servers (format: lang=command)
                                   Example: --lsp-proxy=python=pyright,rust=rust-analyzer

  --max-concurrent <NUM>           Maximum concurrent requests [default: 100]

  --log-requests                   Enable detailed request logging

  --config <PATH>                  Configuration file path (overrides CLI)

  -h, --help                       Print help information
  -V, --version                    Print version information
```

### Configuration File (JSON)

```json
{
  "server": {
    "log_level": "debug",
    "max_concurrent": 200,
    "log_requests": true
  },
  "mcp": {
    "pre_servers": [
      "http://localhost:3000",
      "http://localhost:3001"
    ],
    "post_servers": [
      "http://localhost:4000"
    ],
    "timeout_ms": 5000,
    "enable_cache": true
  },
  "proxy": {
    "servers": {
      "python": "pyright-langserver --stdio",
      "rust": "rust-analyzer",
      "typescript": "typescript-language-server --stdio",
      "go": "gopls"
    }
  }
}
```

Load with:
```bash
universal-lsp --config=config.json
```

## Use Cases

### 1. AI-Enhanced Code Completion

```bash
# Use Claude for pre-processing context
universal-lsp \
  --mcp-pre=http://claude-mcp:3000 \
  --lsp-proxy=python=pyright
```

**Flow**:
1. Editor requests completion at cursor
2. MCP pre-processing sends context to Claude
3. Claude provides relevant documentation/patterns
4. Forward to Pyright for Python-specific completions
5. Merge Claude context with Pyright results
6. Return enhanced completions to editor

### 2. Multi-Model LSP Routing

```bash
# Route different languages to specialized servers
universal-lsp \
  --lsp-proxy=python=pyright \
  --lsp-proxy=rust=rust-analyzer \
  --lsp-proxy=typescript=tsserver \
  --lsp-proxy=go=gopls \
  --mcp-post=http://gpt4-ranker:4000
```

**Benefits**:
- Best-in-class LSP for each language
- Unified MCP post-processing layer
- Consistent AI enhancements across all languages

### 3. Development with Hot-Reloadable MCP

```bash
# Disable caching for MCP development
universal-lsp \
  --mcp-pre=http://localhost:3000 \
  --mcp-cache=false \
  --log-requests \
  --log-level=debug
```

## Implementation Status

### ✅ Completed
- CLI argument parsing with clap
- Configuration module structure
- MCP client architecture
- LSP proxy configuration parsing

### 🚧 In Progress
- MCP HTTP client implementation
- LSP proxy request forwarding
- Response pipeline integration
- Cache implementation

### 📋 Planned
- MCP protocol full implementation
- LSP proxy stdio communication
- Performance optimizations
- Comprehensive testing

## Performance Considerations

### MCP Caching Strategy

```rust
// Pseudo-code for MCP cache
struct McpCache {
    cache: LruCache<RequestHash, Response>,
    ttl: Duration,
}

// Cache key based on:
// - Request type (hover, completion, etc.)
// - File path
// - Cursor position
// - Surrounding context (hash)
```

### Parallel Processing

```rust
// Pseudo-code for parallel MCP requests
async fn process_mcp_pipeline(request: Request) -> Result<Response> {
    // Pre-processing: parallel requests to multiple MCP servers
    let pre_tasks: Vec<_> = config.mcp.pre_servers
        .iter()
        .map(|server| tokio::spawn(mcp_client.request(server, &request)))
        .collect();
    
    let pre_results = futures::future::join_all(pre_tasks).await;
    
    // Merge pre-processing results
    let enhanced_request = merge_mcp_results(request, pre_results);
    
    // Main processing (proxy or local)
    let response = handle_request(enhanced_request).await?;
    
    // Post-processing: parallel requests
    let post_tasks: Vec<_> = config.mcp.post_servers
        .iter()
        .map(|server| tokio::spawn(mcp_client.enhance(server, &response)))
        .collect();
    
    let post_results = futures::future::join_all(post_tasks).await;
    
    // Merge post-processing results
    Ok(merge_mcp_results(response, post_results))
}
```

## Next Steps

1. **Complete MCP HTTP Client**: Implement full MCP protocol communication
2. **LSP Proxy Forwarding**: Implement stdio/TCP communication with proxy servers
3. **Pipeline Integration**: Wire up MCP pipeline in main LSP handlers
4. **Testing**: Add comprehensive integration tests
5. **Documentation**: Add usage examples and tutorials
6. **Benchmarking**: Performance testing with various configurations

## Example Workflows

### Python Development with AI

```bash
# Terminal 1: Start MCP server (Claude)
claude-mcp-server --port 3000

# Terminal 2: Start Universal LSP
universal-lsp \
  --lsp-proxy=python=pyright-langserver \
  --mcp-pre=http://localhost:3000 \
  --log-level=debug
```

### Multi-Language Project

```bash
universal-lsp \
  --lsp-proxy=python=pyright \
  --lsp-proxy=rust=rust-analyzer \
  --lsp-proxy=typescript=tsserver \
  --lsp-proxy=go=gopls \
  --mcp-pre=http://context-analyzer:3000 \
  --mcp-post=http://result-ranker:4000 \
  --max-concurrent=200
```

