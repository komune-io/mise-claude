#!/usr/bin/env bash
SAMPLE_NAME="plugin/context7"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../test/lib.sh"

assert_file ".claude/settings.json"
assert_file_contains ".claude/settings.json" 'context7-plugin'

assert_summary
