# Universal LSP Server

A standalone Language Server Protocol (LSP) implementation supporting **242+ programming languages** with intelligent code analysis, AI-powered features via Model Context Protocol (MCP), and room for extensive IDE capabilities.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

## Features

### Current
- **242+ Language Support** - From C to Zig, covering all major and exotic programming languages
- **LSP Protocol Implementation** - Full Language Server Protocol support
  - Hover information
  - Code completion
  - Document symbols/outline
  - Text synchronization
- **Intelligent Language Detection** - Automatic file extension mapping
- **High Performance** - Async/await architecture with Tokio runtime
- **Comprehensive Testing** - Unit and integration tests
- **Release Optimized** - LTO, stripping, and aggressive optimization for production

### Planned (Architecture Ready)
- **AI Integration via MCP** - Model Context Protocol client for AI-powered features
  - Code suggestions
  - Context-aware completions
  - Intelligent refactoring
- **Additional LSP Features**
  - Go to definition
  - Find references
  - Code actions (refactoring)
  - Diagnostics (linting)
  - Formatting
- **Tree-sitter Integration** - Advanced syntax analysis
- **Extensible Plugin System** - Custom language-specific analyzers

## Quick Start

### Prerequisites

- Rust 1.70 or higher
- Cargo (comes with Rust)

### Building

```bash
# Clone the repository
git clone https://github.com/valknarthing/universal-lsp.git
cd universal-lsp

# Build release binary (optimized)
cargo build --release

# The binary will be at: ./target/release/universal-lsp
```

### Running

```bash
# Run directly
./target/release/universal-lsp

# Or use cargo
cargo run --release
```

The LSP server communicates via stdin/stdout using JSON-RPC 2.0 protocol, as per the LSP specification.

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test language_detection
```

## Supported Languages

The server supports 242+ programming languages, including:

### Systems Programming
C, C++, Rust, Go, Zig

### Web Development
JavaScript, TypeScript, HTML, CSS, SCSS, SASS, Less, PHP, Vue, Svelte, Astro

### Scripting
Python, Ruby, Perl, Lua, Bash, Zsh, Fish, PowerShell

### JVM Languages
Java, Kotlin, Scala, Groovy, Clojure

### .NET Languages
C#, F#, Visual Basic

### Functional Programming
Haskell, OCaml, Erlang, Elixir, Elm, PureScript, Reason

### Mobile Development
Swift, Objective-C, Dart, Kotlin

### Data & Config
JSON, YAML, TOML, XML, INI, SQL

### And many more...
Ada, Assembly, AWK, Cairo, CMake, COBOL, CoffeeScript, Crystal, D, Dockerfile, Fortran, GraphQL, Julia, MATLAB, Nim, Nix, Pascal, R, Scheme, Solidity, Terraform, Verilog, VHDL, WebAssembly, and more!

See `src/language/mod.rs` for the complete list with file extension mappings.

## Architecture

### Project Structure

```
universal-lsp/
├── src/
│   ├── main.rs           # LSP server entry point
│   ├── language/         # Language definitions and detection
│   │   └── mod.rs        # 242+ language definitions
│   └── mcp/              # Model Context Protocol client (planned)
│       └── mod.rs        # MCP client architecture
├── tests/
│   └── integration_test.rs
├── Cargo.toml
└── README.md
```

### Core Components

1. **LSP Server (`src/main.rs`)**
   - Tower-LSP framework integration
   - Async request handling
   - Text document synchronization
   - Hover, completion, and symbol providers

2. **Language System (`src/language/mod.rs`)**
   - Static language definitions
   - Extension-to-language mapping
   - Keyword extraction
   - O(1) language detection via HashMap

3. **MCP Client (`src/mcp/mod.rs`)** *(Architecture Ready)*
   - HTTP/WebSocket/Stdio transport support
   - Context querying interface
   - Response caching
   - Async client with configurable timeout

## Configuration

The LSP server currently uses default settings. Future versions will support configuration via:

- CLI arguments
- Configuration file (`universal-lsp.toml`)
- LSP workspace configuration

### Planned Configuration Schema

```toml
[server]
log_level = "info"
max_concurrent_requests = 100

[languages]
# Per-language overrides
[languages.python]
tab_size = 4

[languages.javascript]
tab_size = 2

[mcp]
server_url = "http://localhost:3000"
transport = "http"  # or "stdio", "websocket"
timeout_ms = 5000
```

## Integration with Editors

### VS Code

```json
{
  "languageServerExample.trace.server": "verbose",
  "universal-lsp": {
    "command": "/path/to/universal-lsp",
    "args": []
  }
}
```

### Neovim (via nvim-lspconfig)

```lua
local configs = require('lspconfig.configs')
local lspconfig = require('lspconfig')

configs.universal_lsp = {
  default_config = {
    cmd = {'/path/to/universal-lsp'},
    filetypes = {'*'},  -- All file types
    root_dir = lspconfig.util.root_pattern('.git'),
  },
}

lspconfig.universal_lsp.setup{}
```

### Emacs (via lsp-mode)

```elisp
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection '("/path/to/universal-lsp"))
                  :major-modes '(all)
                  :server-id 'universal-lsp))
```

## Development

### Adding a New Language

1. Edit `src/language/mod.rs`
2. Add a new `Language` struct to the `LANGUAGES` vector:

```rust
Language {
    name: "YourLanguage",
    extensions: &["ext1", "ext2"],
    keywords: &["keyword1", "keyword2", "..."],
},
```

3. Run tests to ensure no duplicate extensions:

```bash
cargo test test_language_detection
```

### Implementing MCP Client

The MCP client architecture is ready in `src/mcp/mod.rs`. To implement:

1. Add HTTP/WebSocket client dependencies to `Cargo.toml`
2. Implement `get_context()` method with actual MCP protocol communication
3. Add `is_available()` health check logic
4. Integrate with LSP completion/hover providers

## Performance

Optimized release build with:
- **LTO (Link Time Optimization)**: Enabled
- **Code Generation Units**: 1 (maximum optimization)
- **Symbol Stripping**: Enabled for smaller binary
- **Optimization Level**: 3 (aggressive)

Expected binary size: ~5-8MB (depending on platform)

## Roadmap

- [x] Basic LSP server implementation
- [x] 242+ language support
- [x] Language detection system
- [x] MCP client architecture
- [ ] MCP client implementation
- [ ] Tree-sitter integration
- [ ] Go to definition
- [ ] Find references
- [ ] Code actions/refactoring
- [ ] Diagnostics/linting
- [ ] Code formatting
- [ ] Configuration file support
- [ ] Multi-root workspace support
- [ ] Incremental text synchronization
- [ ] Performance benchmarks
- [ ] Editor plugin packages

## Contributing

Contributions are welcome! Areas of interest:

1. Adding more languages
2. Implementing MCP client
3. Adding LSP features (go-to-definition, etc.)
4. Tree-sitter parser integration
5. Editor-specific integrations
6. Performance improvements
7. Documentation

## License

MIT License - see LICENSE file for details

## Links

- **Repository**: https://github.com/valknarthing/universal-lsp
- **Issues**: https://github.com/valknarthing/universal-lsp/issues
- **LSP Specification**: https://microsoft.github.io/language-server-protocol/
- **Model Context Protocol**: https://github.com/modelcontextprotocol/specification

## Acknowledgments

- Built with [tower-lsp](https://github.com/ebkalderon/tower-lsp)
- Powered by [Tokio](https://tokio.rs/) async runtime
- Inspired by the LSP ecosystem and modern editor experiences
