#!/usr/bin/env bash
SAMPLE_NAME="plugin/anthropics/knowledge-work-plugins"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../test/lib.sh"

assert_file ".claude/settings.json"
assert_file_contains ".claude/settings.json" '"productivity@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"sales@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"customer-support@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"product-management@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"marketing@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"legal@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"finance@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"data@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"enterprise-search@knowledge-work-plugins": true'
assert_file_contains ".claude/settings.json" '"cowork-plugin-management@knowledge-work-plugins": true'

assert_summary
