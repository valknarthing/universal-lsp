# Quick Start: Development Environment Setup

Get your development environment configured in minutes with our interactive setup script!

## One-Command Setup

```bash
./dev-env-setup.sh
```

## What It Does

The script will guide you through:

1. **IDE Selection** - Zed, VSCode, Neovim, or custom
2. **AI Assistant** - Claude, GitHub Copilot, or both
3. **Project Type** - Svelte, React, Vue, Python, Rust, Go, and more
4. **Workspace Config** - Single or multi-root workspace
5. **Project Scaffolding** - Ready-to-code project structure

## Example Workflow

```bash
# Run the setup script
./dev-env-setup.sh

# Example selections:
# IDE: Zed (option 1)
# AI: Claude (option 1)  
# Project: Svelte + Tailwind (option 1)
# Name: my-awesome-app
# Multi-root: No

# Result: Complete environment ready in seconds!
```

## Features

✅ **Zero Configuration** - Sensible defaults for everything  
✅ **IDE Integration** - Automatic configuration files  
✅ **AI Ready** - Pre-configured for Claude & Copilot  
✅ **Project Templates** - 9 popular stack options  
✅ **Workspace Support** - Multi-root for monorepos  
✅ **Universal LSP** - Automatic integration  

## Supported Stacks

- **Frontend**: Svelte 5, React, Vue 3, Next.js
- **Backend**: Python (FastAPI), Rust, Go, Node.js
- **Full-stack**: TypeScript Node.js
- **Custom**: Bring your own configuration

## Requirements

- **Rust** (for Universal LSP)
- **Node.js** (for JavaScript projects)
- **Python 3** (for Python projects)
- **Go** (for Go projects)

## Documentation

📖 Full guide: [SETUP_GUIDE.md](SETUP_GUIDE.md)

## Quick Examples

### Svelte + Tailwind Project

```bash
./dev-env-setup.sh
# Select: Zed → Claude → Svelte+Tailwind → my-app
cd my-app
npm install
npm run dev
```

### Python FastAPI Backend

```bash
./dev-env-setup.sh
# Select: VSCode → Copilot → Python → api-server
cd api-server
source venv/bin/activate
pip install -r requirements.txt
uvicorn main:app --reload
```

### Rust CLI Tool

```bash
./dev-env-setup.sh
# Select: Zed → Claude → Rust → my-tool
cd my-tool
cargo run
```

## Customization

All generated configurations can be customized:

- **IDE config**: `~/.config/zed/settings.json` or `.vscode/settings.json`
- **Workspace**: `.universal-lsp.toml`
- **Project**: Standard project files (package.json, Cargo.toml, etc.)

## Troubleshooting

**Script won't run?**
```bash
chmod +x dev-env-setup.sh
```

**Need help?**
```bash
./dev-env-setup.sh --help
```

See [SETUP_GUIDE.md](SETUP_GUIDE.md) for detailed troubleshooting.

## Contributing

Add new project templates or IDE configurations:

1. Edit `dev-env-setup.sh`
2. Add your scaffold function
3. Update the menu options
4. Submit a PR

---

**Happy coding!** 🚀
