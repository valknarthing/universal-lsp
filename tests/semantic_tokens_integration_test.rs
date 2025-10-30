//! Semantic Tokens Integration Tests (Phase 5)
//!
//! Tests enhanced syntax highlighting with semantic token classification.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

struct LspClient {
    process: Child,
    request_id: u64,
}

impl LspClient {
    fn start() -> Self {
        let process = Command::new("cargo")
            .args(&["run", "--bin", "universal-lsp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start universal-lsp");

        Self { process, request_id: 0 }
    }

    fn send_request(&mut self, method: &str, params: Value) -> Value {
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        });

        let message = serde_json::to_string(&request).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", message.len());

        let stdin = self.process.stdin.as_mut().unwrap();
        stdin.write_all(header.as_bytes()).unwrap();
        stdin.write_all(message.as_bytes()).unwrap();
        stdin.flush().unwrap();

        self.read_response()
    }

    fn read_response(&mut self) -> Value {
        let stdout = self.process.stdout.as_mut().unwrap();
        let mut reader = BufReader::new(stdout);

        let mut header = String::new();
        reader.read_line(&mut header).unwrap();

        let content_length: usize = header
            .trim()
            .strip_prefix("Content-Length: ")
            .unwrap()
            .parse()
            .unwrap();

        let mut empty = String::new();
        reader.read_line(&mut empty).unwrap();

        let mut buffer = vec![0u8; content_length];
        reader.read_exact(&mut buffer).unwrap();

        serde_json::from_slice(&buffer).unwrap()
    }

    fn initialize(&mut self) -> Value {
        self.send_request("initialize", json!({
            "processId": null,
            "rootUri": "file:///tmp/test",
            "capabilities": {
                "textDocument": {
                    "semanticTokens": {
                        "requests": { "full": true }
                    }
                }
            }
        }))
    }

    fn initialized(&mut self) {
        self.send_request("initialized", json!({}));
    }

    fn did_open(&mut self, uri: &str, lang_id: &str, content: &str) {
        self.send_request("textDocument/didOpen", json!({
            "textDocument": {
                "uri": uri,
                "languageId": lang_id,
                "version": 1,
                "text": content
            }
        }));
    }

    fn semantic_tokens_full(&mut self, uri: &str) -> Value {
        self.send_request("textDocument/semanticTokens/full", json!({
            "textDocument": { "uri": uri }
        }))
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[test]
fn test_semantic_tokens_initialization() {
    let mut client = LspClient::start();
    let response = client.initialize();

    assert!(response["result"]["capabilities"]["semanticTokensProvider"].is_object());

    let legend = &response["result"]["capabilities"]["semanticTokensProvider"]["legend"];
    assert!(legend["tokenTypes"].is_array(), "Should have token types");
    assert!(legend["tokenModifiers"].is_array(), "Should have token modifiers");

    client.initialized();
}

#[test]
fn test_python_semantic_tokens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def calculate(x, y):
    result = x + y
    return result
"#;

    client.did_open("file:///test.py", "python", python_code);
    let response = client.semantic_tokens_full("file:///test.py");

    if let Some(result) = response["result"].as_object() {
        assert!(result["data"].is_array(), "Should return semantic tokens data");

        let data = result["data"].as_array().unwrap();
        assert!(!data.is_empty(), "Should have tokens for Python code");

        // Data should be in delta encoding format
        assert!(data.len() % 5 == 0, "Data should be in groups of 5 (delta encoding)");
    }
}

#[test]
fn test_javascript_semantic_tokens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let js_code = r#"
function calculate(x, y) {
    const result = x + y;
    return result;
}
"#;

    client.did_open("file:///test.js", "javascript", js_code);
    let response = client.semantic_tokens_full("file:///test.js");

    if let Some(result) = response["result"].as_object() {
        assert!(result["data"].is_array());
        let data = result["data"].as_array().unwrap();
        assert!(!data.is_empty());
    }
}

#[test]
fn test_rust_semantic_tokens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let rust_code = r#"
fn calculate(x: i32, y: i32) -> i32 {
    let result = x + y;
    result
}
"#;

    client.did_open("file:///test.rs", "rust", rust_code);
    let response = client.semantic_tokens_full("file:///test.rs");

    if let Some(result) = response["result"].as_object() {
        assert!(result["data"].is_array());
        let data = result["data"].as_array().unwrap();
        assert!(!data.is_empty());
    }
}

#[test]
fn test_semantic_tokens_protocol_compliance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///test.py", "python", "x = 42\n");
    let response = client.semantic_tokens_full("file:///test.py");

    if let Some(result) = response["result"].as_object() {
        let data = result["data"].as_array().unwrap();

        // Delta encoding: [deltaLine, deltaStart, length, tokenType, tokenModifiers]
        for chunk in data.chunks(5) {
            assert_eq!(chunk.len(), 5, "Each token should have 5 values");
            for val in chunk {
                assert!(val.is_u64(), "All values should be unsigned integers");
            }
        }
    }
}

#[test]
fn test_semantic_tokens_empty_file() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///empty.py", "python", "");
    let response = client.semantic_tokens_full("file:///empty.py");

    if let Some(result) = response["result"].as_object() {
        let data = result["data"].as_array().unwrap();
        assert!(data.is_empty(), "Empty file should have no tokens");
    }
}
