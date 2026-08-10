#!/usr/bin/env bash
set -euo pipefail

ADOPT_FILE="agents/sddk-adopt.md"
PASS_COUNT=0
FAIL_COUNT=0

pass() { printf 'PASS: %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL: %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

check_contains() {
    local pattern="$1" message="$2"
    if grep -qF "$pattern" "$ADOPT_FILE"; then pass "$message"; else fail "$message"; fi
}

check_absent() {
    local pattern="$1" message="$2"
    if grep -qF "$pattern" "$ADOPT_FILE"; then fail "$message"; else pass "$message"; fi
}

printf '%s\n' '=== SDDK Adoption Contract ==='
bash -n "$0" && pass "contract test syntax"
bash -n bootstrap.sh && pass "bootstrap syntax"

check_contains 'sddk adopt status' 'adoption queries canonical status'
check_contains 'sddk adopt apply' 'adoption converges through the CLI'
check_contains 'sddk knowledge status' 'adoption queries the knowledge profile'
check_contains 'sddk knowledge path' 'adoption resolves the vault through the CLI'
check_contains 'Treat the project repository as read-only.' 'repository is read-only'
check_contains 'Engram is optional.' 'Engram remains optional'

check_absent 'PROJECT=$(basename' 'vault identity is not basename-derived'
check_absent 'GITIGNORE=' 'adoption does not mutate .gitignore'
check_absent 'IGNORE_FILE=' 'adoption does not mutate .ignore'
check_absent 'mkdir -p "$project_path"' 'adoption creates no repo-local state'
check_absent '.gitkeep' 'adoption creates no placeholder files'

printf '\nPassed: %s\nFailed: %s\n' "$PASS_COUNT" "$FAIL_COUNT"
if [ "$FAIL_COUNT" -ne 0 ]; then
    exit 1
fi
