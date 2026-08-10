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

banner "REGRESSION 2: Release has correct step order (request-merge -> wait-MERGED -> verify-SHA)"

RELEASE_FILES=(
  "$SDDK_ROOT/agents/sddk-release.md"
  "$SDDK_ROOT/skills/sddk-release/SKILL.md"
  "$SDDK_ROOT/prompts/sddk/phases/release.md"
)

for file in "${RELEASE_FILES[@]}"; do
  if [ -f "$file" ]; then
    fname=$(basename "$file")

    # merge-to-main must NOT call gh pr merge after MERGED
    if grep -qE "merge-to-main.*gh pr merge" "$file" 2>/dev/null; then
      if grep -qE "merge-to-main.*VERIFY.*only|merge-to-main.*do NOT call gh pr merge" "$file" 2>/dev/null; then
        inc_pass "$fname: merge-to-main is verify-only (correct)"
      else
        inc_fail "$fname: merge-to-main calls gh pr merge (should be verify-only)"
      fi
    else
      inc_pass "$fname: no gh pr merge in merge-to-main"
    fi

    wait_line=$(grep -n -m1 '5a.*wait-checks-and-approval' "$file" | cut -d: -f1 || true)
    request_line=$(grep -n -m1 '5b.*request-merge' "$file" | cut -d: -f1 || true)
    merged_line=$(grep -n -m1 '5c.*wait-merged' "$file" | cut -d: -f1 || true)
    verify_line=$(grep -n -m1 '6.*verify-merge' "$file" | cut -d: -f1 || true)

    if [ -n "$wait_line" ] && [ -n "$request_line" ] && [ -n "$merged_line" ] && [ -n "$verify_line" ] \
      && [ "$wait_line" -le "$request_line" ] && [ "$request_line" -le "$merged_line" ] && [ "$merged_line" -lt "$verify_line" ]; then
      inc_pass "$fname: release steps are ordered checks -> request -> MERGED -> verify"
    else
      inc_fail "$fname: release steps are missing or out of order"
    fi

    # auto mode must call gh pr merge --auto --merge in request-merge step
    if grep -qE "auto.*gh pr merge.*--auto.*--merge|gh pr merge.*--auto.*--merge.*auto" "$file" 2>/dev/null; then
      inc_pass "$fname: auto mode uses gh pr merge --auto --merge"
    else
      inc_fail "$fname: auto mode does not request auto-merge"
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

for file in "${RELEASE_FILES[@]}"; do
  if grep -q 'git merge-base --is-ancestor' "$file"; then
    inc_pass "$(basename "$file"): uses executable ancestry verification"
  else
    inc_fail "$(basename "$file"): missing ancestry verification"
  fi
done

if grep -q 'PR_NUM=$(gh pr list.*--state all' "$SDDK_ROOT/prompts/sddk/phases/release.md" \
  && [ "$(grep -c 'PR_NUM=$(gh pr list.*--state all' "$SDDK_ROOT/prompts/sddk/phases/release.md")" -ge 2 ]; then
  inc_pass "Release resolves PR_NUM both before and after PR creation"
else
  inc_fail "Release may leave PR_NUM empty after creating a PR"
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

if grep -q 'ready_for_release: true' "$SDDK_ROOT/agents/sddk-archive.md" \
  && grep -q 'next_recommended: /sddk-release' "$SDDK_ROOT/agents/sddk-archive.md"; then
  inc_pass "Archive hands off to mandatory release instead of closing the cycle"
else
  inc_fail "Archive still claims the cycle is complete before release"
fi

if ! grep -qE 'semver-cli|python -c.*semver|origin/main\.\.HEAD' "$SDDK_ROOT/prompts/sddk/phases/release.md" \
  && grep -q 'git tag --points-at "$MERGE_SHA"' "$SDDK_ROOT/prompts/sddk/phases/release.md" \
  && grep -q 'COMMIT_TEXT=$(gh pr view "$PR_NUM" --json commits' "$SDDK_ROOT/prompts/sddk/phases/release.md"; then
  inc_pass "Semver is dependency-free, PR-based, and idempotent on MERGE_SHA"
else
  inc_fail "Semver calculation is not dependency-free and idempotent"
fi

COMMIT_TEXT_LINE=$(grep -n -m1 'COMMIT_TEXT=$(gh pr view "$PR_NUM" --json commits' "$SDDK_ROOT/prompts/sddk/phases/release.md" | cut -d: -f1 || true)
TAG_REUSE_LINE=$(grep -n -m1 'TAG=$(git tag --points-at "$MERGE_SHA"' "$SDDK_ROOT/prompts/sddk/phases/release.md" | cut -d: -f1 || true)
if [ -n "$COMMIT_TEXT_LINE" ] && [ -n "$TAG_REUSE_LINE" ] && [ "$COMMIT_TEXT_LINE" -lt "$TAG_REUSE_LINE" ]; then
  inc_pass "BUMP_TYPE is computed even when an existing tag is reused"
else
  inc_fail "Semver retry can leave BUMP_TYPE uninitialized"
fi

if grep -q 'A deliberately skipped report has no file gate' "$SDDK_ROOT/prompts/sddk/phases/release.md"; then
  inc_pass "Conditional HTML skip does not trigger an impossible file gate"
else
  inc_fail "Conditional HTML skip is contradicted by an unconditional gate"
fi

if grep -q 'gh pr list --head <branch> --state all' "$MCW_FILE" \
  && grep -q 'git merge-base --is-ancestor "$BRANCH_HEAD" "$MERGE_SHA"' "$MCW_FILE" \
  && ! grep -q 'git log --oneline -1 | grep "<branch>"' "$MCW_FILE"; then
  inc_pass "Authoritative MCW uses idempotent PR lookup and ancestry merge verification"
else
  inc_fail "Authoritative MCW retains the obsolete release algorithm"
fi

git -C "$WORK" tag -a v0.0.1 "$MERGE_SHA" -m "test release"
if [ -z "$(git --git-dir="$ORIGIN" tag --list v0.0.1)" ]; then
  git -C "$WORK" push origin v0.0.1 >/dev/null
fi
if [ "$(git --git-dir="$ORIGIN" tag --list v0.0.1)" = "v0.0.1" ] \
  && grep -q '^git push origin "$TAG"$' "$SDDK_ROOT/prompts/sddk/phases/release.md"; then
  inc_pass "Release retry pushes a locally-created tag that is missing remotely"
else
  inc_fail "Release retry can strand a local-only semver tag"
fi

if ! grep -q 'docs/ROADMAP.md' "$SDDK_ROOT/prompts/sddk/mcw.md" \
  && grep -q 'Step 3.8.*Update Knowledge Graph' "$SDDK_ROOT/prompts/sddk/mcw.md" \
  && grep -q 'Step 3.9.*Release Serialization Lock' "$SDDK_ROOT/prompts/sddk/mcw.md"; then
  inc_pass "MCW uses the external knowledge graph and explicit lock release"
else
  inc_fail "MCW still conflicts with the vault-only knowledge contract"
fi

if ! grep -qE 'required CI.*incompatible|reviewers>0 or required CI|Repo has no branch protection with required reviewers/CI' "$SDDK_ROOT/prompts/sddk/phases/release.md" "$SDDK_ROOT/agents/sddk-release.md" "$SDDK_ROOT/skills/sddk-release/SKILL.md"; then
  inc_pass "Required status checks remain compatible with auto-merge"
else
  inc_fail "Release policy still treats required status checks as auto-merge blockers"
fi

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
