# ACP Quick Start Guide
## Get Claude Assisting You in 15 Minutes!

This is a condensed, action-focused guide to get ACP + Claude working ASAP.

---

## Prerequisites

```bash
# 1. Ensure you have Claude API access
export ANTHROPIC_API_KEY="sk-ant-api03-..."

# 2. Verify it's set
echo $ANTHROPIC_API_KEY

# 3. Make sure universal-lsp builds
cd /home/valknar/Projects/zed/universal-lsp
cargo build --release
```

---

## The Minimal Implementation Path

### Step 1: Integrate Claude Client (30 minutes)

**File**: `src/acp/mod.rs`

1. Add field to `UniversalAgent`:
```rust
use crate::ai::claude::{ClaudeClient, ClaudeConfig};

pub struct UniversalAgent {
    // ... existing fields ...
    claude_client: Option<ClaudeClient>,  // ADD THIS
    sessions: Arc<DashMap<String, Vec<(String, String)>>>,  // ADD THIS
}
```

2. Initialize Claude in constructors:
```rust
pub fn new(tx: mpsc::UnboundedSender<...>) -> Self {
    let claude_client = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .map(|api_key| {
            let config = ClaudeConfig {
                api_key,
                model: "claude-sonnet-4-20250514".to_string(),
                max_tokens: 4096,
                temperature: 0.7,
                timeout_ms: 30000,
            };
            ClaudeClient::new(config).ok()
        })
        .flatten();

    Self {
        session_update_tx: tx,
        next_session_id: Cell::new(1),
        coordinator_client: None,
        claude_client,
        sessions: Arc::new(DashMap::new()),
    }
}
```

3. Replace `generate_response()` in `prompt()` method:
```rust
async fn prompt(&self, arguments: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
    let session_id = arguments.session_id.0.as_ref();

    // Extract user message
    let user_message = arguments.prompt.iter()
        .filter_map(|content| {
            // Parse ACP content types to extract text
            match content {
                acp::ContentType::Text(text) => Some(text.text.to_string()),
                _ => None,
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Get conversation history
    let mut history = self.sessions
        .entry(session_id.to_string())
        .or_insert_with(Vec::new);

    // Build Claude messages
    let mut messages = vec![];

    // Add history
    for (role, content) in history.iter() {
        messages.push(crate::ai::claude::Message {
            role: role.clone(),
            content: content.clone(),
        });
    }

    // Add current message
    messages.push(crate::ai::claude::Message {
        role: "user".to_string(),
        content: user_message.clone(),
    });

    // Call Claude API
    let response_text = if let Some(client) = &self.claude_client {
        match client.send_message(&messages).await {
            Ok(response) => response,
            Err(e) => {
                tracing::error!("Claude API error: {}", e);
                format!("I encountered an error: {}. Please try again.", e)
            }
        }
    } else {
        // Fallback when no API key
        self.generate_response(&user_message)
    };

    // Store in history
    history.push(("user".to_string(), user_message));
    history.push(("assistant".to_string(), response_text.clone()));

    // Send response
    let (tx, rx) = oneshot::channel();
    self.session_update_tx
        .send((
            acp::SessionNotification {
                session_id: arguments.session_id,
                update: acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                    content: response_text.into(),
                    meta: None,
                }),
                meta: None,
            },
            tx,
        ))
        .map_err(|_| acp::Error::internal_error())?;
    rx.await.map_err(|_| acp::Error::internal_error())?;

    Ok(acp::PromptResponse {
        stop_reason: acp::StopReason::EndTurn,
        meta: None,
    })
}
```

4. Add imports at top of file:
```rust
use dashmap::DashMap;
use std::sync::Arc;
```

5. Add to `Cargo.toml` if not present:
```toml
dashmap = "6.0"
```

**Test it**:
```bash
# Run the agent
export ANTHROPIC_API_KEY="your-key-here"
cargo run --release -- acp

# In another terminal, test with a simple JSON-RPC client
# (or configure your editor)
```

---

### Step 2: Add Basic Tools (45 minutes)

**File**: `src/acp/tools.rs` (NEW)

```rust
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<Value>;
}

pub struct ReadFileTool {
    workspace_root: PathBuf,
}

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }

    fn description(&self) -> &str {
        "Read the contents of a file from the workspace"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace root)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;

        let full_path = self.workspace_root.join(path);

        // Security: Prevent directory traversal
        if !full_path.starts_with(&self.workspace_root) {
            return Err(anyhow::anyhow!("Path outside workspace"));
        }

        let content = tokio::fs::read_to_string(&full_path).await?;

        Ok(json!({
            "content": content,
            "path": path,
            "size": content.len()
        }))
    }
}

pub struct WriteFileTool {
    workspace_root: PathBuf,
}

#[async_trait::async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }

    fn description(&self) -> &str {
        "Write or update a file in the workspace"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace root)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let content = args["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("content is required"))?;

        let full_path = self.workspace_root.join(path);

        // Security check
        if !full_path.starts_with(&self.workspace_root) {
            return Err(anyhow::anyhow!("Path outside workspace"));
        }

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, content).await?;

        Ok(json!({
            "success": true,
            "path": path,
            "size": content.len()
        }))
    }
}

pub struct ListFilesTool {
    workspace_root: PathBuf,
}

#[async_trait::async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str { "list_files" }

    fn description(&self) -> &str {
        "List files in a directory"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path (relative to workspace root, defaults to root)"
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args["path"].as_str().unwrap_or(".");
        let full_path = self.workspace_root.join(path);

        if !full_path.starts_with(&self.workspace_root) {
            return Err(anyhow::anyhow!("Path outside workspace"));
        }

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&full_path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await?.is_dir();

            entries.push(json!({
                "name": file_name,
                "type": if is_dir { "directory" } else { "file" }
            }));
        }

        Ok(json!({
            "path": path,
            "entries": entries
        }))
    }
}
```

**Integrate into agent**:

In `src/acp/mod.rs`:
```rust
mod tools;
use tools::{Tool, ReadFileTool, WriteFileTool, ListFilesTool};

pub struct UniversalAgent {
    // ... existing fields ...
    tools: Vec<Box<dyn Tool>>,
    workspace_root: PathBuf,
}

// Update constructors to accept workspace_root
pub fn new_with_workspace(
    tx: mpsc::UnboundedSender<...>,
    workspace_root: PathBuf,
) -> Self {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ReadFileTool { workspace_root: workspace_root.clone() }),
        Box::new(WriteFileTool { workspace_root: workspace_root.clone() }),
        Box::new(ListFilesTool { workspace_root: workspace_root.clone() }),
    ];

    // ... rest of initialization ...

    Self {
        // ... existing fields ...
        tools,
        workspace_root,
    }
}
```

---

### Step 3: Test It!

```bash
# Build
cargo build --release

# Test manually
export ANTHROPIC_API_KEY="your-key-here"
./target/release/universal-lsp acp

# You should see:
# "🚀 Starting ACP agent on stdio..."
# "ACP agent initialized (MCP integration: enabled/disabled)"
# "ACP agent ready and listening on stdio"
```

**From Zed Editor**:
1. Configure Zed to use universal-lsp as ACP agent
2. Open a project
3. Invoke ACP agent (command palette)
4. Ask: "List the files in this workspace"
5. Ask: "What does the main.rs file do?"
6. Ask: "Write a test for the read_file function"

---

## What You Get

After these 3 steps (90 minutes total):

✅ **Real Claude API responses** instead of canned text
✅ **Conversation history** maintained across turns
✅ **File read/write/list** tools working
✅ **Context-aware responses** (Claude knows your workspace)
✅ **Secure tool execution** (sandboxed to workspace)

---

## Next Steps (Optional)

After the basics work, enhance with:

1. **Context Gathering** (45 min)
   - Add workspace structure detection
   - Git information via MCP
   - Diagnostics context

2. **Streaming Responses** (45 min)
   - Token-by-token streaming
   - Reduced latency

3. **Advanced Tools** (2 hours)
   - Code search
   - Apply edits
   - Run commands
   - Git operations

4. **Tests** (1 hour)
   - Unit tests for tools
   - Integration test with real Claude API
   - Manual testing protocol

---

## Troubleshooting

### "Claude API error: unauthorized"
- Check: `echo $ANTHROPIC_API_KEY`
- Verify API key is valid
- Ensure it has Credits available

### "Path outside workspace" error
- Tools are sandboxed to workspace root
- Use relative paths only

### "Agent not responding"
- Check logs: `RUST_LOG=debug ./target/release/universal-lsp acp`
- Verify ACP client is sending correct JSON-RPC messages
- Check Claude API rate limits

### Build errors
```bash
# Clean and rebuild
cargo clean
cargo build --release
```

---

## Cost Estimation

Claude API costs (as of 2025):
- **Sonnet 4**: ~$3 per million input tokens, ~$15 per million output tokens
- **Typical conversation**: 1000-5000 tokens (input) + 500-2000 tokens (output)
- **Estimated cost per conversation**: $0.01 - $0.05

For a day of development with 20 conversations: **~$0.20 - $1.00**

Very affordable for massive productivity boost!

---

## Success Checklist

- [ ] `ANTHROPIC_API_KEY` environment variable set
- [ ] `cargo build --release` succeeds
- [ ] `./target/release/universal-lsp acp` starts without errors
- [ ] Agent responds to test prompt
- [ ] Conversation history works (multi-turn)
- [ ] Tools execute (read_file, write_file, list_files)
- [ ] Editor integration configured
- [ ] First real coding session with Claude complete!

---

**Ready to implement?** Start with Step 1! 🚀

**Questions?** Check the full plan in `docs/ACP-IMPLEMENTATION-PLAN.md`

**Let's make Claude your pair programmer!**
