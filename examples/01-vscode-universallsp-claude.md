# VSCode + Universal LSP + Claude Integration Guide

## Overview

This guide shows you how to integrate the Universal LSP Server with Claude AI in Visual Studio Code for AI-powered code intelligence across 242+ programming languages.

## Architecture

```
VSCode → Universal LSP Server → MCP Pipeline → Claude API
                ↓
        Specialized LSP Servers (optional)
        - rust-analyzer
        - pyright
        - tsserver
```

## Prerequisites

- Visual Studio Code 1.80+
- Node.js 18+ (for Claude MCP server)
- Rust (for building Universal LSP)
- Claude API Key from Anthropic

## Step 1: Build Universal LSP Server

```bash
cd /home/valknar/Projects/zed/universal-lsp

# Build the LSP server
cargo build --release

# The binary will be at: target/release/universal-lsp
```

## Step 2: Create Claude MCP Server

Create a simple MCP server that interfaces with Claude:

```bash
mkdir -p ~/claude-mcp-server
cd ~/claude-mcp-server
npm init -y
npm install express @anthropic-ai/sdk cors body-parser
```

Create `server.js`:

```javascript
const express = require('express');
const Anthropic = require('@anthropic-ai/sdk');
const cors = require('cors');
const bodyParser = require('body-parser');

const app = express();
app.use(cors());
app.use(bodyParser.json());

const anthropic = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({ status: 'ok' });
});

// MCP completion endpoint
app.post('/', async (req, res) => {
  try {
    const { request_type, uri, position, context } = req.body;

    let prompt = '';
    if (request_type === 'completion') {
      prompt = `Suggest code completions for the file ${uri} at line ${position.line}. Context: ${context || 'none'}`;
    } else if (request_type === 'hover') {
      prompt = `Provide documentation for the symbol at ${uri}:${position.line}:${position.character}`;
    }

    const message = await anthropic.messages.create({
      model: 'claude-sonnet-4-20250514',
      max_tokens: 1024,
      messages: [{
        role: 'user',
        content: prompt
      }]
    });

    const suggestions = message.content[0].text.split('\n').filter(s => s.trim());

    res.json({
      suggestions,
      documentation: message.content[0].text,
      confidence: 0.85
    });
  } catch (error) {
    console.error('Error:', error);
    res.status(500).json({ error: error.message });
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`Claude MCP Server running on port ${PORT}`);
});
```

Create `.env`:

```bash
ANTHROPIC_API_KEY=your_claude_api_key_here
PORT=3000
```

Start the server:

```bash
npm install dotenv
node -r dotenv/config server.js
```

## Step 3: Configure VSCode

Create `.vscode/settings.json` in your project:

```json
{
  "universal-lsp.server.path": "/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp",
  "universal-lsp.server.args": [
    "--log-level=info",
    "--mcp-pre=http://localhost:3000",
    "--mcp-timeout=5000",
    "--mcp-cache=true",
    "--max-concurrent=200"
  ],
  "universal-lsp.trace.server": "verbose"
}
```

## Step 4: Create VSCode Extension Configuration

Since Universal LSP needs to be registered, create a minimal extension wrapper:

Create `package.json`:

```json
{
  "name": "universal-lsp-vscode",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.80.0"
  },
  "activationEvents": [
    "onLanguage:*"
  ],
  "main": "./extension.js",
  "contributes": {
    "configuration": {
      "type": "object",
      "title": "Universal LSP",
      "properties": {
        "universal-lsp.server.path": {
          "type": "string",
          "default": "universal-lsp",
          "description": "Path to Universal LSP server binary"
        }
      }
    }
  }
}
```

Create `extension.js`:

```javascript
const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

function activate(context) {
  const config = vscode.workspace.getConfiguration('universal-lsp');
  const serverPath = config.get('server.path');
  const serverArgs = config.get('server.args') || [];

  const serverOptions = {
    command: serverPath,
    args: serverArgs
  };

  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: '*' }],
  };

  client = new LanguageClient(
    'universal-lsp',
    'Universal LSP Server',
    serverOptions,
    clientOptions
  );

  client.start();
}

function deactivate() {
  if (client) {
    return client.stop();
  }
}

module.exports = { activate, deactivate };
```

Install dependencies:

```bash
npm install vscode-languageclient
```

## Step 5: Advanced Configuration - Multi-Language Proxying

For production use with specialized LSP servers:

```json
{
  "universal-lsp.server.args": [
    "--log-level=info",
    "--mcp-pre=http://localhost:3000",
    "--mcp-post=http://localhost:4000",
    "--lsp-proxy=python=pyright",
    "--lsp-proxy=rust=rust-analyzer",
    "--lsp-proxy=typescript=typescript-language-server",
    "--lsp-proxy=go=gopls",
    "--mcp-timeout=3000",
    "--mcp-cache=true"
  ]
}
```

## Step 6: Testing

1. Open a Python file in VSCode
2. Trigger code completion (Ctrl+Space)
3. You should see Claude-enhanced suggestions
4. Hover over a symbol to see AI-powered documentation

## Troubleshooting

### LSP Server Not Starting

Check VSCode Output panel → Universal LSP Server:

```bash
# Manually test the server
/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --log-level=debug \
  --mcp-pre=http://localhost:3000
```

### MCP Server Connection Issues

Test the MCP server directly:

```bash
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{
    "request_type": "completion",
    "uri": "test.py",
    "position": {"line": 10, "character": 5},
    "context": "def hello():"
  }'
```

### Enable Debug Logging

```json
{
  "universal-lsp.server.args": [
    "--log-level=debug",
    "--log-requests"
  ],
  "universal-lsp.trace.server": "verbose"
}
```

## Performance Optimization

### 1. Enable Caching

```json
{
  "universal-lsp.server.args": [
    "--mcp-cache=true",
    "--mcp-timeout=3000"
  ]
}
```

### 2. Limit Concurrent Requests

```json
{
  "universal-lsp.server.args": [
    "--max-concurrent=100"
  ]
}
```

### 3. Use MCP Post-Processing Only for Ranking

```json
{
  "universal-lsp.server.args": [
    "--mcp-post=http://localhost:4000",
    "--mcp-timeout=2000"
  ]
}
```

## Advanced Example: Full Stack Development

For a project with Python backend and TypeScript frontend:

```json
{
  "universal-lsp.server.args": [
    "--lsp-proxy=python=pyright",
    "--lsp-proxy=typescript=typescript-language-server",
    "--mcp-pre=http://localhost:3000",
    "--mcp-post=http://localhost:4000",
    "--log-level=info",
    "--mcp-cache=true",
    "--max-concurrent=200"
  ]
}
```

## Resources

- [Universal LSP Documentation](../README.md)
- [Claude API Documentation](https://docs.anthropic.com/claude/reference)
- [VSCode LSP Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)
