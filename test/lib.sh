#!/usr/bin/env bash
# Shared assertion helpers for sample tests.
# Sourced by each sample's test.sh — expects SAMPLE_NAME to be set.

set -uo pipefail

LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$LIB_DIR/.." 2>/dev/null && pwd || cd "$LIB_DIR" && pwd)"

_PASS=0
_FAIL=0

assert_dir() {
  local path="$1"
  if [ -d "$path" ]; then
    echo "  PASS ${SAMPLE_NAME}: ${path} exists"
    _PASS=$((_PASS + 1))
  else
    echo "  FAIL ${SAMPLE_NAME}: ${path} — directory not found"
    _FAIL=$((_FAIL + 1))
  fi
}

assert_file() {
  local path="$1"
  if [ -f "$path" ]; then
    echo "  PASS ${SAMPLE_NAME}: ${path} exists"
    _PASS=$((_PASS + 1))
  else
    echo "  FAIL ${SAMPLE_NAME}: ${path} — file not found"
    _FAIL=$((_FAIL + 1))
  fi
}

assert_file_contains() {
  local path="$1" pattern="$2"
  if [ -f "$path" ] && grep -q "$pattern" "$path" 2>/dev/null; then
    echo "  PASS ${SAMPLE_NAME}: ${path} contains ${pattern}"
    _PASS=$((_PASS + 1))
  else
    echo "  FAIL ${SAMPLE_NAME}: ${path} — pattern '${pattern}' not found"
    _FAIL=$((_FAIL + 1))
  fi
}

assert_file_count() {
  local glob_pattern="$1" min="$2"
  local count
  count=$(find . -path "$glob_pattern" 2>/dev/null | wc -l | tr -d ' ')
  if [ "$count" -ge "$min" ]; then
    echo "  PASS ${SAMPLE_NAME}: ${glob_pattern} (found ${count})"
    _PASS=$((_PASS + 1))
  else
    echo "  FAIL ${SAMPLE_NAME}: ${glob_pattern} — expected at least ${min}, found ${count}"
    _FAIL=$((_FAIL + 1))
  fi
}

# Call at end of test.sh to report and set exit code
assert_summary() {
  if [ "$_FAIL" -gt 0 ]; then
    echo "FAIL: ${_FAIL} assertion(s) failed"
    exit 1
  fi
  echo "PASS: all ${_PASS} assertions passed"
}
