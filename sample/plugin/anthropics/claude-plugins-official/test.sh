#!/usr/bin/env bash
SAMPLE_NAME="plugin/anthropics/claude-plugins-official"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../test/lib.sh"

assert_file ".claude/settings.json"
assert_file_contains ".claude/settings.json" '"agent-sdk-dev@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"clangd-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"claude-code-setup@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"claude-md-management@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"code-review@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"code-simplifier@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"commit-commands@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"csharp-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"explanatory-output-style@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"feature-dev@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"frontend-design@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"gopls-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"hookify@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"jdtls-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"kotlin-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"learning-output-style@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"lua-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"php-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"playground@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"plugin-dev@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"pyright-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"ralph-loop@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"rust-analyzer-lsp@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"security-guidance@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"skill-creator@claude-plugins-official": true'
assert_file_contains ".claude/settings.json" '"typescript-lsp@claude-plugins-official": true'

assert_summary
