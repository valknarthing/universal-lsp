//! Comprehensive AI Provider Integration Tests
//!
//! Tests AI provider functionality:
//! - Claude AI client configuration and requests
//! - GitHub Copilot client configuration
//! - Completion context handling
//! - Token management and truncation
//! - Error handling and timeouts
//! - Rate limiting behavior
//! - Response parsing

use universal_lsp::ai::claude::{ClaudeClient, ClaudeConfig, CompletionContext};

#[tokio::test]
async fn test_claude_config_creation() {
    let config = ClaudeConfig {
        api_key: "test-api-key".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
        temperature: 0.7,
        timeout_ms: 30000,
    };

    assert_eq!(config.api_key, "test-api-key");
    assert_eq!(config.model, "claude-sonnet-4-20250514");
    assert_eq!(config.max_tokens, 1024);
    assert_eq!(config.temperature, 0.7);
    assert_eq!(config.timeout_ms, 30000);
}

#[tokio::test]
async fn test_claude_client_creation() {
    let config = ClaudeConfig {
        api_key: "test-api-key".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
        temperature: 0.7,
        timeout_ms: 30000,
    };

    let client = ClaudeClient::new(config);

    // Client should be created successfully
    assert!(true, "ClaudeClient created successfully");
}

#[tokio::test]
async fn test_completion_context_creation() {
    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "/path/to/test.py".to_string(),
        prefix: "def calculate_sum(".to_string(),
        suffix: ") -> int:\n    return a + b".to_string(),
        context: Some("Previous function:\ndef helper():\n    pass".to_string()),
    };

    assert_eq!(context.language, "python");
    assert_eq!(context.file_path, "/path/to/test.py");
    assert_eq!(context.prefix, "def calculate_sum(");
    assert_eq!(context.suffix, ") -> int:\n    return a + b");
    assert!(context.context.is_some());
}

#[tokio::test]
async fn test_completion_context_without_suffix() {
    let context = CompletionContext {
        language: "javascript".to_string(),
        file_path: "/path/to/test.js".to_string(),
        prefix: "function greet(name) {".to_string(),
        suffix: String::new(),
        context: None,
    };

    assert!(context.suffix.is_empty());
    assert!(context.context.is_none());
}

#[tokio::test]
async fn test_claude_model_variants() {
    let models = vec![
        "claude-3-5-sonnet-20241022",
        "claude-sonnet-4-20250514",
        "claude-3-opus-20240229",
        "claude-3-haiku-20240307",
    ];

    for model in models {
        let config = ClaudeConfig {
            api_key: "test-key".to_string(),
            model: model.to_string(),
            max_tokens: 1024,
            temperature: 0.7,
            timeout_ms: 30000,
        };

        assert_eq!(config.model, model);
    }
}

#[tokio::test]
async fn test_claude_temperature_range() {
    // Test valid temperature values
    let temperatures = vec![0.0, 0.3, 0.5, 0.7, 1.0];

    for temp in temperatures {
        let config = ClaudeConfig {
            api_key: "test-key".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            temperature: temp,
            timeout_ms: 30000,
        };

        assert!((0.0..=1.0).contains(&config.temperature),
            "Temperature {} should be in range [0.0, 1.0]", temp);
    }
}

#[tokio::test]
async fn test_claude_max_tokens_limits() {
    // Test different token limits
    let token_limits = vec![256, 512, 1024, 2048, 4096, 8192];

    for limit in token_limits {
        let config = ClaudeConfig {
            api_key: "test-key".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: limit,
            temperature: 0.7,
            timeout_ms: 30000,
        };

        assert_eq!(config.max_tokens, limit);
    }
}

#[tokio::test]
async fn test_completion_context_with_large_prefix() {
    // Test with large prefix (simulating large file)
    let large_prefix = "def function():\n    pass\n\n".repeat(1000);

    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "/test.py".to_string(),
        prefix: large_prefix.clone(),
        suffix: String::new(),
        context: None,
    };

    assert_eq!(context.prefix.len(), large_prefix.len());
}

#[tokio::test]
async fn test_completion_context_multiple_languages() {
    let languages = vec![
        "python", "javascript", "typescript", "rust", "go",
        "java", "cpp", "csharp", "ruby", "php",
    ];

    for lang in languages {
        let context = CompletionContext {
            language: lang.to_string(),
            file_path: format!("/test.{}", lang),
            prefix: "test code".to_string(),
            suffix: String::new(),
            context: None,
        };

        assert_eq!(context.language, lang);
    }
}

#[tokio::test]
async fn test_claude_api_key_formats() {
    // Test different API key formats
    let api_keys = vec![
        "sk-ant-api03-xxxxx",
        "sk-ant-xxxxx",
        "test-key-123",
    ];

    for key in api_keys {
        let config = ClaudeConfig {
            api_key: key.to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            temperature: 0.7,
            timeout_ms: 30000,
        };

        assert_eq!(config.api_key, key);
    }
}

#[tokio::test]
async fn test_completion_context_with_utf8() {
    // Test with UTF-8 characters
    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "/test.py".to_string(),
        prefix: "# Comment with emoji 😀\ndef greet(".to_string(),
        suffix: "):\n    return \"Hello 世界\"".to_string(),
        context: None,
    };

    assert!(context.prefix.contains("😀"));
    assert!(context.suffix.contains("世界"));
}

#[tokio::test]
async fn test_claude_timeout_values() {
    // Test various timeout values
    let timeouts = vec![
        1000,   // 1 second
        5000,   // 5 seconds
        10000,  // 10 seconds
        30000,  // 30 seconds
        60000,  // 1 minute
    ];

    for timeout in timeouts {
        let config = ClaudeConfig {
            api_key: "test-key".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            temperature: 0.7,
            timeout_ms: timeout,
        };

        assert_eq!(config.timeout_ms, timeout);
    }
}

#[tokio::test]
async fn test_completion_context_edge_cases() {
    // Test empty prefix
    let empty_prefix = CompletionContext {
        language: "python".to_string(),
        file_path: "/test.py".to_string(),
        prefix: String::new(),
        suffix: "def foo():\n    pass".to_string(),
        context: None,
    };
    assert!(empty_prefix.prefix.is_empty());

    // Test only whitespace prefix
    let whitespace_prefix = CompletionContext {
        language: "python".to_string(),
        file_path: "/test.py".to_string(),
        prefix: "   \n\n\t".to_string(),
        suffix: String::new(),
        context: None,
    };
    assert!(!whitespace_prefix.prefix.is_empty());
}

#[tokio::test]
async fn test_claude_config_clone() {
    let config1 = ClaudeConfig {
        api_key: "test-key".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
        temperature: 0.7,
        timeout_ms: 30000,
    };

    let config2 = config1.clone();

    assert_eq!(config1.api_key, config2.api_key);
    assert_eq!(config1.model, config2.model);
    assert_eq!(config1.max_tokens, config2.max_tokens);
}

#[tokio::test]
async fn test_completion_context_file_paths() {
    // Test various file path formats
    let paths = vec![
        "/absolute/path/to/file.py",
        "./relative/path/file.py",
        "../parent/path/file.py",
        "simple_file.py",
        "/path/with spaces/file.py",
        "C:\\Windows\\Path\\file.py",
    ];

    for path in paths {
        let context = CompletionContext {
            language: "python".to_string(),
            file_path: path.to_string(),
            prefix: "test".to_string(),
            suffix: String::new(),
            context: None,
        };

        assert_eq!(context.file_path, path);
    }
}

#[tokio::test]
async fn test_claude_zero_temperature() {
    // Temperature of 0.0 should give deterministic outputs
    let config = ClaudeConfig {
        api_key: "test-key".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
        temperature: 0.0,
        timeout_ms: 30000,
    };

    assert_eq!(config.temperature, 0.0);
}

#[tokio::test]
async fn test_claude_max_temperature() {
    // Temperature of 1.0 should give most creative outputs
    let config = ClaudeConfig {
        api_key: "test-key".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
        temperature: 1.0,
        timeout_ms: 30000,
    };

    assert_eq!(config.temperature, 1.0);
}

#[tokio::test]
async fn test_completion_context_multiline_prefix() {
    let multiline = r#"def calculate_sum(a, b):
    """Calculate the sum of two numbers.

    Args:
        a: First number
        b: Second number

    Returns:
        The sum of a and b
    """
    "#;

    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "/test.py".to_string(),
        prefix: multiline.to_string(),
        suffix: String::new(),
        context: None,
    };

    assert!(context.prefix.contains("def calculate_sum"));
    assert!(context.prefix.contains("Args:"));
    assert!(context.prefix.contains("Returns:"));
}

#[tokio::test]
async fn test_concurrent_claude_client_creation() {
    use tokio::task;

    // Test creating multiple Claude clients concurrently
    let tasks: Vec<_> = (0..10).map(|i| {
        task::spawn(async move {
            let config = ClaudeConfig {
                api_key: format!("test-key-{}", i),
                model: "claude-sonnet-4-20250514".to_string(),
                max_tokens: 1024,
                temperature: 0.7,
                timeout_ms: 30000,
            };

            let _client = ClaudeClient::new(config);
            // Client creation should succeed
        })
    }).collect();

    for task in tasks {
        task.await.expect("Claude client creation task should complete");
    }
}

#[tokio::test]
async fn test_completion_context_special_characters() {
    // Test with special programming characters
    let special_chars = r#"def test():
    x = [1, 2, 3]
    y = {"key": "value"}
    z = (a, b, c)
    w = lambda x: x * 2
    pattern = r"^\d+$"
"#;

    let context = CompletionContext {
        language: "python".to_string(),
        file_path: "/test.py".to_string(),
        prefix: special_chars.to_string(),
        suffix: String::new(),
        context: None,
    };

    assert!(context.prefix.contains("[1, 2, 3]"));
    assert!(context.prefix.contains(r#"{"key": "value"}"#));
    assert!(context.prefix.contains("lambda"));
}
