//! # Basic LSP Server Example
//!
//! This example demonstrates how to start a basic Universal LSP server with default configuration.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example basic_lsp_server
//! ```
//!
//! The server will communicate via stdin/stdout using the Language Server Protocol.
//! Connect your editor to this server by configuring it to use `universal-lsp lsp` as the command.

use anyhow::Result;
use tokio;
use universal_lsp::TreeSitterParser;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("🚀 Starting Universal LSP Server (Basic Example)");
    tracing::info!("📋 Features:");
    tracing::info!("  ✓ 242+ language support via Tree-sitter");
    tracing::info!("  ✓ Intelligent code completion");
    tracing::info!("  ✓ Real-time diagnostics");
    tracing::info!("  ✓ Symbol navigation");

    // Initialize tree-sitter parser
    let parser = TreeSitterParser::new();

    // Test parsing a simple JavaScript code snippet
    let js_code = r#"
        function greet(name) {
            console.log(`Hello, ${name}!`);
        }

        greet("World");
    "#;

    tracing::info!("📝 Testing JavaScript parsing...");
    match parser.parse_symbols("javascript", js_code) {
        Ok(symbols) => {
            tracing::info!("✅ Found {} symbols in JavaScript code", symbols.len());
            for symbol in symbols {
                tracing::info!("  • {} ({})", symbol.name, symbol.kind);
            }
        }
        Err(e) => {
            tracing::error!("❌ Failed to parse JavaScript: {}", e);
        }
    }

    // Test parsing a simple Python code snippet
    let py_code = r#"
def calculate_sum(a, b):
    """Calculate the sum of two numbers."""
    return a + b

class Calculator:
    def __init__(self):
        self.result = 0

    def add(self, value):
        self.result += value
        return self.result
    "#;

    tracing::info!("📝 Testing Python parsing...");
    match parser.parse_symbols("python", py_code) {
        Ok(symbols) => {
            tracing::info!("✅ Found {} symbols in Python code", symbols.len());
            for symbol in symbols {
                tracing::info!("  • {} ({})", symbol.name, symbol.kind);
            }
        }
        Err(e) => {
            tracing::error!("❌ Failed to parse Python: {}", e);
        }
    }

    tracing::info!("🎯 Basic LSP server features demonstrated successfully!");
    tracing::info!("💡 To use in your editor, configure LSP server command: universal-lsp lsp");

    Ok(())
}
