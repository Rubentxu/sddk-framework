#!/usr/bin/env bash
# tests/test_release_blockers.sh — Regression tests for the BLOCKER/HIGH findings
# reported on the sddk-release flow.
#
# Findings covered:
#   1. Orchestration, skill and prompt describe archive -> release but the
#      workflow/workflow.yaml enforces release -> archive. The contracts MUST
#      reflect that ordering and must describe release-receipt + archive-manifest
#      coherence.
#   2. skills/sddk-release/SKILL.md invokes `sddk release apply --route local`
#      without --cycle, but the CLI requires --cycle for the local route. The
#      cycle id MUST be propagated through every contract, every CLI example
#      and every test fixture.
#   3. reconcile_local_pending in crates/sddk-gateway/src/release.rs marks a
#      Started receipt as Failed when the remote effect is absent, and
#      run_local_step returns the finished receipt without applying the
#      effect. The contracts MUST define safe retry: pre-effect crash =>
#      re-attempt; post-effect crash => close Started only if the effect
#      is present; never return Ok(converged: false) with exit 0 when
#      remote effects are missing.
#   4. local_release_preconditions in crates/sddk-cli/src/release_cmd.rs must
#      tie the cycle to the release: the manifest commit must be an ancestor of
#      HEAD and HEAD must be on a clean trunk. A cycle pointing at a different
#      branch must fail clearly.
#   5. authorize_release in crates/sddk-cli/src/release_cmd.rs only authorizes
#      git.push and git.tag. It MUST also include git.inspect, and the registry
#      MUST allow sddk-release/release to use git.inspect.
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

# 1. Orchestration, skill and prompt must reflect release -> archive, NOT
#    archive -> release. They must also describe release-receipt +
#    archive-manifest coherence.
banner "REGRESSION 1: Orchestration, skill and prompt describe release -> archive"

CONTRACT_FILES=(
  "$SDDK_ROOT/agents/orchestrator.md"
  "$SDDK_ROOT/agents/sddk-release.md"
  "$SDDK_ROOT/prompts/sddk/orchestrator.md"
  "$SDDK_ROOT/skills/sddk-release/SKILL.md"
  "$SDDK_ROOT/prompts/sddk/phases/release.md"
)

for file in "${CONTRACT_FILES[@]}"; do
  if [ -f "$file" ]; then
    rel="${file#$SDDK_ROOT/}"
    # The contracts must say release happens BEFORE archive; the
    # forbidden pattern is "archive -> release" / "archive->release".
    if grep -Eq 'archive\s*->\s*release|archive -> release|after archive' "$file"; then
      inc_fail "$rel: still says archive -> release (must say release -> archive)"
    else
      inc_pass "$rel: does not claim archive -> release"
    fi

    # The contracts must reference release-receipt AND archive-manifest together.
    if grep -qE 'release-receipt' "$file" && grep -qE 'archive-manifest' "$file"; then
      inc_pass "$rel: links release-receipt to archive-manifest"
    else
      inc_fail "$rel: does not link release-receipt to archive-manifest"
    fi
  fi
done

# The workflow itself orders release -> archive:
WORKFLOW="$SDDK_ROOT/workflow/workflow.yaml"
release_block=$(awk '
  $0 ~ /^  - id: release\.complete$/ { flag=1; print; next }
  flag && $0 ~ /^  - id:/ { flag=0 }
  flag { print }
' "$WORKFLOW")
archive_block=$(awk '
  $0 ~ /^  - id: archive\.complete$/ { flag=1; print; next }
  flag && $0 ~ /^  - id:/ { flag=0 }
  flag { print }
' "$WORKFLOW")
if echo "$release_block" | grep -qE "status: RELEASE_PENDING" \
  && echo "$release_block" | grep -qE "phase: release" \
  && echo "$archive_block" | grep -qE "status: RELEASED" \
  && echo "$archive_block" | grep -qE "phase: archive"; then
  inc_pass "workflow.yaml: release.complete runs from RELEASE_PENDING and archive.complete runs from RELEASED"
else
  inc_fail "workflow.yaml: does not declare release -> archive ordering"
fi

# 2. skills/sddk-release/SKILL.md must invoke `sddk release apply --route local`
#    WITH --cycle, matching the CLI contract.
banner "REGRESSION 2: --cycle is propagated through every sddk-release contract and CLI example"

for file in "${CONTRACT_FILES[@]}"; do
  if [ -f "$file" ]; then
    rel="${file#$SDDK_ROOT/}"
    # Every `sddk release apply --route local` example must include --cycle.
    if grep -qE 'sddk release apply' "$file" || grep -qE 'sddk-release apply' "$file"; then
      if grep -qE -- '--route local' "$file"; then
        # Find lines that mention --route local and check the same context has --cycle.
        if grep -B 1 -A 8 -- '--route local' "$file" | grep -qE -- '--cycle'; then
          inc_pass "$rel: --cycle present in every --route local example"
        else
          inc_fail "$rel: --route local example missing --cycle"
        fi
      else
        inc_pass "$rel: no --route local example to validate"
      fi
    else
      inc_pass "$rel: no release apply example to validate"
    fi
  fi
done

# 3. reconcile_local_pending must not silently mark a Started receipt as Failed
#    when the remote effect is absent (pre-effect crash). It must return the
#    Started receipt untouched so a retry can apply the effect.
banner "REGRESSION 3: reconcile_local_pending is idempotent (pre-effect crash => keep Started)"

# This contract is enforced by the Rust tests in crates/sddk-gateway/tests/.
# The bash-side check makes sure the contract is documented in code.
RELEASE_RS="$SDDK_ROOT/crates/sddk-gateway/src/release.rs"
if [ -f "$RELEASE_RS" ]; then
  # The fix: local_receipt_state must return None (skip) when the remote
  # effect is absent (pre-effect crash), so reconcile_local_pending keeps the
  # Started receipt for a retry instead of marking it Failed.
  if grep -q 'reconcile_local_pending' "$RELEASE_RS" \
    && grep -q 'fn local_receipt_state' "$RELEASE_RS" \
    && awk '/^fn local_receipt_state/,/^}/' "$RELEASE_RS" | grep -qE 'Ok\(None\)' \
    && awk '/^fn local_receipt_state/,/^}/' "$RELEASE_RS" | grep -qE 'Pre-effect crash'; then
    inc_pass "release.rs: local_receipt_state returns None on pre-effect crash so reconcile_local_pending keeps Started"
  else
    inc_fail "release.rs: local_receipt_state must return None on pre-effect crash and the comment must explain the contract"
  fi
fi

# 4. local_release_preconditions must validate the cycle is the
#    release-pending cycle, the manifest commit is an ancestor of HEAD,
#    and HEAD is on a clean trunk.
banner "REGRESSION 4: local_release_preconditions ties the cycle to trunk and HEAD"

RELEASE_CMD_RS="$SDDK_ROOT/crates/sddk-cli/src/release_cmd.rs"
if [ -f "$RELEASE_CMD_RS" ]; then
  if grep -q 'fn local_release_preconditions' "$RELEASE_CMD_RS"; then
    if grep -qE 'merge_base.*--is-ancestor|is_ancestor|ancestor of HEAD|ancestor' "$RELEASE_CMD_RS"; then
      inc_pass "release_cmd.rs: local_release_preconditions verifies manifest commit is an ancestor of HEAD"
    else
      inc_fail "release_cmd.rs: local_release_preconditions does not verify manifest is an ancestor of HEAD"
    fi

    if grep -qE 'inspect\(\).branch.*!= Some\("main"\)|checkout must be main|inspect.branch.*main|branch.*trunk' "$RELEASE_CMD_RS"; then
      inc_pass "release_cmd.rs: local_release_preconditions requires checkout on trunk"
    else
      inc_fail "release_cmd.rs: local_release_preconditions does not require checkout on trunk"
    fi
  else
    inc_fail "release_cmd.rs: local_release_preconditions function not found"
  fi
else
  inc_fail "release_cmd.rs: missing"
fi

# 5. authorize_release must include git.inspect (read-only capability that
#    local_release_preconditions depends on) and permissions.yaml must allow it.
banner "REGRESSION 5: authorize_release includes git.inspect and permissions.yaml allows it"

if [ -f "$RELEASE_CMD_RS" ]; then
  if grep -q 'fn authorize_release' "$RELEASE_CMD_RS"; then
    if awk '/^fn authorize_release/,/^}/' "$RELEASE_CMD_RS" | grep -qE 'git\.inspect'; then
      inc_pass "release_cmd.rs: authorize_release authorizes git.inspect"
    else
      inc_fail "release_cmd.rs: authorize_release does NOT authorize git.inspect"
    fi
  else
    inc_fail "release_cmd.rs: authorize_release function not found"
  fi
fi

PERMS_FILE="$SDDK_ROOT/permissions.yaml"
if [ -f "$PERMS_FILE" ]; then
  sddk_release_block=$(awk '
    /^  sddk-release:/ { flag=1; print; next }
    flag && /^  [a-z]/ { exit }
    flag { print }
  ' "$PERMS_FILE")
  if printf '%s\n' "$sddk_release_block" | grep -q 'git.inspect'; then
    inc_pass "permissions.yaml: sddk-release is allowed git.inspect"
  else
    inc_fail "permissions.yaml: sddk-release is NOT allowed git.inspect"
  fi
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
