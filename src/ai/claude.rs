//! Claude API Client for AI-powered code completions
//!
//! This module provides integration with Anthropic's Claude API to generate
//! intelligent, context-aware code completions.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Claude API configuration
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    pub api_key: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub timeout_ms: u64,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            temperature: 0.3,
            timeout_ms: 10000,
        }
    }
}

/// Claude API client
#[derive(Debug)]
pub struct ClaudeClient {
    config: ClaudeConfig,
    http_client: reqwest::Client,
}

/// Claude Messages API request structure
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: usize,
    temperature: f32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Claude Messages API response structure
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

/// Completion request context
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// Programming language
    pub language: String,
    /// File path or URI
    pub file_path: String,
    /// Content before cursor
    pub prefix: String,
    /// Content after cursor (optional)
    pub suffix: Option<String>,
    /// Additional context (e.g., imports, surrounding functions)
    pub context: Option<String>,
}

/// Completion suggestion from Claude
#[derive(Debug, Clone)]
pub struct CompletionSuggestion {
    /// The suggested completion text
    pub text: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Explanation or documentation
    pub detail: Option<String>,
}

/// Enhanced response from Claude with tool support
#[derive(Debug, Clone)]
pub struct ClaudeToolResponse {
    /// Text content blocks
    pub text_blocks: Vec<String>,
    /// Tool use requests
    pub tool_uses: Vec<ToolUse>,
    /// Stop reason
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

impl ClaudeClient {
    /// Create a new Claude API client
    pub fn new(config: ClaudeConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!(
                "Claude API key is required. Set ANTHROPIC_API_KEY environment variable or configure via CLI."
            ));
        }

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "x-api-key",
                    config.api_key.parse()
                        .context("Invalid API key format")?,
                );
                headers.insert(
                    "anthropic-version",
                    "2023-06-01".parse()
                        .context("Invalid header value")?,
                );
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    "application/json".parse()
                        .context("Invalid content type")?,
                );
                headers
            })
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { config, http_client })
    }

    /// Send a multi-turn conversation to Claude and get response
    pub async fn send_message(&self, messages: &[Message]) -> Result<String> {
        let response = self.send_message_with_tools(messages, None, None).await?;
        Ok(response.text_blocks.join("\n"))
    }

    /// Send a multi-turn conversation to Claude with tool support
    pub async fn send_message_with_tools(
        &self,
        messages: &[Message],
        tools: Option<Vec<Value>>,
        system: Option<String>,
    ) -> Result<ClaudeToolResponse> {
        let request = ClaudeRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: messages.to_vec(),
            tools,
            system,
        };

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Claude API returned error status {}: {}",
                status,
                error_body
            ));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude API response")?;

        // Extract text and tool use blocks
        let mut text_blocks = Vec::new();
        let mut tool_uses = Vec::new();

        for block in claude_response.content {
            match block {
                ContentBlock::Text { text } => {
                    text_blocks.push(text);
                }
                ContentBlock::ToolUse { id, name, input } => {
                    tool_uses.push(ToolUse { id, name, input });
                }
            }
        }

        Ok(ClaudeToolResponse {
            text_blocks,
            tool_uses,
            stop_reason: claude_response.stop_reason,
        })
    }

    /// Get code completions from Claude
    pub async fn get_completions(&self, ctx: &CompletionContext) -> Result<Vec<CompletionSuggestion>> {
        let prompt = self.build_completion_prompt(ctx);
        let response = self.query_claude(&prompt).await?;

        // Parse completion suggestions from Claude's response
        let suggestions = self.parse_completion_response(&response, ctx)?;

        Ok(suggestions)
    }

    /// Build a prompt for code completion
    fn build_completion_prompt(&self, ctx: &CompletionContext) -> String {
        let mut prompt = format!(
            "You are an expert {} programmer. ",
            ctx.language
        );

        if let Some(context) = &ctx.context {
            prompt.push_str(&format!("Context:\n{}\n\n", context));
        }

        prompt.push_str(&format!(
            "Complete the following {} code. Provide ONLY the completion text, no explanations.\n\n",
            ctx.language
        ));

        prompt.push_str("Code to complete:\n");
        prompt.push_str(&ctx.prefix);
        prompt.push_str("<CURSOR>");

        if let Some(suffix) = &ctx.suffix {
            prompt.push_str(suffix);
        }

        prompt.push_str("\n\nProvide the completion that should replace <CURSOR>. Return only the code, nothing else.");

        prompt
    }

    /// Query Claude's Messages API
    async fn query_claude(&self, prompt: &str) -> Result<String> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.send_message(&messages).await
    }

    /// Parse Claude's response into completion suggestions
    fn parse_completion_response(&self, response: &str, _ctx: &CompletionContext) -> Result<Vec<CompletionSuggestion>> {
        // Clean up the response
        let cleaned = response.trim();

        // Split into multiple suggestions if Claude provides alternatives
        let suggestions: Vec<CompletionSuggestion> = cleaned
            .lines()
            .filter(|line| !line.is_empty())
            .take(5) // Limit to 5 suggestions
            .enumerate()
            .map(|(idx, text)| CompletionSuggestion {
                text: text.trim().to_string(),
                confidence: 1.0 - (idx as f32 * 0.1), // Decreasing confidence
                detail: Some(format!("AI-generated completion (Claude {})", self.config.model)),
            })
            .collect();

        if suggestions.is_empty() {
            return Ok(vec![CompletionSuggestion {
                text: cleaned.to_string(),
                confidence: 0.8,
                detail: Some("AI-generated completion".to_string()),
            }]);
        }

        Ok(suggestions)
    }

    /// Check if Claude API is available and configured
    pub fn is_available(&self) -> bool {
        !self.config.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_prompt_building() {
        let config = ClaudeConfig::default();
        let client = ClaudeClient {
            config: config.clone(),
            http_client: reqwest::Client::new(),
        };

        let ctx = CompletionContext {
            language: "JavaScript".to_string(),
            file_path: "test.js".to_string(),
            prefix: "function add(a, b) {\n    return ".to_string(),
            suffix: Some(";\n}".to_string()),
            context: None,
        };

        let prompt = client.build_completion_prompt(&ctx);

        assert!(prompt.contains("JavaScript"));
        assert!(prompt.contains("function add"));
        assert!(prompt.contains("<CURSOR>"));
    }

    #[test]
    fn test_config_default() {
        let config = ClaudeConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.max_tokens, 1024);
        assert!(config.temperature > 0.0);
    }
}
