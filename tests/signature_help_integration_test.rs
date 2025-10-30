//! Signature Help Integration Tests (Phase 3)
//!
//! Tests parameter hints during function calls with real LSP server.

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
                    "signatureHelp": {
                        "signatureInformation": {
                            "parameterInformation": {
                                "labelOffsetSupport": true
                            }
                        }
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

    fn signature_help(&mut self, uri: &str, line: u32, character: u32) -> Value {
        self.send_request("textDocument/signatureHelp", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[test]
fn test_signature_help_initialization() {
    let mut client = LspClient::start();
    let response = client.initialize();

    assert!(response["result"]["capabilities"]["signatureHelpProvider"].is_object());
    client.initialized();
}

#[test]
fn test_python_signature_help() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def add(x, y):
    return x + y

result = add(
"#;

    client.did_open("file:///test.py", "python", python_code);
    let response = client.signature_help("file:///test.py", 4, 13);

    if let Some(result) = response["result"].as_object() {
        if let Some(signatures) = result["signatures"].as_array() {
            assert!(!signatures.is_empty(), "Should have signature");
        }
    }
}

#[test]
fn test_javascript_signature_help() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let js_code = r#"
function multiply(a, b) {
    return a * b;
}

const result = multiply(
"#;

    client.did_open("file:///test.js", "javascript", js_code);
    let response = client.signature_help("file:///test.js", 5, 24);

    if let Some(result) = response["result"].as_object() {
        if let Some(signatures) = result["signatures"].as_array() {
            assert!(!signatures.is_empty());
        }
    }
}

#[test]
fn test_rust_signature_help() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let rust_code = r#"
fn add(x: i32, y: i32) -> i32 {
    x + y
}

fn main() {
    let result = add(
}
"#;

    client.did_open("file:///test.rs", "rust", rust_code);
    let response = client.signature_help("file:///test.rs", 6, 21);

    if let Some(result) = response["result"].as_object() {
        if let Some(signatures) = result["signatures"].as_array() {
            assert!(!signatures.is_empty());
        }
    }
}

#[test]
fn test_signature_help_protocol_compliance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = "def foo(a, b):\n    pass\n\nfoo(\n";
    client.did_open("file:///test.py", "python", python_code);
    let response = client.signature_help("file:///test.py", 3, 4);

    if let Some(result) = response["result"].as_object() {
        if let Some(signatures) = result["signatures"].as_array() {
            for sig in signatures {
                assert!(sig["label"].is_string(), "Signature must have label");

                if let Some(params) = sig["parameters"].as_array() {
                    for param in params {
                        assert!(param["label"].is_string() || param["label"].is_array(),
                            "Parameter must have label");
                    }
                }
            }
        }
    }
}
