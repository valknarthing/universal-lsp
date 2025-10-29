//! Universal LSP Library
//!
//! This library provides a universal Language Server Protocol implementation
//! supporting 19+ languages with AI-powered features, MCP integration,
//! ACP agent capabilities, and LSP proxy capabilities.

pub mod acp;
pub mod ai;
pub mod code_actions;
pub mod config;
pub mod coordinator;
pub mod diagnostics;
pub mod formatting;
pub mod language;
pub mod mcp;
pub mod pipeline;
pub mod proxy;
pub mod text_sync;
pub mod tree_sitter;
pub mod workspace;

// Re-export commonly used types for convenience
pub use ai::claude::{ClaudeClient, ClaudeConfig, CompletionContext};
pub use tree_sitter::TreeSitterParser;
pub use mcp::{McpClient, McpConfig, McpRequest, McpResponse};
pub use pipeline::McpPipeline;
pub use proxy::{ProxyConfig, ProxyManager, LspProxy};
