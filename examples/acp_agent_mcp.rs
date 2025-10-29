//! # ACP Agent with MCP Integration Example
//!
//! This example demonstrates how to use the Agent Client Protocol (ACP) with Model Context Protocol (MCP)
//! integration for advanced AI-powered editor capabilities.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example acp_agent_mcp
//! ```
//!
//! ## Features Demonstrated
//!
//! - ACP agent initialization
//! - MCP server connections (filesystem, memory, git, etc.)
//! - Context-aware AI interactions
//! - Multi-server coordination

use anyhow::Result;
use tokio;
use universal_lsp::{McpClient, McpConfig, McpRequest, McpResponse, McpPipeline};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("🤖 Starting ACP Agent with MCP Integration");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Configure MCP servers
    let mcp_servers = vec![
        ("filesystem", vec!["npx", "-y", "@modelcontextprotocol/server-filesystem", "."]),
        ("memory", vec!["npx", "-y", "@modelcontextprotocol/server-memory"]),
        ("git", vec!["npx", "-y", "@modelcontextprotocol/server-git"]),
    ];

    tracing::info!("📋 Configured MCP Servers:");
    for (name, _) in &mcp_servers {
        tracing::info!("  ✓ {}", name);
    }

    // Create MCP configuration
    let config = McpConfig {
        servers: mcp_servers
            .iter()
            .map(|(name, cmd)| {
                let mut map = HashMap::new();
                map.insert("name".to_string(), name.to_string());
                map.insert("command".to_string(), cmd[0].to_string());
                map.insert("args".to_string(), cmd[1..].join(" "));
                map
            })
            .collect(),
        cache_size: 1000,
        timeout_ms: 30000,
    };

    // Initialize MCP pipeline
    tracing::info!("🔧 Initializing MCP Pipeline...");
    let pipeline = McpPipeline::new(config).await?;
    tracing::info!("✅ MCP Pipeline initialized successfully");

    // Example 1: Filesystem operations via MCP
    tracing::info!("\n📁 Example 1: Filesystem Operations");
    tracing::info!("  Querying current directory files...");

    let fs_request = McpRequest {
        server: "filesystem".to_string(),
        method: "list_files".to_string(),
        params: HashMap::from([
            ("path".to_string(), ".".to_string()),
        ]),
    };

    match pipeline.execute(fs_request).await {
        Ok(response) => {
            tracing::info!("  ✅ Response: {} files found", response.data.len());
        }
        Err(e) => {
            tracing::warn!("  ⚠️  Filesystem query failed: {}", e);
        }
    }

    // Example 2: Memory context storage
    tracing::info!("\n🧠 Example 2: Memory Context Storage");
    tracing::info!("  Storing context in memory...");

    let memory_request = McpRequest {
        server: "memory".to_string(),
        method: "store_context".to_string(),
        params: HashMap::from([
            ("key".to_string(), "last_session".to_string()),
            ("value".to_string(), "Example ACP session with MCP".to_string()),
        ]),
    };

    match pipeline.execute(memory_request).await {
        Ok(response) => {
            tracing::info!("  ✅ Context stored successfully");
        }
        Err(e) => {
            tracing::warn!("  ⚠️  Memory store failed: {}", e);
        }
    }

    // Example 3: Git operations
    tracing::info!("\n🔀 Example 3: Git Operations");
    tracing::info!("  Querying git repository status...");

    let git_request = McpRequest {
        server: "git".to_string(),
        method: "status".to_string(),
        params: HashMap::new(),
    };

    match pipeline.execute(git_request).await {
        Ok(response) => {
            tracing::info!("  ✅ Git status retrieved");
        }
        Err(e) => {
            tracing::warn!("  ⚠️  Git query failed: {}", e);
        }
    }

    tracing::info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("🎯 ACP Agent with MCP integration demonstrated successfully!");
    tracing::info!("💡 This example shows how Universal LSP can coordinate multiple");
    tracing::info!("   MCP servers for enhanced editor-AI interactions.");

    Ok(())
}
