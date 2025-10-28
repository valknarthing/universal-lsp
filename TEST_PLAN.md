# Universal LSP Test Suite

## Test Coverage Plan

### 1. Unit Tests (in `src/` modules)

#### Tree-sitter Module Tests (`src/tree_sitter/mod.rs`)
- ✅ Symbol extraction for all 19 languages
- ✅ Language detection from file extension
- ✅ Parser initialization and caching
- ⏳ Edge cases: empty files, malformed syntax, large files

#### MCP Client Tests (`src/mcp/mod.rs`)
- ✅ HTTP client creation
- ✅ Health check
- ⏳ Request/response parsing
- ⏳ Timeout handling
- ⏳ Error handling for unreachable servers

#### MCP Pipeline Tests (`src/pipeline/mod.rs`)
- ✅ Pre-processing pipeline
- ✅ Post-processing pipeline
- ✅ Response merging
- ⏳ Parallel request handling
- ⏳ Failure handling (some servers down)

#### LSP Proxy Tests (`src/proxy/mod.rs`)
- ✅ ProxyConfig parsing
- ✅ ProxyManager lifecycle
- ⏳ Process spawning and communication
- ⏳ LSP protocol framing
- ⏳ Request forwarding and response handling

#### AI Provider Tests (`src/ai/claude.rs`, `src/ai/copilot.rs`)
- ⏳ API key validation
- ⏳ Request formatting
- ⏳ Response parsing
- ⏳ Rate limiting
- ⏳ Error handling (API down, invalid key, quota exceeded)

#### Configuration Tests (`src/config/mod.rs`)
- ⏳ CLI argument parsing
- ⏳ Config file loading
- ⏳ Default values
- ⏳ Validation (invalid URLs, negative timeouts, etc.)

### 2. Integration Tests (in `tests/`)

#### LSP Protocol Tests (`tests/lsp_protocol_test.rs`)
- ⏳ Initialize handshake
- ⏳ textDocument/didOpen
- ⏳ textDocument/didChange
- ⏳ textDocument/completion
- ⏳ textDocument/hover
- ⏳ textDocument/definition
- ⏳ textDocument/documentSymbol
- ⏳ Shutdown sequence

#### Multi-Language Tests (`tests/multi_language_test.rs`)
- ⏳ JavaScript completions
- ⏳ Python completions
- ⏳ Rust completions
- ⏳ Cross-language context (polyglot repos)

#### MCP Integration Tests (`tests/mcp_integration_test.rs`)
- ⏳ End-to-end with mock MCP server
- ⏳ Pre-processing + local handling + post-processing
- ⏳ MCP server failures (graceful degradation)

#### Proxy Integration Tests (`tests/proxy_integration_test.rs`)
- ⏳ Forward to rust-analyzer
- ⏳ Forward to pyright
- ⏳ Fallback when proxy unavailable

### 3. Performance Tests (`tests/performance/`)

#### Latency Tests
- ⏳ Completion latency (p50, p95, p99)
- ⏳ Tree-sitter parsing time
- ⏳ Symbol extraction time
- ⏳ AI provider response time

#### Memory Tests
- ⏳ Memory usage under load
- ⏳ Parser cache size
- ⏳ Leak detection (long-running)

#### Concurrency Tests
- ⏳ 100 simultaneous requests
- ⏳ Race conditions
- ⏳ Deadlock detection

### 4. End-to-End Tests (`tests/e2e/`)

#### Editor Integration
- ✅ VS Code integration (placeholder)
- ✅ Zed integration (placeholder)
- ✅ Terminal/CLI integration (placeholder)
- ✅ Svelte project integration (placeholder)

### 5. Regression Tests

- ⏳ Test cases for reported bugs
- ⏳ Edge cases discovered in production

## Test Execution

### Run All Tests
```bash
cargo test
```

### Run Specific Test Suites
```bash
cargo test --test tree_sitter_test
cargo test --test mcp_test
cargo test --test proxy_test
cargo test --test lsp_protocol_test
```

### Run with Output
```bash
cargo test -- --nocapture
```

### Run Performance Tests
```bash
cargo test --release -- --ignored
```

### Coverage Report
```bash
cargo tarpaulin --out Html
```

## Test Fixtures

### Mock Data
- `tests/fixtures/sample_code/` - Sample files in all 19 languages
- `tests/fixtures/mock_responses/` - Mock AI/MCP responses
- `tests/fixtures/configs/` - Various configuration scenarios

### Test Utilities
- `tests/utils/mock_lsp_client.rs` - Mock LSP client for testing
- `tests/utils/mock_mcp_server.rs` - Mock MCP server for testing
- `tests/utils/assertions.rs` - Custom test assertions

## CI/CD Integration

### GitHub Actions Workflow
```yaml
name: Test Suite
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all
      - run: cargo test --release -- --ignored
```

## Test Metrics

### Coverage Goals
- Unit tests: 80%+ coverage
- Integration tests: Key paths covered
- E2E tests: Happy path + critical errors

### Performance Targets
- Completion latency: <100ms p95
- Memory usage: <150MB per process
- Startup time: <500ms

---

**Status**: Test suite in development
**Priority**: High - Ensure reliability before 1.0 release
