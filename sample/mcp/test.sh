#!/usr/bin/env bash
SAMPLE_NAME="mcp"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../test/lib.sh"

assert_file ".mcp.json"
assert_file_contains ".mcp.json" '"mcpServers"'
assert_file_contains ".mcp.json" '"context7-mcp"'
assert_file_contains ".mcp.json" '"chrome-devtools-mcp"'
assert_file_contains ".mcp.json" '"shadcn"'
assert_file_contains ".mcp.json" '"command"'
assert_file_contains ".mcp.json" '"args"'
assert_file_contains ".mcp.json" '"stdio"'

assert_summary
