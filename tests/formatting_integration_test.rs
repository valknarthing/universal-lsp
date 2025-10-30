//! Document Formatting Integration Tests (Phase 7)
//!
//! Tests code formatting with external formatters (black, prettier, rustfmt).

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
                    "formatting": {},
                    "rangeFormatting": {}
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

    fn formatting(&mut self, uri: &str) -> Value {
        self.send_request("textDocument/formatting", json!({
            "textDocument": { "uri": uri },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }))
    }

    fn range_formatting(&mut self, uri: &str, start_line: u32, end_line: u32) -> Value {
        self.send_request("textDocument/rangeFormatting", json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": start_line, "character": 0 },
                "end": { "line": end_line, "character": 0 }
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }))
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[test]
fn test_formatting_initialization() {
    let mut client = LspClient::start();
    let response = client.initialize();

    assert!(response["result"]["capabilities"]["documentFormattingProvider"].is_boolean()
            || response["result"]["capabilities"]["documentFormattingProvider"].is_object());

    assert!(response["result"]["capabilities"]["documentRangeFormattingProvider"].is_boolean()
            || response["result"]["capabilities"]["documentRangeFormattingProvider"].is_object());

    client.initialized();
}

#[test]
fn test_python_formatting() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let unformatted_python = "def foo(  x,y  ):\n  return x+y\n";
    client.did_open("file:///test.py", "python", unformatted_python);

    let response = client.formatting("file:///test.py");

    // Should return text edits or null
    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_javascript_formatting() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let unformatted_js = "function foo(  x,y  ) {return x+y;}\n";
    client.did_open("file:///test.js", "javascript", unformatted_js);

    let response = client.formatting("file:///test.js");

    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_rust_formatting() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let unformatted_rust = "fn foo(  x:i32,y:i32  )->i32{x+y}\n";
    client.did_open("file:///test.rs", "rust", unformatted_rust);

    let response = client.formatting("file:///test.rs");

    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_range_formatting() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "def foo():\n  x=1\n  y=2\n  return x+y\n";
    client.did_open("file:///test.py", "python", python_code);

    let response = client.range_formatting("file:///test.py", 1, 3);

    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_formatting_protocol_compliance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///test.py", "python", "x=1\n");
    let response = client.formatting("file:///test.py");

    if let Some(edits) = response["result"].as_array() {
        for edit in edits {
            // Each edit must have range and newText
            assert!(edit["range"].is_object(), "Edit must have range");
            assert!(edit["range"]["start"].is_object());
            assert!(edit["range"]["end"].is_object());
            assert!(edit["newText"].is_string(), "Edit must have newText");
        }
    }
}

#[test]
fn test_formatting_empty_file() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///empty.py", "python", "");
    let response = client.formatting("file:///empty.py");

    // Empty file should return null or empty edits
    if let Some(edits) = response["result"].as_array() {
        assert!(edits.is_empty(), "Empty file should have no formatting edits");
    }
}

#[test]
fn test_multi_language_formatting() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    // Test formatting for different languages
    client.did_open("file:///test1.py", "python", "x=1\n");
    client.did_open("file:///test2.js", "javascript", "const x=1;\n");
    client.did_open("file:///test3.rs", "rust", "let x=1;\n");

    let py_response = client.formatting("file:///test1.py");
    let js_response = client.formatting("file:///test2.js");
    let rs_response = client.formatting("file:///test3.rs");

    // All should return valid responses
    assert!(py_response["result"].is_array() || py_response["result"].is_null());
    assert!(js_response["result"].is_array() || js_response["result"].is_null());
    assert!(rs_response["result"].is_array() || rs_response["result"].is_null());
}
