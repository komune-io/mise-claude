#!/usr/bin/env bash
set -uo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASS=0
FAIL=0
SKIP=0
FAILURES=()

pass() {
  echo -e "  ${GREEN}PASS${NC} $1"
  PASS=$((PASS + 1))
}

fail() {
  echo -e "  ${RED}FAIL${NC} $1 — $2"
  FAIL=$((FAIL + 1))
  FAILURES+=("$1: $2")
}

skip() {
  echo -e "  ${BLUE}SKIP${NC} $1 — $2"
  SKIP=$((SKIP + 1))
}

# ─── Assertion helpers ────────────────────────────────────────────────────────

CURRENT_SAMPLE=""
SAMPLE_OK=true

assert_dir() {
  local path="$1" label="$2"
  if [ -d "$path" ]; then
    pass "${CURRENT_SAMPLE}: ${label}"
  else
    fail "${CURRENT_SAMPLE}" "${label} — directory not found: ${path}"
    SAMPLE_OK=false
  fi
}

assert_file() {
  local path="$1" label="$2"
  if [ -f "$path" ]; then
    pass "${CURRENT_SAMPLE}: ${label}"
  else
    fail "${CURRENT_SAMPLE}" "${label} — file not found: ${path}"
    SAMPLE_OK=false
  fi
}

assert_file_contains() {
  local path="$1" pattern="$2" label="$3"
  if [ -f "$path" ] && grep -q "$pattern" "$path" 2>/dev/null; then
    pass "${CURRENT_SAMPLE}: ${label}"
  else
    fail "${CURRENT_SAMPLE}" "${label} — pattern '${pattern}' not found in ${path}"
    SAMPLE_OK=false
  fi
}

assert_file_count() {
  local glob_pattern="$1" min="$2" label="$3"
  local count
  count=$(find . -path "$glob_pattern" 2>/dev/null | wc -l | tr -d ' ')
  if [ "$count" -ge "$min" ]; then
    pass "${CURRENT_SAMPLE}: ${label} (found ${count})"
  else
    fail "${CURRENT_SAMPLE}" "${label} — expected at least ${min}, found ${count}"
    SAMPLE_OK=false
  fi
}

# ─── Per-sample assertion functions ───────────────────────────────────────────

assert_mcp() {
  assert_file ".mcp.json" ".mcp.json exists"
  assert_file_contains ".mcp.json" '"mcpServers"' '.mcp.json contains mcpServers'

  for server in context7-mcp chrome-devtools-mcp shadcn; do
    assert_file_contains ".mcp.json" "\"${server}\"" ".mcp.json has ${server} entry"
  done

  assert_file_contains ".mcp.json" '"command"' '.mcp.json entries have command'
  assert_file_contains ".mcp.json" '"args"' '.mcp.json entries have args'
  assert_file_contains ".mcp.json" '"stdio"' '.mcp.json entries have type stdio'
}

assert_spec_gsd() {
  assert_dir ".claude/commands/gsd" ".claude/commands/gsd/ exists"
  assert_file_count "./.claude/commands/gsd/*.md" 1 "gsd command .md files"

  assert_dir ".claude/agents" ".claude/agents/ exists"
  assert_file_count "./.claude/agents/gsd-*.md" 1 "gsd agent .md files"

  assert_dir ".claude/get-shit-done" ".claude/get-shit-done/ exists"
  assert_file ".claude/settings.json" ".claude/settings.json exists"
}

assert_spec_bmad() {
  assert_file_count "./.claude/commands/bmad-*.md" 1 "bmad command .md files"

  assert_dir "_bmad" "_bmad/ exists"
  assert_dir "_bmad/bmm" "_bmad/bmm/ exists"
  assert_dir "_bmad/core" "_bmad/core/ exists"
}

assert_spec_openspec() {
  assert_dir ".claude/commands/opsx" ".claude/commands/opsx/ exists"
  assert_file_count "./.claude/commands/opsx/*.md" 1 "opsx command .md files"

  assert_file_count "./.claude/skills/openspec-*/SKILL.md" 1 "openspec skill SKILL.md files"

  assert_dir "openspec" "openspec/ exists"
  assert_dir "openspec/specs" "openspec/specs/ exists"
  assert_dir "openspec/changes" "openspec/changes/ exists"
}

assert_skillssh() {
  assert_dir ".agents/skills" ".agents/skills/ exists"
  assert_file_count "./.agents/skills/*/SKILL.md" 1 "SKILL.md files in .agents/skills/"
  assert_file "skills-lock.json" "skills-lock.json exists"
}

# ─── Main ─────────────────────────────────────────────────────────────────────

# Link the plugin once
mise plugin link claude /app

# Find all sample directories with .mise.toml
SAMPLES=$(find /app/sample -name '.mise.toml' -print0 | xargs -0 -n1 dirname | sort)

echo -e "${YELLOW}Running integration tests...${NC}"
echo ""

for sample_dir in $SAMPLES; do
  # Derive a readable name from the path (e.g. "mcp", "spec/gsd", "plugin/anthropics/claude-code")
  name="${sample_dir#/app/sample/}"
  echo -e "${YELLOW}Testing${NC} ${name}"

  # Determine tool type from first path component
  type="${name%%/*}"

  # Plugin tests require claude CLI authentication — skip in CI
  if [ "$type" = "plugin" ]; then
    skip "$name" "requires claude CLI auth"
    continue
  fi

  # Copy sample to a temp dir to avoid polluting the repo
  tmpdir=$(mktemp -d)

  # Extract only claude: tools from the sample's .mise.toml to avoid installing
  # unrelated tools (java, gradle, etc.) that would slow down tests
  echo "[tools]" > "$tmpdir/.mise.toml"
  grep '^"claude:' "$sample_dir/.mise.toml" >> "$tmpdir/.mise.toml"

  cd "$tmpdir"
  CURRENT_SAMPLE="$name"
  SAMPLE_OK=true

  # Trust and install
  if mise trust . && mise install 2>&1; then
    case "$name" in
      mcp)          assert_mcp ;;
      spec/gsd)     assert_spec_gsd ;;
      spec/bmad)    assert_spec_bmad ;;
      spec/openspec) assert_spec_openspec ;;
      skillssh)     assert_skillssh ;;
      *)            pass "$name" ;;
    esac
  else
    fail "$name" "mise install failed"
  fi

  # Cleanup
  rm -rf "$tmpdir"
  cd /app
done

# Summary
echo ""
echo -e "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "  ${GREEN}Passed${NC}: $PASS"
echo -e "  ${RED}Failed${NC}: $FAIL"
echo -e "  ${BLUE}Skipped${NC}: $SKIP"

if [ "$FAIL" -gt 0 ]; then
  echo ""
  echo -e "${RED}Failures:${NC}"
  for f in "${FAILURES[@]}"; do
    echo -e "  - $f"
  done
  exit 1
fi

echo ""
echo -e "${GREEN}All tests passed!${NC}"
