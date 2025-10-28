//! GitHub Copilot API Client for AI-powered code completions
//!
//! This module provides integration with GitHub Copilot API to generate
//! intelligent, context-aware code completions.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// GitHub Copilot API configuration
#[derive(Debug, Clone)]
pub struct CopilotConfig {
    pub api_key: String,
    pub endpoint: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub timeout_ms: u64,
}

impl Default for CopilotConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: "https://api.githubcopilot.com/completions".to_string(),
            max_tokens: 1024,
            temperature: 0.3,
            timeout_ms: 10000,
        }
    }
}

/// GitHub Copilot API client
#[derive(Debug)]
pub struct CopilotClient {
    config: CopilotConfig,
    http_client: reqwest::Client,
}

/// Copilot completion request structure
#[derive(Debug, Serialize)]
struct CopilotRequest {
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    suffix: Option<String>,
    max_tokens: usize,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

/// Copilot completion response structure
#[derive(Debug, Deserialize)]
struct CopilotResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    text: String,
    #[serde(default)]
    finish_reason: Option<String>,
}

// Reuse shared types from claude module
use super::claude::{CompletionContext, CompletionSuggestion};

impl CopilotClient {
    /// Create a new GitHub Copilot API client
    pub fn new(config: CopilotConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!(
                "GitHub Copilot API key is required. Set GITHUB_TOKEN environment variable or configure via CLI."
            ));
        }

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", config.api_key)
                        .parse()
                        .context("Invalid API key format")?,
                );
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    "application/json".parse()
                        .context("Invalid content type")?,
                );
                headers.insert(
                    "editor-version",
                    "universal-lsp/1.0"
                        .parse()
                        .context("Invalid header value")?,
                );
                headers
            })
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { config, http_client })
    }

    /// Get code completions from GitHub Copilot
    pub async fn get_completions(&self, ctx: &CompletionContext) -> Result<Vec<CompletionSuggestion>> {
        let request = self.build_completion_request(ctx);
        let response = self.query_copilot(&request).await?;

        // Parse completion suggestions from Copilot's response
        let suggestions = self.parse_completion_response(&response)?;

        Ok(suggestions)
    }

    /// Build a completion request for Copilot
    fn build_completion_request(&self, ctx: &CompletionContext) -> CopilotRequest {
        let mut prompt = ctx.prefix.clone();

        // Add context if available
        if let Some(context) = &ctx.context {
            prompt = format!("{}\n\n{}", context, prompt);
        }

        CopilotRequest {
            prompt,
            suffix: ctx.suffix.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            language: Some(ctx.language.clone()),
            path: Some(ctx.file_path.clone()),
        }
    }

    /// Query GitHub Copilot API
    async fn query_copilot(&self, request: &CopilotRequest) -> Result<CopilotResponse> {
        let response = self
            .http_client
            .post(&self.config.endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to GitHub Copilot API")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "GitHub Copilot API returned error status {}: {}",
                status,
                error_body
            ));
        }

        let copilot_response: CopilotResponse = response
            .json()
            .await
            .context("Failed to parse GitHub Copilot API response")?;

        Ok(copilot_response)
    }

    /// Parse Copilot's response into completion suggestions
    fn parse_completion_response(&self, response: &CopilotResponse) -> Result<Vec<CompletionSuggestion>> {
        let suggestions: Vec<CompletionSuggestion> = response
            .choices
            .iter()
            .enumerate()
            .filter(|(_, choice)| !choice.text.trim().is_empty())
            .take(5) // Limit to 5 suggestions
            .map(|(idx, choice)| CompletionSuggestion {
                text: choice.text.trim().to_string(),
                confidence: 1.0 - (idx as f32 * 0.1), // Decreasing confidence
                detail: Some("GitHub Copilot".to_string()),
            })
            .collect();

        if suggestions.is_empty() {
            return Err(anyhow::anyhow!("No completions returned from GitHub Copilot"));
        }

        Ok(suggestions)
    }

    /// Check if GitHub Copilot API is available and configured
    pub fn is_available(&self) -> bool {
        !self.config.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::claude::{CompletionContext};

    #[test]
    fn test_completion_request_building() {
        let config = CopilotConfig::default();
        let client = CopilotClient {
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

        let request = client.build_completion_request(&ctx);

        assert!(request.prompt.contains("function add"));
        assert_eq!(request.language, Some("JavaScript".to_string()));
        assert_eq!(request.suffix, Some(";\n}".to_string()));
    }

    #[test]
    fn test_config_default() {
        let config = CopilotConfig::default();
        assert!(config.endpoint.contains("githubcopilot.com"));
        assert_eq!(config.max_tokens, 1024);
        assert!(config.temperature > 0.0);
    }
}
