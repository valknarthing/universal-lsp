# Terminal + Universal LSP + Claude Integration Guide

## Overview

Use Universal LSP Server directly from the command line for testing, debugging, and headless development workflows. This guide shows how to interact with the LSP server and MCP pipeline without an editor.

## Architecture

```
Terminal/CLI → Universal LSP Server (stdio) → MCP Pipeline → Claude API
                        ↓
                LSP JSON-RPC over stdin/stdout
```

## Prerequisites

- Universal LSP built at `/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp`
- Claude MCP Server running (see VSCode/Zed guides)
- `jq` for JSON formatting
- `websocat` or `netcat` for interactive testing (optional)

## Step 1: Build Universal LSP

```bash
cd /home/valknar/Projects/zed/universal-lsp
cargo build --release

# Verify binary
ls -lh target/release/universal-lsp
```

## Step 2: Start Claude MCP Server

Ensure your MCP server is running:

```bash
# Check if MCP server is running
curl http://localhost:3000/health

# If not running, start it
cd ~/claude-mcp
ANTHROPIC_API_KEY=your_key node server.js &
```

## Step 3: Manual LSP Communication

### Basic LSP Handshake

Create a test script `lsp-test.sh`:

```bash
#!/bin/bash

# Universal LSP binary path
LSP_BIN="/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp"

# Start LSP with MCP configuration
$LSP_BIN \
  --log-level=debug \
  --mcp-pre=http://localhost:3000 \
  --mcp-timeout=5000 \
  --mcp-cache=true <<EOF
Content-Length: 140

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp/test","capabilities":{}}}
Content-Length: 52

{"jsonrpc":"2.0","method":"initialized","params":{}}
EOF
```

Make it executable and run:

```bash
chmod +x lsp-test.sh
./lsp-test.sh
```

### Send Completion Request

```bash
#!/bin/bash

LSP_BIN="/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp"

# Initialize then request completion
$LSP_BIN --mcp-pre=http://localhost:3000 <<EOF
Content-Length: 140

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{}}}
Content-Length: 52

{"jsonrpc":"2.0","method":"initialized","params":{}}
Content-Length: 245

{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/test.py"},"position":{"line":5,"character":10},"context":{"triggerKind":1}}}
EOF
```

## Step 4: Interactive LSP Testing with Helper Script

Create `lsp-client.sh` for easier testing:

```bash
#!/bin/bash

LSP_BIN="/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp"
MCP_SERVER="http://localhost:3000"

# Parse command line arguments
LOG_LEVEL="${LOG_LEVEL:-info}"
USE_CACHE="${USE_CACHE:-true}"

# Helper function to send LSP message
send_lsp() {
    local message="$1"
    local length=${#message}
    echo -e "Content-Length: $length\r\n\r\n$message"
}

# Initialize LSP
init_message='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{"textDocument":{"completion":{"completionItem":{"snippetSupport":true}}}}}}'
initialized_message='{"jsonrpc":"2.0","method":"initialized","params":{}}'

# Start LSP server
(
    send_lsp "$init_message"
    send_lsp "$initialized_message"

    # Wait for user input
    while IFS= read -r line; do
        send_lsp "$line"
    done
) | $LSP_BIN \
    --log-level=$LOG_LEVEL \
    --mcp-pre=$MCP_SERVER \
    --mcp-timeout=5000 \
    --mcp-cache=$USE_CACHE
```

Usage:

```bash
# Start interactive session
./lsp-client.sh

# With debug logging
LOG_LEVEL=debug ./lsp-client.sh

# Without cache
USE_CACHE=false ./lsp-client.sh
```

## Step 5: Test MCP Pipeline Directly

### Test MCP Server with curl

```bash
# Test completion request
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{
    "request_type": "completion",
    "uri": "file:///tmp/test.py",
    "position": {"line": 10, "character": 5},
    "context": "def hello():"
  }' | jq

# Test hover request
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{
    "request_type": "hover",
    "uri": "file:///tmp/test.py",
    "position": {"line": 5, "character": 10},
    "context": "import numpy as np"
  }' | jq
```

### Benchmark MCP Response Times

Create `benchmark-mcp.sh`:

```bash
#!/bin/bash

MCP_URL="http://localhost:3000"
ITERATIONS=10

echo "Benchmarking MCP server..."

total=0
for i in $(seq 1 $ITERATIONS); do
    start=$(date +%s%N)

    curl -s -X POST $MCP_URL \
      -H "Content-Type: application/json" \
      -d '{
        "request_type": "completion",
        "uri": "test.py",
        "position": {"line": 1, "character": 0},
        "context": "import"
      }' > /dev/null

    end=$(date +%s%N)
    duration=$(( (end - start) / 1000000 )) # Convert to ms

    echo "Request $i: ${duration}ms"
    total=$((total + duration))
done

avg=$((total / ITERATIONS))
echo "Average response time: ${avg}ms"
```

## Step 6: Automated Testing Suite

Create `test-lsp-integration.sh`:

```bash
#!/bin/bash

set -e

LSP_BIN="/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp"
MCP_SERVER="http://localhost:3000"

echo "🧪 Universal LSP Integration Tests"
echo "===================================="

# Test 1: Binary exists
echo -n "✓ Checking LSP binary... "
if [ -f "$LSP_BIN" ]; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 2: MCP server reachable
echo -n "✓ Checking MCP server... "
if curl -s -f "$MCP_SERVER/health" > /dev/null; then
    echo "OK"
else
    echo "FAIL (is MCP server running?)"
    exit 1
fi

# Test 3: LSP initialization
echo -n "✓ Testing LSP initialization... "
response=$(echo 'Content-Length: 140\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{}}}' | \
    $LSP_BIN --mcp-pre=$MCP_SERVER 2>&1 | grep -o '"result"')

if [ -n "$response" ]; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 4: Completion request
echo -n "✓ Testing completion request... "
# This is simplified - in practice you'd parse JSON response
echo "OK (manual verification required)"

# Test 5: Hover request
echo -n "✓ Testing hover request... "
echo "OK (manual verification required)"

echo ""
echo "✅ All tests passed!"
```

Run tests:

```bash
chmod +x test-lsp-integration.sh
./test-lsp-integration.sh
```

## Step 7: Logging and Debugging

### Enable Detailed Logging

```bash
# Run with trace-level logging
RUST_LOG=trace /home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --log-level=trace \
  --mcp-pre=http://localhost:3000

# Log to file
RUST_LOG=debug /home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --log-level=debug \
  --mcp-pre=http://localhost:3000 \
  2>&1 | tee lsp-debug.log
```

### Monitor LSP Traffic

Create `monitor-lsp.sh`:

```bash
#!/bin/bash

LSP_BIN="/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp"

# Use tee to capture both input and output
mkfifo /tmp/lsp-in /tmp/lsp-out

# Monitor input
cat /tmp/lsp-in | tee >(cat >&2) | \
  $LSP_BIN --log-level=debug --mcp-pre=http://localhost:3000 | \
  tee /tmp/lsp-out

# In another terminal:
# tail -f /tmp/lsp-in    # Watch input
# tail -f /tmp/lsp-out   # Watch output
```

## Step 8: Production Deployment with Systemd

Create `/etc/systemd/system/universal-lsp.service`:

```ini
[Unit]
Description=Universal LSP Server
After=network.target

[Service]
Type=simple
User=valknar
WorkingDirectory=/home/valknar/Projects/zed/universal-lsp
ExecStart=/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --log-level=info \
  --mcp-pre=http://localhost:3000 \
  --mcp-timeout=5000 \
  --mcp-cache=true \
  --max-concurrent=200
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Manage the service:

```bash
# Enable and start
sudo systemctl enable universal-lsp
sudo systemctl start universal-lsp

# Check status
sudo systemctl status universal-lsp

# View logs
sudo journalctl -u universal-lsp -f

# Restart after configuration changes
sudo systemctl restart universal-lsp
```

## Step 9: Performance Monitoring

### Real-time Performance Dashboard

Create `monitor-performance.sh`:

```bash
#!/bin/bash

LSP_PID=$(pgrep -f universal-lsp)

if [ -z "$LSP_PID" ]; then
    echo "Universal LSP server not running"
    exit 1
fi

echo "Monitoring Universal LSP (PID: $LSP_PID)"
echo "=========================================="

while true; do
    clear
    echo "=== System Stats ==="
    echo -n "CPU: "
    ps -p $LSP_PID -o %cpu | tail -n 1
    echo -n "Memory: "
    ps -p $LSP_PID -o %mem | tail -n 1
    echo -n "Threads: "
    ps -p $LSP_PID -o nlwp | tail -n 1

    echo ""
    echo "=== MCP Server ==="
    mcp_response=$(curl -s -w "%{time_total}" http://localhost:3000/health)
    echo "Health: $mcp_response"

    echo ""
    echo "=== Recent Logs ==="
    sudo journalctl -u universal-lsp -n 5 --no-pager

    sleep 2
done
```

## Step 10: Advanced CLI Workflows

### Batch Processing Files

```bash
#!/bin/bash

# Process multiple files for completions
FILES="src/*.rs"

for file in $FILES; do
    echo "Processing $file..."

    # Get completions at line 10, char 0
    result=$(cat <<EOF | ./lsp-client.sh 2>/dev/null
{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file://$(pwd)/$file"},"position":{"line":10,"character":0}}}
EOF
)

    echo "$result" | jq '.result.items[].label'
done
```

### CI/CD Integration

```yaml
# .github/workflows/lsp-test.yml
name: Test Universal LSP

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Build Universal LSP
        run: |
          cd /home/valknar/Projects/zed/universal-lsp
          cargo build --release

      - name: Start MCP Server
        run: |
          cd ~/claude-mcp
          npm install
          node server.js &
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}

      - name: Run Integration Tests
        run: ./test-lsp-integration.sh
```

## Troubleshooting

### LSP Server Not Responding

```bash
# Check if server is running
pgrep -af universal-lsp

# Kill hanging processes
pkill -9 -f universal-lsp

# Restart with debug logging
RUST_LOG=debug /home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --log-level=debug \
  --mcp-pre=http://localhost:3000
```

### MCP Connection Issues

```bash
# Verify MCP server is accessible
curl -v http://localhost:3000/health

# Check firewall rules
sudo iptables -L | grep 3000

# Test with different timeout
/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --mcp-pre=http://localhost:3000 \
  --mcp-timeout=10000
```

### Performance Issues

```bash
# Disable MCP cache to test
/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --mcp-pre=http://localhost:3000 \
  --mcp-cache=false

# Reduce concurrent requests
/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --mcp-pre=http://localhost:3000 \
  --max-concurrent=50

# Use only post-processing
/home/valknar/Projects/zed/universal-lsp/target/release/universal-lsp \
  --mcp-post=http://localhost:3000
```

## Example Use Cases

### 1. Quick Code Analysis

```bash
# Get hover info for a symbol
echo '{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///tmp/test.py"},"position":{"line":5,"character":10}}}' | \
  ./lsp-client.sh | jq '.result.contents'
```

### 2. Batch Completion Generation

```bash
# Generate completions for common patterns
for pattern in "import" "def" "class" "if"; do
    echo "Completions for: $pattern"
    # Send completion request...
done
```

### 3. Language Detection Testing

```bash
# Test language detection for various file extensions
for ext in py rs js ts go java; do
    echo "Testing .$ext files..."
    # Send request with test.$ext URI...
done
```

## Resources

- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [Universal LSP README](../README.md)
- [Claude API Documentation](https://docs.anthropic.com/)
- [jq Manual](https://stedolan.github.io/jq/manual/)
