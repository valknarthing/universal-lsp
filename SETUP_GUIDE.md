# Development Environment Setup Guide

## Interactive Setup Script

The `dev-env-setup.sh` script provides an interactive way to configure your development environment with Universal LSP support.

### Features

- **Multi-IDE Support**: Zed, VSCode, Neovim, or custom configuration
- **AI Integration**: Claude, GitHub Copilot, or both
- **Project Templates**: Quick scaffolding for popular stacks
- **Workspace Configuration**: Automatic `.universal-lsp.toml` generation
- **Smart Defaults**: Sensible configurations out of the box

### Quick Start

```bash
# Run the interactive setup
./dev-env-setup.sh
```

### Supported Project Types

1. **Svelte 5 + Tailwind 4** - Modern SPA with the latest versions
2. **React + TypeScript** - Industry-standard React stack
3. **Vue 3 + Vite** - Progressive framework with Vite
4. **Next.js** - React with SSR and routing
5. **Python (FastAPI)** - Modern async Python backend
6. **Rust** - Systems programming with Cargo
7. **Go** - Efficient backend development
8. **Full-stack Node.js** - TypeScript-based backend
9. **Custom** - Manual configuration

### IDE Configurations

#### Zed

Creates `~/.config/zed/settings.json` with:
- Universal LSP integration
- Language-specific formatters
- AI provider configuration
- Optimized keybindings

#### VSCode

Creates `.vscode/` directory with:
- `settings.json` - Editor preferences
- `extensions.json` - Recommended extensions
- Language-specific configurations

#### Neovim

Manual configuration required. Recommended plugins:
- `neovim/nvim-lspconfig` - LSP client
- `jose-elias-alvarez/null-ls.nvim` - Formatting/linting
- `hrsh7th/nvim-cmp` - Autocompletion

### Workspace Configuration

The script generates `.universal-lsp.toml` in your project root:

```toml
# Universal LSP Workspace Configuration
[workspace]
name = "my-project"
root = "."

[formatting]
indent_size = 2
use_tabs = false
max_line_length = 100

[languages.javascript]
indent_size = 2
formatter = "prettier"

[languages.rust]
indent_size = 4
formatter = "rustfmt"

[excluded_paths]
patterns = [
  "**/node_modules/**",
  "**/target/**",
  "**/.git/**"
]
```

### Multi-root Workspace Setup

For projects with multiple folders (e.g., monorepo):

1. Select "Yes" when prompted for multi-root workspace
2. The script creates a workspace configuration
3. Each subfolder can have its own `.universal-lsp.toml`

Example structure:
```
my-project/
├── .universal-lsp.toml       # Root workspace config
├── frontend/
│   ├── .universal-lsp.toml   # Frontend-specific config
│   └── src/
└── backend/
    ├── .universal-lsp.toml   # Backend-specific config
    └── src/
```

### AI Provider Setup

#### Claude (Anthropic)

1. Get API key from https://console.anthropic.com
2. Configure in your IDE:

**Zed:**
Add to `~/.config/zed/settings.json`:
```json
{
  "assistant": {
    "version": "2",
    "provider": {
      "name": "anthropic",
      "api_key": "your-api-key-here"
    }
  }
}
```

**VSCode:**
Install extension: `anthropics.claude-code`

#### GitHub Copilot

1. Sign up at https://github.com/features/copilot
2. Install IDE extension

**Zed:** Built-in support, sign in via GitHub
**VSCode:** Install `github.copilot` extension

### Project Scaffolding Examples

#### Svelte + Tailwind

```bash
# Script creates:
my-project/
├── src/
│   ├── App.svelte
│   ├── app.css          # Tailwind directives
│   └── main.ts
├── public/
├── package.json
├── tailwind.config.js   # Tailwind 4 config
├── vite.config.ts
└── tsconfig.json
```

#### Python (FastAPI)

```bash
# Script creates:
my-project/
├── main.py             # FastAPI app
├── requirements.txt    # Dependencies
├── venv/              # Virtual environment
└── .gitignore
```

#### Rust

```bash
# Script creates:
my-project/
├── src/
│   └── main.rs
├── Cargo.toml
└── .gitignore
```

### Manual Setup (Custom)

If you select "Custom" project type:

1. Create your project structure manually
2. Copy example configurations from this guide
3. Adjust `.universal-lsp.toml` to your needs
4. Configure your IDE manually

### Troubleshooting

#### Universal LSP not starting

1. Build the LSP server:
   ```bash
   cargo build --release
   ```

2. Check the binary path in your IDE config

3. Verify LSP server is in PATH or use absolute path

#### Formatter not working

1. Install the formatter globally:
   ```bash
   npm install -g prettier  # JavaScript/TypeScript
   pip install black        # Python
   rustup component add rustfmt  # Rust
   ```

2. Verify formatter path in `.universal-lsp.toml`

#### AI provider not responding

1. Verify API key is set correctly
2. Check network connectivity
3. Review IDE logs for errors

### Advanced Configuration

#### Custom Language Support

Add custom languages to `.universal-lsp.toml`:

```toml
[languages.mylang]
indent_size = 4
formatter = "mylang-fmt"
linter = "mylang-lint"
```

#### Excluded Paths

Fine-tune exclusions:

```toml
[excluded_paths]
patterns = [
  "**/node_modules/**",
  "**/target/**",
  "**/dist/**",
  "**/__pycache__/**",
  "**/.pytest_cache/**",
  "**/.mypy_cache/**"
]
```

#### Multiple Formatters

Configure fallback formatters:

```toml
[languages.javascript]
indent_size = 2
formatter = "prettier"
fallback_formatter = "eslint"
```

### Environment-Specific Configs

Use different configs per environment:

```bash
# Development
.universal-lsp.dev.toml

# Production
.universal-lsp.prod.toml

# Testing
.universal-lsp.test.toml
```

Symlink the active config:
```bash
ln -s .universal-lsp.dev.toml .universal-lsp.toml
```

### CI/CD Integration

Add Universal LSP checks to your pipeline:

```yaml
# GitHub Actions example
- name: Check formatting
  run: |
    cargo run --bin universal-lsp -- format --check
```

### Next Steps

1. **Build Universal LSP**: `cargo build --release`
2. **Open your project**: Launch your configured IDE
3. **Test features**: Try hover, completion, formatting
4. **Customize**: Adjust `.universal-lsp.toml` to your preferences

### Resources

- Universal LSP Documentation: [Link to docs]
- Example Projects: `examples/` directory
- Issue Tracker: GitHub Issues
- Community: Discord/Matrix

### Contributing

Help improve this setup script:

1. Fork the repository
2. Add new project templates to `dev-env-setup.sh`
3. Submit a pull request

### License

MIT - See LICENSE file for details
