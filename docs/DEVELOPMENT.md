# Development Guide

Welcome to the Universal LSP development guide! This document provides comprehensive information for contributors, including code style guidelines, testing requirements, and the pull request process.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Project Structure](#project-structure)
- [Building from Source](#building-from-source)
- [Code Style Guidelines](#code-style-guidelines)
- [Testing Requirements](#testing-requirements)
- [Adding New Languages](#adding-new-languages)
- [Pull Request Process](#pull-request-process)
- [CI/CD Integration](#cicd-integration)
- [Development Tools](#development-tools)
- [Debugging Tips](#debugging-tips)
- [Code Review Checklist](#code-review-checklist)
- [Development Roadmap](#development-roadmap)

---

## Getting Started

### Prerequisites

- **Rust** 1.70+ ([install from rust-lang.org](https://www.rust-lang.org/))
- **Cargo** (comes with Rust)
- **Git** (for version control)
- **Editor** with Rust support (Zed, VS Code with rust-analyzer, or similar)

### Clone the Repository

```bash
git clone https://github.com/valknarthing/universal-lsp.git
cd universal-lsp
```

### Quick Start

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the LSP server
cargo run --release
```

---

## Project Structure

```
universal-lsp/
├── src/
│   ├── main.rs              # CLI entry point (lsp, acp, zed init modes)
│   ├── lib.rs               # Library root and public API
│   ├── language/            # Language detection from file extensions
│   │   └── mod.rs
│   ├── tree_sitter/         # Tree-sitter parser integration
│   │   ├── mod.rs           # Parser initialization, symbol extraction
│   │   └── queries/         # Tree-sitter query files
│   ├── completion/          # Multi-tier completion engine
│   │   ├── engine.rs        # Completion orchestration
│   │   ├── claude_provider.rs   # Claude AI integration
│   │   ├── copilot_provider.rs  # GitHub Copilot integration
│   │   └── mod.rs
│   ├── acp/                 # Agent Client Protocol implementation
│   │   └── mod.rs           # UniversalAgent, session management (18 tests)
│   ├── coordinator/         # MCP coordinator client
│   │   ├── client.rs        # HTTP client for MCP coordinator
│   │   └── mod.rs
│   ├── workspace/           # Multi-root workspace management
│   │   └── mod.rs
│   ├── text_sync/           # Document synchronization (full + incremental)
│   │   └── mod.rs
│   └── config.rs            # Configuration loading and CLI parsing
├── lsp-server/              # Standalone LSP server binary (Zed extension)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs          # LSP protocol handler
├── tests/                   # Integration tests
│   ├── integration_svelte_test.rs
│   ├── integration_vscode_test.rs
│   ├── integration_zed_test.rs
│   └── integration_terminal_test.rs
├── examples/                # Working code examples
│   ├── basic_lsp_server.rs
│   ├── acp_agent_mcp.rs
│   └── ai_completions.rs
├── docs/                    # Documentation
│   ├── GETTING_STARTED.md
│   ├── ARCHITECTURE.md
│   ├── LANGUAGES.md
│   ├── TESTING.md
│   └── DEVELOPMENT.md       # This file
├── Cargo.toml               # Main project dependencies
└── README.md                # Project overview
```

### Key Modules

**src/language/mod.rs** (Language Detection)
- Maps file extensions to language identifiers
- Currently supports 19 languages with tree-sitter
- Unit tests: `test_language_detection`, `test_unknown_extension`

**src/tree_sitter/mod.rs** (Parser Integration)
- Initializes tree-sitter parsers for each language
- Extracts symbols (functions, classes, structs, etc.)
- 12 unit tests covering parsing and symbol extraction

**src/acp/mod.rs** (Agent Client Protocol)
- **621 lines of production code**
- **18 comprehensive unit tests**
- Implements `acp::Agent` trait with 8 methods
- Session management for multi-turn conversations
- Custom extension methods: `get-languages`, `get-capabilities`, `get-mcp-status`

**src/completion/engine.rs** (Completion Engine)
- Multi-tier completion strategy
- Parallel gathering from tree-sitter, AI, and MCP sources
- Ranking and deduplication of results

**src/coordinator/client.rs** (MCP Client)
- HTTP client for MCP coordinator daemon
- Connection pooling and retry logic
- Request routing to specialized servers

---

## Building from Source

### Debug Build

```bash
# Standard debug build (faster compilation, slower execution)
cargo build

# Binary location: ./target/debug/universal-lsp
```

### Release Build

```bash
# Optimized release build (slower compilation, faster execution)
cargo build --release

# Binary location: ./target/release/universal-lsp
# Size: ~20MB (with LTO and symbol stripping)
```

### Build Configuration

The `Cargo.toml` includes these optimizations for release builds:

```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = true              # Link-time optimization
strip = true            # Remove debug symbols
codegen-units = 1       # Single codegen unit for better optimization
```

### Incremental Build

```bash
# Enable incremental compilation (faster rebuilds)
export CARGO_INCREMENTAL=1
cargo build
```

---

## Code Style Guidelines

### Rust Conventions

Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/):

1. **Formatting**: Use `rustfmt` for automatic formatting
   ```bash
   cargo fmt
   ```

2. **Linting**: Use `clippy` for additional checks
   ```bash
   cargo clippy -- -D warnings
   ```

3. **Naming Conventions**:
   - **Modules**: `snake_case` (e.g., `tree_sitter`, `text_sync`)
   - **Types**: `PascalCase` (e.g., `TreeSitterParser`, `CompletionEngine`)
   - **Functions**: `snake_case` (e.g., `parse_symbols`, `get_completions`)
   - **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RETRIES`, `DEFAULT_TIMEOUT`)

4. **Documentation**:
   - All public APIs must have doc comments (`///`)
   - Include examples in doc comments where applicable
   - Run `cargo doc` to generate and verify documentation

### Error Handling

**Use `anyhow` for application errors** (in `main.rs`, CLI commands):

```rust
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let content = std::fs::read_to_string("config.toml")
        .context("Failed to read config.toml")?;

    toml::from_str(&content)
        .context("Failed to parse config.toml")
}
```

**Use custom error types for library APIs** (in `src/lib.rs` and modules):

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Parse failed: {0}")]
    ParseFailed(String),
}
```

### Asynchronous Code

**Use `tokio` for async runtime**:

```rust
use tokio::task;

#[tokio::main]
async fn main() -> Result<()> {
    // Async code here
}

async fn fetch_data() -> Result<Data> {
    // Use tokio::spawn for CPU-bound tasks
    let result = task::spawn_blocking(|| {
        expensive_computation()
    }).await?;

    Ok(result)
}
```

### Testing Conventions

1. **Unit tests** in the same file as the code:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_function_name() {
           // Arrange
           let input = "test";

           // Act
           let result = function_to_test(input);

           // Assert
           assert_eq!(result, expected);
       }
   }
   ```

2. **Integration tests** in `tests/` directory:
   ```rust
   use universal_lsp::TreeSitterParser;

   #[test]
   fn test_end_to_end_workflow() {
       // Test complete workflows
   }
   ```

---

## Testing Requirements

### Running Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run with output
cargo test -- --nocapture

# Run specific module tests
cargo test tree_sitter
cargo test acp

# Run specific test
cargo test test_javascript_parsing

# Run tests in release mode (faster execution)
cargo test --release

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

### Test Coverage Requirements

**All new features must have tests.** The project maintains these coverage standards:

| Module | Target Coverage | Current Status |
|--------|----------------|----------------|
| **language/** | 100% | ✅ 100% (2 tests) |
| **tree_sitter/** | 95%+ | ✅ 95% (12 tests) |
| **acp/** | 100% | ✅ 100% (18 tests) |
| **coordinator/** | 85%+ | ✅ 85% (3 tests) |
| **text_sync/** | 90%+ | ✅ 90% (2 tests) |
| **completion/** | 80%+ | ⏳ In progress |

### Writing Good Tests

**1. Test isolation**: Each test should be independent

```rust
#[test]
fn test_parser_initialization() {
    // Create fresh state for each test
    let mut parser = TreeSitterParser::new().unwrap();

    // Test one specific behavior
    assert!(parser.set_language("javascript").is_ok());
}
```

**2. Test naming**: Use descriptive names that explain what is tested

```rust
#[test]
fn test_javascript_function_extraction() { /* ... */ }

#[test]
fn test_python_class_extraction() { /* ... */ }

#[test]
fn test_unknown_language_returns_error() { /* ... */ }
```

**3. Test edge cases**: Don't just test the happy path

```rust
#[test]
fn test_empty_file_parsing() {
    let parser = TreeSitterParser::new().unwrap();
    let symbols = parser.parse_symbols("javascript", "").unwrap();
    assert_eq!(symbols.len(), 0);
}

#[test]
fn test_invalid_syntax_handling() {
    let parser = TreeSitterParser::new().unwrap();
    let result = parser.parse_symbols("javascript", "function {{{");
    assert!(result.is_ok()); // Tree-sitter is error-tolerant
}
```

**4. Integration tests**: Test real-world scenarios

See `tests/integration_svelte_test.rs` for examples:

```rust
#[test]
fn test_svelte_component_parsing() {
    let svelte_code = r#"
<script>
  export let name;
</script>

<style>
  h1 { color: red; }
</style>

<h1>Hello {name}!</h1>
"#;

    let parser = TreeSitterParser::new().unwrap();
    let tree = parser.parse("svelte", svelte_code).unwrap();
    let symbols = parser.extract_symbols(&tree, svelte_code, "svelte").unwrap();

    assert!(symbols.iter().any(|s| s.name == "name"));
    assert!(tree.root_node().child_count() >= 3);
}
```

### Benchmarking

```bash
# Run benchmarks (if implemented)
cargo bench

# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bin universal-lsp
```

---

## Adding New Languages

### Current Limitation

Universal LSP currently supports **19 languages** due to tree-sitter 0.20.x dependency conflicts (cc crate version incompatibilities). See [LANGUAGES.md](LANGUAGES.md) for details.

### After tree-sitter 0.21+ Migration

Once the tree-sitter ecosystem migrates to 0.21+ (estimated Q2 2025), adding new languages will be straightforward:

#### Step 1: Add Grammar Dependency

```toml
# Cargo.toml
[dependencies]
tree-sitter-newlang = "0.21"
```

#### Step 2: Register Parser

```rust
// src/tree_sitter/mod.rs
impl TreeSitterParser {
    pub fn set_language(&mut self, language: &str) -> Result<()> {
        match language {
            // ... existing languages
            "newlang" => {
                let language = tree_sitter_newlang::language();
                self.parser.set_language(language)
                    .context("Failed to set newlang parser")?;
            }
            _ => return Err(anyhow!("Unsupported language: {}", language)),
        }
        Ok(())
    }
}
```

#### Step 3: Add Language Detection

```rust
// src/language/mod.rs
pub fn detect_language(filename: &str) -> &'static str {
    match filename.split('.').last() {
        // ... existing extensions
        Some("newext") => "newlang",
        _ => "unknown",
    }
}
```

#### Step 4: Add Tests

```rust
// src/tree_sitter/mod.rs (in #[cfg(test)] module)
#[test]
fn test_newlang_parsing() {
    let mut parser = TreeSitterParser::new().unwrap();
    assert!(parser.set_language("newlang").is_ok());

    let code = "function example() { ... }";
    let tree = parser.parse("newlang", code).unwrap();

    assert!(tree.root_node().child_count() > 0);
}

#[test]
fn test_newlang_symbol_extraction() {
    let code = "function greet(name) { return `Hello, ${name}`; }";
    let symbols = parse_symbols("newlang", code).unwrap();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "greet");
    assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
}
```

#### Step 5: Update Documentation

- Add language to [LANGUAGES.md](LANGUAGES.md) matrix
- Update README.md language count
- Add language-specific notes if needed

---

## Pull Request Process

### Before Submitting

1. **Run the full test suite**:
   ```bash
   cargo test
   ```

2. **Format your code**:
   ```bash
   cargo fmt
   ```

3. **Check for linting issues**:
   ```bash
   cargo clippy -- -D warnings
   ```

4. **Update documentation** if needed:
   ```bash
   cargo doc --no-deps --document-private-items
   ```

5. **Commit your changes** with a descriptive message:
   ```bash
   git add .
   git commit -m "feat: Add support for NewLanguage parser"
   ```

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Test additions or changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Build process or auxiliary tool changes

**Examples**:
```
feat: Add tree-sitter parser for Kotlin
fix: Resolve incremental text sync bug
docs: Update LANGUAGES.md with new parser count
test: Add integration tests for Zed editor
refactor: Extract completion engine to separate module
perf: Optimize symbol extraction for large files
chore: Update dependencies to tree-sitter 0.21
```

### Pull Request Template

When opening a PR, include:

**Summary**
- Brief description of changes
- Motivation and context

**Changes**
- List of modified files and their purpose
- Breaking changes (if any)

**Testing**
- Which tests were added/modified
- Manual testing steps (if applicable)

**Checklist**
- [ ] Tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (for releases)

### Code Review Process

1. **Automated checks** run on CI (tests, formatting, clippy)
2. **Maintainer review** (typically within 48 hours)
3. **Address feedback** (make changes in response to comments)
4. **Approval and merge**

---

## CI/CD Integration

### GitHub Actions Workflow

The project uses GitHub Actions for continuous integration:

**.github/workflows/ci.yml** (current):

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable]

    steps:
      - uses: actions/checkout@v4

      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}

      - name: Build
        run: cargo build --verbose

      - name: Run tests
        run: cargo test --all-features --verbose

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Check formatting
        run: cargo fmt -- --check
```

### CI Test Matrix

| Platform | Rust Version | Status |
|----------|-------------|---------|
| **Ubuntu 22.04** | stable | ✅ 32/32 tests passing |
| **macOS 13** | stable | ✅ 32/32 tests passing |
| **Windows 11** | stable | ✅ 32/32 tests passing |

### Local CI Simulation

```bash
# Run the same checks as CI
cargo build --verbose
cargo test --all-features --verbose
cargo clippy -- -D warnings
cargo fmt -- --check
```

---

## Development Tools

### Recommended Editor Setup

**Zed Editor** (with rust-analyzer):
```json
{
  "lsp": {
    "rust-analyzer": {
      "initialization_options": {
        "check": {
          "command": "clippy"
        }
      }
    }
  }
}
```

**VS Code** (with rust-analyzer extension):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

### Useful Cargo Commands

```bash
# Check code without building
cargo check

# Build documentation
cargo doc --no-deps --document-private-items

# Open documentation in browser
cargo doc --open

# Show dependency tree
cargo tree

# Update dependencies
cargo update

# Clean build artifacts
cargo clean

# Show outdated dependencies
cargo install cargo-outdated
cargo outdated
```

### Debugging Tools

**1. Print debugging** (simple):
```rust
println!("Debug: {:?}", value);
```

**2. `dbg!` macro** (better):
```rust
let result = dbg!(calculate_value());
```

**3. LLDB/GDB** (advanced):
```bash
# Build with debug symbols
cargo build

# Run with debugger
rust-lldb ./target/debug/universal-lsp
```

**4. Logging** (production):
```rust
use tracing::{info, debug, error};

info!("Server started on port {}", port);
debug!("Processing request: {:?}", request);
error!("Failed to load config: {}", err);
```

---

## Debugging Tips

### Tree-sitter Parsing Issues

**Problem**: Parser fails to load

**Solution**: Check that grammar is correctly registered

```rust
// src/tree_sitter/mod.rs
match language {
    "javascript" => {
        let language = tree_sitter_javascript::language();
        self.parser.set_language(language)?;
    }
    // ...
}
```

**Problem**: Symbol extraction returns empty list

**Solution**: Verify tree-sitter query files exist and are correct

```bash
# Check query files (future, after 0.21 migration)
ls -la queries/javascript/
# Should contain: highlights.scm, symbols.scm, locals.scm
```

### AI Completion Issues

**Problem**: Claude completions not working

**Solution**: Check API key configuration

```bash
# Set API key
export ANTHROPIC_API_KEY="your-key-here"

# Verify in code
println!("API Key set: {}", std::env::var("ANTHROPIC_API_KEY").is_ok());
```

**Problem**: Completions timing out

**Solution**: Adjust timeout settings

```rust
// src/completion/claude_provider.rs
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
```

### LSP Protocol Issues

**Problem**: Editor not receiving completions

**Solution**: Check LSP protocol logging

```bash
# Enable LSP logging
export RUST_LOG=debug

# Run LSP server
cargo run --release 2>&1 | tee lsp.log

# Check for JSON-RPC messages
grep "textDocument/completion" lsp.log
```

---

## Code Review Checklist

Before submitting a PR, ensure:

### Functionality
- [ ] Feature works as intended
- [ ] Edge cases are handled
- [ ] Error messages are clear and helpful
- [ ] Performance is acceptable

### Code Quality
- [ ] Code follows Rust conventions
- [ ] Variable names are descriptive
- [ ] Functions are small and focused
- [ ] Comments explain "why", not "what"
- [ ] No unused imports or dead code

### Testing
- [ ] Unit tests cover new functionality
- [ ] Integration tests pass
- [ ] Edge cases have tests
- [ ] Test names are descriptive

### Documentation
- [ ] Public APIs have doc comments
- [ ] Complex logic has explanatory comments
- [ ] README.md updated (if needed)
- [ ] CHANGELOG.md updated (for releases)

### Safety & Security
- [ ] No unwrap() or expect() in library code (use proper error handling)
- [ ] No hardcoded secrets or API keys
- [ ] Input validation for external data
- [ ] No unsafe code (unless absolutely necessary and documented)

---

## Development Roadmap

### Current (v0.1.0)

- ✅ 19 languages with tree-sitter support
- ✅ AI completions (Claude + Copilot)
- ✅ ACP (Agent Client Protocol) fully implemented
- ✅ MCP client architecture
- ✅ Full LSP protocol implementation
- ✅ 32 unit tests passing
- ✅ CI/CD for multi-platform releases

### Planned (v0.2.0)

**Language Features**:
- [ ] Go to definition
- [ ] Find references
- [ ] Code actions & refactoring
- [ ] Diagnostics & linting integration
- [ ] Formatting integration (prettier, black, rustfmt)

**Performance**:
- [ ] Incremental text sync optimization
- [ ] Lazy parser loading
- [ ] Completion caching

**Language Expansion**:
- [ ] Migrate to tree-sitter 0.21+ (Q2 2025)
- [ ] Add 30+ additional languages
- [ ] On-demand grammar loading

**Infrastructure**:
- [ ] MCP coordinator daemon
- [ ] Plugin system for custom analyzers
- [ ] Configuration UI

### Future (v0.3.0+)

- [ ] Workspace-wide symbol search
- [ ] Cross-file go-to-definition
- [ ] Project-wide find references
- [ ] Dependency graph analysis
- [ ] Language-specific semantic analysis
- [ ] Custom tree-sitter grammar support

---

## Contributing

We welcome contributions! Here's how to get started:

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Create a feature branch**: `git checkout -b feat/my-feature`
4. **Make your changes** and add tests
5. **Run the test suite**: `cargo test`
6. **Format your code**: `cargo fmt`
7. **Commit your changes**: `git commit -m "feat: Add my feature"`
8. **Push to your fork**: `git push origin feat/my-feature`
9. **Open a Pull Request** on GitHub

### Getting Help

- **Issues**: https://github.com/valknarthing/universal-lsp/issues
- **Documentation**: https://valknarthing.github.io/universal-lsp/
- **Examples**: `examples/` directory

---

## License

MIT License - see [LICENSE](../LICENSE) file for details.

## See Also

- **[README.md](../README.md)** - Project overview
- **[GETTING_STARTED.md](GETTING_STARTED.md)** - Installation and quick start
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and architecture
- **[LANGUAGES.md](LANGUAGES.md)** - Language support matrix
- **[TESTING.md](TESTING.md)** - Test suite documentation

---

**Thank you for contributing to Universal LSP!**
