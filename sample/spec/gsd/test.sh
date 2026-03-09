#!/usr/bin/env bash
SAMPLE_NAME="spec/gsd"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../test/lib.sh"

assert_dir ".claude/commands/gsd"
assert_file_count "./.claude/commands/gsd/*.md" 1
assert_dir ".claude/agents"
assert_file_count "./.claude/agents/gsd-*.md" 1
assert_dir ".claude/get-shit-done"
assert_file ".claude/settings.json"

assert_summary
