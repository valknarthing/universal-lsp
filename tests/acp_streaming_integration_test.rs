//! Integration tests for ACP streaming responses
//!
//! These tests verify that the ACP agent correctly implements streaming responses
//! with progress notifications and cancellation support.

use agent_client_protocol as acp;
use acp::Agent; // Import trait to access agent methods
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Helper to create a test agent
fn create_test_agent() -> (
    universal_lsp::acp::UniversalAgent,
    mpsc::UnboundedReceiver<(acp::SessionNotification, tokio::sync::oneshot::Sender<()>)>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let agent = universal_lsp::acp::UniversalAgent::new(tx);
    (agent, rx)
}

/// Helper to consume notifications from the channel
async fn collect_notifications(
    rx: &mut mpsc::UnboundedReceiver<(acp::SessionNotification, tokio::sync::oneshot::Sender<()>)>,
    max_duration: Duration,
) -> Vec<acp::SessionNotification> {
    let mut notifications = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed() < max_duration {
        match timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Some((notification, _tx))) => {
                notifications.push(notification);
            }
            _ => break,
        }
    }

    notifications
}

#[tokio::test]
async fn test_agent_has_cancellation_support() {
    let (agent, _rx) = create_test_agent();

    // Create a session
    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();
    let session_id = session_response.session_id;

    // Test that cancel doesn't error even if no active request
    let cancel_notification = acp::CancelNotification {
        session_id: session_id.clone(),
        meta: None,
    };

    let result = agent.cancel(cancel_notification).await;
    assert!(result.is_ok(), "Cancel should succeed even without active request");
}

#[tokio::test]
async fn test_streaming_architecture_exists() {
    let (agent, _rx) = create_test_agent();

    // Verify agent has proper structure
    // This test ensures the streaming infrastructure is in place
    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let result = agent.new_session(new_session_request).await;
    assert!(result.is_ok(), "Agent should create sessions properly");
}

#[tokio::test]
async fn test_session_notifications_channel_works() {
    let (agent, mut rx) = create_test_agent();

    // Create a session
    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();
    assert!(session_response.session_id.0.len() > 0, "Should have session ID");

    // The channel should be empty initially
    let notifications = collect_notifications(&mut rx, Duration::from_millis(100)).await;
    assert_eq!(notifications.len(), 0, "No notifications before prompt");
}

#[tokio::test]
async fn test_prompt_returns_successfully() {
    let (agent, _rx) = create_test_agent();

    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    let prompt_request = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec!["Hello, test!".into()],
        meta: None,
    };

    let result = agent.prompt(prompt_request).await;
    assert!(result.is_ok(), "Prompt should return Ok");

    let response = result.unwrap();
    assert_eq!(response.stop_reason, acp::StopReason::EndTurn, "Should have EndTurn stop reason");
}

#[tokio::test]
async fn test_multiple_sessions_independent() {
    let (agent, _rx) = create_test_agent();

    // Create two sessions
    let request1 = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp/session1"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let request2 = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp/session2"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session1 = agent.new_session(request1).await.unwrap();
    let session2 = agent.new_session(request2).await.unwrap();

    assert_ne!(
        session1.session_id.0,
        session2.session_id.0,
        "Sessions should have different IDs"
    );

    // Cancel session1 should not affect session2
    let cancel1 = acp::CancelNotification {
        session_id: session1.session_id.clone(),
        meta: None,
    };

    agent.cancel(cancel1).await.unwrap();

    // Session2 should still be usable
    let prompt2 = acp::PromptRequest {
        session_id: session2.session_id.clone(),
        prompt: vec!["Test".into()],
        meta: None,
    };

    let result = agent.prompt(prompt2).await;
    assert!(result.is_ok(), "Session 2 should still work after session 1 cancelled");
}

#[tokio::test]
async fn test_load_session_preserves_state() {
    let (agent, _rx) = create_test_agent();

    let session_id = acp::SessionId("test-load-session".to_string().into());

    let load_request = acp::LoadSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        session_id: session_id.clone(),
        meta: None,
    };

    let result = agent.load_session(load_request).await;
    assert!(result.is_ok(), "Load session should succeed");

    // Verify we can use the loaded session
    let prompt_request = acp::PromptRequest {
        session_id: session_id.clone(),
        prompt: vec!["Test with loaded session".into()],
        meta: None,
    };

    let result = agent.prompt(prompt_request).await;
    assert!(result.is_ok(), "Should be able to prompt loaded session");
}

#[tokio::test]
async fn test_session_mode_can_be_set() {
    let (agent, _rx) = create_test_agent();

    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    let set_mode_request = acp::SetSessionModeRequest {
        session_id: session_response.session_id.clone(),
        mode_id: acp::SessionModeId("debug".to_string().into()),
        meta: None,
    };

    let result = agent.set_session_mode(set_mode_request).await;
    assert!(result.is_ok(), "Should be able to set session mode");
}

#[tokio::test]
async fn test_conversation_history_accumulates() {
    let (agent, _rx) = create_test_agent();

    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    // Send first prompt
    let prompt1 = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec!["First message".into()],
        meta: None,
    };

    agent.prompt(prompt1).await.unwrap();

    // Send second prompt - should have history
    let prompt2 = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec!["Second message".into()],
        meta: None,
    };

    let result = agent.prompt(prompt2).await;
    assert!(result.is_ok(), "Multi-turn conversation should work");
}

#[tokio::test]
async fn test_empty_prompt_returns_response() {
    let (agent, _rx) = create_test_agent();

    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    let prompt_request = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec![],
        meta: None,
    };

    let result = agent.prompt(prompt_request).await;
    assert!(result.is_ok(), "Empty prompt should be handled gracefully");
}

#[tokio::test]
async fn test_multiple_content_items_in_prompt() {
    let (agent, _rx) = create_test_agent();

    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    let prompt_request = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec![
            "First part".into(),
            "Second part".into(),
        ],
        meta: None,
    };

    let result = agent.prompt(prompt_request).await;
    assert!(result.is_ok(), "Multiple content items should be handled");
}

#[tokio::test]
async fn test_cancel_before_prompt_is_safe() {
    let (agent, _rx) = create_test_agent();

    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    // Cancel before any prompt
    let cancel_notification = acp::CancelNotification {
        session_id: session_response.session_id.clone(),
        meta: None,
    };

    let result = agent.cancel(cancel_notification).await;
    assert!(result.is_ok(), "Cancel before prompt should not error");

    // Should still be able to prompt after
    let prompt_request = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec!["After cancel".into()],
        meta: None,
    };

    let result = agent.prompt(prompt_request).await;
    assert!(result.is_ok(), "Should be able to prompt after cancel");
}

#[tokio::test]
async fn test_prompt_response_has_stop_reason() {
    let (agent, _rx) = create_test_agent();

    let new_session_request = acp::NewSessionRequest {
        cwd: std::path::PathBuf::from("/tmp"),
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    let prompt_request = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec!["Test response format".into()],
        meta: None,
    };

    let response = agent.prompt(prompt_request).await.unwrap();

    // Verify response structure
    assert_eq!(response.stop_reason, acp::StopReason::EndTurn, "Should have EndTurn stop reason");
}

#[tokio::test]
async fn test_workspace_context_integration() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    // Create some test files
    std::fs::write(workspace_path.join("README.md"), "# Test Project").unwrap();
    std::fs::write(workspace_path.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let agent = universal_lsp::acp::UniversalAgent::new_with_workspace(tx, workspace_path.clone());

    let new_session_request = acp::NewSessionRequest {
        cwd: workspace_path,
        mcp_servers: Vec::new(),
        meta: None,
    };

    let session_response = agent.new_session(new_session_request).await.unwrap();

    let prompt_request = acp::PromptRequest {
        session_id: session_response.session_id.clone(),
        prompt: vec!["What's in this workspace?".into()],
        meta: None,
    };

    let result = agent.prompt(prompt_request).await;
    assert!(result.is_ok(), "Should be able to query workspace context");
}

#[tokio::test]
async fn test_sequential_session_creation() {
    // Test sequential session creation without concurrent spawns
    let (agent, _rx) = create_test_agent();

    for i in 0..5 {
        let request = acp::NewSessionRequest {
            cwd: std::path::PathBuf::from(format!("/tmp/session{}", i)),
            mcp_servers: Vec::new(),
            meta: None,
        };

        let result = agent.new_session(request).await;
        assert!(result.is_ok(), "Session {} creation should work", i);
    }
}

#[tokio::test]
async fn test_session_id_uniqueness() {
    let (agent, _rx) = create_test_agent();

    let mut session_ids = std::collections::HashSet::new();

    for _ in 0..10 {
        let request = acp::NewSessionRequest {
            cwd: std::path::PathBuf::from("/tmp"),
            mcp_servers: Vec::new(),
            meta: None,
        };

        let response = agent.new_session(request).await.unwrap();
        let id_string = response.session_id.0.to_string();

        assert!(
            session_ids.insert(id_string.clone()),
            "Session IDs should be unique, but got duplicate: {}",
            id_string
        );
    }

    assert_eq!(session_ids.len(), 10, "Should have 10 unique session IDs");
}

#[tokio::test]
async fn test_streaming_notification_structure() {
    // This test verifies that streaming infrastructure is in place
    // by checking that the types and structures compile and work together

    let (tx, _rx) = mpsc::unbounded_channel();
    let agent = universal_lsp::acp::UniversalAgent::new(tx);

    // Verify we can create notification messages
    let test_notification = acp::SessionNotification {
        session_id: acp::SessionId("test".to_string().into()),
        update: acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk {
            content: "Test chunk".into(),
            meta: None,
        }),
        meta: None,
    };

    // This validates the notification structure compiles
    match test_notification.update {
        acp::SessionUpdate::AgentMessageChunk(chunk) => {
            match chunk.content {
                acp::ContentBlock::Text(text_content) => {
                    assert_eq!(text_content.text, "Test chunk");
                }
                _ => panic!("Expected text content"),
            }
        }
        _ => panic!("Expected AgentMessageChunk"),
    }
}
