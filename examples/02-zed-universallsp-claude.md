# Zed + Universal LSP + Claude Integration Guide

## Overview

Integrate Universal LSP Server with Claude AI in Zed editor for blazing-fast, AI-powered code intelligence across 242+ languages.

## Architecture

```
Zed Editor → Universal LSP Server → MCP Pipeline → Claude API
                      ↓
              Local Language Detection
              + Specialized LSP Proxies
```

## Prerequisites

- Zed Editor (latest version)
- Rust toolchain
- Node.js 18+ (for Claude MCP server)
- Claude API Key

## Step 1: Build Universal LSP

```bash
cd /home/valknar/Projects/zed/universal-lsp
cargo build --release

# Binary location: target/release/universal-lsp
```

## Step 2: Set Up Claude MCP Server

Create `~/claude-mcp/server.js`:

```javascript
const express = require('express');
const Anthropic = require('@anthropic-ai/sdk');
const app = express();

app.use(express.json());

const anthropic = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

// Health endpoint
app.get('/health', (req, res) => res.json({ status: 'healthy' }));

// Main MCP endpoint
app.post('/', async (req, res) => {
  const { request_type, uri, position, context } = req.body;

  try {
    let systemPrompt = '';
    let userPrompt = '';

    if (request_type === 'completion') {
      systemPrompt = 'You are an expert code completion assistant. Provide concise, accurate code suggestions.';
      userPrompt = `File: ${uri}\nLine: ${position.line}\nContext: ${context || ''}\n\nProvide 3-5 relevant code completion suggestions.`;
    } else if (request_type === 'hover') {
      systemPrompt = 'You are a code documentation expert. Provide clear, helpful documentation.';
      userPrompt = `Explain the code at ${uri}:${position.line}:${position.character}`;
    }

    const message = await anthropic.messages.create({
      model: 'claude-sonnet-4-20250514',
      max_tokens: 512,
      system: systemPrompt,
      messages: [{ role: 'user', content: userPrompt }]
    });

    const text = message.content[0].text;
    const suggestions = text.split('\n')
      .filter(line => line.trim())
      .slice(0, 10);

    res.json({
      suggestions,
      documentation: text,
      confidence: 0.9
    });
  } catch (error) {
    console.error('MCP Error:', error);
    res.status(500).json({
      error: error.message,
      suggestions: [],
      documentation: null
    });
  }
});

const PORT = 3000;
app.listen(PORT, () => {
  console.log(`🚀 Claude MCP Server running on http://localhost:${PORT}`);
});
```

Install and run:

```bash
cd ~/claude-mcp
npm install express @anthropic-ai/sdk
ANTHROPIC_API_KEY=your_key_here node server.js
```

Or use PM2 for persistence:

```bash
pm2 start server.js --name claude-mcp --env ANTHROPIC_API_KEY=your_key_here
pm2 save
```

## Step 3: Configure Zed

Edit `~/.config/zed/settings.json`:

```json
{
  "lsp": {
    "universal-lsp": {
      "binary": {
        "path": "/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp",
        "arguments": [
          "--log-level=info",
          "--mcp-pre=http://localhost:3000",
          "--mcp-timeout=3000",
          "--mcp-cache=true",
          "--max-concurrent=150"
        ]
      },
      "settings": {
        "enable": true
      }
    }
  },
  "languages": {
    "Python": {
      "language_servers": ["universal-lsp", "..."]
    },
    "JavaScript": {
      "language_servers": ["universal-lsp", "..."]
    },
    "TypeScript": {
      "language_servers": ["universal-lsp", "..."]
    },
    "Rust": {
      "language_servers": ["universal-lsp", "..."]
    }
  }
}
```

## Step 4: Advanced Configuration - Multi-Language Setup

For projects with multiple languages:

```json
{
  "lsp": {
    "universal-lsp": {
      "binary": {
        "path": "/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp",
        "arguments": [
          "--log-level=info",
          "--mcp-pre=http://localhost:3000",
          "--mcp-post=http://localhost:4000",
          "--lsp-proxy=python=pyright",
          "--lsp-proxy=rust=rust-analyzer",
          "--lsp-proxy=typescript=typescript-language-server",
          "--lsp-proxy=go=gopls",
          "--mcp-timeout=2500",
          "--mcp-cache=true",
          "--max-concurrent=200"
        ]
      }
    }
  }
}
```

## Step 5: Per-Project Configuration

Create `.zed/settings.json` in your project root:

```json
{
  "lsp": {
    "universal-lsp": {
      "binary": {
        "arguments": [
          "--log-level=debug",
          "--mcp-pre=http://localhost:3000",
          "--log-requests"
        ]
      }
    }
  }
}
```

## Step 6: Testing the Integration

1. **Open a file in Zed**
   ```bash
   zed test.py
   ```

2. **Trigger Completion**
   - Type some code
   - Press `Ctrl+Space` (or your completion hotkey)
   - You should see Claude-enhanced suggestions

3. **Hover Documentation**
   - Hover over a function or variable
   - You'll see AI-powered documentation

4. **Check LSP Status**
   - Open Command Palette (`Cmd/Ctrl+Shift+P`)
   - Type "LSP Status"
   - Verify `universal-lsp` is running

## Performance Tuning

### 1. Optimize MCP Timeouts

For faster responses:

```json
{
  "arguments": [
    "--mcp-timeout=2000",
    "--max-concurrent=200"
  ]
}
```

### 2. Enable Aggressive Caching

```json
{
  "arguments": [
    "--mcp-cache=true",
    "--mcp-timeout=5000"
  ]
}
```

### 3. Use Post-Processing Only

For better performance, only use MCP for ranking:

```json
{
  "arguments": [
    "--mcp-post=http://localhost:4000",
    "--lsp-proxy=python=pyright",
    "--lsp-proxy=typescript=typescript-language-server"
  ]
}
```

## Troubleshooting

### LSP Server Not Starting

Check Zed logs:

```bash
tail -f ~/.local/share/zed/logs/Zed.log | grep universal-lsp
```

Test server manually:

```bash
/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --log-level=debug
```

### MCP Connection Timeout

1. Check if MCP server is running:
   ```bash
   curl http://localhost:3000/health
   ```

2. Increase timeout:
   ```json
   {
     "arguments": ["--mcp-timeout=10000"]
   }
   ```

### No Completions Appearing

1. Enable debug logging:
   ```json
   {
     "arguments": [
       "--log-level=debug",
       "--log-requests"
     ]
   }
   ```

2. Check Zed LSP panel (View → LSP Logs)

## Example Workflow: Full-Stack Project

For a SvelteKit + Python project:

```json
{
  "lsp": {
    "universal-lsp": {
      "binary": {
        "path": "/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp",
        "arguments": [
          "--lsp-proxy=python=pyright",
          "--lsp-proxy=typescript=typescript-language-server",
          "--lsp-proxy=svelte=svelteserver",
          "--mcp-pre=http://localhost:3000",
          "--mcp-post=http://localhost:4000",
          "--mcp-cache=true",
          "--log-level=info"
        ]
      }
    }
  },
  "languages": {
    "Python": {
      "language_servers": ["universal-lsp"]
    },
    "TypeScript": {
      "language_servers": ["universal-lsp"]
    },
    "Svelte": {
      "language_servers": ["universal-lsp"]
    }
  }
}
```

## Keyboard Shortcuts

Enhance your workflow with these Zed shortcuts:

```json
{
  "bindings": {
    "ctrl-space": "editor::ShowCompletions",
    "cmd-k cmd-i": "editor::Hover",
    "f12": "editor::GoToDefinition",
    "shift-f12": "editor::FindAllReferences"
  }
}
```

## Monitoring and Debugging

### Enable Detailed Logging

```bash
# Start Zed with verbose logging
RUST_LOG=debug zed
```

### Monitor MCP Requests

Add logging to your MCP server:

```javascript
app.use((req, res, next) => {
  console.log(`${new Date().toISOString()} ${req.method} ${req.path}`);
  console.log('Body:', JSON.stringify(req.body, null, 2));
  next();
});
```

### Performance Metrics

Check response times:

```bash
tail -f ~/.local/share/zed/logs/Zed.log | grep -i "universal-lsp\|mcp"
```

## Resources

- [Zed LSP Documentation](https://zed.dev/docs/extensions/languages)
- [Universal LSP README](../README.md)
- [Claude API Docs](https://docs.anthropic.com/)
