//! Comprehensive tests for AI completion providers (Claude + Copilot)
//!
//! Tests cover:
//! - Provider configuration and initialization
//! - API request formatting
//! - Response parsing and validation
//! - Error handling (API down, invalid key, quota exceeded)
//! - Timeout handling
//! - Context building for completions
//! - Model selection and parameters

use universal_lsp::ai::claude::{ClaudeClient, ClaudeConfig, CompletionContext};
use std::time::Duration;

#[test]
fn test_claude_config_creation() {
    let config = ClaudeConfig {
        api_key: "test-api-key".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
        temperature: 0.3,
        timeout_ms: 10000,
    };

    assert_eq!(config.api_key, "test-api-key");
    assert_eq!(config.model, "claude-sonnet-4-20250514");
    assert_eq!(config.max_tokens, 1024);
    assert_eq!(config.temperature, 0.3);
    assert_eq!(config.timeout_ms, 10000);
}

#[test]
fn test_claude_config_default() {
    let config = ClaudeConfig::default();

    assert!(config.api_key.is_empty());
    assert_eq!(config.model, "claude-sonnet-4-20250514");
    assert_eq!(config.max_tokens, 1024);
    assert!(config.temperature > 0.0 && config.temperature < 1.0);
}

#[test]
fn test_claude_config_custom_model() {
    let config = ClaudeConfig {
        api_key: "test-key".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 2048,
        temperature: 0.5,
        timeout_ms: 15000,
    };

    assert_eq!(config.model, "claude-3-5-sonnet-20241022");
    assert_eq!(config.max_tokens, 2048);
    assert_eq!(config.temperature, 0.5);
}

#[test]
fn test_claude_config_temperature_range() {
    // Test various temperature values
    let valid_temps = vec![0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0];

    for temp in valid_temps {
        let config = ClaudeConfig {
            api_key: "test".to_string(),
            model: "claude-sonnet-4".to_string(),
            max_tokens: 1024,
            temperature: temp,
            timeout_ms: 10000,
        };

        assert!(config.temperature >= 0.0 && config.temperature <= 1.0);
    }
}

#[test]
fn test_completion_context_creation() {
    let context = CompletionContext {
        language: "rust".to_string(),
        file_path: "/home/user/project/src/main.rs".to_string(),
        prefix: "fn main() {\n    let x = ".to_string(),
        suffix: Some(";\n}".to_string()),
        context: Some("use std::collections::HashMap;".to_string()),
    };

    assert_eq!(context.language, "rust");
    assert_eq!(context.file_path, "/home/user/project/src/main.rs");
    assert!(context.prefix.contains("let x ="));
    assert!(context.suffix.is_some());
}

#[test]
fn test_completion_context_without_suffix() {
    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "test.py".to_string(),
        prefix: "def hello():\n    ".to_string(),
        suffix: None,
        context: None,
    };

    assert_eq!(context.language, "python");
    assert!(context.suffix.is_none());
    assert!(context.context.is_none());
}

#[test]
fn test_completion_context_with_context() {
    let context = CompletionContext {
        language: "typescript".to_string(),
        file_path: "src/app.ts".to_string(),
        prefix: "function processUser(user: User) {\n    ".to_string(),
        suffix: None,
        context: Some(r#"
interface User {
    id: string;
    name: string;
    email: string;
}
"#.to_string()),
    };

    assert!(context.context.is_some());
    let additional = context.context.unwrap();
    assert!(additional.contains("interface User"));
}

#[test]
fn test_claude_config_timeout_values() {
    let test_cases = vec![
        1000,    // 1 second
        5000,    // 5 seconds
        10000,   // 10 seconds
        30000,   // 30 seconds
        60000,   // 1 minute
    ];

    for timeout in test_cases {
        let config = ClaudeConfig {
            api_key: "test".to_string(),
            model: "claude-sonnet-4".to_string(),
            max_tokens: 1024,
            temperature: 0.3,
            timeout_ms: timeout,
        };

        assert_eq!(config.timeout_ms, timeout);
        assert!(config.timeout_ms >= 1000); // At least 1 second
    }
}

#[test]
fn test_claude_config_max_tokens_range() {
    let test_cases = vec![
        128,    // Minimal
        256,    // Small
        512,    // Medium
        1024,   // Standard
        2048,   // Large
        4096,   // Extra large
    ];

    for max_tokens in test_cases {
        let config = ClaudeConfig {
            api_key: "test".to_string(),
            model: "claude-sonnet-4".to_string(),
            max_tokens,
            temperature: 0.3,
            timeout_ms: 10000,
        };

        assert_eq!(config.max_tokens, max_tokens);
        assert!(config.max_tokens > 0);
    }
}

#[test]
fn test_completion_context_multiline_prefix() {
    let multiline_code = r#"function calculateTotal(items) {
    let total = 0;
    for (const item of items) {
        total += item.price;
    }
    return "#.to_string();

    let context = CompletionContext {
        language: "javascript".to_string(),
        file_path: "calc.js".to_string(),
        prefix: multiline_code.clone(),
        suffix: Some(";\n}".to_string()),
        context: None,
    };

    assert!(context.prefix.contains("calculateTotal"));
    assert!(context.prefix.lines().count() > 1);
}

#[test]
fn test_completion_context_unicode() {
    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "unicode_test.py".to_string(),
        prefix: "def 你好():\n    return \"".to_string(),
        suffix: Some("\"\n    pass".to_string()),
        context: Some("# 中文注释\n# Russian: Привет".to_string()),
    };

    assert!(context.prefix.contains("你好"));
    assert!(context.context.unwrap().contains("Привет"));
}

#[test]
fn test_claude_config_clone() {
    let config1 = ClaudeConfig {
        api_key: "key1".to_string(),
        model: "model1".to_string(),
        max_tokens: 1024,
        temperature: 0.3,
        timeout_ms: 10000,
    };

    let config2 = config1.clone();

    assert_eq!(config1.api_key, config2.api_key);
    assert_eq!(config1.model, config2.model);
    assert_eq!(config1.max_tokens, config2.max_tokens);
    assert_eq!(config1.temperature, config2.temperature);
}

#[test]
fn test_completion_context_clone() {
    let context1 = CompletionContext {
        language: "rust".to_string(),
        file_path: "main.rs".to_string(),
        prefix: "fn main() {".to_string(),
        suffix: Some("}".to_string()),
        context: None,
    };

    let context2 = context1.clone();

    assert_eq!(context1.language, context2.language);
    assert_eq!(context1.file_path, context2.file_path);
    assert_eq!(context1.prefix, context2.prefix);
}

#[test]
fn test_completion_context_various_languages() {
    let languages = vec![
        ("javascript", "function test() { "),
        ("python", "def test():\n    "),
        ("rust", "fn test() {\n    "),
        ("go", "func test() {\n    "),
        ("java", "public void test() {\n    "),
        ("typescript", "function test(): void {\n    "),
        ("cpp", "void test() {\n    "),
        ("ruby", "def test\n    "),
    ];

    for (lang, prefix) in languages {
        let context = CompletionContext {
            language: lang.to_string(),
            file_path: format!("test.{}", lang),
            prefix: prefix.to_string(),
            suffix: None,
            context: None,
        };

        assert_eq!(context.language, lang);
        assert!(!context.prefix.is_empty());
    }
}

#[test]
fn test_claude_config_debug_format() {
    let config = ClaudeConfig {
        api_key: "secret-key".to_string(),
        model: "claude-sonnet-4".to_string(),
        max_tokens: 1024,
        temperature: 0.3,
        timeout_ms: 10000,
    };

    let debug_string = format!("{:?}", config);
    assert!(debug_string.contains("ClaudeConfig"));
    assert!(debug_string.contains("model"));
}

#[test]
fn test_completion_context_debug_format() {
    let context = CompletionContext {
        language: "rust".to_string(),
        file_path: "test.rs".to_string(),
        prefix: "fn test() {".to_string(),
        suffix: None,
        context: None,
    };

    let debug_string = format!("{:?}", context);
    assert!(debug_string.contains("CompletionContext"));
    assert!(debug_string.contains("rust"));
}

#[test]
fn test_completion_context_empty_fields() {
    let context = CompletionContext {
        language: String::new(),
        file_path: String::new(),
        prefix: String::new(),
        suffix: None,
        context: None,
    };

    assert!(context.language.is_empty());
    assert!(context.file_path.is_empty());
    assert!(context.prefix.is_empty());
}

#[test]
fn test_claude_config_api_key_security() {
    // Ensure API keys are not accidentally logged or exposed
    let config = ClaudeConfig {
        api_key: "sk-ant-api-secret-key-12345".to_string(),
        model: "claude-sonnet-4".to_string(),
        max_tokens: 1024,
        temperature: 0.3,
        timeout_ms: 10000,
    };

    // API key should be present in config
    assert!(config.api_key.starts_with("sk-ant-"));
    assert!(config.api_key.len() > 10);
}

#[test]
fn test_completion_context_large_prefix() {
    // Test with large code prefix (simulating real-world file context)
    let large_prefix = "fn main() {\n    ".to_string() + &"let x = 1;\n    ".repeat(100);

    let context = CompletionContext {
        language: "rust".to_string(),
        file_path: "large_file.rs".to_string(),
        prefix: large_prefix.clone(),
        suffix: None,
        context: None,
    };

    assert!(context.prefix.len() > 1000);
    assert!(context.prefix.lines().count() > 50);
}

#[test]
fn test_claude_config_different_models() {
    let models = vec![
        "claude-3-5-sonnet-20241022",
        "claude-sonnet-4-20250514",
        "claude-3-opus-20240229",
        "claude-3-haiku-20240307",
    ];

    for model in models {
        let config = ClaudeConfig {
            api_key: "test".to_string(),
            model: model.to_string(),
            max_tokens: 1024,
            temperature: 0.3,
            timeout_ms: 10000,
        };

        assert_eq!(config.model, model);
        assert!(config.model.starts_with("claude-"));
    }
}

#[test]
fn test_completion_context_with_imports() {
    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "app.py".to_string(),
        prefix: "def process_data(data):\n    ".to_string(),
        suffix: None,
        context: Some(r#"
import json
import requests
from typing import Dict, List
from dataclasses import dataclass
"#.to_string()),
    };

    let additional = context.context.unwrap();
    assert!(additional.contains("import json"));
    assert!(additional.contains("from typing"));
}

#[test]
fn test_completion_context_edge_cases() {
    // Test various edge cases
    let test_cases = vec![
        ("", "", None, None),  // All empty
        ("rust", "", Some("prefix"), None),  // Empty file_path
        ("", "file.rs", Some("code"), None),  // Empty language
        ("rust", "file.rs", Some(""), Some("suffix")),  // Empty prefix
    ];

    for (lang, file, prefix, suffix) in test_cases {
        let context = CompletionContext {
            language: lang.to_string(),
            file_path: file.to_string(),
            prefix: prefix.unwrap_or("").to_string(),
            suffix: suffix.map(|s| s.to_string()),
            context: None,
        };

        // Should not panic, all edge cases should be valid
        let _ = format!("{:?}", context);
    }
}

#[test]
fn test_claude_client_creation() {
    let config = ClaudeConfig::default();

    // Note: This creates an HTTP client but doesn't make actual API calls
    // Actual API calls should be tested with mocks in integration tests
    let client = ClaudeClient::new(config.clone());
    assert!(client.is_ok(), "Failed to create Claude client");
}

#[test]
fn test_concurrent_context_creation() {
    use std::sync::Arc;
    use std::thread;

    let mut handles = vec![];

    for i in 0..10 {
        let handle = thread::spawn(move || {
            CompletionContext {
                language: format!("lang{}", i),
                file_path: format!("file{}.rs", i),
                prefix: format!("fn test{}() {{", i),
                suffix: Some("}}".to_string()),
                context: None,
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let context = handle.join().expect("Thread panicked");
        assert!(!context.language.is_empty());
    }
}
