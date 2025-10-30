//! Code Lens Integration Tests
//!
//! Comprehensive end-to-end tests for Phase 8 (Code Lens) functionality.
//! Tests protocol compliance, reference counting, test actions, and performance.

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
                        "codeLens": {}
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

    fn code_lens(&mut self, uri: &str) -> Value {
        self.send_request(
            "textDocument/codeLens",
            json!({
                "textDocument": {
                    "uri": uri
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
fn test_code_lens_initialization() {
    let mut client = LspClient::start();

    let response = client.initialize();

    // Verify server supports code lens
    assert!(response["result"]["capabilities"]["codeLensProvider"].is_object());

    client.initialized();
}

#[test]
fn test_python_function_reference_lens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def add(x, y):
    return x + y

def multiply(a, b):
    return a * b
"#;

    client.did_open("file:///test.py", "python", python_code);

    let response = client.code_lens("file:///test.py");

    // Should have code lenses
    let lenses = response["result"].as_array();
    assert!(lenses.is_some(), "Should return code lens array");

    if let Some(lenses_array) = lenses {
        assert!(!lenses_array.is_empty(), "Should have lenses for functions");

        // Check for reference count lenses
        let has_reference_lens = lenses_array.iter().any(|lens| {
            if let Some(command) = &lens["command"].as_object() {
                if let Some(title) = command.get("title").and_then(|t| t.as_str()) {
                    return title.contains("reference");
                }
            }
            false
        });

        assert!(has_reference_lens, "Should have reference count lenses");
    }
}

#[test]
fn test_python_test_function_lens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def test_addition():
    assert add(2, 3) == 5

def test_multiplication():
    assert multiply(2, 3) == 6
"#;

    client.did_open("file:///test.py", "python", python_code);

    let response = client.code_lens("file:///test.py");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        // Should have Run test and Debug test lenses
        let has_run_test = lenses_array.iter().any(|lens| {
            lens["command"]["title"].as_str().map_or(false, |t| t.contains("Run test"))
        });

        let has_debug_test = lenses_array.iter().any(|lens| {
            lens["command"]["title"].as_str().map_or(false, |t| t.contains("Debug test"))
        });

        assert!(has_run_test, "Should have 'Run test' action");
        assert!(has_debug_test, "Should have 'Debug test' action");
    }
}

#[test]
fn test_python_class_lens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
class Calculator:
    def add(self, x, y):
        return x + y

    def subtract(self, x, y):
        return x - y
"#;

    client.did_open("file:///test.py", "python", python_code);

    let response = client.code_lens("file:///test.py");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        // Should have lens for class
        assert!(!lenses_array.is_empty(), "Should have lenses for class");
    }
}

#[test]
fn test_javascript_function_lens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let js_code = r#"
function add(a, b) {
    return a + b;
}

const multiply = (x, y) => x * y;

class Calculator {
    subtract(a, b) {
        return a - b;
    }
}
"#;

    client.did_open("file:///test.js", "javascript", js_code);

    let response = client.code_lens("file:///test.js");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        assert!(!lenses_array.is_empty(), "Should have lenses for functions");
    }
}

#[test]
fn test_javascript_test_lens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let js_code = r#"
test('addition works', () => {
    expect(add(2, 3)).toBe(5);
});

it('should multiply numbers', () => {
    expect(multiply(2, 3)).toBe(6);
});

describe('Calculator', () => {
    it('should subtract', () => {
        expect(subtract(5, 3)).toBe(2);
    });
});
"#;

    client.did_open("file:///test.js", "javascript", js_code);

    let response = client.code_lens("file:///test.js");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        // Should have test action lenses
        let has_test_actions = lenses_array.iter().any(|lens| {
            lens["command"]["title"].as_str().map_or(false, |t| {
                t.contains("Run test") || t.contains("Debug test")
            })
        });

        assert!(has_test_actions, "Should have test action lenses");
    }
}

#[test]
fn test_rust_function_lens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let rust_code = r#"
fn add(x: i32, y: i32) -> i32 {
    x + y
}

fn multiply(x: i32, y: i32) -> i32 {
    x * y
}

struct Calculator {
    value: i32,
}

impl Calculator {
    fn new() -> Self {
        Self { value: 0 }
    }
}
"#;

    client.did_open("file:///test.rs", "rust", rust_code);

    let response = client.code_lens("file:///test.rs");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        assert!(!lenses_array.is_empty(), "Should have lenses for functions and struct");
    }
}

#[test]
fn test_rust_test_lens() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let rust_code = r#"
#[test]
fn test_addition() {
    assert_eq!(add(2, 3), 5);
}

#[tokio::test]
async fn test_async_operation() {
    let result = async_add(2, 3).await;
    assert_eq!(result, 5);
}
"#;

    client.did_open("file:///test.rs", "rust", rust_code);

    let response = client.code_lens("file:///test.rs");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        // Should have Run test and Debug test for #[test] functions
        let has_run_test = lenses_array.iter().any(|lens| {
            lens["command"]["title"].as_str().map_or(false, |t| t.contains("Run test"))
        });

        let has_debug_test = lenses_array.iter().any(|lens| {
            lens["command"]["title"].as_str().map_or(false, |t| t.contains("Debug test"))
        });

        assert!(has_run_test, "Should have 'Run test' for Rust tests");
        assert!(has_debug_test, "Should have 'Debug test' for Rust tests");
    }
}

#[test]
fn test_code_lens_empty_file() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///empty.py", "python", "");

    let response = client.code_lens("file:///empty.py");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        assert!(lenses_array.is_empty(), "Empty file should have no lenses");
    }
}

#[test]
fn test_code_lens_unsupported_language() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///test.txt", "plaintext", "some text");

    let response = client.code_lens("file:///test.txt");

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        assert!(lenses_array.is_empty(), "Unsupported language should have no lenses");
    }
}

#[test]
fn test_code_lens_protocol_compliance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "def foo():\n    pass\n";
    client.did_open("file:///test.py", "python", python_code);

    let response = client.code_lens("file:///test.py");

    // Verify response structure matches LSP spec
    assert!(response["result"].is_array() || response["result"].is_null());

    if let Some(lenses) = response["result"].as_array() {
        for lens in lenses {
            // Each lens must have range
            assert!(lens["range"].is_object(), "Lens must have range");
            assert!(lens["range"]["start"].is_object());
            assert!(lens["range"]["end"].is_object());

            // Lens may have command or data (for resolve)
            assert!(lens["command"].is_object() || lens["data"].is_object(),
                "Lens must have command or data");

            if let Some(command) = lens["command"].as_object() {
                assert!(command.contains_key("title"), "Command must have title");
                assert!(command.contains_key("command"), "Command must have command name");
            }
        }
    }
}

#[test]
fn test_code_lens_performance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Large file with many functions
    let mut large_code = String::from("# Python file\n");
    for i in 0..100 {
        large_code.push_str(&format!("def func{}(a, b):\n    return a + b\n\n", i));
        large_code.push_str(&format!("def test_func{}():\n    assert func{}(1, 2) == 3\n\n", i, i));
    }

    client.did_open("file:///large.py", "python", &large_code);

    let start = Instant::now();
    let response = client.code_lens("file:///large.py");
    let duration = start.elapsed();

    // Should complete within reasonable time
    assert!(duration < Duration::from_millis(500),
        "Code lens should compute in < 500ms, took {:?}", duration);

    let lenses = response["result"].as_array();
    assert!(lenses.is_some());

    if let Some(lenses_array) = lenses {
        // Should have lenses for 100 functions + 100 test functions
        assert!(lenses_array.len() >= 100, "Should have lenses for all functions");
    }
}

#[test]
fn test_multi_language_session() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Open files in different languages
    client.did_open("file:///test.py", "python", "def foo():\n    pass\n");
    client.did_open("file:///test.js", "javascript", "function bar() {}\n");
    client.did_open("file:///test.rs", "rust", "fn baz() {}\n");

    // Get lenses for each
    let py_response = client.code_lens("file:///test.py");
    let js_response = client.code_lens("file:///test.js");
    let rs_response = client.code_lens("file:///test.rs");

    // All should return valid lenses
    assert!(py_response["result"].is_array());
    assert!(js_response["result"].is_array());
    assert!(rs_response["result"].is_array());
}

#[test]
fn test_code_lens_command_structure() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def test_something():
    pass
"#;

    client.did_open("file:///test.py", "python", python_code);

    let response = client.code_lens("file:///test.py");

    let lenses = response["result"].as_array().unwrap();

    for lens in lenses {
        if let Some(command) = lens["command"].as_object() {
            // Verify command structure
            let title = command["title"].as_str().unwrap();
            let cmd_name = command["command"].as_str().unwrap();

            assert!(!title.is_empty(), "Command title should not be empty");
            assert!(!cmd_name.is_empty(), "Command name should not be empty");

            // Commands should follow naming convention
            if title.contains("Run test") {
                assert!(cmd_name.contains("runTest") || cmd_name.contains("run_test"),
                    "Run test command should have appropriate name");
            }

            if title.contains("Debug test") {
                assert!(cmd_name.contains("debugTest") || cmd_name.contains("debug_test"),
                    "Debug test command should have appropriate name");
            }

            if title.contains("reference") {
                assert!(cmd_name.contains("showReferences") || cmd_name.contains("show_references"),
                    "Reference command should have appropriate name");
            }
        }
    }
}

#[test]
fn test_code_lens_position_accuracy() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def function_one():
    pass

def function_two():
    pass
"#;

    client.did_open("file:///test.py", "python", python_code);

    let response = client.code_lens("file:///test.py");

    let lenses = response["result"].as_array().unwrap();

    // Lenses should be positioned at function definitions
    for lens in lenses {
        let start_line = lens["range"]["start"]["line"].as_u64().unwrap();

        // Should be on a line with 'def' keyword (lines 1 or 4)
        assert!(start_line == 1 || start_line == 4,
            "Lens should be positioned at function definition");
    }
}

#[test]
fn test_code_lens_concurrent_requests() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "def foo():\n    pass\n";
    client.did_open("file:///test.py", "python", python_code);

    // Simulate rapid requests
    let start = Instant::now();
    for _ in 0..10 {
        let response = client.code_lens("file:///test.py");
        assert!(response["result"].is_array());
    }
    let duration = start.elapsed();

    // Average should be well under 100ms per request
    let avg_per_request = duration / 10;
    assert!(avg_per_request < Duration::from_millis(100),
        "Average code lens time should be < 100ms, got {:?}", avg_per_request);
}

#[test]
fn test_code_lens_reference_count_format() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "def my_function():\n    pass\n";
    client.did_open("file:///test.py", "python", python_code);

    let response = client.code_lens("file:///test.py");

    let lenses = response["result"].as_array().unwrap();

    // Find reference count lens
    let ref_lens = lenses.iter().find(|lens| {
        lens["command"]["title"].as_str().map_or(false, |t| t.contains("reference"))
    });

    assert!(ref_lens.is_some(), "Should have reference count lens");

    if let Some(lens) = ref_lens {
        let title = lens["command"]["title"].as_str().unwrap();
        // Should show count like "0 references", "1 reference", "5 references"
        assert!(title.contains("reference"), "Should mention references");
    }
}
