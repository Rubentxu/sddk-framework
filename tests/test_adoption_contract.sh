#!/usr/bin/env bash
# test_adoption_contract.sh — Verify adoption contract for SDDK hotfix v3.6
#
# Self-contained Bash test: no network, uses grep/awk/shell assertions.
# Must FAIL before the fix and PASS after.
#
# Usage:
#   bash tests/test_adoption_contract.sh

set -euo pipefail

# Derive paths relative to script location
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SHARED_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$SHARED_DIR"

FAIL_COUNT=0
PASS_COUNT=0

fail() {
    echo "FAIL: $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

pass() {
    echo "PASS: $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

info() {
    echo "INFO: $1"
}

echo "=== SDDK Adoption Contract Test (v3.6) ==="
echo ""

# ---------------------------------------------------------------------------
# Test 1: bootstrap.sh syntax check
# ---------------------------------------------------------------------------
info "Test 1: bootstrap.sh syntax check"
if bash -n bootstrap.sh 2>/dev/null; then
    pass "bootstrap.sh has valid syntax"
else
    fail "bootstrap.sh has syntax errors"
fi

# ---------------------------------------------------------------------------
# Test 2: test_adoption_contract.sh syntax check
# ---------------------------------------------------------------------------
info "Test 2: test_adoption_contract.sh syntax check"
if bash -n tests/test_adoption_contract.sh 2>/dev/null; then
    pass "test_adoption_contract.sh has valid syntax"
else
    fail "test_adoption_contract.sh has syntax errors"
fi

# ---------------------------------------------------------------------------
# Test 3: sddk-adopt.md does NOT have literal ~ inside bash snippets
#    The bug: cat >> "$GITIGNORE" << 'EOF' ... ~/.sddk-knowledge/{project}/ ...
#    Tilde inside heredoc is literal, not expanded. Must use $HOME.
# ---------------------------------------------------------------------------
info "Test 3: No literal tilde inside heredocs in sddk-adopt.md"
ADOPT_FILE="agents/sddk-adopt.md"
# Look for heredoc content that contains ~ (not $HOME) — tilde is only
# expanded when NOT inside single-quoted heredoc delimiter
if awk '/<<[ ]*'\''EOF'\''/,/^EOF$/' "$ADOPT_FILE" 2>/dev/null | grep -n '~/' | grep -v '\$HOME' | grep -v '^#' > /dev/null 2>&1; then
    fail "sddk-adopt.md contains literal tilde (~) inside heredoc — use \$HOME instead"
else
    pass "No literal tilde found inside heredocs"
fi

# ---------------------------------------------------------------------------
# Test 4: No {project_path} or {PROJECT} literal inside bash code blocks
#    The bug: {project_path} was used as literal string in bash snippets,
#    but it should be $project_path (bash variable).
# ---------------------------------------------------------------------------
info "Test 4: No {project_path} literal in bash code blocks"
# Extract bash blocks and check for {project_path} as literal (not $project_path)
BASH_BLOCKS=$(awk '/^```bash/,/^```$/' "$ADOPT_FILE" 2>/dev/null)
if echo "$BASH_BLOCKS" | grep -n '{project_path}' > /dev/null 2>&1; then
    fail "sddk-adopt.md uses literal {project_path} in bash block — use \$project_path"
else
    pass "No literal {project_path} in bash code blocks"
fi

# ---------------------------------------------------------------------------
# Test 5: adoption.json marker uses atomic mv pattern
# ---------------------------------------------------------------------------
info "Test 5: adoption.json marker uses atomic mv"
if grep -A5 'ADOPTION_TMP' "$ADOPT_FILE" 2>/dev/null | grep -q 'mv.*ADOPTION_JSON'; then
    pass "adoption.json uses atomic mv pattern"
else
    fail "adoption.json does not use atomic mv pattern"
fi

# ---------------------------------------------------------------------------
# Test 6: Test pipeline uses pipefail
# ---------------------------------------------------------------------------
info "Test 6: Test pipeline uses pipefail"
if grep -n 'set -o pipefail' "$ADOPT_FILE" 2>/dev/null | grep -q 'set -o pipefail'; then
    pass "pipefail is set in test pipeline"
else
    fail "pipefail not found in test pipeline"
fi

# ---------------------------------------------------------------------------
# Test 7: No duplicate YAML keys per YAML block
#    The bug: type: was duplicated (type: cycle + type: adoption)
#    Check each yaml block separately, not the whole document
# ---------------------------------------------------------------------------
info "Test 7: No duplicate top-level YAML keys per block"
if DUP_KEYS=$(awk '
    /^```yaml$/ { in_yaml=1; block++; next }
    in_yaml && /^```$/ { delete seen; in_yaml=0; next }
    in_yaml && /^[A-Za-z_][A-Za-z0-9_-]*:/ {
        key=$1
        sub(/:.*/, "", key)
        if (seen[key]++) print "block " block ": " key
    }
' "$ADOPT_FILE") && [ -n "$DUP_KEYS" ]; then
    fail "Duplicate top-level YAML keys found: $DUP_KEYS"
else
    pass "No duplicate YAML keys in adoption report block"
fi

# ---------------------------------------------------------------------------
# Test 8: No .gitkeep in gitignored paths
#    The bug: .gitkeep was used in sddk/, openspec/changes/, .atl/
#    which are gitignored — wasteful and unnecessary
# ---------------------------------------------------------------------------
info "Test 8: No .gitkeep in gitignored paths"
if grep -n '\.gitkeep' "$ADOPT_FILE" 2>/dev/null | grep -E '(sddk/openspec/changes/|\.atl/)' | grep -v '^\s*#' > /dev/null 2>&1; then
    fail "sddk-adopt.md still uses .gitkeep in gitignored paths"
else
    pass "No .gitkeep found in gitignored paths"
fi

# ---------------------------------------------------------------------------
# Test 9: bootstrap.sh links OpenCode agents
# ---------------------------------------------------------------------------
info "Test 9: bootstrap.sh links OpenCode agents"
if grep -q 'OPENCODE_DIR/agents' bootstrap.sh && grep -q 'ln -sf "$f" "$target"' bootstrap.sh; then
    pass "bootstrap.sh links agents to OpenCode"
else
    fail "bootstrap.sh does not link agents to OpenCode"
fi

# ---------------------------------------------------------------------------
# Test 10: bootstrap.sh links BOOK-*.md files
# ---------------------------------------------------------------------------
info "Test 10: bootstrap.sh links BOOK-*.md files"
if grep -q 'skills/BOOK-\*\.md' bootstrap.sh && grep -q 'ln -sf "$f" "$target"' bootstrap.sh; then
    pass "bootstrap.sh links BOOK-*.md files"
else
    fail "bootstrap.sh does not link BOOK-*.md files"
fi

# ---------------------------------------------------------------------------
# Test 11: .atl/ is in framework .gitignore
# ---------------------------------------------------------------------------
info "Test 11: .atl/ in framework .gitignore"
if grep -q '\.atl/' .gitignore 2>/dev/null; then
    pass ".atl/ is in framework .gitignore"
else
    fail ".atl/ not found in framework .gitignore"
fi

# ---------------------------------------------------------------------------
# Test 12: .atl/ is in sddk.gitignore.template
# ---------------------------------------------------------------------------
info "Test 12: .atl/ in sddk.gitignore.template"
if grep -q '\.atl/' prompts/sdd-kernel/templates/sddk.gitignore.template 2>/dev/null; then
    pass ".atl/ is in sddk.gitignore.template"
else
    fail ".atl/ not found in sddk.gitignore.template"
fi

# ---------------------------------------------------------------------------
# Test 13: .atl/ is re-included in dotignore for local readability
# ---------------------------------------------------------------------------
info "Test 13: .atl/ is re-included in dotignore template"
if grep -q '!\.atl/' prompts/sdd-kernel/templates/sddk.dotignore.template 2>/dev/null; then
    pass ".atl/ is re-included in dotignore template"
else
    fail ".atl/ not found in dotignore template"
fi

# ---------------------------------------------------------------------------
# Test 14: Step numbering is sequential without duplicates
# ---------------------------------------------------------------------------
info "Test 14: No duplicate step numbers in sddk-adopt.md"
STEPS=$(grep -n '^### [0-9]' "$ADOPT_FILE" 2>/dev/null | awk '{print $2}' | sort -n)
DUP=$(echo "$STEPS" | uniq -d)
if [ -n "$DUP" ]; then
    fail "Duplicate step numbers found: $DUP"
else
    pass "No duplicate step numbers found"
fi

# ---------------------------------------------------------------------------
# Test 15: Vault parent directory created before copy
# ---------------------------------------------------------------------------
info "Test 15: Vault parent created before copy"
if grep -n 'mkdir -p.*dirname.*VAULT' "$ADOPT_FILE" 2>/dev/null | grep -q 'mkdir -p "$(dirname'; then
    pass "Vault parent directory is created before copy"
else
    fail "Vault parent directory mkdir not found before copy"
fi

# ---------------------------------------------------------------------------
# Test 16: sddk-kernel-init.md checks adoption.json (not just directory)
# ---------------------------------------------------------------------------
info "Test 16: sddk-kernel-init.md checks adoption.json"
INIT_FILE="agents/sdd-kernel-init.md"
if grep -q 'adoption\.json' "$INIT_FILE" 2>/dev/null; then
    pass "sddk-kernel-init.md references adoption.json"
else
    fail "sddk-kernel-init.md does not check adoption.json"
fi

# ---------------------------------------------------------------------------
# Test 17: No ADOPTED variable in sddk-kernel-init.md
#    The bug: ADOPTED was used as a variable that wasn't properly defined
# ---------------------------------------------------------------------------
info "Test 17: No ADOPTED variable in sddk-kernel-init.md"
if grep -n '^ADOPTED=' "$INIT_FILE" 2>/dev/null | grep -v '^#' > /dev/null 2>&1; then
    fail "sddk-kernel-init.md still uses ADOPTED variable"
else
    pass "No ADOPTED variable references found"
fi

# ---------------------------------------------------------------------------
# Test 18: bootstrap.sh vault message uses correct path (outside repo, in HOME)
# ---------------------------------------------------------------------------
info "Test 18: bootstrap.sh vault message correct"
if grep -n 'sddk-knowledge.*inside repo' bootstrap.sh 2>/dev/null | grep -v '^#' > /dev/null 2>&1; then
    fail "bootstrap.sh vault message incorrectly says 'inside repo'"
else
    pass "bootstrap.sh vault message is correct"
fi

# ---------------------------------------------------------------------------
# Test 19: bootstrap.sh next steps mention adoption (not just init/new)
# ---------------------------------------------------------------------------
info "Test 19: bootstrap.sh next steps mention /sddk-adopt"
if grep -A3 'Next steps' bootstrap.sh 2>/dev/null | grep -q 'sddk-adopt'; then
    pass "bootstrap.sh next steps mention /sddk-adopt"
else
    fail "bootstrap.sh next steps do not mention /sddk-adopt"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
info "Test 20: Adopted projects repair policy before stopping"
EARLY_GUARD_LINE=$(grep -n -m1 'already_adopted=true' "$ADOPT_FILE" | cut -d: -f1 || true)
STEP_ONE_LINE=$(grep -n -m1 '^### 1\.' "$ADOPT_FILE" | cut -d: -f1 || true)
EARLY_GUARD_BLOCK=$(awk '/if \[ -f "\$ADOPTION_JSON" \]/,/^fi$/' "$ADOPT_FILE")
if [ -n "$EARLY_GUARD_LINE" ] && [ -n "$STEP_ONE_LINE" ] \
    && [ "$EARLY_GUARD_LINE" -lt "$STEP_ONE_LINE" ] \
    && printf '%s\n' "$EARLY_GUARD_BLOCK" | grep -q 'repair_local_ignore_policy'; then
    pass "Valid adoption marker repairs policy before early exit"
else
    fail "Early adoption exit can skip incremental policy repair"
fi

info "Test 21: Installed ignore blocks include .atl and merge existing .ignore"
if grep -q "'.atl/'" "$ADOPT_FILE" \
    && grep -q "'!.atl/'" "$ADOPT_FILE" \
    && grep -q 'cat >> "$IGNORE_FILE"' "$ADOPT_FILE"; then
    pass "Adoption installs .atl ignore rules idempotently"
else
    fail "Adoption does not install/merge .atl ignore rules"
fi

info "Test 22: Adoption upgrades older managed ignore blocks"
if grep -q "for pattern in 'sddk/' 'openspec/changes/' '.atl/'" "$ADOPT_FILE" \
    && grep -q 'ensure_line "$GITIGNORE" "$pattern"' "$ADOPT_FILE"; then
    pass "Older adoption blocks are upgraded pattern by pattern"
else
    fail "Adoption skips missing patterns when an older managed block exists"
fi

echo ""
echo "=== Results ==="
echo "Passed: $PASS_COUNT"
echo "Failed: $FAIL_COUNT"
echo ""

if [ "$FAIL_COUNT" -gt 0 ]; then
    echo "Some tests failed. Fix required before merge."
    exit 1
else
    echo "All tests passed!"
    exit 0
fi
