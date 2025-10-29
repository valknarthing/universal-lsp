# Integration Test Timeout Analysis

**Date**: 2025-10-29
**Test**: `tests/integration_svelte_test.rs::test_fullstack_initialization`
**Status**: ✅ **FIXED AND PASSING**
**Root Cause**: Invalid CLI arguments causing LSP server to exit with status 2
**Solution**: Updated test to use correct argument names matching actual CLI implementation

**Final Result**: Test passes in 0.71s with proper LSP initialization handshake.

---

## Problem Summary

The integration test `test_fullstack_initialization` spawns the actual `target/release/universal-lsp` binary and attempts LSP initialize handshake, but times out waiting for response.

## Investigation Results

### 1. ✅ Binary Exists and Runs
```bash
$ ls -lh target/release/universal-lsp
-rwxrwxr-x 2 valknar valknar 20M 28. Okt 11:54 target/release/universal-lsp
```

### 2. ✅ Binary Responds to --help
```bash
$ target/release/universal-lsp --help
Universal LSP Server with MCP integration and LSP proxying
...
```

### 3. ❌ LSP Initialize Request Fails with Parse Error

**Test Command**:
```bash
printf 'Content-Length: 175\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize",...}' \
  | timeout 2 target/release/universal-lsp
```

**Server Response**:
```
[ERROR] tower_lsp::transport: failed to decode message:
unable to parse JSON body: EOF while parsing a string at line 1 column 175

Content-Length: 75
{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}
```

## Root Cause

The LSP server is **responding** (not hanging), but with a **JSON parse error** (code `-32700`). This suggests:

1. **Message framing issue**: The Content-Length calculation or message formatting has a bug
2. **Stdio buffering**: The test's `AsyncWriteExt::write_all()` + `flush()` might not be synchronizing properly with the server's stdin reader
3. **Character encoding**: The JSON might have encoding issues that cause the parser to see a truncated string

## Test Code Structure

The integration test in `tests/integration_svelte_test.rs` (lines 230-265):

```rust
async fn initialize(&mut self) -> anyhow::Result<Value> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": self.request_id,
        "method": "initialize",
        "params": { ... }
    });

    self.send_request(request).await?;  // Line 255

    let initialized = json!({            // Send initialized notification
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    self.send_notification(initialized).await?;  // Line 262

    self.read_response().await  // TIMES OUT HERE
}
```

The test sends TWO messages in sequence:
1. `initialize` request (expects response)
2. `initialized` notification (no response expected)

Then waits for the `initialize` response, which never arrives due to the parse error.

## Likely Issues

### Issue A: Content-Length Calculation
The `send_request` method (lines 283-292) calculates Content-Length:

```rust
async fn send_request(&mut self, request: Value) -> anyhow::Result<()> {
    let message = serde_json::to_string(&request)?;
    let header = format!("Content-Length: {}\r\n\r\n", message.len());

    self.stdin.write_all(header.as_bytes()).await?;
    self.stdin.write_all(message.as_bytes()).await?;
    self.stdin.flush().await?;

    Ok(())
}
```

**Problem**: `.len()` returns **byte** count, but if the JSON contains non-ASCII characters (like in `initializationOptions`), the byte count might not match character count.

### Issue B: Response Reading Before Server Ready
The test immediately sends `initialized` notification after `initialize` request without waiting for response. This could cause the server's response parser to get confused.

**LSP Spec**: The `initialized` notification should only be sent **after** receiving the `initialize` response.

## Recommended Fixes

### Fix 1: Reorder Initialize Handshake (CRITICAL)

**In `tests/integration_svelte_test.rs`**, change the `initialize` method to match LSP spec:

```rust
async fn initialize(&mut self) -> anyhow::Result<Value> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": self.request_id,
        "method": "initialize",
        "params": { ... }
    });

    self.request_id += 1;
    self.send_request(request).await?;

    // WAIT FOR RESPONSE FIRST
    let response = self.read_response().await?;

    // THEN send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    self.send_notification(initialized).await?;

    Ok(response)  // Return the initialize response
}
```

### Fix 2: Add Debug Logging to Test

Add logging to see what's being sent/received:

```rust
async fn send_request(&mut self, request: Value) -> anyhow::Result<()> {
    let message = serde_json::to_string(&request)?;
    let header = format!("Content-Length: {}\r\n\r\n", message.len());

    eprintln!("SENDING: {}{}", header, message);  // DEBUG

    self.stdin.write_all(header.as_bytes()).await?;
    self.stdin.write_all(message.as_bytes()).await?;
    self.stdin.flush().await?;

    Ok(())
}
```

### Fix 3: Validate JSON Before Sending

Add validation to catch malformed JSON:

```rust
async fn send_request(&mut self, request: Value) -> anyhow::Result<()> {
    let message = serde_json::to_string(&request)?;

    // Validate JSON can be parsed back
    let _: Value = serde_json::from_str(&message)?;

    let header = format!("Content-Length: {}\r\n\r\n", message.len());
    // ... rest of send logic
}
```

## Next Steps

1. **Apply Fix 1** (reorder handshake) - this is likely the main issue
2. Run integration test again: `cargo test --test integration_svelte_test test_fullstack_initialization`
3. If still fails, add debug logging (Fix 2) and inspect exact messages
4. Check LSP server's JSON parsing code for any quirks

## Status of Unit Tests

**All 32 unit tests are passing** ✅ (0.21s execution time)

The integration test failure is a separate issue testing actual binary behavior, not the library code itself.

---

## Test Commands

```bash
# Run only unit tests (ALL PASSING)
cargo test --lib

# Run only integration tests
cargo test --test integration_svelte_test

# Run specific integration test
cargo test --test integration_svelte_test test_fullstack_initialization

# Run with output for debugging
cargo test --test integration_svelte_test test_fullstack_initialization -- --nocapture
```

---

## Final Resolution (2025-10-29 23:34)

### The Actual Problem

The test was passing **invalid CLI arguments** to the LSP server binary:
- `--mcp-enable=http://localhost:3001` (should be `--mcp-pre`)
- `--mcp-cache=true` (should be `--mcp-cache` without `=true`)

This caused the server to exit with status 2 (command-line argument error) before it could respond to any LSP messages, making the test timeout while waiting for a response that would never come.

### The Fix

**File**: `tests/integration_svelte_test.rs` (lines 204-213)

**Changed**:
```rust
.args(&[
    "--log-level=info",
    &format!("--mcp-pre={}", mcp_url),      // ✅ Changed from --mcp-enable
    "--lsp-proxy=typescript=typescript-language-server",
    "--lsp-proxy=python=pyright",
    "--lsp-proxy=svelte=svelteserver",
    "--mcp-timeout=3000",
    "--mcp-cache",                           // ✅ Changed from --mcp-cache=true
    "--max-concurrent=200",
])
```

### Debug Process

1. **Added debug logging** to see exact messages being sent/received
2. **Added process status check** to detect if server exits early
3. **Manual testing** with `target/release/universal-lsp --help` to discover actual CLI arguments
4. **Fixed arguments** to match actual implementation

### Test Results

```
test test_fullstack_initialization ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.71s
```

**Success**: The test now:
- ✅ Spawns the LSP server successfully
- ✅ Sends initialize request with proper JSON-RPC format
- ✅ Receives initialize response with server capabilities
- ✅ Sends initialized notification
- ✅ Properly shuts down the server
- ✅ Completes in 0.71 seconds (well under the 5s timeout)

### Lessons Learned

1. **Early exit detection** - Adding `process.try_wait()` immediately after spawn revealed the server was exiting with status 2
2. **Manual testing** - Running the binary with `--help` showed the disconnect between test assumptions and reality
3. **Debug logging** - Comprehensive logging of message bytes helped verify the LSP protocol implementation was correct
4. **Root cause vs symptoms** - The timeout was a symptom; the real problem was invalid CLI arguments causing immediate exit
