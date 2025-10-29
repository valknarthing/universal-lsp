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

    /// MCP servers (comma-separated, format: name=command or name=url)
    /// Stdio examples:
    ///   --mcp-server=smart-tree=smart-tree,in-memoria=npx -y @pi22by7/in-memoria
    /// HTTP example:
    ///   --mcp-server=remote=http://localhost:3000
    #[arg(long, value_delimiter = ',')]
    pub mcp_server: Vec<String>,

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
    /// Map of name -> MCP server config (command or URL)
    pub servers: std::collections::HashMap<String, McpServerConfig>,
    pub timeout_ms: u64,
    pub enable_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server identifier (name)
    pub name: String,
    /// Command or URL for the MCP server
    pub target: String,
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

        let mut mcp_servers = std::collections::HashMap::new();
        for mcp_entry in args.mcp_server {
            if let Some((name, target)) = mcp_entry.split_once('=') {
                mcp_servers.insert(
                    name.to_string(),
                    McpServerConfig {
                        name: name.to_string(),
                        target: target.to_string(),
                    },
                );
            }
        }

        Config {
            server: ServerConfig {
                log_level: args.log_level,
                max_concurrent: args.max_concurrent,
                log_requests: args.log_requests,
            },
            mcp: McpConfig {
                servers: mcp_servers,
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
            if !args.mcp_server.is_empty() {
                let mut mcp_servers = std::collections::HashMap::new();
                for mcp_entry in args.mcp_server {
                    if let Some((name, target)) = mcp_entry.split_once('=') {
                        mcp_servers.insert(
                            name.to_string(),
                            McpServerConfig {
                                name: name.to_string(),
                                target: target.to_string(),
                            },
                        );
                    }
                }
                config.mcp.servers = mcp_servers;
            }

            Ok(config)
        } else {
            Ok(Config::from(args))
        }
    }

    pub fn has_mcp_pipeline(&self) -> bool {
        !self.mcp.servers.is_empty()
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
            mcp_server: vec![
                "smart-tree=smart-tree".to_string(),
                "in-memoria=npx -y @pi22by7/in-memoria".to_string(),
            ],
            mcp_timeout: 5000,
            mcp_cache: true,
            lsp_proxy: vec!["python=pyright".to_string()],
            max_concurrent: 100,
            log_requests: false,
            config: None,
        };

        let config = Config::from(args);
        assert_eq!(config.server.log_level, "debug");
        assert_eq!(config.mcp.servers.len(), 2);
        assert!(config.mcp.servers.contains_key("smart-tree"));
        assert!(config.mcp.servers.contains_key("in-memoria"));
        assert_eq!(config.proxy.servers.get("python"), Some(&"pyright".to_string()));
    }

    #[test]
    fn test_pipeline_detection() {
        let mut mcp_servers = std::collections::HashMap::new();
        mcp_servers.insert(
            "smart-tree".to_string(),
            McpServerConfig {
                name: "smart-tree".to_string(),
                target: "smart-tree".to_string(),
            },
        );

        let config = Config {
            server: ServerConfig {
                log_level: "info".to_string(),
                max_concurrent: 100,
                log_requests: false,
            },
            mcp: McpConfig {
                servers: mcp_servers,
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
