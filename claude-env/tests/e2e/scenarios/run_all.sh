#!/bin/bash
set -euo pipefail

PASS=0
FAIL=0

for scenario in /scenarios/*.sh; do
    [[ "$(basename "$scenario")" == "run_all.sh" ]] && continue
    echo ""
    if bash "$scenario"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
