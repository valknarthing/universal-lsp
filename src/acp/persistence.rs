//! Session Persistence for ACP Agent
//!
//! This module handles saving and loading conversation history to disk,
//! enabling multi-turn conversations that persist across restarts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use super::ConversationMessage;

/// Maximum number of messages to keep in a session before summarization
const MAX_MESSAGES_BEFORE_SUMMARY: usize = 50;

/// Approximate token limit for context window (Claude Sonnet 4 has 200k, use conservative 100k)
const MAX_TOKENS_ESTIMATE: usize = 100_000;

/// Rough estimate: 1 token ≈ 4 characters
const CHARS_PER_TOKEN: usize = 4;

/// Persisted session data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    /// Unique session identifier
    pub session_id: String,
    /// Workspace root directory
    pub cwd: PathBuf,
    /// Conversation message history
    pub messages: Vec<PersistedMessage>,
    /// Session creation timestamp
    pub created_at: i64,
    /// Last update timestamp
    pub updated_at: i64,
    /// Estimated total tokens used (rough calculation)
    pub estimated_tokens: usize,
}

/// Persisted message structure (serializable version of ConversationMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedMessage {
    /// Message role ("user" or "assistant")
    pub role: String,
    /// Message content text
    pub content: String,
    /// Unix timestamp when message was created
    pub timestamp: i64,
    /// Estimated token count for this message
    pub estimated_tokens: usize,
}

impl From<&ConversationMessage> for PersistedMessage {
    fn from(msg: &ConversationMessage) -> Self {
        let content = msg.content.clone();
        let estimated_tokens = estimate_tokens(&content);

        Self {
            role: msg.role.clone(),
            content,
            timestamp: msg.timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            estimated_tokens,
        }
    }
}

impl PersistedMessage {
    /// Convert to ConversationMessage
    pub fn to_conversation_message(&self) -> ConversationMessage {
        ConversationMessage {
            role: self.role.clone(),
            content: self.content.clone(),
            timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp as u64),
        }
    }
}

/// Session persistence manager
#[derive(Debug, Clone)]
pub struct SessionPersistence {
    /// Base directory for storing sessions
    storage_dir: PathBuf,
}

impl SessionPersistence {
    /// Create a new session persistence manager
    pub fn new() -> Result<Self> {
        // Use ~/.universal-lsp/sessions/ for storage
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        let storage_dir = home.join(".universal-lsp").join("sessions");

        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_dir)
            .context("Failed to create session storage directory")?;

        info!("Session persistence initialized: {}", storage_dir.display());

        Ok(Self { storage_dir })
    }

    /// Get the file path for a session
    fn session_path(&self, session_id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", session_id))
    }

    /// Save a session to disk
    pub fn save_session(&self, session: &PersistedSession) -> Result<()> {
        let path = self.session_path(&session.session_id);
        let temp_path = path.with_extension("json.tmp");

        // Serialize to JSON with pretty formatting
        let json = serde_json::to_string_pretty(session)
            .context("Failed to serialize session")?;

        // Write to temporary file first (atomic write pattern)
        fs::write(&temp_path, json)
            .context("Failed to write temporary session file")?;

        // Rename to final location (atomic on most filesystems)
        fs::rename(&temp_path, &path)
            .context("Failed to finalize session file")?;

        debug!("Saved session {} ({} messages, ~{} tokens)",
            session.session_id, session.messages.len(), session.estimated_tokens);

        Ok(())
    }

    /// Load a session from disk
    pub fn load_session(&self, session_id: &str) -> Result<PersistedSession> {
        let path = self.session_path(session_id);

        if !path.exists() {
            return Err(anyhow::anyhow!("Session file not found: {}", session_id));
        }

        let json = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session file: {}", path.display()))?;

        let session: PersistedSession = serde_json::from_str(&json)
            .with_context(|| format!("Failed to parse session file: {}", path.display()))?;

        info!("Loaded session {} ({} messages)", session_id, session.messages.len());

        Ok(session)
    }

    /// Check if a session exists on disk
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.session_path(session_id).exists()
    }

    /// Delete a session from disk
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let path = self.session_path(session_id);

        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete session: {}", session_id))?;
            info!("Deleted session {}", session_id);
        }

        Ok(())
    }

    /// List all available sessions
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    sessions.push(file_name.to_string());
                }
            }
        }

        Ok(sessions)
    }

    /// Clean up old sessions (optional: can be called periodically)
    pub fn cleanup_old_sessions(&self, days: u64) -> Result<usize> {
        let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86400);
        let mut deleted = 0;

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        if let Err(e) = fs::remove_file(&path) {
                            warn!("Failed to delete old session {:?}: {}", path, e);
                        } else {
                            deleted += 1;
                        }
                    }
                }
            }
        }

        if deleted > 0 {
            info!("Cleaned up {} old sessions", deleted);
        }

        Ok(deleted)
    }
}

impl Default for SessionPersistence {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            error!("Failed to initialize session persistence: {}", e);
            // Fallback to a temporary directory
            Self {
                storage_dir: std::env::temp_dir().join("universal-lsp-sessions"),
            }
        })
    }
}

/// Estimate token count for a string (rough approximation)
pub fn estimate_tokens(text: &str) -> usize {
    // Simple heuristic: ~4 characters per token
    text.len() / CHARS_PER_TOKEN
}

/// Check if a session should be summarized based on size
pub fn should_summarize(messages: &[ConversationMessage]) -> bool {
    if messages.len() < MAX_MESSAGES_BEFORE_SUMMARY {
        return false;
    }

    // Estimate total tokens
    let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
    let estimated_tokens = total_chars / CHARS_PER_TOKEN;

    estimated_tokens > MAX_TOKENS_ESTIMATE
}

/// Create a summary of old messages to compress context
pub async fn summarize_conversation(
    messages: &[ConversationMessage],
    claude_client: &crate::ai::claude::ClaudeClient,
) -> Result<String> {
    // Take the first 80% of messages for summarization
    let cutoff = (messages.len() as f32 * 0.8) as usize;
    let to_summarize = &messages[..cutoff];

    // Build conversation text
    let mut conversation_text = String::new();
    for msg in to_summarize {
        conversation_text.push_str(&format!("{}: {}\n\n", msg.role, msg.content));
    }

    // Ask Claude to summarize
    let prompt = format!(
        "Please provide a concise summary of this conversation history. \
         Focus on key points, decisions made, and important context. \
         Keep it under 500 words.\n\n{}",
        conversation_text
    );

    let messages = vec![crate::ai::claude::Message {
        role: "user".to_string(),
        content: prompt,
    }];

    let summary = claude_client.send_message(&messages).await
        .context("Failed to generate conversation summary")?;

    info!("Summarized {} messages into {} chars", to_summarize.len(), summary.len());

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1); // 4 chars = 1 token
        assert_eq!(estimate_tokens("this is a test"), 3); // 14 chars ≈ 3 tokens
    }

    #[test]
    fn test_should_summarize() {
        let mut messages = Vec::new();

        // Not enough messages
        assert!(!should_summarize(&messages));

        // Add many small messages - shouldn't trigger
        for i in 0..30 {
            messages.push(ConversationMessage {
                role: "user".to_string(),
                content: format!("Message {}", i),
                timestamp: std::time::SystemTime::now(),
            });
        }
        assert!(!should_summarize(&messages));

        // Add large messages - should trigger
        // Need >100k tokens, which is >400k chars at 4 chars/token
        // With 30 messages: 400k / 30 ≈ 14k chars each
        for _ in 0..30 {
            let large_content = "x".repeat(15000); // 15k characters each
            messages.push(ConversationMessage {
                role: "assistant".to_string(),
                content: large_content,
                timestamp: std::time::SystemTime::now(),
            });
        }
        // 60 messages with 30×15k = 450k chars = 112.5k tokens > 100k
        assert!(should_summarize(&messages));
    }

    #[test]
    fn test_persisted_message_conversion() {
        let original = ConversationMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            timestamp: std::time::SystemTime::now(),
        };

        let persisted = PersistedMessage::from(&original);
        let restored = persisted.to_conversation_message();

        assert_eq!(restored.role, original.role);
        assert_eq!(restored.content, original.content);
    }

    #[tokio::test]
    async fn test_session_persistence_roundtrip() {
        let persistence = SessionPersistence::new().unwrap();

        let session = PersistedSession {
            session_id: "test-session-roundtrip".to_string(),
            cwd: PathBuf::from("/tmp"),
            messages: vec![
                PersistedMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: 1234567890,
                    estimated_tokens: 1,
                },
                PersistedMessage {
                    role: "assistant".to_string(),
                    content: "Hi there!".to_string(),
                    timestamp: 1234567891,
                    estimated_tokens: 2,
                },
            ],
            created_at: 1234567890,
            updated_at: 1234567891,
            estimated_tokens: 3,
        };

        // Save
        persistence.save_session(&session).unwrap();

        // Load
        let loaded = persistence.load_session(&session.session_id).unwrap();

        assert_eq!(loaded.session_id, session.session_id);
        assert_eq!(loaded.messages.len(), session.messages.len());
        assert_eq!(loaded.estimated_tokens, session.estimated_tokens);

        // Cleanup
        persistence.delete_session(&session.session_id).unwrap();
    }

    #[test]
    fn test_session_exists() {
        let persistence = SessionPersistence::new().unwrap();

        assert!(!persistence.session_exists("nonexistent-session"));
    }
}
