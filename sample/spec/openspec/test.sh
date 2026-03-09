#!/usr/bin/env bash
SAMPLE_NAME="spec/openspec"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../test/lib.sh"

assert_dir ".claude/commands/opsx"
assert_file_count "./.claude/commands/opsx/*.md" 1
assert_file_count "./.claude/skills/openspec-*/SKILL.md" 1
assert_dir "openspec"
assert_dir "openspec/specs"
assert_dir "openspec/changes"

assert_summary
