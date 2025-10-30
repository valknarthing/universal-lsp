# ACP (Agent Client Protocol) Implementation Plan
## Enabling Claude-Assisted Development in Universal LSP

**Goal**: Transform the existing ACP agent infrastructure into a fully functional Claude-powered development assistant that can help with coding tasks in real-time through the editor.

**Current Status**:
- ✅ ACP protocol infrastructure exists (`src/acp/mod.rs`, 621 lines)
- ✅ UniversalAgent implements the `Agent` trait
- ✅ CLI command `universal-lsp acp` runs the agent
- ✅ MCP integration framework ready
- ✅ Claude API client exists (`src/ai/claude.rs`)
- ⚠️ Agent returns **canned responses** instead of real Claude completions
- ⚠️ No conversation history management
- ⚠️ No file/code context integration
- ⚠️ No tool execution (read file, write file, etc.)

---

## Phase 1: Claude API Integration (Priority: CRITICAL)
**Timeline**: 2-3 hours
**Impact**: ⭐⭐⭐⭐⭐

### 1.1 Integrate Claude Client into UniversalAgent
**File**: `src/acp/mod.rs`

**Changes Needed**:
```rust
use crate::ai::claude::{ClaudeClient, ClaudeConfig};

pub struct UniversalAgent {
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    next_session_id: Cell<u64>,
    coordinator_client: Option<CoordinatorClient>,
    // ADD THIS:
    claude_client: Option<ClaudeClient>,
    // ADD THIS: Store conversation history per session
    sessions: Arc<DashMap<String, Vec<(String, String)>>>, // session_id -> [(role, content)]
}
```

**Implementation Steps**:
1. Add Claude client initialization in `UniversalAgent::new()` and `::with_coordinator()`
2. Load API key from environment variable `ANTHROPIC_API_KEY`
3. Initialize with sensible defaults (claude-sonnet-4, temperature 0.7 for creativity)
4. Handle case where API key is missing (graceful degradation)

### 1.2 Real Claude Responses in prompt() Handler
**Current Code** (`src/acp/mod.rs:166-203`):
```rust
async fn prompt(&self, arguments: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
    // Currently just returns canned response:
    let response_text = self.generate_response(&prompt_summary);
    // ...
}
```

**New Implementation**:
```rust
async fn prompt(&self, arguments: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
    let session_id = arguments.session_id.0.as_ref();

    // 1. Extract user message from prompt content
    let user_message = extract_message_from_prompt(&arguments.prompt)?;

    // 2. Get conversation history for this session
    let mut history = self.sessions
        .entry(session_id.to_string())
        .or_insert_with(Vec::new);

    // 3. Build messages for Claude API
    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        }
    ];

    // Add conversation history
    for (role, content) in history.iter() {
        messages.push(Message {
            role: role.clone(),
            content: content.clone(),
        });
    }

    // Add current user message
    messages.push(Message {
        role: "user".to_string(),
        content: user_message.clone(),
    });

    // 4. Call Claude API
    let claude_response = if let Some(client) = &self.claude_client {
        client.send_message(&messages).await
            .map_err(|e| acp::Error::internal_error_with_message(e.to_string()))?
    } else {
        // Fallback to canned response if no Claude client
        self.generate_response(&user_message)
    };

    // 5. Store in conversation history
    history.push(("user".to_string(), user_message));
    history.push(("assistant".to_string(), claude_response.clone()));

    // 6. Send response as streaming chunks (optional: implement streaming)
    self.send_response_chunk(arguments.session_id, claude_response).await?;

    Ok(acp::PromptResponse {
        stop_reason: acp::StopReason::EndTurn,
        meta: None,
    })
}
```

### 1.3 System Prompt for Development Assistant
**New Constant** (`src/acp/mod.rs`):
```rust
const SYSTEM_PROMPT: &str = r#"You are a highly skilled software development assistant integrated into the Universal LSP server.

Your capabilities:
- Multi-language expertise: Python, JavaScript, TypeScript, Rust, Go, Java, Ruby, C/C++, and 11+ more
- Code generation, refactoring, and optimization
- Debugging assistance and error explanation
- Best practices and design patterns
- Documentation generation
- Test writing and TDD support

You have access to:
- File system operations (via MCP filesystem server)
- Git repository information (via MCP git server)
- Web search for documentation (via MCP web search server)
- Current workspace context
- Open files and cursor positions

Guidelines:
1. Be concise and actionable
2. Provide code examples in markdown code blocks with language tags
3. Explain complex concepts clearly
4. Suggest best practices proactively
5. Ask clarifying questions when needed
6. Reference specific files and line numbers when relevant

Format your responses with markdown for better readability in the editor."#;
```

**Success Criteria**:
- ✅ Agent responds with actual Claude-generated content
- ✅ Conversation history maintained across multiple turns
- ✅ System prompt guides behavior appropriately
- ✅ Graceful fallback when API key missing

---

## Phase 2: Tool Execution Framework (Priority: HIGH)
**Timeline**: 3-4 hours
**Impact**: ⭐⭐⭐⭐⭐

### 2.1 Define Development Tools
**File**: `src/acp/tools.rs` (NEW)

**Tools to Implement**:

1. **read_file**
   - Purpose: Read file contents from workspace
   - Parameters: `path: String`
   - Returns: File contents or error

2. **write_file**
   - Purpose: Write/update file contents
   - Parameters: `path: String, content: String`
   - Returns: Success/error status

3. **list_files**
   - Purpose: List files in directory with optional glob pattern
   - Parameters: `path: String, pattern: Option<String>`
   - Returns: List of file paths

4. **search_code**
   - Purpose: Search for code patterns across workspace
   - Parameters: `query: String, file_pattern: Option<String>`
   - Returns: List of matches with line numbers

5. **get_diagnostics**
   - Purpose: Get current diagnostics (errors/warnings) for a file
   - Parameters: `path: String`
   - Returns: List of diagnostics

6. **apply_edit**
   - Purpose: Apply text edit to a file (LSP-style edit)
   - Parameters: `path: String, range: Range, new_text: String`
   - Returns: Success/error status

7. **run_command**
   - Purpose: Execute shell command in workspace
   - Parameters: `command: String`
   - Returns: stdout, stderr, exit code

8. **git_status**
   - Purpose: Get git status of workspace (via MCP)
   - Parameters: None
   - Returns: Git status information

### 2.2 Tool Execution Architecture
```rust
// src/acp/tools.rs

use anyhow::Result;
use serde_json::Value;

#[async_trait::async_trait]
pub trait Tool {
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
        let content = tokio::fs::read_to_string(&full_path).await?;

        Ok(json!({
            "content": content,
            "path": path,
            "size": content.len()
        }))
    }
}

// Similar implementations for other tools...
```

### 2.3 Integrate Tools into UniversalAgent
**Modifications to `src/acp/mod.rs`**:

```rust
use crate::acp::tools::{Tool, ReadFileTool, WriteFileTool, /* ... */};

pub struct UniversalAgent {
    // ... existing fields ...
    tools: Vec<Box<dyn Tool + Send + Sync>>,
    workspace_root: PathBuf,
}

impl UniversalAgent {
    pub fn new_with_workspace(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
        workspace_root: PathBuf,
    ) -> Self {
        let tools: Vec<Box<dyn Tool + Send + Sync>> = vec![
            Box::new(ReadFileTool { workspace_root: workspace_root.clone() }),
            Box::new(WriteFileTool { workspace_root: workspace_root.clone() }),
            Box::new(ListFilesTool { workspace_root: workspace_root.clone() }),
            Box::new(SearchCodeTool { workspace_root: workspace_root.clone() }),
            // ... more tools
        ];

        Self {
            session_update_tx,
            next_session_id: Cell::new(1),
            coordinator_client: None,
            claude_client: None,
            sessions: Arc::new(DashMap::new()),
            tools,
            workspace_root,
        }
    }
}
```

### 2.4 Tool-Aware Claude Prompting
**Enhanced prompt() method**:

```rust
async fn prompt(&self, arguments: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
    // ... existing code ...

    // Add tool definitions to system prompt
    let tools_description = self.tools.iter()
        .map(|t| format!("- {}: {}", t.name(), t.description()))
        .collect::<Vec<_>>()
        .join("\n");

    let enhanced_system_prompt = format!(
        "{}\n\nAvailable Tools:\n{}",
        SYSTEM_PROMPT,
        tools_description
    );

    // When Claude response includes tool calls, execute them
    // (This requires extended Claude API with function calling)
    if response.contains_tool_calls() {
        for tool_call in response.tool_calls {
            let result = self.execute_tool(&tool_call).await?;
            // Add tool result to conversation
            // Call Claude again with tool results
        }
    }

    // ... rest of implementation ...
}
```

**Success Criteria**:
- ✅ 8 core development tools implemented
- ✅ Tool execution sandbox (prevents escaping workspace)
- ✅ Claude can request and use tool results
- ✅ Tool errors handled gracefully

---

## Phase 3: Context Awareness (Priority: HIGH)
**Timeline**: 2-3 hours
**Impact**: ⭐⭐⭐⭐⭐

### 3.1 Workspace Context Injection
**Goal**: Provide Claude with rich context about the current coding session.

**Context Types**:
1. **Workspace Structure**
   - Project root path
   - Language detected (from files)
   - Git branch, commit
   - Build system (Cargo.toml, package.json, etc.)

2. **Active File Context**
   - Currently open files
   - Cursor position
   - Selected text
   - Recent edits

3. **Diagnostics Context**
   - Errors and warnings in workspace
   - Failed tests
   - Linting issues

4. **MCP Context**
   - Available MCP servers
   - Recent MCP queries
   - External knowledge sources

### 3.2 Context Provider Module
**File**: `src/acp/context.rs` (NEW)

```rust
use crate::coordinator::CoordinatorClient;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct ContextProvider {
    workspace_root: PathBuf,
    coordinator: Option<CoordinatorClient>,
}

impl ContextProvider {
    pub async fn gather_context(&self) -> Result<Value> {
        let mut context = json!({});

        // 1. Workspace info
        context["workspace"] = json!({
            "root": self.workspace_root.display().to_string(),
            "language": self.detect_primary_language().await?,
            "build_system": self.detect_build_system().await?,
        });

        // 2. Git info (via MCP if available)
        if let Some(coordinator) = &self.coordinator {
            if let Ok(git_info) = self.get_git_info(coordinator).await {
                context["git"] = git_info;
            }
        }

        // 3. File structure
        context["files"] = self.get_file_tree().await?;

        // 4. Recent diagnostics
        context["diagnostics"] = self.get_diagnostics_summary().await?;

        Ok(context)
    }

    async fn detect_primary_language(&self) -> Result<String> {
        // Check for Cargo.toml -> Rust
        // Check for package.json -> JavaScript/TypeScript
        // Check for requirements.txt -> Python
        // etc.
        todo!()
    }

    async fn get_git_info(&self, coordinator: &CoordinatorClient) -> Result<Value> {
        // Query MCP git server for branch, status, recent commits
        todo!()
    }

    async fn get_file_tree(&self) -> Result<Value> {
        // Generate tree of important files (ignore node_modules, target, etc.)
        todo!()
    }

    async fn get_diagnostics_summary(&self) -> Result<Value> {
        // Get counts of errors/warnings by file
        todo!()
    }
}
```

### 3.3 Context-Aware Prompting
**Modification to `prompt()` method**:

```rust
async fn prompt(&self, arguments: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
    // ... existing code ...

    // Gather context
    let context = self.context_provider.gather_context().await
        .map_err(|e| acp::Error::internal_error_with_message(e.to_string()))?;

    // Inject context into system message
    let system_with_context = format!(
        "{}\n\n## Current Workspace Context\n```json\n{}\n```",
        SYSTEM_PROMPT,
        serde_json::to_string_pretty(&context).unwrap()
    );

    // Use this enhanced system prompt when calling Claude
    // ...
}
```

**Success Criteria**:
- ✅ Claude receives workspace structure automatically
- ✅ Git information included (branch, status)
- ✅ Current diagnostics provided
- ✅ Context refreshed on each prompt
- ✅ Context size managed (avoid overwhelming Claude)

---

## Phase 4: Streaming Responses (Priority: MEDIUM)
**Timeline**: 2-3 hours
**Impact**: ⭐⭐⭐⭐

### 4.1 Claude Streaming API Integration
**Modifications to `src/ai/claude.rs`**:

```rust
impl ClaudeClient {
    /// Send message with streaming response
    pub async fn send_message_streaming(
        &self,
        messages: &[Message],
        mut chunk_callback: impl FnMut(String) -> Result<()>,
    ) -> Result<String> {
        let request = ClaudeRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: messages.to_vec(),
            stream: true, // NEW: Enable streaming
        };

        let response = self.http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let chunk_str = String::from_utf8(chunk.to_vec())?;

            // Parse SSE (Server-Sent Events) format
            if let Some(delta) = parse_sse_delta(&chunk_str) {
                full_response.push_str(&delta);
                chunk_callback(delta)?; // Callback for each chunk
            }
        }

        Ok(full_response)
    }
}
```

### 4.2 Streaming Response in ACP Agent
**Modifications to `prompt()` method**:

```rust
async fn prompt(&self, arguments: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
    // ... setup code ...

    // Stream Claude response
    let session_id = arguments.session_id.clone();
    let update_tx = self.session_update_tx.clone();

    let full_response = client.send_message_streaming(&messages, |chunk| {
        // Send each chunk as it arrives
        let (tx, rx) = oneshot::channel();
        update_tx.send((
            acp::SessionNotification {
                session_id: session_id.clone(),
                update: acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                    content: chunk.into(),
                    meta: None,
                }),
                meta: None,
            },
            tx,
        ))?;
        rx.blocking_recv()?;
        Ok(())
    }).await?;

    // ... rest of implementation ...
}
```

**Success Criteria**:
- ✅ Claude responses stream token-by-token
- ✅ User sees incremental updates in editor
- ✅ Reduced perceived latency
- ✅ Error handling for interrupted streams

---

## Phase 5: Advanced Features (Priority: LOW)
**Timeline**: 4-6 hours
**Impact**: ⭐⭐⭐

### 5.1 Multi-Turn Conversations with Memory
- Persist conversation history to disk
- Load previous sessions
- Context window management (summarize old turns)

### 5.2 Code Action Integration
- "Ask Claude" code action on selected code
- "Explain this" code action
- "Fix this error" code action
- Integration with LSP code actions

### 5.3 Inline Completions
- Ghost text completions as you type (like Copilot)
- ACP agent provides FIM (Fill-In-the-Middle) completions
- Debounced requests

### 5.4 Workspace Indexing
- Index all code symbols in workspace
- Provide Claude with symbol database
- Semantic search across codebase

### 5.5 Test Generation
- Dedicated tool for test generation
- Analyze function signatures and generate tests
- Run tests and fix based on failures

---

## Phase 6: Testing Strategy (Priority: HIGH)
**Timeline**: 2-3 hours
**Impact**: ⭐⭐⭐⭐

### 6.1 Unit Tests
**File**: `src/acp/mod.rs` (existing tests section)

Add tests for:
- ✅ Claude client integration
- ✅ Conversation history management
- ✅ Tool execution
- ✅ Context gathering
- ✅ Streaming responses

### 6.2 Integration Tests
**File**: `tests/acp_claude_integration_test.rs` (NEW)

```rust
#[tokio::test]
async fn test_real_claude_conversation() {
    // Requires ANTHROPIC_API_KEY
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Skipping Claude integration test (no API key)");
        return;
    }

    let (tx, mut rx) = mpsc::unbounded_channel();
    let agent = UniversalAgent::with_coordinator(tx).await;

    // Test initialization
    let init_response = agent.initialize(/* ... */).await.unwrap();
    assert!(init_response.agent_info.is_some());

    // Create session
    let session_response = agent.new_session(/* ... */).await.unwrap();
    let session_id = session_response.session_id;

    // Send prompt
    let prompt = acp::PromptRequest {
        session_id: session_id.clone(),
        prompt: vec!["Write a Python function to calculate factorial".into()],
        meta: None,
    };

    let response = agent.prompt(prompt).await.unwrap();
    assert_eq!(response.stop_reason, acp::StopReason::EndTurn);

    // Verify we received Claude response
    let notification = rx.recv().await.unwrap().0;
    match notification.update {
        acp::SessionUpdate::AgentMessageChunk(chunk) => {
            let content = chunk.content.to_string();
            assert!(content.contains("def") || content.contains("factorial"));
        }
        _ => panic!("Expected AgentMessageChunk"),
    }
}

#[tokio::test]
async fn test_tool_execution_read_file() {
    // Test that agent can read files via tool
    // ...
}

#[tokio::test]
async fn test_multi_turn_conversation() {
    // Test conversation history
    // ...
}

#[tokio::test]
async fn test_context_awareness() {
    // Test that agent receives workspace context
    // ...
}
```

### 6.3 Manual Testing with Zed
**Setup**:
1. Build universal-lsp: `cargo build --release`
2. Configure Zed to use ACP agent
3. Set `ANTHROPIC_API_KEY` environment variable
4. Open a project in Zed
5. Invoke ACP agent via command palette

**Test Cases**:
- Ask "What files are in this workspace?"
- Ask "Can you explain what this function does?" (with file open)
- Ask "Write a test for this function"
- Ask "Fix the error on line 42"
- Ask multi-turn question with follow-ups

---

## Phase 7: Documentation (Priority: MEDIUM)
**Timeline**: 2-3 hours
**Impact**: ⭐⭐⭐⭐

### 7.1 User Documentation
**File**: `docs/ACP-USER-GUIDE.md` (NEW)

Topics:
- What is ACP and how does it work?
- Setting up Claude API key
- Available commands and tools
- Best practices for prompting
- Examples of common workflows
- Troubleshooting

### 7.2 Developer Documentation
**File**: `docs/ACP-ARCHITECTURE.md` (NEW)

Topics:
- ACP protocol overview
- UniversalAgent architecture
- Tool system design
- Context gathering pipeline
- Adding new tools
- Extending agent capabilities

### 7.3 Update CLAUDE.md
Add ACP section to main docs:
- ACP commands
- Configuration options
- Integration with LSP
- Testing ACP features

---

## Implementation Priority Order

### Sprint 1: Foundation (6-8 hours) - CRITICAL
1. **Phase 1**: Claude API Integration (2-3h)
   - Integrate ClaudeClient into UniversalAgent
   - Replace canned responses with real Claude calls
   - Conversation history management

2. **Phase 2**: Basic Tool Execution (3-4h)
   - Implement read_file, write_file, list_files tools
   - Tool execution framework
   - Basic error handling

3. **Phase 6.1**: Unit Tests (1h)
   - Test Claude integration
   - Test tool execution

### Sprint 2: Context & UX (5-7 hours) - HIGH PRIORITY
4. **Phase 3**: Context Awareness (2-3h)
   - Workspace context gathering
   - Git integration via MCP
   - Diagnostics context

5. **Phase 4**: Streaming Responses (2-3h)
   - Claude streaming API
   - Incremental response updates

6. **Phase 6.2**: Integration Tests (1-2h)
   - End-to-end ACP tests
   - Tool execution tests

### Sprint 3: Advanced Features (6-8 hours) - MEDIUM PRIORITY
7. **Phase 2 (cont.)**: Advanced Tools (2-3h)
   - search_code, apply_edit, run_command tools
   - Tool sandboxing and security

8. **Phase 5**: Advanced Features (3-4h)
   - Code action integration
   - Session persistence
   - Workspace indexing

9. **Phase 7**: Documentation (2-3h)
   - User guide
   - Developer docs
   - Examples

---

## Success Metrics

### Functional Success
- ✅ Agent responds with Claude-generated content
- ✅ Tools execute successfully (read/write files)
- ✅ Conversation history maintained across turns
- ✅ Context automatically provided to Claude
- ✅ Streaming responses work in editor
- ✅ All tests pass (unit + integration)

### User Experience Success
- ✅ Responses feel natural and helpful
- ✅ Tool usage is transparent to user
- ✅ Latency is acceptable (<2s first token)
- ✅ Errors are clear and actionable
- ✅ Works seamlessly in Zed editor

### Technical Success
- ✅ No memory leaks in long conversations
- ✅ Graceful error handling
- ✅ Secure tool execution (sandbox)
- ✅ Clean separation of concerns
- ✅ Well-tested and documented

---

## Dependencies & Prerequisites

### Required
- `ANTHROPIC_API_KEY` environment variable
- Claude API access (Sonnet 4 recommended)
- Tokio async runtime (already present)
- `agent-client-protocol` crate (already present)

### Optional
- MCP coordinator running (for enhanced context)
- Git repository (for git context)
- Active Zed editor session (for testing)

---

## Risk Mitigation

### API Cost Control
- **Risk**: Claude API calls can be expensive
- **Mitigation**:
  - Implement token usage tracking
  - Set max_tokens limits (1024-4096)
  - Cache common responses
  - Provide usage statistics

### Security Concerns
- **Risk**: Arbitrary code execution via tools
- **Mitigation**:
  - Sandbox tool execution
  - Whitelist allowed commands
  - Validate file paths (prevent directory traversal)
  - User confirmation for destructive operations

### Context Window Overflow
- **Risk**: Conversations exceed Claude's context limit
- **Mitigation**:
  - Implement conversation summarization
  - Sliding window of recent messages
  - Compress old context

### Network Failures
- **Risk**: Claude API unavailable
- **Mitigation**:
  - Retry logic with exponential backoff
  - Graceful error messages
  - Fallback to local tools without AI

---

## Next Steps

1. **Review this plan** with team/stakeholders
2. **Set up development environment**:
   - Export `ANTHROPIC_API_KEY`
   - Verify Claude API access
   - Run existing tests: `cargo test`
3. **Start Sprint 1, Phase 1**: Claude API Integration
4. **Create feature branch**: `git checkout -b feature/acp-claude-integration`
5. **Begin implementation** following the plan step-by-step

---

## Questions & Clarifications Needed

1. **Editor Integration**: Which editor(s) should we prioritize?
   - Zed (primary)
   - VSCode
   - Neovim

2. **Tool Permissions**: Should tool execution require user confirmation?
   - Always confirm?
   - Confirm only for write operations?
   - Whitelist certain safe operations?

3. **Conversation Storage**: Where should conversation history persist?
   - In-memory only
   - Disk cache (~/.cache/universal-lsp/sessions/)
   - Database

4. **Context Size**: How much context should we provide per prompt?
   - Minimal (current file only)
   - Moderate (current file + imports)
   - Maximal (entire workspace index)

---

**Document Version**: 1.0
**Created**: 2025-10-30
**Author**: Claude (Sonnet 4.5)
**Status**: Ready for Implementation
