#!/usr/bin/env bash
SAMPLE_NAME="spec/bmad"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../test/lib.sh"

assert_file_count "./.claude/commands/bmad-*.md" 1
assert_dir "_bmad"
assert_dir "_bmad/bmm"
assert_dir "_bmad/core"

assert_summary
