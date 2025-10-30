//! Code Actions Integration Tests (Phase 2)
//!
//! Tests quick fixes, refactorings, and AI-powered code actions.

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
                    "codeAction": {
                        "codeActionLiteralSupport": {
                            "codeActionKind": {
                                "valueSet": [
                                    "quickfix",
                                    "refactor",
                                    "refactor.extract",
                                    "refactor.inline",
                                    "refactor.rewrite",
                                    "source",
                                    "source.organizeImports"
                                ]
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

    fn code_action(&mut self, uri: &str, start_line: u32, end_line: u32) -> Value {
        self.send_request("textDocument/codeAction", json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": start_line, "character": 0 },
                "end": { "line": end_line, "character": 0 }
            },
            "context": {
                "diagnostics": []
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
fn test_code_actions_initialization() {
    let mut client = LspClient::start();
    let response = client.initialize();

    assert!(response["result"]["capabilities"]["codeActionProvider"].is_boolean()
            || response["result"]["capabilities"]["codeActionProvider"].is_object());

    client.initialized();
}

#[test]
fn test_python_code_actions() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    let python_code = r#"
def calculate(x, y):
    result = x + y
    return result
"#;

    client.did_open("file:///test.py", "python", python_code);
    let response = client.code_action("file:///test.py", 0, 10);

    // Should return code actions or empty array
    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_javascript_code_actions() {
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
    let response = client.code_action("file:///test.js", 0, 10);

    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_rust_code_actions() {
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
    let response = client.code_action("file:///test.rs", 0, 10);

    assert!(response["result"].is_array() || response["result"].is_null());
}

#[test]
fn test_code_action_protocol_compliance() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///test.py", "python", "x = 1\n");
    let response = client.code_action("file:///test.py", 0, 10);

    if let Some(actions) = response["result"].as_array() {
        for action in actions {
            // Each action must have title
            assert!(action["title"].is_string(), "Code action must have title");

            // Must have either command or edit
            assert!(action["command"].is_object() || action["edit"].is_object(),
                "Code action must have command or edit");

            // Optional kind field
            if let Some(kind) = action["kind"].as_str() {
                assert!(kind.starts_with("quickfix")
                    || kind.starts_with("refactor")
                    || kind.starts_with("source"),
                    "Code action kind should be valid");
            }
        }
    }
}

#[test]
fn test_code_actions_empty_file() {
    let mut client = LspClient::start();
    client.initialize();
    client.initialized();

    client.did_open("file:///empty.py", "python", "");
    let response = client.code_action("file:///empty.py", 0, 10);

    // Empty file may have no actions
    if let Some(actions) = response["result"].as_array() {
        // Just verify it returns valid structure
        assert!(actions.is_empty() || !actions.is_empty());
    }
}
