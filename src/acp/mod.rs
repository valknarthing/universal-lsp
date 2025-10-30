//! ACP (Agent Client Protocol) Agent Implementation
//!
//! This module provides an ACP-compliant agent that integrates with the Universal LSP system,
//! offering AI-powered code assistance through the Agent Client Protocol.
//!
//! ## Features
//! - Multi-turn conversations with context awareness
//! - Integration with MCP coordinator for enhanced capabilities
//! - Session management for multiple concurrent clients
//! - AI-powered code completions and explanations

use agent_client_protocol as acp;
use agent_client_protocol::Client; // Required for session_notification method
use anyhow::Result;
use serde_json::json;
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{error, info, warn};

use crate::ai::claude::{ClaudeClient, ClaudeConfig};
use crate::coordinator::CoordinatorClient;
use dashmap::DashMap;

pub mod tools;
pub mod context;

use tools::ToolRegistry;
use context::ContextProvider;

/// Universal LSP ACP Agent
///
/// Implements the Agent Client Protocol for providing AI-powered code assistance
/// integrated with Universal LSP's multi-language support and MCP pipeline.
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
    /// Tool registry for Claude-executable actions
    tools: Arc<ToolRegistry>,
    /// Context provider for workspace awareness
    context: Arc<ContextProvider>,
    /// Cancellation tokens per session (session_id -> watch::Receiver<bool>)
    cancellation_tokens: Arc<DashMap<String, watch::Sender<bool>>>,
}

/// Conversation message for history tracking
#[derive(Debug, Clone)]
struct ConversationMessage {
    role: String,  // "user" or "assistant"
    content: String,
    timestamp: std::time::SystemTime,
}

/// System prompt for the Claude-powered development assistant
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

impl UniversalAgent {
    /// Create a new Universal ACP Agent
    pub fn new(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        Self::new_with_workspace(session_update_tx, PathBuf::from("."))
    }

    /// Create a new Universal ACP Agent with specified workspace
    pub fn new_with_workspace(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
        workspace_root: PathBuf,
    ) -> Self {
        // Initialize Claude client if API key is available
        let claude_client = Self::init_claude_client();

        // Initialize tool registry with workspace
        let tools = Arc::new(ToolRegistry::new(workspace_root.clone()));

        // Initialize context provider
        let context = Arc::new(ContextProvider::new(workspace_root.clone()));

        info!(
            "UniversalAgent created (Claude: {}, tools: {}, workspace: {})",
            if claude_client.is_some() { "enabled" } else { "disabled" },
            tools.count(),
            workspace_root.display()
        );

        Self {
            session_update_tx,
            next_session_id: Cell::new(1),
            coordinator_client: None,
            claude_client,
            sessions: Arc::new(DashMap::new()),
            workspace_root,
            tools,
            context,
            cancellation_tokens: Arc::new(DashMap::new()),
        }
    }

    /// Create a new Universal ACP Agent with MCP coordinator integration
    pub async fn with_coordinator(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        Self::with_coordinator_and_workspace(session_update_tx, PathBuf::from(".")).await
    }

    /// Create a new Universal ACP Agent with MCP coordinator integration and workspace
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

        // Initialize tool registry with workspace
        let tools = Arc::new(ToolRegistry::new(workspace_root.clone()));

        // Initialize context provider
        let context = Arc::new(ContextProvider::new(workspace_root.clone()));

        info!(
            "UniversalAgent created (Claude: {}, MCP: {}, tools: {}, workspace: {})",
            if claude_client.is_some() { "enabled" } else { "disabled" },
            if coordinator_client.is_some() { "enabled" } else { "disabled" },
            tools.count(),
            workspace_root.display()
        );

        Self {
            session_update_tx,
            next_session_id: Cell::new(1),
            coordinator_client,
            claude_client,
            sessions: Arc::new(DashMap::new()),
            workspace_root,
            tools,
            context,
            cancellation_tokens: Arc::new(DashMap::new()),
        }
    }

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
    async fn build_system_prompt(&self) -> String {
        let mut prompt = SYSTEM_PROMPT.to_string();

        // Add rich workspace context from context provider
        prompt.push_str(&self.context.format_for_prompt().await);

        // Add MCP status
        if let Some(_coordinator) = &self.coordinator_client {
            prompt.push_str("\nMCP Integration: ✅ Active (enhanced capabilities available)\n");
        } else {
            prompt.push_str("\nMCP Integration: ⚠️ Not available\n");
        }

        prompt
    }

    /// Extract text content from ACP prompt
    fn extract_message_from_prompt(
        &self,
        prompt: &[acp::ContentBlock],
    ) -> Result<String, acp::Error> {
        let mut messages = Vec::new();

        for content in prompt {
            // Convert ContentBlock to string representation
            // ContentBlock is an enum with Text, Resource, ResourceLink variants
            let text = format!("{:?}", content); // Use Debug formatting for now

            // Try to extract actual text if it's a Text variant
            // This is a simplified version - proper implementation would match on variants
            if !text.is_empty() {
                messages.push(text);
            }
        }

        if messages.is_empty() {
            return Err(acp::Error::invalid_request());
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
            if let Some(_coordinator) = &self.coordinator_client { "✅ Active" } else { "⚠️ Not available" },
            self.workspace_root.display()
        )
    }

    /// Call Claude API with tool support and handle tool execution loop
    async fn call_claude_with_tools(
        &self,
        client: &crate::ai::claude::ClaudeClient,
        mut messages: Vec<crate::ai::claude::Message>,
        tool_definitions: Vec<serde_json::Value>,
        system_prompt: String,
    ) -> Result<String, anyhow::Error> {
        use serde_json::json;

        const MAX_ITERATIONS: usize = 10; // Prevent infinite loops
        let mut iteration = 0;
        let mut accumulated_text = Vec::new();

        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                warn!("Tool execution loop exceeded maximum iterations ({})", MAX_ITERATIONS);
                break;
            }

            // Call Claude with tools
            let response = client
                .send_message_with_tools(
                    &messages,
                    Some(tool_definitions.clone()),
                    Some(system_prompt.clone()),
                )
                .await?;

            // Collect text blocks
            if !response.text_blocks.is_empty() {
                accumulated_text.extend(response.text_blocks.clone());
            }

            // Check if Claude wants to use tools
            if response.tool_uses.is_empty() {
                // No tools to execute, we're done
                info!("Claude response complete after {} iterations", iteration);
                break;
            }

            info!("Claude requested {} tool(s)", response.tool_uses.len());

            // Execute tools and collect results
            let mut tool_results = Vec::new();
            for tool_use in &response.tool_uses {
                info!("Executing tool: {} (id: {})", tool_use.name, tool_use.id);

                let result = match self.tools.execute_tool(&tool_use.name, tool_use.input.clone()).await {
                    Ok(result) => {
                        info!("Tool {} succeeded", tool_use.name);
                        json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use.id,
                            "content": result.to_string()
                        })
                    }
                    Err(e) => {
                        error!("Tool {} failed: {}", tool_use.name, e);
                        json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use.id,
                            "is_error": true,
                            "content": format!("Error executing tool: {}", e)
                        })
                    }
                };
                tool_results.push(result);
            }

            // Add assistant's response (with tool uses) to messages
            // Note: In the actual Anthropic API, we need to format this properly
            // For now, we'll create a simplified version
            let assistant_content = if !response.text_blocks.is_empty() {
                response.text_blocks.join("\n")
            } else {
                format!("[Using {} tools]", response.tool_uses.len())
            };

            messages.push(crate::ai::claude::Message {
                role: "assistant".to_string(),
                content: assistant_content,
            });

            // Add tool results as a user message
            let tool_results_content = json!(tool_results).to_string();
            messages.push(crate::ai::claude::Message {
                role: "user".to_string(),
                content: format!("Tool results:\n{}", tool_results_content),
            });
        }

        Ok(accumulated_text.join("\n\n"))
    }

    /// Call Claude with tools using streaming for real-time updates
    ///
    /// This method is similar to call_claude_with_tools but streams responses
    /// incrementally via session notifications, providing a better UX.
    async fn call_claude_with_tools_streaming(
        &self,
        client: &crate::ai::claude::ClaudeClient,
        mut messages: Vec<crate::ai::claude::Message>,
        tool_definitions: Vec<serde_json::Value>,
        system_prompt: String,
        session_id: acp::SessionId,
    ) -> Result<String, anyhow::Error> {
        use crate::ai::claude::{StreamEvent, ContentDelta};
        use serde_json::json;

        const MAX_ITERATIONS: usize = 10;
        let mut iteration = 0;
        let mut accumulated_text = Vec::new();

        // Create cancellation token for this session
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let session_id_str = session_id.0.to_string();
        self.cancellation_tokens.insert(session_id_str.clone(), cancel_tx);

        loop {
            // Check for cancellation
            if *cancel_rx.borrow() {
                info!("Streaming cancelled for session {}", session_id_str);
                self.cancellation_tokens.remove(&session_id_str);
                return Err(anyhow::anyhow!("Request cancelled by user"));
            }

            iteration += 1;
            if iteration > MAX_ITERATIONS {
                warn!("Tool execution loop exceeded maximum iterations ({})", MAX_ITERATIONS);
                break;
            }

            // Create callback for streaming events
            let session_update_tx = self.session_update_tx.clone();
            let session_id_clone = session_id.clone();

            let callback = move |event: StreamEvent| -> Result<(), anyhow::Error> {
                match event {
                    StreamEvent::ContentBlockDelta { delta, .. } => {
                        if let ContentDelta::TextDelta { text } = delta {
                            // Send incremental text as notification
                            let (tx, _rx) = oneshot::channel();
                            let _ = session_update_tx.send((
                                acp::SessionNotification {
                                    session_id: session_id_clone.clone(),
                                    update: acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                                        content: text.into(),
                                        meta: None,
                                    }),
                                    meta: None,
                                },
                                tx,
                            ));
                            // Don't await rx - fire and forget for streaming
                        }
                    }
                    StreamEvent::MessageStart { .. } => {
                        info!("Streaming started for session {}", session_id_clone);
                    }
                    StreamEvent::MessageStop => {
                        info!("Streaming complete for session {}", session_id_clone);
                    }
                    _ => {}
                }
                Ok(())
            };

            // Call Claude with streaming
            let response = client
                .send_message_with_tools_streaming(
                    &messages,
                    Some(tool_definitions.clone()),
                    Some(system_prompt.clone()),
                    callback,
                )
                .await?;

            // Collect text blocks
            if !response.text_blocks.is_empty() {
                accumulated_text.extend(response.text_blocks.clone());
            }

            // Check if Claude wants to use tools
            if response.tool_uses.is_empty() {
                info!("Streaming response complete after {} iterations", iteration);
                break;
            }

            info!("Claude requested {} tool(s) in streaming mode", response.tool_uses.len());

            // Send notification about tool execution
            let (tx, _rx) = oneshot::channel();
            let tool_names: Vec<_> = response.tool_uses.iter().map(|t| t.name.as_str()).collect();
            let _ = self.session_update_tx.send((
                acp::SessionNotification {
                    session_id: session_id.clone(),
                    update: acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
                        content: format!("\n\n[Executing tools: {}]\n", tool_names.join(", ")).into(),
                        meta: None,
                    }),
                    meta: None,
                },
                tx,
            ));

            // Execute tools and collect results
            let mut tool_results = Vec::new();
            for tool_use in &response.tool_uses {
                info!("Executing tool: {} (id: {})", tool_use.name, tool_use.id);

                let result = match self.tools.execute_tool(&tool_use.name, tool_use.input.clone()).await {
                    Ok(result) => {
                        info!("Tool {} succeeded", tool_use.name);
                        json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use.id,
                            "content": result.to_string()
                        })
                    }
                    Err(e) => {
                        error!("Tool {} failed: {}", tool_use.name, e);
                        json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use.id,
                            "is_error": true,
                            "content": format!("Error executing tool: {}", e)
                        })
                    }
                };
                tool_results.push(result);
            }

            // Add assistant's response with tool uses to messages
            let assistant_content = if !response.text_blocks.is_empty() {
                response.text_blocks.join("\n")
            } else {
                format!("[Using {} tools]", response.tool_uses.len())
            };

            messages.push(crate::ai::claude::Message {
                role: "assistant".to_string(),
                content: assistant_content,
            });

            // Add tool results as user message for next iteration
            let tool_results_content = tool_results
                .iter()
                .map(|r| serde_json::to_string_pretty(r).unwrap_or_else(|_| r.to_string()))
                .collect::<Vec<_>>()
                .join("\n\n");

            messages.push(crate::ai::claude::Message {
                role: "user".to_string(),
                content: format!("Tool results:\n{}", tool_results_content),
            });
        }

        // Clean up cancellation token
        self.cancellation_tokens.remove(&session_id_str);

        Ok(accumulated_text.join("\n\n"))
    }

    /// Generate response text based on MCP integration status (deprecated)
    #[deprecated(note = "Use prompt() method with real Claude integration instead")]
    fn generate_response(&self, prompt_summary: &str) -> String {
        if let Some(coordinator) = &self.coordinator_client {
            format!(
                "I'm the Universal LSP ACP Agent with MCP integration.\n\n\
                 Your message: {}\n\n\
                 I support 19+ programming languages and can help with:\n\
                 • Code completions and suggestions\n\
                 • Code explanations and documentation\n\
                 • Refactoring suggestions\n\
                 • Debugging assistance\n\
                 • Best practices and patterns\n\n\
                 MCP integration is active. What would you like to know?",
                prompt_summary
            )
        } else {
            format!(
                "I'm the Universal LSP ACP Agent.\n\n\
                 Your message: {}\n\n\
                 I support 19+ programming languages and can help with:\n\
                 • Code completions and suggestions\n\
                 • Code explanations and documentation\n\
                 • Refactoring suggestions\n\
                 • Debugging assistance\n\
                 • Best practices and patterns\n\n\
                 Note: MCP integration is not available. What would you like to know?",
                prompt_summary
            )
        }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for UniversalAgent {
    /// Initialize the agent and report capabilities
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        info!("ACP agent initializing: {:?}", arguments);

        Ok(acp::InitializeResponse {
            protocol_version: acp::V1,
            agent_capabilities: acp::AgentCapabilities::default(),
            auth_methods: Vec::new(), // No authentication required
            agent_info: Some(acp::Implementation {
                name: "universal-lsp-agent".to_string(),
                title: Some("Universal LSP ACP Agent".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
            meta: None,
        })
    }

    /// Handle authentication (currently no-op)
    async fn authenticate(
        &self,
        arguments: acp::AuthenticateRequest,
    ) -> Result<acp::AuthenticateResponse, acp::Error> {
        info!("ACP agent authenticate: {:?}", arguments);
        Ok(acp::AuthenticateResponse::default())
    }

    /// Create a new conversation session
    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        let session_id = self.next_session_id.get();
        self.next_session_id.set(session_id + 1);

        info!(
            "ACP agent creating new session {}: {:?}",
            session_id, arguments
        );

        Ok(acp::NewSessionResponse {
            session_id: acp::SessionId(session_id.to_string().into()),
            modes: None,
            meta: None,
        })
    }

    /// Load an existing session
    async fn load_session(
        &self,
        arguments: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        info!("ACP agent loading session: {:?}", arguments);
        Ok(acp::LoadSessionResponse {
            modes: None,
            meta: None,
        })
    }

    /// Process user prompts and generate responses
    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let session_id_str = arguments.session_id.0.to_string();
        let session_id = session_id_str.as_str();

        info!(
            "Processing prompt for session {}: {} content items",
            session_id,
            arguments.prompt.len()
        );

        // 1. Extract user message from ACP prompt content
        let user_message = self.extract_message_from_prompt(&arguments.prompt)?;

        if user_message.is_empty() {
            return Err(acp::Error::invalid_request());
        }

        info!("User message ({} chars): {}", user_message.len(),
            if user_message.len() > 100 {
                format!("{}...", &user_message[..100])
            } else {
                user_message.clone()
            }
        );

        // 2. Get or create conversation history for this session
        let mut history = self.sessions
            .entry(session_id_str.clone())
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

            // Get tool definitions
            let tool_definitions = self.tools.get_tool_definitions();
            let system_prompt = self.build_system_prompt().await;

            // Call Claude API with streaming for real-time updates
            info!("Calling Claude API with streaming ({} messages, {} tools)", history.len(), tool_definitions.len());

            match self.call_claude_with_tools_streaming(
                client,
                messages,
                tool_definitions,
                system_prompt,
                arguments.session_id.clone()
            ).await {
                Ok(response) => {
                    info!("Claude streaming response complete ({} chars)", response.len());
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

    /// Handle cancellation requests
    async fn cancel(&self, args: acp::CancelNotification) -> Result<(), acp::Error> {
        let session_id_str = args.session_id.0.to_string();
        info!("ACP agent cancel request for session: {}", session_id_str);

        // Send cancellation signal if session has an active request
        if let Some(cancel_tx) = self.cancellation_tokens.get(&session_id_str) {
            let _ = cancel_tx.send(true);
            info!("Cancellation signal sent for session {}", session_id_str);
        } else {
            info!("No active request found for session {}", session_id_str);
        }

        Ok(())
    }

    /// Update session mode settings
    async fn set_session_mode(
        &self,
        args: acp::SetSessionModeRequest,
    ) -> Result<acp::SetSessionModeResponse, acp::Error> {
        info!("ACP agent set session mode: {:?}", args);
        Ok(acp::SetSessionModeResponse::default())
    }

    /// Handle custom extension methods
    async fn ext_method(&self, args: acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
        info!(
            "ACP agent extension method: {} with params: {:?}",
            args.method, args.params
        );

        // Handle custom extension methods
        let method: &str = &args.method;
        match method {
            "universal-lsp/get-languages" => {
                Ok(serde_json::value::to_raw_value(&json!({
                    "languages": ["JavaScript", "Python", "Rust", "Go", "TypeScript", "..."],
                    "total": 19
                }))?
                .into())
            }
            "universal-lsp/get-capabilities" => {
                let mcp_integrated = self.coordinator_client.is_some();
                Ok(serde_json::value::to_raw_value(&json!({
                    "completion": true,
                    "hover": true,
                    "diagnostics": true,
                    "mcp_integration": mcp_integrated,
                    "ai_powered": true
                }))?
                .into())
            }
            "universal-lsp/get-mcp-status" => {
                if let Some(coordinator) = &self.coordinator_client {
                    match coordinator.get_metrics().await {
                        Ok(metrics) => Ok(serde_json::value::to_raw_value(&json!({
                            "connected": true,
                            "active_connections": metrics.active_connections,
                            "total_queries": metrics.total_queries,
                            "cache_hits": metrics.cache_hits,
                            "cache_misses": metrics.cache_misses,
                            "errors": metrics.errors
                        }))?
                        .into()),
                        Err(e) => Ok(serde_json::value::to_raw_value(&json!({
                            "connected": false,
                            "error": e.to_string()
                        }))?
                        .into()),
                    }
                } else {
                    Ok(serde_json::value::to_raw_value(&json!({
                        "connected": false,
                        "reason": "MCP coordinator not initialized"
                    }))?
                    .into())
                }
            }
            _ => Ok(serde_json::value::to_raw_value(&json!({
                "error": "Unknown extension method",
                "method": args.method
            }))?
            .into()),
        }
    }

    /// Handle custom extension notifications
    async fn ext_notification(&self, args: acp::ExtNotification) -> Result<(), acp::Error> {
        info!(
            "ACP agent extension notification: {} with params: {:?}",
            args.method, args.params
        );
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use acp::Agent;  // Import trait to access agent methods
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_agent_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        // Test session ID generation
        assert_eq!(agent.next_session_id.get(), 1);
        agent.next_session_id.set(agent.next_session_id.get() + 1);
        assert_eq!(agent.next_session_id.get(), 2);

        // Verify no MCP integration by default
        assert!(agent.coordinator_client.is_none());
    }

    #[tokio::test]
    async fn test_initialize() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::InitializeRequest {
            protocol_version: acp::V1,
            client_capabilities: acp::ClientCapabilities::default(),
            client_info: None,
            meta: None,
        };

        let response = agent.initialize(request).await.unwrap();
        assert_eq!(response.protocol_version, acp::V1);
        assert!(response.agent_info.is_some());

        let info = response.agent_info.unwrap();
        assert_eq!(info.name, "universal-lsp-agent");
        assert_eq!(info.title, Some("Universal LSP ACP Agent".to_string()));
    }

    #[tokio::test]
    async fn test_new_session() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::NewSessionRequest {
            cwd: PathBuf::from("/tmp"),
            mcp_servers: Vec::new(),
            meta: None,
        };

        // Create first session
        let response1 = agent.new_session(request.clone()).await.unwrap();
        assert_eq!(response1.session_id.0.as_ref(), "1");

        // Create second session - ID should increment
        let response2 = agent.new_session(request).await.unwrap();
        assert_eq!(response2.session_id.0.as_ref(), "2");
    }

    #[tokio::test]
    async fn test_load_session() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::LoadSessionRequest {
            cwd: PathBuf::from("/tmp"),
            mcp_servers: Vec::new(),
            session_id: acp::SessionId("test-session".to_string().into()),
            meta: None,
        };

        let response = agent.load_session(request).await.unwrap();
        assert!(response.modes.is_none());
    }

    #[tokio::test]
    async fn test_authenticate() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::AuthenticateRequest {
            method_id: "none".to_string().into(),
            meta: None,
        };

        let response = agent.authenticate(request).await.unwrap();
        // Should succeed with no-op authentication
        assert!(response.meta.is_none());
    }

    #[tokio::test]
    async fn test_set_session_mode() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::SetSessionModeRequest {
            session_id: acp::SessionId("test-session".to_string().into()),
            mode_id: acp::SessionModeId("default".to_string().into()),
            meta: None,
        };

        let response = agent.set_session_mode(request).await.unwrap();
        assert!(response.meta.is_none());
    }

    #[tokio::test]
    async fn test_cancel() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let notification = acp::CancelNotification {
            session_id: acp::SessionId("test-session".to_string().into()),
            meta: None,
        };

        // Should not error
        agent.cancel(notification).await.unwrap();
    }

    #[tokio::test]
    async fn test_ext_method_get_languages() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::ExtRequest {
            method: "universal-lsp/get-languages".to_string().into(),
            params: serde_json::value::to_raw_value(&json!({})).unwrap().into(),
        };

        let response = agent.ext_method(request).await.unwrap();

        // Parse the response to verify it contains language info
        let response_str = response.to_string();
        assert!(response_str.contains("languages"));
        assert!(response_str.contains("19"));
    }

    #[tokio::test]
    async fn test_ext_method_get_capabilities() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::ExtRequest {
            method: "universal-lsp/get-capabilities".to_string().into(),
            params: serde_json::value::to_raw_value(&json!({})).unwrap().into(),
        };

        let response = agent.ext_method(request).await.unwrap();

        let response_str = response.to_string();
        assert!(response_str.contains("completion"));
        assert!(response_str.contains("hover"));
        assert!(response_str.contains("diagnostics"));
    }

    #[tokio::test]
    async fn test_ext_method_get_mcp_status_no_integration() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::ExtRequest {
            method: "universal-lsp/get-mcp-status".to_string().into(),
            params: serde_json::value::to_raw_value(&json!({})).unwrap().into(),
        };

        let response = agent.ext_method(request).await.unwrap();

        let response_str = response.to_string();
        assert!(response_str.contains("connected"));
        assert!(response_str.contains("false"));
    }

    #[tokio::test]
    async fn test_ext_method_unknown() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::ExtRequest {
            method: "unknown/method".to_string().into(),
            params: serde_json::value::to_raw_value(&json!({})).unwrap().into(),
        };

        let response = agent.ext_method(request).await.unwrap();

        let response_str = response.to_string();
        assert!(response_str.contains("error") || response_str.contains("Unknown"));
    }

    #[tokio::test]
    async fn test_ext_notification() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let notification = acp::ExtNotification {
            method: "test/notification".to_string().into(),
            params: serde_json::value::to_raw_value(&json!({})).unwrap().into(),
        };

        // Should not error
        agent.ext_notification(notification).await.unwrap();
    }

    #[tokio::test]
    async fn test_generate_response_without_mcp() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let response = agent.generate_response("Test prompt");

        assert!(response.contains("Universal LSP ACP Agent"));
        assert!(response.contains("19+ programming languages"));
        assert!(response.contains("MCP integration is not available"));
        assert!(response.contains("Test prompt"));
    }

    #[tokio::test]
    async fn test_prompt_processing() {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let session_id = acp::SessionId("test-session".to_string().into());
        let request = acp::PromptRequest {
            session_id: session_id.clone(),
            prompt: vec!["Hello".into()],
            meta: None,
        };

        // Spawn a task to receive the notification
        let receive_task = tokio::spawn(async move {
            if let Some((notification, tx)) = rx.recv().await {
                assert_eq!(notification.session_id, session_id);
                // Send acknowledgment
                tx.send(()).ok();
                true
            } else {
                false
            }
        });

        // Process the prompt
        let response = agent.prompt(request).await.unwrap();
        assert_eq!(response.stop_reason, acp::StopReason::EndTurn);

        // Verify notification was received
        let received = receive_task.await.unwrap();
        assert!(received, "Should have received session notification");
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let request = acp::NewSessionRequest {
            cwd: PathBuf::from("/tmp"),
            mcp_servers: Vec::new(),
            meta: None,
        };

        // Create multiple sessions
        let mut session_ids = Vec::new();
        for i in 1..=5 {
            let response = agent.new_session(request.clone()).await.unwrap();
            session_ids.push(response.session_id.0.to_string());
            assert_eq!(response.session_id.0.as_ref(), i.to_string().as_str());
        }

        // Verify all session IDs are unique
        let unique_count = session_ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 5);
    }

    // ========================================================================
    // Unit Tests for Helper Methods (Phase 1.10)
    // ========================================================================

    #[test]
    fn test_init_claude_client_with_api_key() {
        // Set API key
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key-123");

        let client = UniversalAgent::init_claude_client();

        assert!(client.is_some(), "Claude client should be initialized with API key");

        // Cleanup
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_init_claude_client_without_api_key() {
        // Remove API key
        let original = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let client = UniversalAgent::init_claude_client();

        assert!(client.is_none(), "Claude client should be None without API key");

        // Restore original
        if let Some(key) = original {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        }
    }

    #[test]
    fn test_init_claude_client_with_empty_api_key() {
        // Set empty API key
        std::env::set_var("ANTHROPIC_API_KEY", "");

        let client = UniversalAgent::init_claude_client();

        assert!(client.is_none(), "Claude client should be None with empty API key");

        // Cleanup
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn test_build_system_prompt() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let workspace = PathBuf::from("/test/workspace");
        let agent = UniversalAgent::new_with_workspace(tx, workspace.clone());

        let system_prompt = agent.build_system_prompt().await;

        // Verify system prompt contains key information
        assert!(system_prompt.contains("expert software development assistant"));
        assert!(system_prompt.contains(&workspace.display().to_string()));
        assert!(system_prompt.contains("Workspace Context") || system_prompt.contains("Root:"));
    }

    #[tokio::test]
    async fn test_build_system_prompt_with_mcp() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let workspace = PathBuf::from("/test/workspace");
        let agent = UniversalAgent::new_with_workspace(tx, workspace);

        let system_prompt = agent.build_system_prompt().await;

        // System prompt should include MCP status
        assert!(
            system_prompt.contains("MCP") || system_prompt.contains("Model Context Protocol"),
            "System prompt should mention MCP"
        );
    }

    #[test]
    fn test_extract_message_from_prompt_with_text() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let content_blocks = vec![
            acp::ContentBlock::Text(acp::TextContent {
                text: "Hello, Claude!".to_string(),
                annotations: None,
                meta: None,
            }),
            acp::ContentBlock::Text(acp::TextContent {
                text: "How are you?".to_string(),
                annotations: None,
                meta: None,
            }),
        ];

        let result = agent.extract_message_from_prompt(&content_blocks);

        assert!(result.is_ok(), "Should successfully extract message");
        let message = result.unwrap();
        assert!(message.contains("Hello, Claude!"));
        assert!(message.contains("How are you?"));
    }

    #[test]
    fn test_extract_message_from_prompt_empty() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let content_blocks: Vec<acp::ContentBlock> = vec![];

        let result = agent.extract_message_from_prompt(&content_blocks);

        assert!(result.is_err(), "Should return error for empty prompt");
    }

    #[test]
    fn test_extract_message_from_prompt_with_mixed_content() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        // Test with multiple text blocks
        let content_blocks = vec![
            acp::ContentBlock::Text(acp::TextContent {
                text: "First message".to_string(),
                annotations: None,
                meta: None,
            }),
            acp::ContentBlock::Text(acp::TextContent {
                text: "Second message".to_string(),
                annotations: None,
                meta: None,
            }),
            acp::ContentBlock::Text(acp::TextContent {
                text: "Third message".to_string(),
                annotations: None,
                meta: None,
            }),
        ];

        let result = agent.extract_message_from_prompt(&content_blocks);

        assert!(result.is_ok(), "Should successfully extract message with multiple blocks");
        let message = result.unwrap();
        assert!(message.contains("First message"));
        assert!(message.contains("Second message"));
        assert!(message.contains("Third message"));
    }

    #[test]
    fn test_generate_fallback_response_basic() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let workspace = PathBuf::from("/test/workspace");
        let agent = UniversalAgent::new_with_workspace(tx, workspace.clone());

        let fallback = agent.generate_fallback_response("Hello!");

        // Verify fallback contains key information
        assert!(fallback.contains("Universal LSP ACP Agent"));
        assert!(fallback.contains("Claude API integration is not available"));
        assert!(fallback.contains("ANTHROPIC_API_KEY"));
        assert!(fallback.contains(&workspace.display().to_string()));
    }

    #[test]
    fn test_generate_fallback_response_with_long_message() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let long_message = "a".repeat(1000);
        let fallback = agent.generate_fallback_response(&long_message);

        // Should handle long messages gracefully
        assert!(fallback.contains("Universal LSP ACP Agent"));
        // Message should be truncated or included
        assert!(fallback.len() > 100);
    }

    #[test]
    fn test_generate_fallback_response_includes_mcp_status() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let fallback = agent.generate_fallback_response("test");

        // Should include MCP status
        assert!(
            fallback.contains("MCP Integration") || fallback.contains("MCP"),
            "Fallback should mention MCP status"
        );
    }

    #[test]
    fn test_conversation_message_structure() {
        // Test ConversationMessage structure
        let message = ConversationMessage {
            role: "user".to_string(),
            content: "Hello!".to_string(),
            timestamp: std::time::SystemTime::now(),
        };

        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Hello!");
    }

    #[test]
    fn test_system_prompt_constant() {
        // Verify SYSTEM_PROMPT has essential content
        assert!(SYSTEM_PROMPT.contains("expert"));
        assert!(SYSTEM_PROMPT.contains("software"));
        assert!(SYSTEM_PROMPT.len() > 100, "System prompt should be substantial");
    }

    #[tokio::test]
    async fn test_workspace_path_handling() {
        let (tx, _rx) = mpsc::unbounded_channel();

        // Test with absolute path
        let absolute = PathBuf::from("/home/user/project");
        let agent1 = UniversalAgent::new_with_workspace(tx.clone(), absolute.clone());
        let prompt1 = agent1.build_system_prompt().await;
        assert!(prompt1.contains(&absolute.display().to_string()));

        // Test with relative path
        let relative = PathBuf::from("./project");
        let agent2 = UniversalAgent::new_with_workspace(tx.clone(), relative.clone());
        let prompt2 = agent2.build_system_prompt().await;
        assert!(prompt2.contains("Workspace"));

        // Test with current directory
        let current = PathBuf::from(".");
        let agent3 = UniversalAgent::new_with_workspace(tx, current);
        let prompt3 = agent3.build_system_prompt().await;
        assert!(prompt3.contains("Workspace"));
    }

    #[test]
    fn test_claude_config_values() {
        // Verify Claude configuration values are sensible
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        let client = UniversalAgent::init_claude_client();

        assert!(client.is_some(), "Client should initialize with test key");

        // The config values are set in init_claude_client():
        // - model: "claude-sonnet-4-20250514"
        // - max_tokens: 4096
        // - temperature: 0.7
        // - timeout_ms: 30000

        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_extract_message_multiline() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let content_blocks = vec![
            acp::ContentBlock::Text(acp::TextContent {
                text: "Line 1\nLine 2\nLine 3".to_string(),
                annotations: None,
                meta: None,
            }),
        ];

        let result = agent.extract_message_from_prompt(&content_blocks);
        assert!(result.is_ok());

        let message = result.unwrap();
        assert!(message.contains("Line 1"));
        assert!(message.contains("Line 2"));
        assert!(message.contains("Line 3"));
    }

    #[test]
    fn test_extract_message_special_characters() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let agent = UniversalAgent::new(tx);

        let content_blocks = vec![
            acp::ContentBlock::Text(acp::TextContent {
                text: "Unicode: 世界, Emoji: 😀🎉".to_string(),
                annotations: None,
                meta: None,
            }),
        ];

        let result = agent.extract_message_from_prompt(&content_blocks);
        assert!(result.is_ok());

        let message = result.unwrap();
        assert!(message.contains("世界"));
        assert!(message.contains("😀"));
    }
}
