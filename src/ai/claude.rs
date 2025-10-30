//! Claude API Client for AI-powered code completions
//!
//! This module provides integration with Anthropic's Claude API to generate
//! intelligent, context-aware code completions.

use anyhow::{Context, Result};
use futures::StreamExt;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
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

/// Streaming event from Claude API
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: StreamMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: usize, content_block: Value },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: Value },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: Value },
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamMessage {
    pub id: String,
    pub model: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

/// Callback type for streaming responses
pub type StreamCallback = Box<dyn Fn(StreamEvent) -> Result<()> + Send + Sync>;

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
            stream: None, // Non-streaming request
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

    /// Send a streaming message to Claude with tool support
    ///
    /// This method sends a request to Claude's streaming API and calls the provided
    /// callback for each streaming event received. This allows for progressive
    /// display of responses and real-time feedback.
    pub async fn send_message_with_tools_streaming<F>(
        &self,
        messages: &[Message],
        tools: Option<Vec<Value>>,
        system: Option<String>,
        mut callback: F,
    ) -> Result<ClaudeToolResponse>
    where
        F: FnMut(StreamEvent) -> Result<()>,
    {
        let request = ClaudeRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: messages.to_vec(),
            tools,
            system,
            stream: Some(true), // Enable streaming
        };

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Claude API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Claude API returned error status {}: {}",
                status,
                error_body
            ));
        }

        // Parse SSE stream
        let mut text_blocks: Vec<String> = Vec::new();
        let mut tool_uses: Vec<ToolUse> = Vec::new();
        let mut current_text = String::new();
        let mut stop_reason: Option<String> = None;

        // Read response body as stream
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read streaming response")?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete SSE events (separated by \n\n)
            while let Some(event_end) = buffer.find("\n\n") {
                let event_data = buffer[..event_end].to_string();
                buffer = buffer[event_end + 2..].to_string();

                // Parse SSE event
                if let Some(event) = self.parse_sse_event(&event_data) {
                    // Call user callback
                    callback(event.clone())?;

                    // Accumulate response
                    match event {
                        StreamEvent::ContentBlockDelta { delta, .. } => {
                            if let ContentDelta::TextDelta { text } = delta {
                                current_text.push_str(&text);
                            }
                        }
                        StreamEvent::ContentBlockStop { .. } => {
                            if !current_text.is_empty() {
                                text_blocks.push(current_text.clone());
                                current_text.clear();
                            }
                        }
                        StreamEvent::MessageDelta { delta } => {
                            if let Some(reason) = delta.get("stop_reason").and_then(|v| v.as_str()) {
                                stop_reason = Some(reason.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Final accumulated text
        if !current_text.is_empty() {
            text_blocks.push(current_text);
        }

        Ok(ClaudeToolResponse {
            text_blocks,
            tool_uses,
            stop_reason,
        })
    }

    /// Parse a single SSE event from the stream
    fn parse_sse_event(&self, event_data: &str) -> Option<StreamEvent> {
        // SSE format: "event: message_start\ndata: {...}\n"
        let mut event_type: Option<&str> = None;
        let mut data: Option<&str> = None;

        for line in event_data.lines() {
            if let Some(evt) = line.strip_prefix("event: ") {
                event_type = Some(evt.trim());
            } else if let Some(d) = line.strip_prefix("data: ") {
                data = Some(d.trim());
            }
        }

        // Parse the data JSON
        if let Some(data_str) = data {
            if let Ok(mut event_json) = serde_json::from_str::<Value>(data_str) {
                // Add type field from SSE event type if present
                if let Some(evt_type) = event_type {
                    if event_json.is_object() {
                        event_json.as_object_mut().unwrap().insert(
                            "type".to_string(),
                            Value::String(evt_type.to_string()),
                        );
                    }
                }

                // Try to deserialize into StreamEvent
                if let Ok(stream_event) = serde_json::from_value::<StreamEvent>(event_json) {
                    return Some(stream_event);
                }
            }
        }

        None
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
