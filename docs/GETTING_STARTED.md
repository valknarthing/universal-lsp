# Getting Started with Universal LSP

## Installation

### Prerequisites

- **Rust** 1.70 or later ([install from rust-lang.org](https://www.rust-lang.org/))
- **Cargo** (comes with Rust)
- **Git** (for cloning the repository)

### Build from Source

```bash
# 1. Clone the repository
git clone https://github.com/valknarthing/universal-lsp.git
cd universal-lsp

# 2. Build the release binary (optimized)
cargo build --release

# 3. Binary is located at: ./target/release/universal-lsp
```

### Verify Installation

```bash
# Run the LSP server (should wait for JSON-RPC input)
./target/release/universal-lsp

# In another terminal, check it's running
ps aux | grep universal-lsp
```

---

## Quick Test

### 1. Run Unit Tests

```bash
# Run all tests
cargo test

# Expected output: "32 passed"
```

### 2. Try an Example

```bash
# Run the basic tree-sitter example
cargo run --example basic_lsp_server

# Output shows JavaScript and Python symbol extraction
```

---

## Editor Configuration

### Zed Editor

Universal LSP works automatically with Zed's extension system. No manual configuration needed.

### VS Code

Create `.vscode/settings.json`:

```json
{
  "universal-lsp": {
    "command": "/absolute/path/to/universal-lsp",
    "args": [],
    "filetypes": ["javascript", "typescript", "python", "rust"]
  }
}
```

### Neovim (via nvim-lspconfig)

Add to your Neovim config:

```lua
local lspconfig = require('lspconfig')

lspconfig.universal_lsp.setup{
  cmd = {'/absolute/path/to/universal-lsp'},
  filetypes = {'javascript', 'typescript', 'python', 'rust', 'go'},
  root_dir = lspconfig.util.root_pattern('.git', 'Cargo.toml', 'package.json'),
}
```

---

## Features Overview

### What Works Now

**Tree-sitter Analysis (19 languages)**
- Symbol extraction (functions, classes, methods)
- Hover information
- Document outline

**AI-Powered Completions**
- Claude Sonnet 4 integration
- GitHub Copilot support
- Multi-tier completion strategy

**LSP Protocol**
- Text synchronization (full and incremental)
- Hover provider
- Completion provider
- Document symbols

### What's Planned

- Go to definition
- Find references
- Code actions
- Diagnostics
- Formatting

---

## Configuration

Currently, Universal LSP uses default settings. Future versions will support configuration via:

### Configuration File (Planned)

Create `.universal-lsp.toml` in your project root:

```toml
[workspace]
name = "my-project"
root = "."

[formatting]
indent_size = 2
use_tabs = false

[languages.python]
indent_size = 4

[ai]
provider = "claude"  # or "copilot"
```

---

## Testing Your Setup

### 1. Open a JavaScript File

Create `test.js`:

```javascript
function greet(name) {
  console.log(`Hello, ${name}!`);
}

class User {
  constructor(name) {
    this.name = name;
  }
}
```

### 2. Test Features

**Hover**: Place cursor over `greet` - should show function signature

**Completion**: Type `cons` - should suggest `console`

**Symbols**: Request document symbols - should list `greet` and `User`

---

## Troubleshooting

### LSP Server Not Starting

**Check binary path**:
```bash
which universal-lsp
# or
ls -la /path/to/universal-lsp
```

**Check it's executable**:
```bash
chmod +x target/release/universal-lsp
```

### No Completions Showing

**Check language support**:
- Tree-sitter: JavaScript, TypeScript, Python, Rust, Go, etc. (19 languages)
- AI-only: All other languages require AI provider configuration

**Check logs**:
- Zed: `~/.local/share/zed/logs/Zed.log`
- VS Code: Output panel → Language Server
- Neovim: `:LspLog`

### Build Errors

**Update Rust**:
```bash
rustup update stable
```

**Clean build**:
```bash
cargo clean
cargo build --release
```

---

## Next Steps

- Read [LANGUAGES.md](LANGUAGES.md) for complete language support matrix
- See [ARCHITECTURE.md](ARCHITECTURE.md) for system design
- Check [examples/](../examples/) for code samples
- View [API docs](https://valknarthing.github.io/universal-lsp/) for detailed API reference

---

## Getting Help

- **Issues**: https://github.com/valknarthing/universal-lsp/issues
- **Documentation**: https://valknarthing.github.io/universal-lsp/
- **Examples**: `examples/` directory in the repository
