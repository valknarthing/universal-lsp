# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Universal LSP is a sophisticated Language Server Protocol implementation combining tree-sitter parsing (19+ languages) with AI-powered features through Claude and GitHub Copilot integration. It includes Agent Client Protocol (ACP) support and Model Context Protocol (MCP) orchestration for extensible AI capabilities.

## Common Commands

### Building & Running

```bash
# Build (debug)
cargo build

# Build (optimized release)
cargo build --release

# Run LSP server (default mode)
cargo run --release
# or
./target/release/universal-lsp
# or explicitly
./target/release/universal-lsp lsp

# Run ACP agent mode
cargo run --release -- acp
# or
./target/release/universal-lsp acp

# Initialize Zed workspace with full MCP/AI configuration
cargo run --release -- zed init . --with-mcp --with-claude --with-copilot --with-acp
```

### Testing

```bash
# Run all tests
cargo test

# Run with output (to see println!/dbg! statements)
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run specific test file
cargo test --test integration_test

# Run integration tests for specific components
cargo test --test tree_sitter_comprehensive_test
cargo test --test ai_providers_comprehensive_test
cargo test --test mcp_comprehensive_test
cargo test --test coordinator_test

# Run benchmarks
cargo bench
```

### Documentation

```bash
# Generate and open docs
cargo doc --no-deps --document-private-items --open

# Check without building
cargo check

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings
```

## Architecture

### Request Flow (LSP Mode)

```
Editor (Zed/VSCode/Neovim)
  ↓ JSON-RPC 2.0 over stdio
UniversalLsp Server (src/main.rs)
  ├─→ Text Sync (text_sync/)
  ├─→ Language Detection (language/)
  └─→ Multi-source Processing:
      ├─→ Tree-sitter Parser (tree_sitter/)
      ├─→ AI Clients (ai/claude.rs, ai/copilot.rs)
      ├─→ MCP Coordinator (coordinator/)
      │   └─→ MCP Clients (mcp/)
      └─→ LSP Proxy (proxy/) [optional]
  ↓
Response Assembly → Editor
```

### Multi-Modal Operation

The binary supports three modes (determined by CLI args in `Config::from_args()`):

1. **LSP Mode** (`ulsp` or `ulsp lsp`): Standard LSP server via stdin/stdout
2. **ACP Mode** (`ulsp acp`): Agent Client Protocol server for editor-to-AI communication
3. **Zed Init** (`ulsp zed init`): Workspace configuration generator

### Component Responsibilities

**Core LSP (main.rs:42-757)**
- `UniversalLsp` struct: Main LSP server implementation
- Implements `tower_lsp::LanguageServer` trait
- Manages document state in `DashMap<String, String>`
- Delegates to specialized providers (diagnostics, code actions, formatting)

**MCP Coordinator Architecture (coordinator/)**

The MCP Coordinator is a shared daemon that manages MCP server connections:
- **coordinator/mod.rs**: Unix socket server (`/tmp/universal-mcp.sock`)
- **coordinator/client.rs**: Client for LSP/ACP processes to communicate with coordinator
- **coordinator/pool.rs**: Connection pool for MCP server processes
- **coordinator/cache.rs**: Response caching with TTL
- **coordinator/protocol.rs**: IPC message protocol

This architecture allows multiple LSP instances to share MCP connections efficiently.

**MCP Pipeline (pipeline/mod.rs)**

Legacy pipeline for direct MCP integration (pre-coordinator):
- `McpPipeline::pre_process()`: Query MCP servers before LSP processing
- `McpPipeline::post_process()`: Enhance LSP responses with MCP data
- `merge_mcp_responses()`: Aggregate multi-server results

**AI Integration (ai/)**

Two AI providers with unified `CompletionContext` interface:
- **ai/claude.rs**: Anthropic Claude API client (requires `ANTHROPIC_API_KEY`)
- **ai/copilot.rs**: GitHub Copilot API client (requires `GITHUB_TOKEN`)
- Both return `Vec<CompletionSuggestion>` with confidence scores

AI completions are sorted with prefix `0_claude_` or `0_copilot_` to appear first in completion lists.

**Tree-sitter Integration (tree_sitter/mod.rs)**

- `TreeSitterParser::new()`: Lazy initialization
- `TreeSitterParser::set_language(lang)`: Load grammar for language
- `TreeSitterParser::parse(code, uri)`: Parse to tree
- `TreeSitterParser::extract_symbols()`: Convert tree to LSP symbols
- `TreeSitterParser::find_definition()`: Navigate to symbol definition
- `TreeSitterParser::find_references()`: Find all symbol usages

Symbols get sorted with prefix `1_` to appear after AI suggestions.

**ACP Agent (acp/mod.rs)**

Implements Agent Client Protocol for conversational AI assistance:
- `UniversalAgent::new()`: Basic agent without MCP
- `UniversalAgent::with_coordinator()`: Agent with MCP coordinator integration
- Multi-turn conversations with session management
- Integrates with MCP coordinator for enhanced context

## Language Support Strategy

### Tree-sitter Supported (19 languages)

Full syntax analysis, symbol extraction, go-to-definition, find-references:

- **Web**: JavaScript, TypeScript, TSX, HTML, CSS, JSON, Svelte
- **Systems**: C, C++, Rust, Go
- **Application**: Python, Ruby, PHP, Java
- **JVM/.NET**: Scala, Kotlin, C#
- **Shell**: Bash

### AI-Only Support

All other languages get AI-powered completions from Claude/Copilot but no tree-sitter parsing.

### Adding New Tree-sitter Languages

Due to `cc` crate conflicts in tree-sitter 0.20.x ecosystem, adding languages requires:
1. Check `tree-sitter-{lang}` uses compatible `cc` version
2. Add dependency to `Cargo.toml` with `tag = "v0.20.x"`
3. Update `language/mod.rs` with file extension mapping
4. Add grammar initialization in `tree_sitter/mod.rs`

See `docs/LANGUAGES.md` (referenced in README) for migration path to tree-sitter 0.21+.

## Configuration System

**Configuration Sources** (loaded in order, `config/mod.rs`):
1. `universal-lsp.toml` in project root
2. `~/.config/universal-lsp/config.toml`
3. CLI arguments (highest priority)

**Key Configuration Sections**:
- `[server]`: Log level, max concurrency
- `[mcp]`: MCP servers, timeout, cache settings
- `[mcp.servers.*]`: Individual MCP server configs (stdio/http/websocket)
- `[proxy]`: Language-specific LSP proxy configurations
- `[ai.claude]`: Claude API settings
- `[ai.copilot]`: Copilot settings

## Testing Strategy

**Test Organization**:
- `tests/integration_test.rs`: Basic LSP protocol tests
- `tests/tree_sitter_comprehensive_test.rs`: Parser and symbol extraction (11 tests)
- `tests/ai_providers_comprehensive_test.rs`: AI client tests (6 tests)
- `tests/mcp_comprehensive_test.rs`: MCP integration tests (8 tests)
- `tests/coordinator_test.rs`: MCP coordinator daemon tests (7 tests)
- `tests/proxy_comprehensive_test.rs`: LSP proxy tests
- `tests/integration_*_test.rs`: Editor-specific integration tests
- `tests/mock_mcp_server.rs`: Mock MCP server binary for testing

**Running Specific Test Categories**:
```bash
cargo test tree_sitter    # All tree-sitter tests
cargo test ai_            # All AI provider tests
cargo test mcp_           # All MCP tests
cargo test coordinator    # All coordinator tests
```

## Development Workflow

### Adding New LSP Features

1. Implement handler in `UniversalLsp` struct (main.rs)
2. Update `ServerCapabilities` in `initialize()` method
3. Add tests in appropriate `tests/*_test.rs` file
4. Update `src/lib.rs` documentation if public API changes

### Adding New MCP Servers

MCP servers are configured, not coded. Add to config:
```toml
[[mcp.servers]]
name = "my-server"
target = "stdio"
command = "npx"
args = ["-y", "@my-org/my-mcp-server"]
```

### Adding New AI Providers

1. Create `src/ai/provider.rs` with `ProviderClient` struct
2. Implement `get_completions(&self, context: &CompletionContext)` method
3. Add client initialization in `main.rs:UniversalLsp::new()`
4. Update completion handler to query new provider

### Performance Considerations

- Tree-sitter parsers are cached in `DashMap` per-language
- MCP responses are cached with 5-minute TTL in coordinator
- Document content is stored in memory (`Arc<DashMap<String, String>>`)
- AI calls are concurrent (launched in parallel where possible)
- Coordinator uses connection pooling to avoid spawning duplicate MCP processes

### Binary Size Optimization

Release profile uses aggressive optimization:
```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Strip symbols for smaller binary
```

Result: ~20MB release binary with all 19 tree-sitter grammars.

## CI/CD

Three GitHub Actions workflows (`.github/workflows/`):

1. **ci.yml**: Test on Linux/macOS/Windows, all commits
2. **release.yml**: Build multi-platform binaries on git tags
3. **deploy-docs.yml**: Generate and deploy rustdoc to GitHub Pages

## Environment Variables

- `ANTHROPIC_API_KEY`: Required for Claude AI completions
- `GITHUB_TOKEN`: Required for GitHub Copilot completions
- `RUST_LOG`: Override log level (e.g., `RUST_LOG=debug`)

## Common Patterns

### Position/Offset Conversion

LSP uses line/character positions, tree-sitter uses byte offsets:
```rust
// LSP Position → byte offset
let byte_offset = position_to_byte(&source_code, lsp_position);

// Used throughout main.rs for tree-sitter queries
```

### Error Handling

- Internal errors use `anyhow::Result`
- LSP handlers return `tower_lsp::jsonrpc::Result` (no panic propagation to editor)
- MCP errors use custom `McpError` type with conversion to `anyhow::Error`

### Async Patterns

- Main LSP handlers are `async` (tower-lsp requirement)
- AI/MCP calls use `tokio` runtime
- Coordinator uses `tokio::net::UnixListener` for IPC

## Debugging Tips

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run -- lsp
```

### Test with Mock MCP Server

```bash
# Build mock server
cargo build --bin mock-mcp-server

# Run in background
./target/debug/mock-mcp-server &

# Configure to use it in universal-lsp.toml
[[mcp.servers]]
name = "mock"
target = "stdio"
command = "./target/debug/mock-mcp-server"
args = []
```

### Testing LSP Protocol Manually

Use a JSON-RPC client or test with editor integration. For stdio testing:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/debug/universal-lsp
```

## Known Limitations

1. **Tree-sitter Dependency Conflicts**: Limited to 19 languages due to `cc` crate incompatibilities in 0.20.x ecosystem
2. **MCP Coordinator**: Requires manual daemon start (not auto-launched yet)
3. **Incremental Sync**: Implemented but limited testing with complex edits
4. **Proxy Mode**: Configured but not fully implemented (placeholders in code)

## Future Architecture Notes

Planned v0.2.0 features (see README roadmap):
- Auto-launching MCP coordinator daemon
- Expanding to 50+ tree-sitter languages (requires migration to 0.21+)
- Full proxy implementation for delegating to language-specific LSP servers
- Plugin system for custom analyzers
