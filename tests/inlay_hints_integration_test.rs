//! Inlay Hints Integration Tests
//!
//! Comprehensive end-to-end tests for Phase 6 (Inlay Hints) functionality.
//! Tests protocol compliance, multi-language support, and performance.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Mock LSP client for testing
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

        Self {
            process,
            request_id: 0,
        }
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

        // Read Content-Length header
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();

        let content_length: usize = header
            .trim()
            .strip_prefix("Content-Length: ")
            .unwrap()
            .parse()
            .unwrap();

        // Skip empty line
        let mut empty = String::new();
        reader.read_line(&mut empty).unwrap();

        // Read message body
        let mut buffer = vec![0u8; content_length];
        reader.read_exact(&mut buffer).unwrap();

        serde_json::from_slice(&buffer).unwrap()
    }

    fn initialize(&mut self) -> Value {
        self.send_request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": "file:///tmp/test-workspace",
                "capabilities": {
                    "textDocument": {
                        "inlayHint": {}
                    }
                }
            }),
        )
    }

    fn initialized(&mut self) {
        self.send_request("initialized", json!({}));
    }

    fn did_open(&mut self, uri: &str, language_id: &str, content: &str) {
        self.send_request(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": content
                }
            }),
        );
    }

    fn inlay_hint(&mut self, uri: &str, start_line: u32, end_line: u32) -> Value {
        self.send_request(
            "textDocument/inlayHint",
            json!({
                "textDocument": {
                    "uri": uri
                },
                "range": {
                    "start": { "line": start_line, "character": 0 },
                    "end": { "line": end_line, "character": 0 }
                }
            }),
        )
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[test]
fn test_inlay_hints_initialization() {
    let mut client = LspClient::start();

    let response = client.initialize();

    // Verify server supports inlay hints
    assert!(response["result"]["capabilities"]["inlayHintProvider"].is_boolean()
            || response["result"]["capabilities"]["inlayHintProvider"].is_object());

    client.initialized();
}

#[test]
fn test_python_parameter_hints() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def add(x, y):
    return x + y

result = add(5, 10)
"#;

    client.did_open("file:///test.py", "python", python_code);

    let response = client.inlay_hint("file:///test.py", 0, 10);

    // Should have hints for the function call
    let hints = response["result"].as_array();
    assert!(hints.is_some(), "Should return inlay hints array");

    // Python should have reference count hints
    if let Some(hints_array) = hints {
        assert!(!hints_array.is_empty(), "Should have at least one hint");
    }
}

#[test]
fn test_python_type_hints() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
x = 42
name = "test"
items = [1, 2, 3]
data = {"key": "value"}
"#;

    client.did_open("file:///test.py", "python", python_code);

    let response = client.inlay_hint("file:///test.py", 0, 10);

    let hints = response["result"].as_array();
    assert!(hints.is_some());

    // Should infer int, str, list, dict types
    if let Some(hints_array) = hints {
        // At least some type hints should be present
        let has_type_hints = hints_array.iter().any(|hint| {
            if let Some(label) = hint["label"].as_str() {
                label.contains("int") || label.contains("str")
                    || label.contains("list") || label.contains("dict")
            } else {
                false
            }
        });
        assert!(has_type_hints, "Should have type inference hints");
    }
}

#[test]
fn test_javascript_parameter_hints() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let js_code = r#"
function multiply(a, b) {
    return a * b;
}

const result = multiply(3, 4);
"#;

    client.did_open("file:///test.js", "javascript", js_code);

    let response = client.inlay_hint("file:///test.js", 0, 10);

    let hints = response["result"].as_array();
    assert!(hints.is_some());
}

#[test]
fn test_javascript_type_hints() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let js_code = r#"
const count = 42;
const message = "hello";
const isActive = true;
const items = [1, 2, 3];
"#;

    client.did_open("file:///test.js", "javascript", js_code);

    let response = client.inlay_hint("file:///test.js", 0, 10);

    let hints = response["result"].as_array();
    assert!(hints.is_some());

    if let Some(hints_array) = hints {
        let has_js_types = hints_array.iter().any(|hint| {
            if let Some(label) = hint["label"].as_str() {
                label.contains("number") || label.contains("string") || label.contains("boolean")
            } else {
                false
            }
        });
        assert!(has_js_types, "Should infer JavaScript types");
    }
}

#[test]
fn test_rust_parameter_hints() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let rust_code = r#"
fn add(x: i32, y: i32) -> i32 {
    x + y
}

fn main() {
    let result = add(5, 10);
}
"#;

    client.did_open("file:///test.rs", "rust", rust_code);

    let response = client.inlay_hint("file:///test.rs", 0, 10);

    let hints = response["result"].as_array();
    assert!(hints.is_some());
}

#[test]
fn test_rust_type_hints() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let rust_code = r#"
fn main() {
    let x = 42;
    let y = 3.14;
    let s = "hello";
    let v = vec![1, 2, 3];
}
"#;

    client.did_open("file:///test.rs", "rust", rust_code);

    let response = client.inlay_hint("file:///test.rs", 0, 10);

    let hints = response["result"].as_array();
    assert!(hints.is_some());

    if let Some(hints_array) = hints {
        let has_rust_types = hints_array.iter().any(|hint| {
            if let Some(label) = hint["label"].as_str() {
                label.contains("i32") || label.contains("f64")
                    || label.contains("&str") || label.contains("Vec")
            } else {
                false
            }
        });
        assert!(has_rust_types, "Should infer Rust types");
    }
}

#[test]
fn test_inlay_hints_range_filtering() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def func1():
    x = 1

def func2():
    y = 2

def func3():
    z = 3
"#;

    client.did_open("file:///test.py", "python", python_code);

    // Request hints for only middle function
    let response = client.inlay_hint("file:///test.py", 3, 6);

    let hints = response["result"].as_array();
    assert!(hints.is_some());

    // Hints should only be in requested range
    if let Some(hints_array) = hints {
        for hint in hints_array {
            let line = hint["position"]["line"].as_u64().unwrap() as u32;
            assert!(line >= 3 && line < 6, "Hints should be within requested range");
        }
    }
}

#[test]
fn test_inlay_hints_empty_file() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///empty.py", "python", "");

    let response = client.inlay_hint("file:///empty.py", 0, 10);

    let hints = response["result"].as_array();
    assert!(hints.is_some());

    // Empty file should return empty hints array
    if let Some(hints_array) = hints {
        assert!(hints_array.is_empty(), "Empty file should have no hints");
    }
}

#[test]
fn test_inlay_hints_unsupported_language() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///test.unknown", "unknown", "some content");

    let response = client.inlay_hint("file:///test.unknown", 0, 10);

    // Should return empty array for unsupported languages
    let hints = response["result"].as_array();
    assert!(hints.is_some());

    if let Some(hints_array) = hints {
        assert!(hints_array.is_empty(), "Unsupported language should have no hints");
    }
}

#[test]
fn test_inlay_hints_performance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Large file with many hints
    let mut large_code = String::from("# Python file\n");
    for i in 0..100 {
        large_code.push_str(&format!("x{} = {}\n", i, i));
        large_code.push_str(&format!("def func{}(a, b):\n    return a + b\n\n", i));
    }

    client.did_open("file:///large.py", "python", &large_code);

    let start = Instant::now();
    let response = client.inlay_hint("file:///large.py", 0, 400);
    let duration = start.elapsed();

    // Should complete within reasonable time
    assert!(duration < Duration::from_millis(500),
        "Inlay hints should compute in < 500ms, took {:?}", duration);

    let hints = response["result"].as_array();
    assert!(hints.is_some());
}

#[test]
fn test_inlay_hints_protocol_compliance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "x = 42\n";
    client.did_open("file:///test.py", "python", python_code);

    let response = client.inlay_hint("file:///test.py", 0, 10);

    // Verify response structure matches LSP spec
    assert!(response["result"].is_array() || response["result"].is_null());

    if let Some(hints) = response["result"].as_array() {
        for hint in hints {
            // Each hint must have position and label
            assert!(hint["position"].is_object(), "Hint must have position");
            assert!(hint["position"]["line"].is_number(), "Position must have line");
            assert!(hint["position"]["character"].is_number(), "Position must have character");

            // Label can be string or array of InlayHintLabelPart
            assert!(hint["label"].is_string() || hint["label"].is_array(),
                "Hint must have label");

            // Optional fields
            if hint["kind"].is_number() {
                let kind = hint["kind"].as_u64().unwrap();
                // kind should be 1 (Type) or 2 (Parameter)
                assert!(kind == 1 || kind == 2, "Kind must be Type(1) or Parameter(2)");
            }
        }
    }
}

#[test]
fn test_multi_language_session() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Open multiple files in different languages
    client.did_open("file:///test.py", "python", "x = 42\n");
    client.did_open("file:///test.js", "javascript", "const x = 42;\n");
    client.did_open("file:///test.rs", "rust", "let x = 42;\n");

    // Get hints for each
    let py_response = client.inlay_hint("file:///test.py", 0, 10);
    let js_response = client.inlay_hint("file:///test.js", 0, 10);
    let rs_response = client.inlay_hint("file:///test.rs", 0, 10);

    // All should return valid hints
    assert!(py_response["result"].is_array());
    assert!(js_response["result"].is_array());
    assert!(rs_response["result"].is_array());
}

#[test]
fn test_inlay_hints_concurrent_requests() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "x = 42\ny = 10\nz = x + y\n";
    client.did_open("file:///test.py", "python", python_code);

    // Simulate rapid typing by requesting hints multiple times
    let start = Instant::now();
    for _ in 0..10 {
        let response = client.inlay_hint("file:///test.py", 0, 10);
        assert!(response["result"].is_array());
    }
    let duration = start.elapsed();

    // Average should be well under 100ms per request
    let avg_per_request = duration / 10;
    assert!(avg_per_request < Duration::from_millis(100),
        "Average inlay hint time should be < 100ms, got {:?}", avg_per_request);
}
