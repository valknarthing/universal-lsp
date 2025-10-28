# SvelteKit + Tailwind Stack + Universal LSP + Claude Integration Guide

## Overview

Complete integration guide for full-stack development with SvelteKit, Tailwind CSS 4, Universal LSP Server, and Claude AI. This guide demonstrates AI-powered development across multiple languages (TypeScript, Svelte, Python, JavaScript) with intelligent code completion, formatting, and debugging.

## Architecture

```
SvelteKit Frontend (TypeScript/Svelte) ──┐
Python Backend (FastAPI)                 ├──→ Universal LSP Server
Tailwind CSS 4 (PostCSS)                ─┘         ↓
                                             MCP Pipeline
                                                   ↓
                                             Claude API
                                                   ↓
                                    AI-Powered Completions & Docs
```

## Stack Components

- **Frontend**: SvelteKit 2.x + Svelte 4/5
- **Styling**: Tailwind CSS 4 + PostCSS
- **Backend**: Python 3.11+ with FastAPI
- **Database**: PostgreSQL 15+ (optional)
- **Build**: Vite 5.x
- **Testing**: Vitest + Playwright

## Prerequisites

- Node.js 18+ (via nvm)
- Python 3.11+ (via pyenv)
- pnpm 8+
- Universal LSP built and configured
- Claude MCP Server running
- PostgreSQL (optional)

## Step 1: Project Setup

```bash
# Create project directory
mkdir my-fullstack-app
cd my-fullstack-app

# Initialize SvelteKit
pnpm create svelte@latest frontend
cd frontend
pnpm install

# Add Tailwind CSS 4
pnpm add -D tailwindcss@next @tailwindcss/vite@next
pnpm add -D autoprefixer postcss

# Initialize Tailwind
pnpm tailwindcss init

cd ..

# Setup Python backend
mkdir backend
cd backend
python -m venv venv
source venv/bin/activate
pip install fastapi uvicorn sqlalchemy alembic pydantic
pip freeze > requirements.txt

cd ..
```

## Step 2: Configure Tailwind CSS 4

### frontend/tailwind.config.js

```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#f0f9ff',
          100: '#e0f2fe',
          500: '#0ea5e9',
          900: '#0c4a6e',
        },
      },
    },
  },
  plugins: [],
};
```

### frontend/vite.config.ts

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    sveltekit(),
    tailwindcss(),
  ],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:8000',
        changeOrigin: true,
      },
    },
  },
});
```

### frontend/src/app.css

```css
@import 'tailwindcss';
```

### frontend/src/routes/+layout.svelte

```svelte
<script lang="ts">
  import '../app.css';
</script>

<slot />
```

## Step 3: Setup Claude MCP Server for Multi-Language Support

### ~/claude-mcp/server.js

```javascript
const express = require('express');
const Anthropic = require('@anthropic-ai/sdk');
const app = express();

app.use(express.json());

const anthropic = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

// Language-specific system prompts
const PROMPTS = {
  typescript: 'You are a TypeScript and SvelteKit expert. Provide type-safe, modern code suggestions.',
  svelte: 'You are a Svelte expert. Focus on component composition, reactivity, and best practices.',
  python: 'You are a Python and FastAPI expert. Provide async-first, type-hinted code suggestions.',
  javascript: 'You are a JavaScript expert. Provide modern ES2023+ code suggestions.',
  css: 'You are a Tailwind CSS and modern CSS expert. Focus on utility-first patterns.',
};

// Detect language from file URI
function detectLanguage(uri) {
  if (uri.endsWith('.ts')) return 'typescript';
  if (uri.endsWith('.svelte')) return 'svelte';
  if (uri.endsWith('.py')) return 'python';
  if (uri.endsWith('.js') || uri.endsWith('.jsx')) return 'javascript';
  if (uri.endsWith('.css')) return 'css';
  return 'typescript'; // default
}

// Main MCP endpoint
app.post('/', async (req, res) => {
  const { request_type, uri, position, context } = req.body;

  try {
    const language = detectLanguage(uri);
    let systemPrompt = PROMPTS[language] || PROMPTS.typescript;
    let userPrompt = '';

    if (request_type === 'completion') {
      systemPrompt += ' Provide 5-10 relevant code completions. Be concise.';
      userPrompt = `File: ${uri}\nLine: ${position.line}\nContext: ${context || ''}\n\nProvide completions:`;
    } else if (request_type === 'hover') {
      systemPrompt += ' Provide clear, helpful documentation.';
      userPrompt = `Explain the code at ${uri}:${position.line}:${position.character}`;
    }

    const message = await anthropic.messages.create({
      model: 'claude-sonnet-4-20250514',
      max_tokens: 1024,
      system: systemPrompt,
      messages: [{ role: 'user', content: userPrompt }]
    });

    const text = message.content[0].text;
    const suggestions = text.split('\n')
      .filter(line => line.trim())
      .slice(0, 15);

    res.json({
      suggestions,
      documentation: text,
      confidence: 0.9,
      language,
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

// Health check
app.get('/health', (req, res) => res.json({ status: 'healthy' }));

const PORT = 3000;
app.listen(PORT, () => {
  console.log(`🚀 Multi-Language Claude MCP Server running on http://localhost:${PORT}`);
});
```

Start MCP server:

```bash
cd ~/claude-mcp
npm install express @anthropic-ai/sdk
ANTHROPIC_API_KEY=your_key node server.js
```

## Step 4: Configure Zed for Full-Stack Development

### ~/.config/zed/settings.json

```json
{
  "lsp": {
    "universal-lsp": {
      "binary": {
        "path": "/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp",
        "arguments": [
          "--log-level=info",
          "--mcp-pre=http://localhost:3000",
          "--lsp-proxy=typescript=typescript-language-server",
          "--lsp-proxy=python=pyright",
          "--lsp-proxy=svelte=svelteserver",
          "--mcp-timeout=3000",
          "--mcp-cache=true",
          "--max-concurrent=200"
        ]
      },
      "settings": {
        "enable": true
      }
    }
  },
  "languages": {
    "TypeScript": {
      "language_servers": ["universal-lsp", "typescript-language-server"],
      "format_on_save": "on",
      "formatter": "prettier"
    },
    "Svelte": {
      "language_servers": ["universal-lsp", "svelteserver"],
      "format_on_save": "on"
    },
    "Python": {
      "language_servers": ["universal-lsp", "pyright"],
      "format_on_save": "on",
      "formatter": "black"
    },
    "JavaScript": {
      "language_servers": ["universal-lsp"],
      "format_on_save": "on",
      "formatter": "prettier"
    }
  },
  "prettier": {
    "printWidth": 100,
    "singleQuote": true,
    "trailingComma": "es5",
    "plugins": ["prettier-plugin-svelte", "prettier-plugin-tailwindcss"]
  }
}
```

## Step 5: VSCode Alternative Configuration

### .vscode/settings.json

```json
{
  "universal-lsp.server.path": "/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp",
  "universal-lsp.server.args": [
    "--log-level=info",
    "--mcp-pre=http://localhost:3000",
    "--lsp-proxy=typescript=typescript-language-server",
    "--lsp-proxy=python=pyright",
    "--lsp-proxy=svelte=svelteserver",
    "--mcp-timeout=3000",
    "--mcp-cache=true"
  ],
  "editor.formatOnSave": true,
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "[svelte]": {
    "editor.defaultFormatter": "svelte.svelte-vscode"
  },
  "[python]": {
    "editor.defaultFormatter": "ms-python.black-formatter"
  },
  "tailwindCSS.experimental.classRegex": [
    ["class:\\s*[\"'`]([^\"'`]*).*?[\"'`]", "([a-zA-Z0-9\\-:]+)"]
  ]
}
```

## Step 6: Example SvelteKit Frontend

### frontend/src/routes/+page.svelte

```svelte
<script lang="ts">
  import { onMount } from 'svelte';

  interface Todo {
    id: number;
    title: string;
    completed: boolean;
  }

  let todos: Todo[] = [];
  let newTodo = '';
  let loading = false;

  async function fetchTodos() {
    loading = true;
    try {
      const res = await fetch('/api/todos');
      todos = await res.json();
    } catch (error) {
      console.error('Failed to fetch todos:', error);
    } finally {
      loading = false;
    }
  }

  async function addTodo() {
    if (!newTodo.trim()) return;

    try {
      const res = await fetch('/api/todos', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title: newTodo }),
      });

      if (res.ok) {
        const todo = await res.json();
        todos = [...todos, todo];
        newTodo = '';
      }
    } catch (error) {
      console.error('Failed to add todo:', error);
    }
  }

  async function toggleTodo(id: number) {
    const todo = todos.find(t => t.id === id);
    if (!todo) return;

    try {
      const res = await fetch(`/api/todos/${id}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ completed: !todo.completed }),
      });

      if (res.ok) {
        todos = todos.map(t =>
          t.id === id ? { ...t, completed: !t.completed } : t
        );
      }
    } catch (error) {
      console.error('Failed to toggle todo:', error);
    }
  }

  onMount(() => {
    fetchTodos();
  });
</script>

<div class="min-h-screen bg-gradient-to-br from-brand-50 to-brand-100 py-12 px-4">
  <div class="max-w-2xl mx-auto">
    <h1 class="text-4xl font-bold text-brand-900 mb-8">
      SvelteKit + Tailwind + Claude LSP
    </h1>

    <div class="bg-white rounded-lg shadow-lg p-6 mb-6">
      <form on:submit|preventDefault={addTodo} class="flex gap-2">
        <input
          type="text"
          bind:value={newTodo}
          placeholder="Add a new todo..."
          class="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-brand-500"
        />
        <button
          type="submit"
          class="px-6 py-2 bg-brand-500 text-white rounded-lg hover:bg-brand-600 transition"
        >
          Add
        </button>
      </form>
    </div>

    {#if loading}
      <div class="text-center py-8">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-brand-500 mx-auto"></div>
      </div>
    {:else}
      <div class="space-y-2">
        {#each todos as todo (todo.id)}
          <div
            class="bg-white rounded-lg shadow p-4 flex items-center gap-3 hover:shadow-md transition"
          >
            <input
              type="checkbox"
              checked={todo.completed}
              on:change={() => toggleTodo(todo.id)}
              class="w-5 h-5 text-brand-500 rounded focus:ring-brand-500"
            />
            <span
              class:line-through={todo.completed}
              class:text-gray-400={todo.completed}
              class="flex-1"
            >
              {todo.title}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
```

## Step 7: Example Python FastAPI Backend

### backend/main.py

```python
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import List, Optional
import uvicorn

app = FastAPI(title="SvelteKit Backend API")

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# In-memory storage (replace with database in production)
todos_db: List[dict] = []
next_id = 1

class TodoCreate(BaseModel):
    title: str

class TodoUpdate(BaseModel):
    title: Optional[str] = None
    completed: Optional[bool] = None

class Todo(BaseModel):
    id: int
    title: str
    completed: bool

@app.get("/")
async def root():
    return {"message": "SvelteKit + FastAPI + Universal LSP API"}

@app.get("/api/todos", response_model=List[Todo])
async def get_todos():
    """Get all todos"""
    return todos_db

@app.post("/api/todos", response_model=Todo, status_code=201)
async def create_todo(todo: TodoCreate):
    """Create a new todo"""
    global next_id
    new_todo = {
        "id": next_id,
        "title": todo.title,
        "completed": False,
    }
    todos_db.append(new_todo)
    next_id += 1
    return new_todo

@app.patch("/api/todos/{todo_id}", response_model=Todo)
async def update_todo(todo_id: int, update: TodoUpdate):
    """Update a todo"""
    for todo in todos_db:
        if todo["id"] == todo_id:
            if update.title is not None:
                todo["title"] = update.title
            if update.completed is not None:
                todo["completed"] = update.completed
            return todo
    raise HTTPException(status_code=404, detail="Todo not found")

@app.delete("/api/todos/{todo_id}")
async def delete_todo(todo_id: int):
    """Delete a todo"""
    global todos_db
    todos_db = [t for t in todos_db if t["id"] != todo_id]
    return {"message": "Todo deleted"}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000, reload=True)
```

## Step 8: Development Workflow

Create a development script `dev.sh`:

```bash
#!/bin/bash

set -e

echo "🚀 Starting Full-Stack Development Environment"
echo "=============================================="

# Start Claude MCP Server
echo "📡 Starting Claude MCP Server..."
cd ~/claude-mcp
ANTHROPIC_API_KEY=$(cat ~/.anthropic-api-key) node server.js > /tmp/mcp-server.log 2>&1 &
MCP_PID=$!
echo "   MCP Server PID: $MCP_PID"

# Wait for MCP server to be ready
sleep 2
if curl -s http://localhost:3000/health > /dev/null; then
    echo "   ✓ MCP Server healthy"
else
    echo "   ✗ MCP Server failed to start"
    exit 1
fi

# Start Python backend
echo "🐍 Starting FastAPI Backend..."
cd backend
source venv/bin/activate
python main.py > /tmp/backend.log 2>&1 &
BACKEND_PID=$!
echo "   Backend PID: $BACKEND_PID"

# Wait for backend
sleep 2
if curl -s http://localhost:8000 > /dev/null; then
    echo "   ✓ Backend healthy"
else
    echo "   ✗ Backend failed to start"
    exit 1
fi

# Start SvelteKit frontend
echo "⚡ Starting SvelteKit Frontend..."
cd ../frontend
pnpm dev &
FRONTEND_PID=$!
echo "   Frontend PID: $FRONTEND_PID"

echo ""
echo "✨ Development Environment Ready!"
echo "================================="
echo "Frontend:  http://localhost:5173"
echo "Backend:   http://localhost:8000"
echo "MCP:       http://localhost:3000"
echo ""
echo "Press Ctrl+C to stop all services..."

# Cleanup on exit
trap "echo 'Stopping services...'; kill $MCP_PID $BACKEND_PID $FRONTEND_PID 2>/dev/null; exit" INT

# Wait
wait
```

Make it executable and run:

```bash
chmod +x dev.sh
./dev.sh
```

## Step 9: Testing with Claude-Enhanced Completions

Open files in Zed and test:

### TypeScript Completions

Open `frontend/src/lib/api.ts` and type:

```typescript
export async function fetchData<T>(
  endpoint: string
): Promise<T> {
  // Claude will suggest fetch implementation
  // Try typing "const res = " and wait for suggestions
}
```

### Svelte Component Completions

Open a `.svelte` file and type:

```svelte
<script lang="ts">
  // Try typing "import { " and see Claude suggestions
  // Try typing "let " for reactive variable suggestions
</script>
```

### Python FastAPI Completions

Open `backend/main.py` and type:

```python
@app.get("/api/stats")
async def get_stats():
    # Try typing "return {" and see Claude suggestions
    # Claude knows your database schema and API patterns
```

## Step 10: Advanced Configuration

### Per-Project LSP Settings

Create `.zed/settings.json` in project root:

```json
{
  "lsp": {
    "universal-lsp": {
      "binary": {
        "arguments": [
          "--log-level=debug",
          "--mcp-pre=http://localhost:3000",
          "--lsp-proxy=typescript=typescript-language-server --stdio",
          "--lsp-proxy=python=pyright",
          "--lsp-proxy=svelte=svelteserver --stdio",
          "--mcp-cache=true",
          "--max-concurrent=200"
        ]
      }
    }
  }
}
```

### Tailwind IntelliSense Enhancement

Universal LSP can enhance Tailwind suggestions:

```json
{
  "lsp": {
    "universal-lsp": {
      "settings": {
        "tailwind": {
          "enabled": true,
          "config": "./frontend/tailwind.config.js"
        }
      }
    }
  }
}
```

## Troubleshooting

### MCP Server Not Responding

```bash
# Check MCP server logs
tail -f /tmp/mcp-server.log

# Test MCP manually
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{"request_type":"completion","uri":"test.ts","position":{"line":1,"character":0},"context":"import"}'
```

### Backend CORS Issues

```python
# Update backend/main.py CORS settings
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # For development only
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

### TypeScript Server Conflicts

```bash
# Check running language servers
ps aux | grep -E "typescript-language-server|universal-lsp"

# Kill conflicting servers
pkill -f typescript-language-server
```

## Performance Optimization

### 1. Enable Aggressive Caching

```bash
# In LSP arguments
--mcp-cache=true
--mcp-timeout=2000
```

### 2. Limit MCP to Specific Languages

Update MCP server to ignore CSS/HTML:

```javascript
if (uri.endsWith('.css') || uri.endsWith('.html')) {
  return res.json({ suggestions: [], documentation: null });
}
```

### 3. Use Post-Processing Only

```bash
# Only use MCP for ranking, not generation
--mcp-post=http://localhost:3000
--lsp-proxy=typescript=typescript-language-server
```

## Production Deployment

### Build Frontend

```bash
cd frontend
pnpm build
pnpm preview  # Test production build
```

### Deploy Backend

```bash
cd backend
pip install gunicorn
gunicorn -w 4 -k uvicorn.workers.UvicornWorker main:app
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  mcp-server:
    build: ./claude-mcp
    ports:
      - "3000:3000"
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}

  backend:
    build: ./backend
    ports:
      - "8000:8000"
    depends_on:
      - mcp-server

  frontend:
    build: ./frontend
    ports:
      - "5173:5173"
    depends_on:
      - backend
```

## Resources

- [SvelteKit Documentation](https://kit.svelte.dev/docs)
- [Tailwind CSS 4](https://tailwindcss.com/docs)
- [FastAPI Documentation](https://fastapi.tiangolo.com/)
- [Universal LSP README](../README.md)
- [Claude API Docs](https://docs.anthropic.com/)
