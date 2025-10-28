//! LSP Proxy Module
//!
//! Forwards LSP requests to specialized language servers (rust-analyzer, pyright, etc.)
//! while maintaining the ability to enhance responses with MCP processing.

use anyhow::{Result, Context};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// Configuration for a single LSP proxy
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub language: String,
    pub command: String,
    pub args: Vec<String>,
}

impl ProxyConfig {
    /// Parse from "language=command args" format
    pub fn from_string(s: &str) -> Option<Self> {
        let (lang, cmd_str) = s.split_once('=')?;
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();

        if parts.is_empty() {
            return None;
        }

        let command = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        Some(ProxyConfig {
            language: lang.to_string(),
            command,
            args,
        })
    }
}

/// Manages LSP proxy processes for different languages
#[derive(Debug)]
pub struct ProxyManager {
    proxies: Arc<Mutex<HashMap<String, LspProxy>>>,
    configs: HashMap<String, ProxyConfig>,
}

impl ProxyManager {
    /// Create a new proxy manager from configuration
    pub fn new(configs: HashMap<String, ProxyConfig>) -> Self {
        Self {
            proxies: Arc::new(Mutex::new(HashMap::new())),
            configs,
        }
    }

    /// Get or start a proxy for the given language
    pub async fn get_proxy(&self, language: &str) -> Result<Option<Arc<Mutex<LspProxy>>>> {
        // Check if we have a config for this language
        let config = match self.configs.get(language) {
            Some(c) => c.clone(),
            None => return Ok(None),
        };

        let mut proxies = self.proxies.lock().await;

        // Check if proxy already exists and is running
        if let Some(proxy) = proxies.get(language) {
            if proxy.is_running() {
                return Ok(Some(Arc::new(Mutex::new(proxy.clone()))));
            }
        }

        // Start new proxy
        let proxy = LspProxy::start(config).await?;
        proxies.insert(language.to_string(), proxy.clone());

        Ok(Some(Arc::new(Mutex::new(proxy))))
    }

    /// Forward a request to the appropriate proxy
    pub async fn forward_request(&self, language: &str, request: Value) -> Result<Option<Value>> {
        let proxy = match self.get_proxy(language).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let mut proxy = proxy.lock().await;
        let response = proxy.send_request(request).await?;
        Ok(Some(response))
    }

    /// Check if a proxy exists for this language
    pub fn has_proxy_for(&self, language: &str) -> bool {
        self.configs.contains_key(language)
    }
}

/// A single LSP proxy process
#[derive(Debug, Clone)]
pub struct LspProxy {
    config: ProxyConfig,
    // Note: We can't actually clone Child, so this is a simplified version
    // In a real implementation, we'd use Arc<Mutex<>> for the process
    running: bool,
}

impl LspProxy {
    /// Start a new LSP proxy process
    pub async fn start(config: ProxyConfig) -> Result<Self> {
        // TODO: Actually start the process
        // For now, just create the structure
        Ok(Self {
            config,
            running: false,
        })
    }

    /// Send a JSON-RPC request to the proxy server
    pub async fn send_request(&mut self, _request: Value) -> Result<Value> {
        // TODO: Implement actual stdio communication
        // For now, return placeholder
        Ok(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": null
        }))
    }

    /// Check if the proxy process is still running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the proxy process
    pub async fn stop(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }
}

/// Full implementation of LSP proxy with actual process management
/// (This would be the production version)
pub struct LspProxyProcess {
    _process: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    _config: ProxyConfig,
}

impl LspProxyProcess {
    /// Start a new LSP proxy process with full stdio communication
    pub async fn start(config: ProxyConfig) -> Result<Self> {
        let mut child = Command::new(&config.command)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(format!("Failed to start LSP proxy: {}", config.command))?;

        let stdin = child.stdin.take()
            .context("Failed to get stdin")?;
        let stdout = child.stdout.take()
            .context("Failed to get stdout")?;

        Ok(Self {
            _process: child,
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            _config: config,
        })
    }

    /// Send a JSON-RPC request and wait for response
    pub async fn send_request(&self, request: Value) -> Result<Value> {
        let request_str = serde_json::to_string(&request)?;
        let content_length = request_str.len();

        // Write LSP protocol headers + JSON body
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            content_length,
            request_str
        );

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await
            .context("Failed to write to LSP proxy")?;
        stdin.flush().await
            .context("Failed to flush stdin")?;
        drop(stdin); // Release lock

        // Read response
        let mut stdout = self.stdout.lock().await;
        let response = self.read_lsp_message(&mut *stdout).await?;

        Ok(response)
    }

    /// Read an LSP message from stdout
    async fn read_lsp_message(&self, reader: &mut BufReader<ChildStdout>) -> Result<Value> {
        // Read headers
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            if line == "\r\n" {
                break; // End of headers
            }

            if line.starts_with("Content-Length:") {
                let len_str = line.trim_start_matches("Content-Length:")
                    .trim();
                content_length = len_str.parse()
                    .context("Invalid Content-Length")?;
            }
        }

        // Read body
        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(reader, &mut body).await?;

        let response: Value = serde_json::from_slice(&body)
            .context("Failed to parse LSP response")?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_parsing() {
        let config = ProxyConfig::from_string("python=pyright-langserver --stdio").unwrap();
        assert_eq!(config.language, "python");
        assert_eq!(config.command, "pyright-langserver");
        assert_eq!(config.args, vec!["--stdio"]);
    }

    #[test]
    fn test_proxy_config_no_args() {
        let config = ProxyConfig::from_string("rust=rust-analyzer").unwrap();
        assert_eq!(config.language, "rust");
        assert_eq!(config.command, "rust-analyzer");
        assert!(config.args.is_empty());
    }

    #[tokio::test]
    async fn test_proxy_manager_creation() {
        let mut configs = HashMap::new();
        configs.insert(
            "python".to_string(),
            ProxyConfig {
                language: "python".to_string(),
                command: "pyright".to_string(),
                args: vec!["--stdio".to_string()],
            },
        );

        let manager = ProxyManager::new(configs);
        assert!(manager.has_proxy_for("python"));
        assert!(!manager.has_proxy_for("rust"));
    }
}
