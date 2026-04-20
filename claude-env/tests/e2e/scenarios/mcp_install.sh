#!/bin/bash
set -euo pipefail

echo "=== E2E: MCP Install ==="

cd /tmp
mkdir -p mcp-test && cd mcp-test

cat > claude-env.toml <<'EOF'
[mcp]
context7 = "latest"
EOF

claude-env install

if [[ ! -f .mcp.json ]]; then
    echo "FAIL: .mcp.json not created"
    exit 1
fi

if ! grep -q "context7" .mcp.json; then
    echo "FAIL: context7 not in .mcp.json"
    exit 1
fi

echo "PASS: MCP install"
