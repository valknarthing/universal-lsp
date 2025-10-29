//! Configuration module for Universal LSP Server
//!
//! Supports CLI-based configuration for:
//! - MCP pipeline (pre/post-processing)
//! - LSP proxy servers
//! - Server settings
//! - Multi-command CLI (LSP, ACP, Zed init)

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "ulsp")]
#[command(about = "Universal Language Server - LSP and ACP agent with MCP integration", long_about = None)]
#[command(version)]
pub struct CliArgs {
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info", global = true)]
    pub log_level: String,

    /// Configuration file path (optional, overrides CLI)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Start the LSP server (default mode)
    Lsp {
        /// MCP servers (comma-separated, format: name=command or name=url)
        /// Stdio examples:
        ///   --mcp-server=smart-tree=smart-tree,in-memoria=npx -y @pi22by7/in-memoria
        /// HTTP example:
        ///   --mcp-server=remote=http://localhost:3000
        #[arg(long, value_delimiter = ',')]
        mcp_server: Vec<String>,

        /// MCP request timeout in milliseconds
        #[arg(long, default_value = "5000")]
        mcp_timeout: u64,

        /// Enable MCP caching
        #[arg(long, default_value = "true")]
        mcp_cache: bool,

        /// LSP proxy servers (comma-separated, format: lang=command)
        /// Example: --lsp-proxy=python=pyright-langserver,rust=rust-analyzer
        #[arg(long, value_delimiter = ',')]
        lsp_proxy: Vec<String>,

        /// Maximum concurrent requests
        #[arg(long, default_value = "100")]
        max_concurrent: usize,

        /// Enable request logging
        #[arg(long)]
        log_requests: bool,
    },

    /// Start the ACP agent process
    Acp {
        /// MCP servers (comma-separated, format: name=command or name=url)
        #[arg(long, value_delimiter = ',')]
        mcp_server: Vec<String>,

        /// MCP request timeout in milliseconds
        #[arg(long, default_value = "5000")]
        mcp_timeout: u64,

        /// Enable MCP caching
        #[arg(long, default_value = "true")]
        mcp_cache: bool,

        /// Maximum concurrent requests
        #[arg(long, default_value = "100")]
        max_concurrent: usize,

        /// Enable request logging
        #[arg(long)]
        log_requests: bool,
    },

    /// Zed editor utilities
    Zed {
        #[command(subcommand)]
        zed_command: ZedCommands,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ZedCommands {
    /// Initialize a perfectly configured Zed workspace
    Init {
        /// Project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Project name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,

        /// Include MCP server configuration
        #[arg(long)]
        with_mcp: bool,

        /// Enable Claude AI integration
        #[arg(long)]
        with_claude: bool,

        /// Enable GitHub Copilot integration
        #[arg(long)]
        with_copilot: bool,

        /// Enable ACP agent configuration
        #[arg(long)]
        with_acp: bool,
    },
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

/// Runtime command mode
#[derive(Debug, Clone)]
pub enum CommandMode {
    /// LSP server mode (default)
    Lsp,
    /// ACP agent mode
    Acp,
    /// Zed workspace initialization
    ZedInit {
        path: PathBuf,
        name: Option<String>,
        with_mcp: bool,
        with_claude: bool,
        with_copilot: bool,
        with_acp: bool,
    },
}

impl Config {
    /// Create config from LSP command arguments
    fn from_lsp_args(
        log_level: String,
        mcp_server: Vec<String>,
        mcp_timeout: u64,
        mcp_cache: bool,
        lsp_proxy: Vec<String>,
        max_concurrent: usize,
        log_requests: bool,
    ) -> Self {
        let mut proxy_servers = std::collections::HashMap::new();
        for proxy_entry in lsp_proxy {
            if let Some((lang, cmd)) = proxy_entry.split_once('=') {
                proxy_servers.insert(lang.to_string(), cmd.to_string());
            }
        }

        let mut mcp_servers = std::collections::HashMap::new();
        for mcp_entry in mcp_server {
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
                log_level,
                max_concurrent,
                log_requests,
            },
            mcp: McpConfig {
                servers: mcp_servers,
                timeout_ms: mcp_timeout,
                enable_cache: mcp_cache,
            },
            proxy: ProxyConfig {
                servers: proxy_servers,
            },
        }
    }

    /// Create config from ACP command arguments
    fn from_acp_args(
        log_level: String,
        mcp_server: Vec<String>,
        mcp_timeout: u64,
        mcp_cache: bool,
        max_concurrent: usize,
        log_requests: bool,
    ) -> Self {
        let mut mcp_servers = std::collections::HashMap::new();
        for mcp_entry in mcp_server {
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
                log_level,
                max_concurrent,
                log_requests,
            },
            mcp: McpConfig {
                servers: mcp_servers,
                timeout_ms: mcp_timeout,
                enable_cache: mcp_cache,
            },
            proxy: ProxyConfig {
                servers: std::collections::HashMap::new(), // ACP doesn't use LSP proxies
            },
        }
    }

    /// Parse CLI arguments and create config
    /// Returns tuple: (Config, CommandMode)
    pub fn from_args() -> Result<(Self, CommandMode)> {
        let args = CliArgs::parse();

        match &args.command {
            // Default command: LSP server
            None | Some(Commands::Lsp { .. }) => {
                let (
                    mcp_server,
                    mcp_timeout,
                    mcp_cache,
                    lsp_proxy,
                    max_concurrent,
                    log_requests,
                ) = if let Some(Commands::Lsp {
                    mcp_server,
                    mcp_timeout,
                    mcp_cache,
                    lsp_proxy,
                    max_concurrent,
                    log_requests,
                }) = args.command
                {
                    (
                        mcp_server,
                        mcp_timeout,
                        mcp_cache,
                        lsp_proxy,
                        max_concurrent,
                        log_requests,
                    )
                } else {
                    (vec![], 5000, true, vec![], 100, false)
                };

                let config = Self::from_lsp_args(
                    args.log_level,
                    mcp_server,
                    mcp_timeout,
                    mcp_cache,
                    lsp_proxy,
                    max_concurrent,
                    log_requests,
                );
                Ok((config, CommandMode::Lsp))
            }

            // ACP agent command
            Some(Commands::Acp {
                mcp_server,
                mcp_timeout,
                mcp_cache,
                max_concurrent,
                log_requests,
            }) => {
                let config = Self::from_acp_args(
                    args.log_level,
                    mcp_server.clone(),
                    *mcp_timeout,
                    *mcp_cache,
                    *max_concurrent,
                    *log_requests,
                );
                Ok((config, CommandMode::Acp))
            }

            // Zed init command
            Some(Commands::Zed { zed_command }) => match zed_command {
                ZedCommands::Init {
                    path,
                    name,
                    with_mcp,
                    with_claude,
                    with_copilot,
                    with_acp,
                } => {
                    let config = Config::default_zed_init();
                    Ok((
                        config,
                        CommandMode::ZedInit {
                            path: path.clone(),
                            name: name.clone(),
                            with_mcp: *with_mcp,
                            with_claude: *with_claude,
                            with_copilot: *with_copilot,
                            with_acp: *with_acp,
                        },
                    ))
                }
            },
        }
    }

    /// Create default config for Zed init command
    fn default_zed_init() -> Self {
        Config {
            server: ServerConfig {
                log_level: "info".to_string(),
                max_concurrent: 100,
                log_requests: false,
            },
            mcp: McpConfig {
                servers: std::collections::HashMap::new(),
                timeout_ms: 5000,
                enable_cache: true,
            },
            proxy: ProxyConfig {
                servers: std::collections::HashMap::new(),
            },
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
    fn test_lsp_config_creation() {
        let config = Config::from_lsp_args(
            "debug".to_string(),
            vec![
                "smart-tree=smart-tree".to_string(),
                "in-memoria=npx -y @pi22by7/in-memoria".to_string(),
            ],
            5000,
            true,
            vec!["python=pyright".to_string()],
            100,
            false,
        );

        assert_eq!(config.server.log_level, "debug");
        assert_eq!(config.mcp.servers.len(), 2);
        assert!(config.mcp.servers.contains_key("smart-tree"));
        assert!(config.mcp.servers.contains_key("in-memoria"));
        assert_eq!(config.proxy.servers.get("python"), Some(&"pyright".to_string()));
    }

    #[test]
    fn test_acp_config_creation() {
        let config = Config::from_acp_args(
            "info".to_string(),
            vec!["smart-tree=smart-tree".to_string()],
            5000,
            true,
            100,
            false,
        );

        assert_eq!(config.server.log_level, "info");
        assert_eq!(config.mcp.servers.len(), 1);
        assert!(config.mcp.servers.contains_key("smart-tree"));
        // ACP should not have LSP proxies
        assert!(config.proxy.servers.is_empty());
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
