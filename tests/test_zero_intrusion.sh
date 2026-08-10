#!/usr/bin/env bash
set -euo pipefail

# Zero-intrusion + namespace contract (ADR-0011).
# Guards the executable surfaces (prompts/, agents/, skills/) against
# re-introducing repo-local state, basename-derived vault identity, or legacy
# namespace aliases (sdd-*, sdd-kernel-*, gentle-orchestrator).

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PASS_COUNT=0
FAIL_COUNT=0

pass() { printf 'PASS: %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL: %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

check_absent() {
    local pattern="$1" message="$2" path="$3"
    if grep -rnI --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules \
        --exclude=test_zero_intrusion.sh -- "$pattern" "$path" 2>/dev/null; then fail "$message"; else pass "$message"; fi
}

printf '%s\n' '=== SDDK Zero-Intrusion + Namespace Contract ==='
bash -n "$0" && pass "contract test syntax"

# 1. Legacy namespace aliases must not resurface on executable surfaces.
check_absent 'gentle-orchestrator' 'no gentle-orchestrator alias' "$ROOT/agents"
check_absent 'sdd-kernel-' 'no sdd-kernel-* aliases' "$ROOT/agents"
check_absent 'sdd-apply' 'no sdd-apply alias' "$ROOT/agents"
check_absent 'sdd-design' 'no sdd-design alias' "$ROOT/agents"
check_absent 'sdd-init' 'no sdd-init alias' "$ROOT/agents"
check_absent 'sdd-propose' 'no sdd-propose alias' "$ROOT/agents"
check_absent 'sdd-spec' 'no sdd-spec alias' "$ROOT/agents"
check_absent 'sdd-tasks' 'no sdd-tasks alias' "$ROOT/agents"
check_absent 'sdd-verify' 'no sdd-verify alias' "$ROOT/agents"
check_absent 'sdd-archive' 'no sdd-archive alias' "$ROOT/agents"

# 2. Vault identity must never be derived from a directory basename.
check_absent 'PROJECT=$(basename "$PROJECT_ROOT")' 'vault identity is not basename-derived' "$ROOT/agents"
check_absent 'PROJECT="$(basename "$PROJECT_ROOT")"' 'vault identity is not basename-derived' "$ROOT/prompts"
check_absent 'sddk-knowledge/$PROJECT' 'vault is not basename-derived' "$ROOT/agents"
check_absent 'sddk-knowledge/$PROJECT' 'vault is not basename-derived' "$ROOT/prompts"

# 3. The framework must never instruct planting repo-local state.
check_absent 'Plant .gitignore' 'no .gitignore planting' "$ROOT/prompts"
check_absent 'Plant .ignore' 'no .ignore planting' "$ROOT/prompts"
check_absent 'write the contents of .gitignore' 'no .gitignore template writing' "$ROOT/prompts"
check_absent 'write the contents of .ignore' 'no .ignore template writing' "$ROOT/prompts"
check_absent 'sddk.gitignore.template' 'no legacy gitignore template references' "$ROOT"
check_absent 'sddk.dotignore.template' 'no legacy dotignore template references' "$ROOT"
check_absent 'Build .atl/skill-registry.md' 'no repo-local .atl registry' "$ROOT/prompts"
check_absent '.atl/skill-registry.md' 'no repo-local .atl registry' "$ROOT/agents"

# 4. Legacy XDG pre-fix paths must not resurface.
check_absent '~/.sddk/projects' 'no legacy ~/.sddk path' "$ROOT/prompts"
check_absent '~/.sddk/projects' 'no legacy ~/.sddk path' "$ROOT/agents"
check_absent '~/.sddk/projects' 'no legacy ~/.sddk path' "$ROOT/skills"
check_absent 'opencode/sddk/metrics' 'no opencode-scoped metrics path' "$ROOT/prompts"
check_absent 'opencode/sddk/metrics' 'no opencode-scoped metrics path' "$ROOT/skills"

# 5. Repo-local cycle artifact paths must not resurface.
check_absent 'checkpoint: sddk/' 'no repo-local checkpoint path' "$ROOT/prompts"
check_absent 'sddk/{change}/apply-checkpoint' 'no repo-local apply checkpoint' "$ROOT/prompts"
check_absent 'sddk/my-change' 'no repo-local artifact references' "$ROOT/prompts"
check_absent 'sddk/{next_change}/tuning.md' 'no repo-local tuning path' "$ROOT/prompts"

# 6. Templates directory must not resurrect obsolete ignore templates.
if [ -f "$ROOT/prompts/sddk/templates/sddk.gitignore.template" ] ||
   [ -f "$ROOT/prompts/sddk/templates/sddk.dotignore.template" ]; then
    fail "obsolete ignore templates were resurrected"
else
    pass "obsolete ignore templates are gone"
fi

printf '\nPassed: %s\nFailed: %s\n' "$PASS_COUNT" "$FAIL_COUNT"
if [ "$FAIL_COUNT" -ne 0 ]; then
    exit 1
fi
