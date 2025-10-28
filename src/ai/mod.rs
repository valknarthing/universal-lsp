//! AI-powered features for Universal LSP Server
//!
//! This module provides integration with various AI providers for intelligent
//! code completion, hover information, and other AI-enhanced features.

pub mod claude;
pub mod copilot;

// Re-export provider-specific types
pub use claude::{ClaudeClient, ClaudeConfig};
pub use copilot::{CopilotClient, CopilotConfig};

// Shared types (from claude module, but used by both providers)
pub use claude::{CompletionContext, CompletionSuggestion};
