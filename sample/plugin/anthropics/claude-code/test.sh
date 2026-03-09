#!/usr/bin/env bash
SAMPLE_NAME="plugin/anthropics/claude-code"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../test/lib.sh"

assert_file ".claude/settings.json"
assert_file_contains ".claude/settings.json" '"agent-sdk-dev@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"claude-opus-4-5-migration@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"code-review@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"commit-commands@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"explanatory-output-style@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"feature-dev@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"frontend-design@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"hookify@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"learning-output-style@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"plugin-dev@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"pr-review-toolkit@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"ralph-wiggum@claude-code-plugins": true'
assert_file_contains ".claude/settings.json" '"security-guidance@claude-code-plugins": true'

assert_summary
