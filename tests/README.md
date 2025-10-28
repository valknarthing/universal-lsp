# Universal LSP Integration Test Suite

Comprehensive integration tests for Universal LSP Server with mocked actors for all integration scenarios.

## Overview

This test suite provides **complete integration testing** for the four main integration guides:

1. **VSCode Integration** (`integration_vscode_test.rs`)
2. **Zed Editor Integration** (`integration_zed_test.rs`)
3. **Terminal/CLI Integration** (`integration_terminal_test.rs`)
4. **SvelteKit + Tailwind Stack** (`integration_svelte_test.rs`)

Each test suite mocks all external actors (MCP servers, editors, language servers) to enable **deterministic, reproducible testing** without external dependencies.

## Architecture

### Mocked Actors

#### 1. Mock MCP Servers
- **HTTP Server**: warp-based mock servers on localhost
- **Language Detection**: Automatic language detection from file URIs
- **Context-Aware Responses**: Different suggestions for completion vs hover
- **Request Tracking**: Statistics and request counting
- **Health Endpoints**: `/health` for monitoring

#### 2. Mock LSP Client
- **Stdio Communication**: Full LSP protocol over stdin/stdout
- **JSON-RPC 2.0**: Proper message framing with Content-Length headers
- **Request/Response Cycle**: Async request handling
- **Initialization Sequence**: Proper initialize → initialized flow

#### 3. Mock Language Servers
- **Proxy Configuration**: Test with/without language-specific proxies
- **Multi-Language Support**: Python, TypeScript, Rust, Svelte, etc.
- **Configuration Testing**: Various CLI argument combinations

## Running Tests

### Prerequisites

```bash
# Build Universal LSP first
cargo build --release

# Ensure binary is at expected path
ls -lh target/release/universal-lsp
```

### Run All Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run with output
cargo test --test '*' -- --nocapture

# Run specific suite
cargo test --test integration_vscode_test
cargo test --test integration_zed_test
cargo test --test integration_terminal_test
cargo test --test integration_svelte_test
```

### Run Individual Tests

```bash
# VSCode tests
cargo test --test integration_vscode_test test_vscode_initialization
cargo test --test integration_vscode_test test_vscode_completion_with_mcp

# Zed tests
cargo test --test integration_zed_test test_zed_python_completion
cargo test --test integration_zed_test test_zed_performance_target

# Terminal tests
cargo test --test integration_terminal_test test_terminal_mcp_health_check
cargo test --test integration_terminal_test test_terminal_batch_completions

# SvelteKit tests
cargo test --test integration_svelte_test test_svelte_component_completion
cargo test --test integration_svelte_test test_fullstack_performance
```

## Test Coverage

### 1. VSCode Integration Tests (`integration_vscode_test.rs`)

| Test | What It Tests |
|------|---------------|
| `test_vscode_initialization` | LSP initialization with VSCode capabilities |
| `test_vscode_completion_with_mcp` | MCP-enhanced completions |
| `test_vscode_hover_with_mcp` | AI-powered hover documentation |
| `test_vscode_mcp_fallback` | Graceful degradation when MCP unavailable |
| `test_vscode_multiple_languages` | Python, TypeScript, Rust support |
| `test_vscode_performance` | Latency under 500ms |

**Mocked Actors**:
- Mock MCP Server (port 3001-3005)
- VSCode LSP Client
- TypeScript Language Server (conceptually)

### 2. Zed Integration Tests (`integration_zed_test.rs`)

| Test | What It Tests |
|------|---------------|
| `test_zed_basic_initialization` | Zed-specific capabilities |
| `test_zed_python_completion` | Python-specific suggestions |
| `test_zed_rust_completion` | Rust-specific suggestions |
| `test_zed_svelte_completion` | Svelte component completions |
| `test_zed_with_proxies` | LSP proxy configuration |
| `test_zed_multi_language_session` | Multi-language project support |
| `test_zed_concurrent_requests` | Rapid typing simulation |
| `test_zed_performance_target` | P95 latency < 100ms |

**Mocked Actors**:
- Multi-Language MCP Server (port 4001-4009)
- Zed LSP Client with workspace folders
- pyright, rust-analyzer, tsserver (via proxy config)

### 3. Terminal/CLI Integration Tests (`integration_terminal_test.rs`)

| Test | What It Tests |
|------|---------------|
| `test_terminal_mcp_health_check` | Direct curl health endpoint |
| `test_terminal_mcp_completion_request` | HTTP POST completion request |
| `test_terminal_lsp_stdio_protocol` | LSP message framing |
| `test_terminal_batch_completions` | Batch file processing |
| `test_terminal_performance_measurement` | CLI latency metrics |
| `test_terminal_error_handling` | Graceful failure handling |
| `test_terminal_json_rpc_protocol` | JSON-RPC 2.0 compliance |
| `test_terminal_systemd_like_daemon` | Long-running daemon mode |
| `test_terminal_streaming_requests` | Rapid request streaming |

**Mocked Actors**:
- CLI MCP Server with request counting (port 5001-5008)
- Direct subprocess LSP instances
- curl for HTTP testing

### 4. SvelteKit + Tailwind Integration Tests (`integration_svelte_test.rs`)

| Test | What It Tests |
|------|---------------|
| `test_fullstack_initialization` | Full-stack project setup |
| `test_svelte_component_completion` | Svelte-specific completions |
| `test_typescript_sveltekit_completion` | TypeScript + SvelteKit |
| `test_python_fastapi_completion` | FastAPI backend completions |
| `test_tailwind_css_completion` | Tailwind utility classes |
| `test_fullstack_multi_file_session` | Frontend + backend editing |
| `test_fullstack_concurrent_edits` | Rapid cross-stack edits |
| `test_fullstack_performance` | Latency < 150ms |
| `test_fullstack_mcp_stats` | Request statistics tracking |
| `test_fullstack_error_resilience` | Invalid URI handling |

**Mocked Actors**:
- Full-Stack MCP Server with language detection (port 6001-6010)
- Multi-language LSP client
- Svelte, TypeScript, Python language servers (via proxy)
- FastAPI backend (conceptually)

## Mock Server Details

### Port Allocation

| Test Suite | Port Range | Purpose |
|------------|------------|---------|
| VSCode | 3001-3005 | Mock MCP server |
| Zed | 4001-4009 | Multi-language MCP server |
| Terminal | 5001-5008 | CLI MCP server |
| SvelteKit | 6001-6010 | Full-stack MCP server |

### Mock MCP Server Responses

All mock servers return JSON responses:

```json
{
  "suggestions": ["completion1", "completion2", ...],
  "documentation": "AI-enhanced documentation",
  "confidence": 0.9,
  "metadata": {
    "language": "detected language",
    "framework": "detected framework"
  }
}
```

### Language Detection

Mock servers detect language from file URI:
- `.svelte` → Svelte
- `.ts`, `.tsx` → TypeScript
- `.py` → Python
- `.rs` → Rust
- `.css` → CSS (Tailwind)

## Performance Targets

| Scenario | Target | Test |
|----------|--------|------|
| VSCode Completion | < 500ms avg | `test_vscode_performance` |
| Zed Completion | < 100ms p95 | `test_zed_performance_target` |
| Terminal Request | < 200ms avg | `test_terminal_performance_measurement` |
| Full-Stack | < 150ms avg | `test_fullstack_performance` |

## Debugging Tests

### Enable Verbose Logging

```bash
# Run with debug logging
RUST_LOG=debug cargo test --test integration_vscode_test -- --nocapture

# Run specific test with trace logging
RUST_LOG=trace cargo test --test integration_zed_test test_zed_performance_target -- --nocapture
```

### Check Mock Server Health

```bash
# While tests are running, check mock server
curl http://localhost:3001/health  # VSCode tests
curl http://localhost:4001/health  # Zed tests
curl http://localhost:5001/health  # Terminal tests
curl http://localhost:6001/health  # SvelteKit tests
```

### Manual LSP Testing

```bash
# Start LSP manually for debugging
target/release/universal-lsp \
  --log-level=debug \
  --mcp-pre=http://localhost:3001 \
  --mcp-timeout=5000

# In another terminal, send LSP requests
echo 'Content-Length: 140\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{}}}' | target/release/universal-lsp --mcp-pre=http://localhost:3001
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build
        run: cargo build --release

      - name: Run Integration Tests
        run: cargo test --test '*' --release

      - name: Run Performance Tests
        run: |
          cargo test --test integration_zed_test test_zed_performance_target
          cargo test --test integration_vscode_test test_vscode_performance
```

## Troubleshooting

### Tests Hanging

**Cause**: Mock server not starting properly
**Solution**: Check port availability

```bash
# Check if ports are in use
netstat -tulpn | grep -E '(3001|4001|5001|6001)'

# Kill processes using ports
pkill -f universal-lsp
```

### Connection Refused Errors

**Cause**: Mock server takes time to start
**Solution**: Tests include 500ms sleep after server start. If still failing, increase the delay.

### Timeout Errors

**Cause**: LSP server not responding
**Solution**: Increase timeout in test:

```rust
timeout(Duration::from_secs(10), lsp_client.initialize())
```

### JSON Parsing Errors

**Cause**: Invalid LSP message framing
**Solution**: Check Content-Length header format:

```rust
let header = format!("Content-Length: {}\r\n\r\n", message.len());
```

## Contributing

When adding new tests:

1. Follow existing patterns for mock servers
2. Use unique port numbers
3. Add test to appropriate suite
4. Update this README
5. Ensure tests are deterministic

## License

MIT License - See LICENSE file for details
