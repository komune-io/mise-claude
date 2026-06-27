#!/usr/bin/env bash
SAMPLE_NAME="plugin/anthropics/financial-services-plugins"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../test/lib.sh"

assert_file ".claude/settings.json"
assert_file_contains ".claude/settings.json" '"financial-analysis@claude-for-financial-services": true'
assert_file_contains ".claude/settings.json" '"investment-banking@claude-for-financial-services": true'
assert_file_contains ".claude/settings.json" '"equity-research@claude-for-financial-services": true'
assert_file_contains ".claude/settings.json" '"private-equity@claude-for-financial-services": true'
assert_file_contains ".claude/settings.json" '"wealth-management@claude-for-financial-services": true'

assert_summary
