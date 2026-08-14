#!/usr/bin/env bash
# tests/test_workflow_contract.sh — Deterministic regression tests for SDDK v3.6 hotfix
# Run: bash -n tests/test_workflow_contract.sh && bash tests/test_workflow_contract.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDDK_ROOT="${SDDK_ROOT:-$SCRIPT_DIR/..}"
PASS=0
FAIL=0

banner() {
  echo ""
  echo "=== $1"
  echo ""
}

inc_pass() {
  PASS=$((PASS + 1))
  echo "  [PASS] $1"
}

inc_fail() {
  FAIL=$((FAIL + 1))
  echo "  [FAIL] $1"
}

banner "REGRESSION 1: No ghost plugin references or runtime enforcement claims in SDDK core"

GHOST_PATTERNS=(
  "plugins/circuit-breaker\.ts"
  "plugins/git-boundary\.ts"
  "plugins/phase-telemetry\.ts"
)

SDDK_CORE_FILES=(
  "$SDDK_ROOT/agents/orchestrator.md"
  "$SDDK_ROOT/agents/sddk-release.md"
  "$SDDK_ROOT/agents/sddk-debt-verify.md"
  "$SDDK_ROOT/agents/sddk-apply.md"
  "$SDDK_ROOT/prompts/sddk/orchestrator.md"
  "$SDDK_ROOT/prompts/sddk/mcw.md"
  "$SDDK_ROOT/prompts/sddk/git-contract.md"
  "$SDDK_ROOT/prompts/sddk/phase-contracts.md"
  "$SDDK_ROOT/prompts/sddk/phases/apply.md"
  "$SDDK_ROOT/prompts/sddk/phases/release.md"
  "$SDDK_ROOT/prompts/sddk/phases/debt-verify.md"
  "$SDDK_ROOT/skills/sddk-release/SKILL.md"
  "$SDDK_ROOT/skills/sddk-debt-verify/SKILL.md"
)

for plugin in "${GHOST_PATTERNS[@]}"; do
  for file in "${SDDK_CORE_FILES[@]}"; do
    if [ -f "$file" ]; then
      if grep -qE "$plugin" "$file" 2>/dev/null; then
        inc_fail "Ghost plugin reference: $plugin in $(basename "$file")"
      else
        inc_pass "No ghost plugin: $plugin in $(basename "$file")"
      fi
    fi
  done
done

# Also reject git-boundary plugin enforcement claims
for file in "${SDDK_CORE_FILES[@]}"; do
  if [ -f "$file" ]; then
    if grep -qE "git-boundary|enforced by.*plugin" "$file" 2>/dev/null; then
      inc_fail "git-boundary plugin enforcement claim in $(basename "$file")"
    else
      inc_pass "No git-boundary plugin claim in $(basename "$file")"
    fi
  fi
done

banner "REGRESSION 2: Release authority is local Git, not PR checks or CI/CD"

RELEASE_FILES=(
  "$SDDK_ROOT/agents/sddk-release.md"
  "$SDDK_ROOT/skills/sddk-release/SKILL.md"
  "$SDDK_ROOT/prompts/sddk/phases/release.md"
)

for file in "${RELEASE_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")

    if grep -q '^gh pr checks\|^gh pr merge' "$file" 2>/dev/null; then
      inc_fail "$fname: executes a provider PR command"
    else
      inc_pass "$fname: has no executable provider PR command"
    fi

    if grep -q 'local verify -> push main -> verify HEAD' "$file" \
      && grep -qi 'annotated.*tag' "$file" \
      && grep -qi 'optional.*post-tag\|post-tag.*optional' "$file"; then
      inc_pass "$fname: local SHA and annotated tag are authoritative"
    else
      inc_fail "$fname: missing local release authority contract"
    fi
  fi
done

banner "REGRESSION 3: Consolidation gate uses --no-merged, not branch --list"

MCW_FILE="$SDDK_ROOT/prompts/sddk/mcw.md"
if [ -f "$MCW_FILE" ]; then
  if grep -qE "git branch -r --list.*origin/feat" "$MCW_FILE" 2>/dev/null; then
    inc_fail "mcw.md: uses legacy git branch --list (should be --no-merged)"
  else
    inc_pass "mcw.md: no legacy --list usage"
  fi

  if grep -qE "git branch -r --no-merged" "$MCW_FILE" 2>/dev/null; then
    inc_pass "mcw.md: uses --no-merged for consolidation"
  else
    inc_fail "mcw.md: missing --no-merged in consolidation gate"
  fi
fi

banner "REGRESSION 4: Debt-verify is mandatory on A-*, disabled on B-direct, no user opt-in, no refactor/debt- branches"

DEBT_FILES=(
  "$SDDK_ROOT/agents/sddk-debt-verify.md"
  "$SDDK_ROOT/skills/sddk-debt-verify/SKILL.md"
  "$SDDK_ROOT/prompts/sddk/phases/debt-verify.md"
  "$SDDK_ROOT/prompts/sddk/phase-contracts.md"
  "$SDDK_ROOT/prompts/sddk/git-contract.md"
  "$SDDK_ROOT/prompts/sddk/mcw.md"
  "$SDDK_ROOT/agents/orchestrator.md"
  "$SDDK_ROOT/prompts/sddk/orchestrator.md"
)

for file in "${DEBT_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")

    # Must NOT contain legacy opt-in language
    if grep -qiE "user opted in|opt-in trigger|debt_user_opted_in|asks the user after.*whether to run" "$file" 2>/dev/null; then
      inc_fail "$fname: contains legacy opt-in language"
    else
      inc_pass "$fname: no legacy opt-in language"
    fi

    # Must NOT use refactor/debt- separate branch pattern
    if grep -qiE "refactor/debt-|fix cycle.*branch|debt-fix.*branch" "$file" 2>/dev/null; then
      inc_fail "$fname: contains refactor/debt- separate branch pattern (should use remediation_round on same branch)"
    else
      inc_pass "$fname: no refactor/debt- separate branch"
    fi

    # Must NOT allow skipping debt-verify via reversibility
    if grep -qiE "policy: SKIP|skip debt-verify entirely|HIGH.*skip.*debt" "$file" 2>/dev/null; then
      inc_fail "$fname: allows skipping debt-verify via reversibility (not permitted)"
    else
      inc_pass "$fname: no debt skip via reversibility"
    fi

    # Must contain mandatory policy for A-*
    if grep -qE "MANDATORY.*A-\*|mandatory.*A-|A-\*.*mandatory" "$file" 2>/dev/null; then
      inc_pass "$fname: mandatory policy confirmed for A-*"
    else
      inc_fail "$fname: missing mandatory A-* policy"
    fi
  fi
done

banner "REGRESSION 5: Adoption guard uses proper Bash (no quoted tilde, no literal {project})"

ORCHESTRATOR_FILES=(
  "$SDDK_ROOT/agents/orchestrator.md"
  "$SDDK_ROOT/prompts/sddk/orchestrator.md"
)

for file in "${ORCHESTRATOR_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")

    GUARD_BLOCK=$(awk '/## SDD Init Guard \(MANDATORY\)/,/^## Execution Mode/' "$file")
    GUARD_BASH=$(printf '%s\n' "$GUARD_BLOCK" | awk '/^```bash$/,/^```$/')

    if printf '%s\n' "$GUARD_BASH" | grep -qE '"~|\{project\}'; then
      inc_fail "$fname: adoption guard Bash contains quoted tilde or literal placeholder"
    else
      inc_pass "$fname: adoption guard Bash uses resolved variables"
    fi

    if printf '%s\n' "$GUARD_BLOCK" | grep -qE 'adoption\.json'; then
      inc_pass "$fname: uses adoption.json marker"
    else
      inc_fail "$fname: missing adoption.json marker check"
    fi

    if printf '%s\n' "$GUARD_BASH" | grep -qE 'VAULT=.*HOME.*PROJECT|VAULT_PATH=.*sddk knowledge path'; then
      inc_pass "$fname: canonical vault resolution"
    else
      inc_fail "$fname: missing canonical vault resolution"
    fi
  fi
done

banner "REGRESSION 6: Git gates execute correctly"

TMP_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/sddk-git-gates.XXXXXX")
ORIGIN="$TMP_ROOT/origin.git"
WORK="$TMP_ROOT/work"
git init --bare "$ORIGIN" >/dev/null
git init -b main "$WORK" >/dev/null
git -C "$WORK" config user.name "SDDK Test"
git -C "$WORK" config user.email "sddk-test@example.invalid"
printf 'base\n' > "$WORK/state.txt"
git -C "$WORK" add state.txt
git -C "$WORK" commit -m "chore: base" >/dev/null
git -C "$WORK" remote add origin "$ORIGIN"
git -C "$WORK" push -u origin main >/dev/null
git -C "$WORK" checkout -b feat/open >/dev/null
printf 'feature\n' >> "$WORK/state.txt"
git -C "$WORK" commit -am "feat: open branch" >/dev/null
BRANCH_HEAD=$(git -C "$WORK" rev-parse HEAD)
git -C "$WORK" push -u origin feat/open >/dev/null
git -C "$WORK" checkout main >/dev/null
git -C "$WORK" fetch origin >/dev/null

UNMERGED=$(git -C "$WORK" branch -r --no-merged origin/main \
  | sed 's/^[*[:space:]]*//' \
  | grep -E '^origin/(feat|fix|refactor|chore|perf|test|docs)/' \
  || true)
if printf '%s\n' "$UNMERGED" | grep -qx 'origin/feat/open'; then
  inc_pass "Unmerged remote branch is detected after whitespace normalization"
else
  inc_fail "Unmerged remote branch gate missed origin/feat/open"
fi

git -C "$WORK" merge --no-ff feat/open -m "merge: feature" >/dev/null
MERGE_SHA=$(git -C "$WORK" rev-parse HEAD)
if git -C "$WORK" merge-base --is-ancestor "$BRANCH_HEAD" "$MERGE_SHA" \
  && git -C "$WORK" merge-base --is-ancestor "$MERGE_SHA" main; then
  inc_pass "Merge ancestry gate accepts a valid no-ff merge"
else
  inc_fail "Merge ancestry gate rejected a valid no-ff merge"
fi

if grep -q 'UNMERGED=$(git branch -r --no-merged' "$MCW_FILE" \
  && grep -q '\[ -z "$UNMERGED" \] || BLOCK' "$MCW_FILE" \
  && ! grep -q 'no active lock.*OR' "$MCW_FILE"; then
  inc_pass "Consolidation gate blocks on normalized unmerged branches and cannot bypass a lock"
else
  inc_fail "Consolidation gate is not deterministically blocking"
fi

if grep -q 'release-lock.*fails.*BLOCK' "$SDDK_ROOT/agents/sddk-release.md" \
  && grep -q 'release-lock.*BLOCK' "$SDDK_ROOT/skills/sddk-release/SKILL.md"; then
  inc_pass "Release-lock failure is blocking in agent and skill"
else
  inc_fail "Release-lock failure policy is inconsistent"
fi

# Archive runs AFTER release; it must NOT claim it hands off to release.
if ! grep -q 'ready_for_release: true' "$SDDK_ROOT/agents/sddk-archive.md" \
  && ! grep -q 'next_recommended: /sddk-release' "$SDDK_ROOT/agents/sddk-archive.md"; then
  inc_pass "Archive does not claim to hand off to release (correct: release before archive)"
else
  inc_fail "Archive still claims it hands off to release (wrong order)"
fi

if grep -q 'git push origin main' "$SDDK_ROOT/prompts/sddk/phases/release.md" \
  && grep -q 'git rev-parse origin/main' "$SDDK_ROOT/prompts/sddk/phases/release.md" \
  && grep -q 'git tag -a "$TAG" "$SHA"' "$SDDK_ROOT/prompts/sddk/phases/release.md" \
  && grep -q 'refs/tags/$TAG^{}' "$SDDK_ROOT/prompts/sddk/phases/release.md"; then
  inc_pass "Local release is dependency-free and verifies main SHA plus annotated remote tag"
else
  inc_fail "Local release is missing a required Git postcondition"
fi

if ! grep -qE 'gh pr checks|gh pr merge|wait-checks-and-approval' "$MCW_FILE"; then
  inc_pass "Authoritative MCW has no PR or CI/CD release gate"
else
  inc_fail "Authoritative MCW retains the obsolete provider release algorithm"
fi

git -C "$WORK" tag -a v0.0.1 "$MERGE_SHA" -m "test release"
if [ -z "$(git --git-dir="$ORIGIN" tag --list v0.0.1)" ]; then
  git -C "$WORK" push origin v0.0.1 >/dev/null
fi
if [ "$(git --git-dir="$ORIGIN" tag --list v0.0.1)" = "v0.0.1" ] \
  && grep -q 'git push origin "refs/tags/$TAG"' "$SDDK_ROOT/prompts/sddk/phases/release.md"; then
  inc_pass "Release retry pushes a locally-created annotated tag that is missing remotely"
else
  inc_fail "Release retry can strand a local-only semver tag"
fi

if ! grep -q 'docs/ROADMAP.md' "$SDDK_ROOT/prompts/sddk/mcw.md" \
  && grep -q 'Step 3.3.*Local Receipts And Bookkeeping' "$SDDK_ROOT/prompts/sddk/mcw.md" \
  && grep -q 'serialization lock' "$SDDK_ROOT/prompts/sddk/mcw.md"; then
  inc_pass "MCW uses the external knowledge graph and explicit lock release"
else
  inc_fail "MCW still conflicts with the vault-only knowledge contract"
fi

if grep -q 'CI/CD.*excluded\|excluded.*CI/CD' "$SDDK_ROOT/prompts/sddk/phases/release.md" "$SDDK_ROOT/agents/sddk-release.md" "$SDDK_ROOT/skills/sddk-release/SKILL.md"; then
  inc_pass "CI/CD is explicitly excluded from local release gates"
else
  inc_fail "Release policy does not explicitly exclude CI/CD from local release gates"
fi

banner "REGRESSION 4: evaluate-gate calls include --outcome passed"

AGENT_SKILL_PROMPT_FILES=(
  "$SDDK_ROOT/agents/sddk-explore.md"
  "$SDDK_ROOT/agents/sddk-propose.md"
  "$SDDK_ROOT/agents/sddk-spec.md"
  "$SDDK_ROOT/agents/sddk-design.md"
  "$SDDK_ROOT/agents/sddk-tasks.md"
  "$SDDK_ROOT/agents/sddk-apply.md"
  "$SDDK_ROOT/agents/sddk-verify.md"
  "$SDDK_ROOT/agents/sddk-archive.md"
  "$SDDK_ROOT/agents/sddk-debt-verify.md"
  "$SDDK_ROOT/skills/sddk-release/SKILL.md"
)

for file in "${AGENT_SKILL_PROMPT_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")
    if grep -E 'evaluate-gate.*--transition' "$file" 2>/dev/null | grep -vq '\-\-outcome passed'; then
      inc_fail "$fname: evaluate-gate call missing --outcome passed"
    else
      inc_pass "$fname: evaluate-gate calls include --outcome passed"
    fi
  fi
done

banner "REGRESSION 5: No obsolete --artifact phase aliases"

OBSOLETE_ALIASES=(
  '--artifact explore='
  '--artifact propose='
  '--artifact spec='
  '--artifact apply='
  '--artifact verify='
  '--artifact tasks='
  '--artifact debt-verify='
)

ALIAS_FILES=(
  "$SDDK_ROOT/agents/sddk-explore.md"
  "$SDDK_ROOT/agents/sddk-propose.md"
  "$SDDK_ROOT/agents/sddk-spec.md"
  "$SDDK_ROOT/agents/sddk-design.md"
  "$SDDK_ROOT/agents/sddk-tasks.md"
  "$SDDK_ROOT/agents/sddk-apply.md"
  "$SDDK_ROOT/agents/sddk-verify.md"
  "$SDDK_ROOT/agents/sddk-archive.md"
  "$SDDK_ROOT/agents/sddk-debt-verify.md"
)

for alias in "${OBSOLETE_ALIASES[@]}"; do
  found=0
  for file in "${ALIAS_FILES[@]}"; do
    if [ -f "$file" ] && grep -qF "$alias" "$file" 2>/dev/null; then
      inc_fail "$(basename "$file"): contains obsolete alias $alias"
      found=1
      break
    fi
  done
  if [ "$found" -eq 0 ]; then
    inc_pass "No obsolete alias $alias"
  fi
done

banner "REGRESSION 6: Release-before-archive ordering (no archive→release)"

ARCHIVE_ORDER_FILES=(
  "$SDDK_ROOT/agents/sddk-archive.md"
  "$SDDK_ROOT/agents/sddk-release.md"
  "$SDDK_ROOT/skills/sddk-archive/SKILL.md"
  "$SDDK_ROOT/skills/sddk-release/SKILL.md"
  "$SDDK_ROOT/prompts/sddk/phases/archive.md"
  "$SDDK_ROOT/prompts/sddk/phases/release.md"
)

ARCHIVE_WRONG_PATTERNS=(
  'ready_for_release'
  'release-handoff'
  'archive.*then.*release'
  'archived.*release completes'
)

for file in "${ARCHIVE_ORDER_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")
    bad=0
    for pat in "${ARCHIVE_WRONG_PATTERNS[@]}"; do
      if grep -qiE "$pat" "$file" 2>/dev/null; then
        inc_fail "$fname: contains wrong-order pattern: $pat"
        bad=1
        break
      fi
    done
    if [ "$bad" -eq 0 ]; then
      inc_pass "$fname: no archive→release ordering claim"
    fi
  fi
done

banner "REGRESSION 7: Knowledge pipeline — with_knowledge, knowledge_approved, quarantine guard"

KNOWLEDGE_PIPELINE_FILES=(
  "$SDDK_ROOT/prompts/sddk/orchestrator.md"
  "$SDDK_ROOT/prompts/sddk/dynamic-workflow.md"
  "$SDDK_ROOT/prompts/sddk/launch-plan-helper.md"
)

for file in "${KNOWLEDGE_PIPELINE_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")
    if grep -q 'with_knowledge' "$file" 2>/dev/null; then
      inc_pass "$fname: contains with_knowledge"
    else
      inc_fail "$fname: missing with_knowledge"
    fi
    if grep -q 'knowledge_approved' "$file" 2>/dev/null; then
      inc_pass "$fname: contains knowledge_approved"
    else
      inc_fail "$fname: missing knowledge_approved"
    fi
  fi
done

QUARANTINE_FILES=(
  "$SDDK_ROOT/skills/sddk-init/SKILL.md"
  "$SDDK_ROOT/agents/sddk-init.md"
  "$SDDK_ROOT/prompts/sddk/phases/init.md"
  "$SDDK_ROOT/skills/_shared/sddk-phase-common.md"
)

for file in "${QUARANTINE_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")
    if grep -qiE 'quarantine.*auto-import\|quarantine.*auto-approve\|auto.*quarantine' "$file" 2>/dev/null; then
      inc_fail "$fname: quarantine auto-import/approve claim found"
    else
      inc_pass "$fname: no quarantine auto-import/approve"
    fi
  fi
done

banner "SUMMARY"
echo "  PASSED: $PASS"
echo "  FAILED: $FAIL"
echo ""

if [ "$FAIL" -gt 0 ]; then
  echo "REGRESSION TEST FAILED"
  exit 1
else
  echo "ALL REGRESSION TESTS PASSED"
  exit 0
fi
