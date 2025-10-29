//! # AI-Powered Completions Example
//!
//! This example demonstrates how to use Universal LSP's AI-powered completion features
//! with Claude and GitHub Copilot integration.
//!
//! ## Usage
//!
//! ```bash
//! export ANTHROPIC_API_KEY="your-api-key"
//! cargo run --example ai_completions
//! ```
//!
//! ## Features Demonstrated
//!
//! - Claude AI completions
//! - Context-aware suggestions
//! - Multi-tier completion system
//! - Intelligent code generation

use anyhow::Result;
use tokio;
use universal_lsp::{ClaudeClient, ClaudeConfig, CompletionContext};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("🤖 AI-Powered Completions Example");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Check for API key
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) => {
            tracing::info!("✅ Anthropic API key found");
            key
        }
        Err(_) => {
            tracing::error!("❌ ANTHROPIC_API_KEY environment variable not set");
            tracing::info!("   Please set it with: export ANTHROPIC_API_KEY='your-key'");
            return Ok(());
        }
    };

    // Configure Claude client
    let config = ClaudeConfig {
        api_key,
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
    };

    tracing::info!("🔧 Initializing Claude client...");
    tracing::info!("  Model: {}", config.model);
    tracing::info!("  Max tokens: {}", config.max_tokens);

    let client = ClaudeClient::new(config);
    tracing::info!("✅ Claude client initialized");

    // Example 1: JavaScript function completion
    tracing::info!("\n📝 Example 1: JavaScript Function Completion");
    tracing::info!("  Context: function calculateSum(");

    let js_context = CompletionContext {
        prefix: "function calculateSum(".to_string(),
        suffix: ") {\n  // TODO: Implement\n}".to_string(),
        language: "javascript".to_string(),
    };

    tracing::info!("  Requesting AI completions...");
    match client.get_completions(js_context).await {
        Ok(suggestions) => {
            tracing::info!("  ✅ Received {} suggestions:", suggestions.len());
            for (i, suggestion) in suggestions.iter().enumerate().take(3) {
                tracing::info!("    {}. {}", i + 1, suggestion);
            }
        }
        Err(e) => {
            tracing::error!("  ❌ Failed to get completions: {}", e);
        }
    }

    // Example 2: Python class method completion
    tracing::info!("\n📝 Example 2: Python Class Method Completion");
    tracing::info!("  Context: class Calculator: def add(self, ");

    let py_context = CompletionContext {
        prefix: "class Calculator:\n    def __init__(self):\n        self.result = 0\n\n    def add(self, ".to_string(),
        suffix: "):\n        pass".to_string(),
        language: "python".to_string(),
    };

    tracing::info!("  Requesting AI completions...");
    match client.get_completions(py_context).await {
        Ok(suggestions) => {
            tracing::info!("  ✅ Received {} suggestions:", suggestions.len());
            for (i, suggestion) in suggestions.iter().enumerate().take(3) {
                tracing::info!("    {}. {}", i + 1, suggestion);
            }
        }
        Err(e) => {
            tracing::error!("  ❌ Failed to get completions: {}", e);
        }
    }

    // Example 3: Rust trait implementation completion
    tracing::info!("\n📝 Example 3: Rust Trait Implementation");
    tracing::info!("  Context: impl Display for Point { fn fmt(&self, ");

    let rust_context = CompletionContext {
        prefix: "use std::fmt;\n\nstruct Point { x: i32, y: i32 }\n\nimpl fmt::Display for Point {\n    fn fmt(&self, ".to_string(),
        suffix: ") -> fmt::Result {\n        todo!()\n    }\n}".to_string(),
        language: "rust".to_string(),
    };

    tracing::info!("  Requesting AI completions...");
    match client.get_completions(rust_context).await {
        Ok(suggestions) => {
            tracing::info!("  ✅ Received {} suggestions:", suggestions.len());
            for (i, suggestion) in suggestions.iter().enumerate().take(3) {
                tracing::info!("    {}. {}", i + 1, suggestion);
            }
        }
        Err(e) => {
            tracing::error!("  ❌ Failed to get completions: {}", e);
        }
    }

    tracing::info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("🎯 AI-powered completions demonstrated successfully!");
    tracing::info!("💡 Universal LSP provides intelligent, context-aware code");
    tracing::info!("   suggestions powered by Claude AI across 242+ languages.");

    Ok(())
}
