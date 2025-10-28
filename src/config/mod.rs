//! Configuration module for Universal LSP Server
//! 
//! Supports CLI-based configuration for:
//! - MCP pipeline (pre/post-processing)
//! - LSP proxy servers
//! - Server settings

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "universal-lsp")]
#[command(about = "Universal LSP Server with MCP integration and LSP proxying", long_about = None)]
pub struct CliArgs {
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// MCP servers for preprocessing (comma-separated URLs)
    /// Example: --mcp-pre=http://localhost:3000,http://localhost:3001
    #[arg(long, value_delimiter = ',')]
    pub mcp_pre: Vec<String>,

    /// MCP servers for postprocessing (comma-separated URLs)
    /// Example: --mcp-post=http://localhost:4000
    #[arg(long, value_delimiter = ',')]
    pub mcp_post: Vec<String>,

    /// MCP request timeout in milliseconds
    #[arg(long, default_value = "5000")]
    pub mcp_timeout: u64,

    /// Enable MCP caching
    #[arg(long, default_value = "true")]
    pub mcp_cache: bool,

    /// LSP proxy servers (comma-separated, format: lang=command)
    /// Example: --lsp-proxy=python=pyright-langserver,rust=rust-analyzer
    #[arg(long, value_delimiter = ',')]
    pub lsp_proxy: Vec<String>,

    /// Maximum concurrent requests
    #[arg(long, default_value = "100")]
    pub max_concurrent: usize,

    /// Enable request logging
    #[arg(long)]
    pub log_requests: bool,

    /// Configuration file path (optional, overrides CLI)
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub mcp: McpConfig,
    pub proxy: ProxyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub log_level: String,
    pub max_concurrent: usize,
    pub log_requests: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// MCP servers for preprocessing requests
    pub pre_servers: Vec<String>,
    /// MCP servers for postprocessing responses
    pub post_servers: Vec<String>,
    pub timeout_ms: u64,
    pub enable_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Map of language -> LSP server command
    pub servers: std::collections::HashMap<String, String>,
}

impl From<CliArgs> for Config {
    fn from(args: CliArgs) -> Self {
        let mut proxy_servers = std::collections::HashMap::new();
        for proxy_entry in args.lsp_proxy {
            if let Some((lang, cmd)) = proxy_entry.split_once('=') {
                proxy_servers.insert(lang.to_string(), cmd.to_string());
            }
        }

        Config {
            server: ServerConfig {
                log_level: args.log_level,
                max_concurrent: args.max_concurrent,
                log_requests: args.log_requests,
            },
            mcp: McpConfig {
                pre_servers: args.mcp_pre,
                post_servers: args.mcp_post,
                timeout_ms: args.mcp_timeout,
                enable_cache: args.mcp_cache,
            },
            proxy: ProxyConfig {
                servers: proxy_servers,
            },
        }
    }
}

impl Config {
    pub fn from_args() -> Result<Self> {
        let args = CliArgs::parse();
        
        // If config file specified, load from file and merge with CLI args
        if let Some(config_path) = &args.config {
            let file_config = std::fs::read_to_string(config_path)?;
            let mut config: Config = serde_json::from_str(&file_config)?;
            
            // CLI args override file config
            if !args.mcp_pre.is_empty() {
                config.mcp.pre_servers = args.mcp_pre.clone();
            }
            if !args.mcp_post.is_empty() {
                config.mcp.post_servers = args.mcp_post.clone();
            }
            
            Ok(config)
        } else {
            Ok(Config::from(args))
        }
    }

    pub fn has_mcp_pipeline(&self) -> bool {
        !self.mcp.pre_servers.is_empty() || !self.mcp.post_servers.is_empty()
    }

    pub fn has_proxy_servers(&self) -> bool {
        !self.proxy.servers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let args = CliArgs {
            log_level: "debug".to_string(),
            mcp_pre: vec!["http://localhost:3000".to_string()],
            mcp_post: vec!["http://localhost:4000".to_string()],
            mcp_timeout: 5000,
            mcp_cache: true,
            lsp_proxy: vec!["python=pyright".to_string()],
            max_concurrent: 100,
            log_requests: false,
            config: None,
        };

        let config = Config::from(args);
        assert_eq!(config.server.log_level, "debug");
        assert_eq!(config.mcp.pre_servers.len(), 1);
        assert_eq!(config.mcp.post_servers.len(), 1);
        assert_eq!(config.proxy.servers.get("python"), Some(&"pyright".to_string()));
    }

    #[test]
    fn test_pipeline_detection() {
        let config = Config {
            server: ServerConfig {
                log_level: "info".to_string(),
                max_concurrent: 100,
                log_requests: false,
            },
            mcp: McpConfig {
                pre_servers: vec!["http://localhost:3000".to_string()],
                post_servers: vec![],
                timeout_ms: 5000,
                enable_cache: true,
            },
            proxy: ProxyConfig {
                servers: std::collections::HashMap::new(),
            },
        };

        assert!(config.has_mcp_pipeline());
        assert!(!config.has_proxy_servers());
    }
}
