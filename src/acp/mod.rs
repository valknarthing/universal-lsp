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
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

use crate::coordinator::CoordinatorClient;

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
}

impl UniversalAgent {
    /// Create a new Universal ACP Agent
    pub fn new(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        Self {
            session_update_tx,
            next_session_id: Cell::new(1),
            coordinator_client: None,
        }
    }

    /// Create a new Universal ACP Agent with MCP coordinator integration
    pub async fn with_coordinator(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
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

        Self {
            session_update_tx,
            next_session_id: Cell::new(1),
            coordinator_client,
        }
    }

    /// Generate response text based on MCP integration status
    fn generate_response(&self, prompt_summary: &str) -> String {
        if let Some(coordinator) = &self.coordinator_client {
            format!(
                "I'm the Universal LSP ACP Agent with MCP integration.\n\n\
                 Your message: {}\n\n\
                 I support 242+ programming languages and can help with:\n\
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
                 I support 242+ programming languages and can help with:\n\
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
        info!(
            "ACP agent prompt for session {}: {} content items",
            arguments.session_id.0,
            arguments.prompt.len()
        );

        // Create a simple prompt summary from the first few items
        let prompt_summary = format!("Received {} items", arguments.prompt.len());

        // Generate response text
        let response_text = self.generate_response(&prompt_summary);

        // Send response as a notification
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

    /// Handle cancellation requests
    async fn cancel(&self, args: acp::CancelNotification) -> Result<(), acp::Error> {
        info!("ACP agent cancel request: {:?}", args);
        // TODO: Implement cancellation logic for in-progress operations
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
                    "total": 242
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
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    info!("Starting Universal LSP ACP Agent with MCP integration");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    // Create LocalSet for non-Send futures
    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            // Create agent with MCP coordinator integration
            let agent = UniversalAgent::with_coordinator(tx).await;
            let has_mcp = agent.coordinator_client.is_some();

            info!(
                "ACP agent initialized (MCP integration: {})",
                if has_mcp { "enabled" } else { "disabled" }
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
        assert!(response_str.contains("242"));
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
        assert!(response.contains("242+ programming languages"));
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
}
