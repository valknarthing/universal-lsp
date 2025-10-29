//! MCP Pipeline Module
//!
//! Orchestrates pre-processing and post-processing of LSP requests/responses
//! through MCP servers for AI-powered enhancements.

use anyhow::Result;
use std::sync::Arc;

use crate::config::Config;
use crate::mcp::{McpClient, McpConfig, McpRequest, McpResponse, Position, TransportType};

/// Pipeline for MCP-enhanced LSP processing
pub struct McpPipeline {
    clients: Vec<Arc<McpClient>>,
}

impl McpPipeline {
    /// Create a new MCP pipeline from configuration
    pub fn new(config: &Config) -> Self {
        let clients = config
            .mcp
            .servers
            .values()
            .map(|server_config| {
                // Determine transport type based on target
                let transport = if server_config.target.starts_with("http://")
                    || server_config.target.starts_with("https://")
                {
                    TransportType::Http
                } else {
                    TransportType::Stdio
                };

                Arc::new(McpClient::new(McpConfig {
                    server_url: server_config.target.clone(),
                    transport,
                    timeout_ms: config.mcp.timeout_ms,
                }))
            })
            .collect();

        Self { clients }
    }

    /// Pre-process a request through MCP servers
    pub async fn pre_process(&self, request: McpRequest) -> Result<Vec<McpResponse>> {
        if self.clients.is_empty() {
            return Ok(Vec::new());
        }

        // Parallel requests to all MCP servers
        let mut tasks = Vec::new();
        for client in &self.clients {
            let client = Arc::clone(client);
            let req = request.clone();
            tasks.push(tokio::spawn(async move {
                client.query(&req).await
            }));
        }

        // Collect results, ignoring failures
        let mut results = Vec::new();
        for task in tasks {
            if let Ok(Ok(response)) = task.await {
                results.push(response);
            }
        }

        Ok(results)
    }

    /// Post-process responses through MCP servers
    pub async fn post_process(
        &self,
        request: McpRequest,
        _original_response: &str,
    ) -> Result<Vec<McpResponse>> {
        // Same as pre_process - all MCP servers provide context for both phases
        self.pre_process(request).await
    }

    /// Check if pipeline has any pre-processing servers
    pub fn has_pre_processing(&self) -> bool {
        !self.clients.is_empty()
    }

    /// Check if pipeline has any post-processing servers
    pub fn has_post_processing(&self) -> bool {
        !self.clients.is_empty()
    }
}

/// Merge multiple MCP responses into a single result
pub fn merge_mcp_responses(responses: Vec<McpResponse>) -> McpResponse {
    if responses.is_empty() {
        return McpResponse {
            suggestions: Vec::new(),
            documentation: None,
            confidence: None,
        };
    }

    // Collect all suggestions
    let mut all_suggestions = Vec::new();
    let mut all_docs = Vec::new();
    let mut confidences = Vec::new();

    for response in responses {
        all_suggestions.extend(response.suggestions);
        if let Some(doc) = response.documentation {
            all_docs.push(doc);
        }
        if let Some(conf) = response.confidence {
            confidences.push(conf);
        }
    }

    // Deduplicate suggestions
    all_suggestions.sort();
    all_suggestions.dedup();

    // Average confidence scores
    let avg_confidence = if !confidences.is_empty() {
        Some(confidences.iter().sum::<f32>() / confidences.len() as f32)
    } else {
        None
    };

    McpResponse {
        suggestions: all_suggestions,
        documentation: if all_docs.is_empty() {
            None
        } else {
            Some(all_docs.join("\n\n"))
        },
        confidence: avg_confidence,
    }
}

/// Convert LSP position to MCP position
pub fn lsp_position_to_mcp(line: u32, character: u32) -> Position {
    Position { line, character }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_empty_responses() {
        let merged = merge_mcp_responses(Vec::new());
        assert!(merged.suggestions.is_empty());
        assert!(merged.documentation.is_none());
        assert!(merged.confidence.is_none());
    }

    #[test]
    fn test_merge_multiple_responses() {
        let responses = vec![
            McpResponse {
                suggestions: vec!["fn".to_string(), "class".to_string()],
                documentation: Some("First doc".to_string()),
                confidence: Some(0.8),
            },
            McpResponse {
                suggestions: vec!["fn".to_string(), "struct".to_string()],
                documentation: Some("Second doc".to_string()),
                confidence: Some(0.9),
            },
        ];

        let merged = merge_mcp_responses(responses);
        assert_eq!(merged.suggestions.len(), 3); // Deduplicated
        assert!(merged.documentation.unwrap().contains("First doc"));
        assert_eq!(merged.confidence, Some(0.85)); // Average of 0.8 and 0.9
    }
}
