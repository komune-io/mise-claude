#!/usr/bin/env bash
set -uo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0
FAILURES=()

# ─── Build claude-env from source ─────────────────────────────────────────────
# claude-env is not yet published to crates.io, so we build from the repo.

echo -e "${YELLOW}Building claude-env...${NC}"
if ! cargo build --manifest-path /app/claude-env/Cargo.toml --release --quiet 2>&1; then
  echo -e "${RED}Failed to build claude-env — cannot run integration tests${NC}"
  exit 1
fi
CLAUDE_ENV=/app/claude-env/target/release/claude-env
echo -e "${GREEN}claude-env ready${NC}"
echo ""

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

  # Copy sample to a temp dir to avoid polluting the repo
  tmpdir=$(mktemp -d)

  # Strip the [tools] section entirely — claude-env handles tool installs directly,
  # and claude-code is pre-installed globally in the Docker image.
  awk '
    /^\[tools\]/ { in_tools=1; next }
    /^\[/         { in_tools=0; print; next }
    in_tools      { next }
    { print }
  ' "$sample_dir/.mise.toml" > "$tmpdir/.mise.toml"

  # Copy claude-env.toml (the new source of truth for Claude tools)
  if [ -f "$sample_dir/claude-env.toml" ]; then
    cp "$sample_dir/claude-env.toml" "$tmpdir/claude-env.toml"
  fi

  # Copy test script and assertion helpers
  cp /app/test/lib.sh "$tmpdir/lib.sh"
  if [ -f "$sample_dir/test.sh" ]; then
    # Rewrite source path to use local lib.sh copy in tmpdir
    sed 's|^SCRIPT_DIR=.*|# (rewritten for integration test)|;s|^source .*lib\.sh.*|source lib.sh|' \
      "$sample_dir/test.sh" > "$tmpdir/test.sh"
  fi

  cd "$tmpdir"

  if ! mise trust . 2>&1; then
    FAIL=$((FAIL + 1))
    FAILURES+=("$name: mise trust failed")
    echo -e "  ${RED}FAIL${NC} ${name} — mise trust failed"
    rm -rf "$tmpdir"
    cd /app
    continue
  fi

  # Install Claude tools via claude-env (reads claude-env.toml)
  if [ -f "claude-env.toml" ]; then
    if ! "$CLAUDE_ENV" install 2>&1; then
      FAIL=$((FAIL + 1))
      FAILURES+=("$name: claude-env install failed")
      echo -e "  ${RED}FAIL${NC} ${name} — claude-env install failed"
      rm -rf "$tmpdir"
      cd /app
      continue
    fi
  fi

  # Run the sample's test task
  output=$(mise run test 2>&1)
  rc=$?
  echo "$output"

  if [ "$rc" -eq 0 ]; then
    PASS=$((PASS + 1))
    echo -e "  ${GREEN}PASS${NC} ${name}"
  else
    FAIL=$((FAIL + 1))
    FAILURES+=("$name")
    echo -e "  ${RED}FAIL${NC} ${name}"
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
