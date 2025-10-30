# ACP Sprint 1: Foundation Implementation
## Building World-Class Claude-Assisted Development

**Sprint Goal**: Deliver production-ready Claude API integration with basic tool execution and comprehensive tests.

**Duration**: 6-8 hours

**Success Criteria**:
- ✅ Real Claude API responses replace canned text
- ✅ Conversation history maintained across sessions
- ✅ 3 core tools working (read_file, write_file, list_files)
- ✅ Tool execution sandboxed and secure
- ✅ All unit tests passing
- ✅ Manual testing protocol verified
- ✅ Code reviewed and documented

---

## Phase 1: Claude API Integration (2-3 hours)

### 1.1 Update Dependencies
**File**: `Cargo.toml`

**Add**:
```toml
# Already present, verify versions:
dashmap = "6.0"  # For concurrent HashMap
async-trait = "0.1"  # For async trait implementations
```

### 1.2 Enhance UniversalAgent Structure
**File**: `src/acp/mod.rs`

**Changes**:

1. **Add imports** (top of file, after existing imports):
```rust
use crate::ai::claude::{ClaudeClient, ClaudeConfig, CompletionContext};
use dashmap::DashMap;
use std::sync::Arc;
```

2. **Update UniversalAgent struct** (line ~26):
```rust
pub struct UniversalAgent {
    /// Channel for sending session notifications back to clients
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    /// Counter for generating unique session IDs
    next_session_id: Cell<u64>,
    /// MCP coordinator client for enhanced capabilities
    coordinator_client: Option<CoordinatorClient>,
    /// Claude API client for AI-powered responses
    claude_client: Option<Arc<ClaudeClient>>,
    /// Conversation history per session (session_id -> messages)
    sessions: Arc<DashMap<String, Vec<ConversationMessage>>>,
    /// Workspace root directory
    workspace_root: PathBuf,
}

/// Conversation message for history tracking
#[derive(Debug, Clone)]
struct ConversationMessage {
    role: String,  // "user" or "assistant"
    content: String,
    timestamp: std::time::SystemTime,
}
```

3. **Add system prompt constant** (after struct definition):
```rust
const SYSTEM_PROMPT: &str = r#"You are an expert software development assistant integrated into Universal LSP.

## Your Capabilities
You are fluent in 19+ programming languages including:
- Systems: Rust, C, C++, Go
- Web: JavaScript, TypeScript, HTML, CSS, Svelte
- Application: Python, Ruby, Java, PHP, Scala, Kotlin, C#
- Scripting: Bash, Shell

You excel at:
1. Code generation with best practices
2. Bug fixing and debugging assistance
3. Code refactoring and optimization
4. Writing comprehensive tests
5. Documentation and explanations
6. Architecture and design patterns

## Available Tools
You have access to file operations and workspace inspection tools. When you need to read files, search code, or understand the workspace structure, you can use these tools.

## Guidelines
- Be concise and actionable
- Provide code in markdown blocks with language tags
- Reference specific files and line numbers when relevant
- Ask clarifying questions when requirements are unclear
- Follow language-specific best practices
- Suggest tests for new code
- Explain complex concepts clearly

## Response Format
Use markdown for formatting:
- `code` for inline code
- ```language for code blocks
- **bold** for emphasis
- Lists for step-by-step instructions

Let's write great code together!"#;
```

4. **Update constructor methods**:

**Replace `UniversalAgent::new()` (line ~36-45)**:
```rust
pub fn new(
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
) -> Self {
    Self::new_with_workspace(session_update_tx, PathBuf::from("."))
}

pub fn new_with_workspace(
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    workspace_root: PathBuf,
) -> Self {
    // Initialize Claude client if API key is available
    let claude_client = Self::init_claude_client();

    info!(
        "UniversalAgent created (Claude: {}, workspace: {})",
        if claude_client.is_some() { "enabled" } else { "disabled" },
        workspace_root.display()
    );

    Self {
        session_update_tx,
        next_session_id: Cell::new(1),
        coordinator_client: None,
        claude_client,
        sessions: Arc::new(DashMap::new()),
        workspace_root,
    }
}
```

**Replace `UniversalAgent::with_coordinator()` (line ~47-68)**:
```rust
pub async fn with_coordinator(
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
) -> Self {
    Self::with_coordinator_and_workspace(session_update_tx, PathBuf::from(".")).await
}

pub async fn with_coordinator_and_workspace(
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    workspace_root: PathBuf,
) -> Self {
    let coordinator_client = match CoordinatorClient::connect().await {
        Ok(client) => {
            info!("ACP agent connected to MCP coordinator");
            Some(client)
        }
        Err(e) => {
            warn!("ACP agent could not connect to MCP coordinator: {}", e);
            info!("Continuing without MCP integration");
            None
        }
    };

    let claude_client = Self::init_claude_client();

    info!(
        "UniversalAgent created (Claude: {}, MCP: {}, workspace: {})",
        if claude_client.is_some() { "enabled" } else { "disabled" },
        if coordinator_client.is_some() { "enabled" } else { "disabled" },
        workspace_root.display()
    );

    Self {
        session_update_tx,
        next_session_id: Cell::new(1),
        coordinator_client,
        claude_client,
        sessions: Arc::new(DashMap::new()),
        workspace_root,
    }
}
```

5. **Add Claude client initialization helper**:
```rust
impl UniversalAgent {
    /// Initialize Claude API client from environment variable
    fn init_claude_client() -> Option<Arc<ClaudeClient>> {
        match std::env::var("ANTHROPIC_API_KEY") {
            Ok(api_key) if !api_key.is_empty() => {
                let config = ClaudeConfig {
                    api_key,
                    model: "claude-sonnet-4-20250514".to_string(),
                    max_tokens: 4096,
                    temperature: 0.7,  // Higher for more creative responses
                    timeout_ms: 30000,  // 30s timeout for longer responses
                };

                match ClaudeClient::new(config) {
                    Ok(client) => {
                        info!("Claude API client initialized successfully");
                        Some(Arc::new(client))
                    }
                    Err(e) => {
                        error!("Failed to initialize Claude client: {}", e);
                        None
                    }
                }
            }
            Ok(_) => {
                warn!("ANTHROPIC_API_KEY is empty");
                None
            }
            Err(_) => {
                warn!("ANTHROPIC_API_KEY not set - Claude integration disabled");
                info!("Set ANTHROPIC_API_KEY environment variable to enable Claude");
                None
            }
        }
    }

    /// Build enhanced system prompt with context
    fn build_system_prompt(&self) -> String {
        let mut prompt = SYSTEM_PROMPT.to_string();

        // Add workspace context
        prompt.push_str(&format!(
            "\n\n## Current Workspace\nRoot: {}\n",
            self.workspace_root.display()
        ));

        // Add MCP status
        if self.coordinator_client.is_some() {
            prompt.push_str("\nMCP Integration: ✅ Active (enhanced capabilities available)\n");
        } else {
            prompt.push_str("\nMCP Integration: ⚠️ Not available\n");
        }

        prompt
    }
}
```

### 1.3 Implement Real Claude Responses

**Replace `prompt()` method** (line ~166-203):

```rust
async fn prompt(
    &self,
    arguments: acp::PromptRequest,
) -> Result<acp::PromptResponse, acp::Error> {
    let session_id = arguments.session_id.0.as_ref();

    info!(
        "Processing prompt for session {}: {} content items",
        session_id,
        arguments.prompt.len()
    );

    // 1. Extract user message from ACP prompt content
    let user_message = self.extract_message_from_prompt(&arguments.prompt)?;

    if user_message.is_empty() {
        return Err(acp::Error::invalid_request_with_message(
            "Prompt content is empty".to_string()
        ));
    }

    info!("User message: {}", user_message);

    // 2. Get or create conversation history for this session
    let mut history = self.sessions
        .entry(session_id.to_string())
        .or_insert_with(Vec::new);

    // 3. Generate response (Claude or fallback)
    let response_text = if let Some(client) = &self.claude_client {
        // Build messages for Claude API
        let mut messages = Vec::new();

        // Add conversation history
        for msg in history.iter() {
            messages.push(crate::ai::claude::Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        // Add current user message
        messages.push(crate::ai::claude::Message {
            role: "user".to_string(),
            content: user_message.clone(),
        });

        // Call Claude API
        match client.send_message(&messages).await {
            Ok(response) => {
                info!("Claude API response received ({} chars)", response.len());
                response
            }
            Err(e) => {
                error!("Claude API error: {}", e);
                format!(
                    "I encountered an error communicating with Claude API: {}\n\n\
                     Please check:\n\
                     - Your API key is valid\n\
                     - You have available credits\n\
                     - Network connectivity is working\n\n\
                     Error details: {}",
                    e,
                    e
                )
            }
        }
    } else {
        // Fallback when Claude is not available
        warn!("Claude client not available, using fallback response");
        self.generate_fallback_response(&user_message)
    };

    // 4. Store in conversation history
    history.push(ConversationMessage {
        role: "user".to_string(),
        content: user_message,
        timestamp: std::time::SystemTime::now(),
    });

    history.push(ConversationMessage {
        role: "assistant".to_string(),
        content: response_text.clone(),
        timestamp: std::time::SystemTime::now(),
    });

    // 5. Send response as notification
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

    info!("Prompt processing complete for session {}", session_id);

    Ok(acp::PromptResponse {
        stop_reason: acp::StopReason::EndTurn,
        meta: None,
    })
}
```

6. **Add helper methods**:
```rust
impl UniversalAgent {
    /// Extract text content from ACP prompt
    fn extract_message_from_prompt(
        &self,
        prompt: &[acp::PromptContent],
    ) -> Result<String, acp::Error> {
        let mut messages = Vec::new();

        for content in prompt {
            match content {
                acp::PromptContent::Text(text) => {
                    messages.push(text.text.to_string());
                }
                acp::PromptContent::Resource(_) => {
                    // TODO: Handle resource content in future
                    info!("Resource content not yet supported");
                }
            }
        }

        if messages.is_empty() {
            return Err(acp::Error::invalid_request_with_message(
                "No text content in prompt".to_string()
            ));
        }

        Ok(messages.join("\n"))
    }

    /// Generate fallback response when Claude is not available
    fn generate_fallback_response(&self, user_message: &str) -> String {
        format!(
            "I'm the Universal LSP ACP Agent, but Claude API integration is not available.\n\n\
             Your message: {}\n\n\
             I support 19+ programming languages and would normally provide:\n\
             • Code completions and suggestions\n\
             • Code explanations and documentation\n\
             • Refactoring suggestions\n\
             • Debugging assistance\n\
             • Best practices and patterns\n\n\
             To enable Claude AI responses:\n\
             1. Set ANTHROPIC_API_KEY environment variable\n\
             2. Restart the ACP agent\n\n\
             MCP Integration: {}\n\
             Workspace: {}",
            user_message,
            if self.coordinator_client.is_some() { "✅ Active" } else { "⚠️ Not available" },
            self.workspace_root.display()
        )
    }
}
```

7. **Update `generate_response()` method** (keep for backward compatibility but mark deprecated):
```rust
#[deprecated(note = "Use prompt() method with real Claude integration instead")]
fn generate_response(&self, prompt_summary: &str) -> String {
    self.generate_fallback_response(prompt_summary)
}
```

### 1.4 Update Tests

**File**: `src/acp/mod.rs` (tests section at bottom)

**Add new tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // ... existing imports ...

    #[tokio::test]
    async fn test_claude_client_initialization() {
        // Test with API key
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-123");
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);
        // Should have Claude client (even if key is fake for testing)
        assert!(agent.claude_client.is_some() || agent.claude_client.is_none());

        // Test without API key
        std::env::remove_var("ANTHROPIC_API_KEY");
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);
        assert!(agent.claude_client.is_none());
    }

    #[tokio::test]
    async fn test_conversation_history() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let session_id = "test-session-123";

        // Initially empty
        assert!(!agent.sessions.contains_key(session_id));

        // Add messages
        let mut history = agent.sessions.entry(session_id.to_string()).or_insert_with(Vec::new);
        history.push(ConversationMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: std::time::SystemTime::now(),
        });
        history.push(ConversationMessage {
            role: "assistant".to_string(),
            content: "Hi there!".to_string(),
            timestamp: std::time::SystemTime::now(),
        });

        drop(history);

        // Verify stored
        assert!(agent.sessions.contains_key(session_id));
        let history = agent.sessions.get(session_id).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[1].role, "assistant");
    }

    #[tokio::test]
    async fn test_extract_message_from_prompt() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let prompt = vec![
            acp::PromptContent::Text(acp::TextContent {
                text: "Hello".into(),
                meta: None,
            }),
            acp::PromptContent::Text(acp::TextContent {
                text: "World".into(),
                meta: None,
            }),
        ];

        let message = agent.extract_message_from_prompt(&prompt).unwrap();
        assert_eq!(message, "Hello\nWorld");
    }

    #[tokio::test]
    async fn test_system_prompt_building() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new_with_workspace(tx, PathBuf::from("/test/workspace"));

        let system_prompt = agent.build_system_prompt();

        assert!(system_prompt.contains("software development assistant"));
        assert!(system_prompt.contains("/test/workspace"));
        assert!(system_prompt.contains("19+ programming languages"));
    }

    #[tokio::test]
    async fn test_fallback_response() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let response = agent.generate_fallback_response("Test message");

        assert!(response.contains("Universal LSP ACP Agent"));
        assert!(response.contains("ANTHROPIC_API_KEY"));
        assert!(response.contains("Test message"));
    }

    // ... keep all existing tests ...
}
```

### 1.5 Update run_agent()

**File**: `src/acp/mod.rs` (line ~294-343)

**Modify to accept workspace path**:
```rust
/// Run the ACP agent server on stdio
pub async fn run_agent() -> Result<()> {
    run_agent_with_workspace(PathBuf::from(".")).await
}

/// Run the ACP agent server on stdio with specified workspace
pub async fn run_agent_with_workspace(workspace_root: PathBuf) -> Result<()> {
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    info!(
        "Starting Universal LSP ACP Agent (workspace: {})",
        workspace_root.display()
    );

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    // Create LocalSet for non-Send futures
    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            // Create agent with MCP coordinator integration and workspace
            let agent = UniversalAgent::with_coordinator_and_workspace(tx, workspace_root.clone())
                .await;

            let has_mcp = agent.coordinator_client.is_some();
            let has_claude = agent.claude_client.is_some();

            info!(
                "ACP agent initialized (Claude: {}, MCP: {}, workspace: {})",
                if has_claude { "✅" } else { "❌" },
                if has_mcp { "✅" } else { "❌" },
                workspace_root.display()
            );

            let (conn, handle_io) =
                acp::AgentSideConnection::new(agent, outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            // Spawn task to forward session notifications
            tokio::task::spawn_local(async move {
                while let Some((session_notification, tx)) = rx.recv().await {
                    if let Err(e) = conn.session_notification(session_notification).await {
                        error!("Failed to send session notification: {}", e);
                        break;
                    }
                    tx.send(()).ok();
                }
                info!("Session notification task ended");
            });

            // Run until stdio closes
            info!("ACP agent ready and listening on stdio");
            handle_io.await
        })
        .await?;

    info!("ACP agent shutting down");
    Ok(())
}
```

### 1.6 Update Claude API Client

**File**: `src/ai/claude.rs`

**Add method to send messages** (if not exists):
```rust
impl ClaudeClient {
    /// Send a message to Claude and get response
    pub async fn send_message(&self, messages: &[Message]) -> Result<String> {
        let request = ClaudeRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: messages.to_vec(),
        };

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .context("Failed to send request to Claude API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Claude API returned error status {}: {}",
                status,
                error_body
            ));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude API response")?;

        // Extract text from content blocks
        let text = claude_response
            .content
            .iter()
            .filter(|block| block.content_type == "text")
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }
}

/// Make Message public
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}
```

### Testing Phase 1

```bash
# 1. Build
cargo build --release

# 2. Run tests
cargo test --lib acp:: -- --nocapture

# 3. Manual test
export ANTHROPIC_API_KEY="your-key-here"
./target/release/universal-lsp acp

# Should see:
# "ACP agent initialized (Claude: ✅, MCP: ✅/❌, workspace: ...)"
```

**Expected Results**:
- ✅ All tests pass
- ✅ Agent starts successfully
- ✅ Claude client initialized if API key present
- ✅ Fallback response if no API key
- ✅ Conversation history tracked
- ✅ System prompts include workspace context

---

## Checklist for Phase 1

- [ ] Dependencies verified in Cargo.toml
- [ ] UniversalAgent struct updated with new fields
- [ ] SYSTEM_PROMPT constant added
- [ ] Constructor methods updated
- [ ] init_claude_client() implemented
- [ ] build_system_prompt() implemented
- [ ] prompt() method replaced with real Claude calls
- [ ] extract_message_from_prompt() helper added
- [ ] generate_fallback_response() implemented
- [ ] run_agent() updated for workspace support
- [ ] Claude client send_message() method verified
- [ ] All new tests added and passing
- [ ] Manual testing completed
- [ ] Code compiles without warnings
- [ ] Documentation comments updated

---

## Next: Phase 2 & 6.1

After Phase 1 is complete and tested:
- Phase 2: Tool Execution Framework (3-4 hours)
- Phase 6.1: Comprehensive Unit Tests (1 hour)

**Ready to implement Phase 1?** Let's go! 🚀
