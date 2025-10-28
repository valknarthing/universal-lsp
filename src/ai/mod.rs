//! AI-powered features for Universal LSP Server
//!
//! This module provides integration with various AI providers for intelligent
//! code completion, hover information, and other AI-enhanced features.

pub mod claude;

pub use claude::{ClaudeClient, ClaudeConfig, CompletionContext, CompletionSuggestion};
