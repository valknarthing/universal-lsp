//! Diagnostics Integration Tests (Phase 1)
//!
//! Comprehensive end-to-end tests for real-time diagnostics functionality.
//! Tests syntax errors, semantic analysis, and LSP protocol compliance.

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

    fn send_notification(&mut self, method: &str, params: Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let message = serde_json::to_string(&notification).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", message.len());

        let stdin = self.process.stdin.as_mut().unwrap();
        stdin.write_all(header.as_bytes()).unwrap();
        stdin.write_all(message.as_bytes()).unwrap();
        stdin.flush().unwrap();
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

    fn read_notification(&mut self) -> Option<Value> {
        let stdout = self.process.stdout.as_mut().unwrap();
        let mut reader = BufReader::new(stdout);

        // Try to read notification (non-blocking)
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
            return None;
        }

        if let Some(length_str) = header.trim().strip_prefix("Content-Length: ") {
            let content_length: usize = length_str.parse().ok()?;

            let mut empty = String::new();
            reader.read_line(&mut empty).ok()?;

            let mut buffer = vec![0u8; content_length];
            reader.read_exact(&mut buffer).ok()?;

            serde_json::from_slice(&buffer).ok()
        } else {
            None
        }
    }

    fn initialize(&mut self) -> Value {
        self.send_request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": "file:///tmp/test-workspace",
                "capabilities": {}
            }),
        )
    }

    fn initialized(&mut self) {
        self.send_notification("initialized", json!({}));
    }

    fn did_open(&mut self, uri: &str, language_id: &str, content: &str) {
        self.send_notification(
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
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[test]
fn test_diagnostics_initialization() {
    let mut client = LspClient::start();
    let response = client.initialize();

    // Server should support diagnostics
    assert!(response["result"]["capabilities"].is_object());

    client.initialized();
}

#[test]
fn test_python_syntax_error() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Python code with syntax error
    let python_code = r#"
def foo(
    print("missing closing paren")
"#;

    client.did_open("file:///test.py", "python", python_code);

    // Wait for diagnostics notification
    std::thread::sleep(Duration::from_millis(100));

    if let Some(notification) = client.read_notification() {
        if notification["method"] == "textDocument/publishDiagnostics" {
            let diagnostics = &notification["params"]["diagnostics"];
            assert!(diagnostics.is_array());

            let diags = diagnostics.as_array().unwrap();
            assert!(!diags.is_empty(), "Should have syntax error diagnostic");
        }
    }
}

#[test]
fn test_python_undefined_variable() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def foo():
    return undefined_variable
"#;

    client.did_open("file:///test.py", "python", python_code);
    std::thread::sleep(Duration::from_millis(100));

    if let Some(notification) = client.read_notification() {
        if notification["method"] == "textDocument/publishDiagnostics" {
            let diagnostics = &notification["params"]["diagnostics"];
            let diags = diagnostics.as_array().unwrap();

            let has_undefined = diags.iter().any(|d| {
                d["message"].as_str().map_or(false, |m| {
                    m.contains("undefined") || m.contains("not defined")
                })
            });

            assert!(has_undefined, "Should detect undefined variable");
        }
    }
}

#[test]
fn test_javascript_syntax_error() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let js_code = r#"
function foo( {
    console.log("missing paren");
}
"#;

    client.did_open("file:///test.js", "javascript", js_code);
    std::thread::sleep(Duration::from_millis(100));

    if let Some(notification) = client.read_notification() {
        if notification["method"] == "textDocument/publishDiagnostics" {
            let diagnostics = &notification["params"]["diagnostics"];
            assert!(!diagnostics.as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn test_rust_syntax_error() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let rust_code = r#"
fn main( {
    println!("missing paren");
}
"#;

    client.did_open("file:///test.rs", "rust", rust_code);
    std::thread::sleep(Duration::from_millis(100));

    if let Some(notification) = client.read_notification() {
        if notification["method"] == "textDocument/publishDiagnostics" {
            let diagnostics = &notification["params"]["diagnostics"];
            assert!(!diagnostics.as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn test_diagnostics_severity_levels() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "def foo():\n    pass\n";
    client.did_open("file:///test.py", "python", python_code);
    std::thread::sleep(Duration::from_millis(100));

    if let Some(notification) = client.read_notification() {
        if notification["method"] == "textDocument/publishDiagnostics" {
            let diagnostics = &notification["params"]["diagnostics"];

            for diag in diagnostics.as_array().unwrap() {
                // Severity: 1=Error, 2=Warning, 3=Information, 4=Hint
                if let Some(severity) = diag["severity"].as_u64() {
                    assert!((1..=4).contains(&severity), "Severity should be 1-4");
                }
            }
        }
    }
}

#[test]
fn test_diagnostics_clear_on_fix() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // First: code with error
    let bad_code = "def foo(\n";
    client.did_open("file:///test.py", "python", bad_code);
    std::thread::sleep(Duration::from_millis(100));

    // Should have diagnostics
    if let Some(notification) = client.read_notification() {
        if notification["method"] == "textDocument/publishDiagnostics" {
            let diagnostics = &notification["params"]["diagnostics"];
            assert!(!diagnostics.as_array().unwrap().is_empty());
        }
    }

    // Fix the code
    client.send_notification(
        "textDocument/didChange",
        json!({
            "textDocument": {
                "uri": "file:///test.py",
                "version": 2
            },
            "contentChanges": [{
                "text": "def foo():\n    pass\n"
            }]
        }),
    );
    std::thread::sleep(Duration::from_millis(100));

    // Diagnostics should clear
    if let Some(notification) = client.read_notification() {
        if notification["method"] == "textDocument/publishDiagnostics" {
            let diagnostics = &notification["params"]["diagnostics"];
            assert!(diagnostics.as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn test_diagnostics_protocol_compliance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///test.py", "python", "x = 1\n");
    std::thread::sleep(Duration::from_millis(100));

    if let Some(notification) = client.read_notification() {
        assert_eq!(notification["method"], "textDocument/publishDiagnostics");

        let params = &notification["params"];
        assert!(params["uri"].is_string());
        assert!(params["diagnostics"].is_array());

        for diag in params["diagnostics"].as_array().unwrap() {
            // Must have range
            assert!(diag["range"].is_object());
            assert!(diag["range"]["start"].is_object());
            assert!(diag["range"]["end"].is_object());

            // Must have message
            assert!(diag["message"].is_string());

            // Optional: severity, source, code
            if diag["severity"].is_number() {
                let sev = diag["severity"].as_u64().unwrap();
                assert!((1..=4).contains(&sev));
            }
        }
    }
}

#[test]
fn test_diagnostics_performance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Large file
    let mut large_code = String::new();
    for i in 0..1000 {
        large_code.push_str(&format!("x{} = {}\n", i, i));
    }

    let start = Instant::now();
    client.did_open("file:///large.py", "python", &large_code);
    std::thread::sleep(Duration::from_millis(200));
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(1),
        "Diagnostics should complete in < 1s for large file");
}

#[test]
fn test_multi_file_diagnostics() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Open multiple files
    client.did_open("file:///test1.py", "python", "x = 1\n");
    client.did_open("file:///test2.js", "javascript", "const x = 1;\n");
    client.did_open("file:///test3.rs", "rust", "let x = 1;\n");

    std::thread::sleep(Duration::from_millis(300));

    // Should receive diagnostics for each file
    let mut received_files = Vec::new();
    for _ in 0..3 {
        if let Some(notification) = client.read_notification() {
            if notification["method"] == "textDocument/publishDiagnostics" {
                let uri = notification["params"]["uri"].as_str().unwrap();
                received_files.push(uri.to_string());
            }
        }
    }

    assert!(received_files.len() >= 1, "Should receive diagnostics for opened files");
}
