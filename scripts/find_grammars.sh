#!/bin/bash
# Script to find and categorize tree-sitter grammars from crates.io

echo "Searching for tree-sitter grammars..."
cargo search tree-sitter- --limit 300 2>/dev/null | grep "^tree-sitter-" | sort > /tmp/all_ts_grammars.txt

echo "Found $(wc -l < /tmp/all_ts_grammars.txt) grammars"
echo ""
echo "Sample of available grammars:"
head -50 /tmp/all_ts_grammars.txt
