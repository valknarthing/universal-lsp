//! Mock MCP Server for Testing
//!
//! A simple JSON-RPC 2.0 server that implements the MCP protocol for testing purposes.
//! Communicates via stdio and responds to standard MCP requests.

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn main() {
    eprintln!("Mock MCP Server starting...");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        eprintln!("Received: {}", line);

        // Parse JSON-RPC request
        let request: Value = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error parsing JSON: {}", e);
                continue;
            }
        };

        // Extract request ID and method
        let id = request.get("id").and_then(|v| v.as_i64()).unwrap_or(1);
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");

        eprintln!("Method: {}, ID: {}", method, id);

        // Create response based on method
        let response = match method {
            "initialize" => {
                // MCP initialization response
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "experimental": {},
                            "sampling": {},
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "mock-mcp-server",
                            "version": "0.1.0"
                        }
                    }
                })
            }
            "ping" | "health" => {
                // Health check / availability
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "status": "ok"
                    }
                })
            }
            "query" => {
                // MCP query response
                let params = request.get("params").cloned().unwrap_or(json!({}));
                eprintln!("Query params: {:?}", params);

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "suggestions": [
                            "Mock suggestion 1",
                            "Mock suggestion 2",
                            "Context-aware completion"
                        ],
                        "documentation": "This is a mock response from the test MCP server",
                        "confidence": 0.95
                    }
                })
            }
            _ => {
                // Unknown method - return error
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method)
                    }
                })
            }
        };

        // Send response
        let response_str = serde_json::to_string(&response).unwrap();
        eprintln!("Sending: {}", response_str);

        if let Err(e) = writeln!(stdout, "{}", response_str) {
            eprintln!("Error writing response: {}", e);
            break;
        }

        if let Err(e) = stdout.flush() {
            eprintln!("Error flushing stdout: {}", e);
            break;
        }
    }

    eprintln!("Mock MCP Server exiting...");
}
