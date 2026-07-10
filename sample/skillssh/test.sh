#!/usr/bin/env bash
SAMPLE_NAME="skillssh"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../test/lib.sh"

# chord materializes each skill into its own .chord/<owner>/<repo>/<name>/ store
# (real files) and symlinks .claude/skills/<owner>__<repo>__<name> at it. find
# -path does not traverse symlinks, so we count the store (scoped to the
# owner/repo to skip the .chord/.cache git checkouts).
assert_file_count "./.chord/vercel-labs/agent-skills/*/SKILL.md" 1

# The Claude-facing symlink (namespaced, one level deep so Claude Code discovers
# it) must resolve to a real SKILL.md (-f follows symlinks).
assert_file "./.claude/skills/vercel-labs__agent-skills__web-design-guidelines/SKILL.md"

# chord.lock is the single source of truth (skills-lock.json is no longer written).
assert_file "chord.lock"

assert_summary
