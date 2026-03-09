#!/usr/bin/env bash
SAMPLE_NAME="skillssh"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../test/lib.sh"

assert_dir ".agents/skills"
assert_file_count "./.agents/skills/*/SKILL.md" 1
assert_file "skills-lock.json"

assert_summary
