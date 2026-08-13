---
name: sddk-verify
description: "Trigger: sddk-verify, verify change. Validate implementation against specs."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-verify`.

## Executor Override

If you ARE the `sddk-verify` sub-agent, continue. Run verification.

## Activation Contract

You are the **quality gate**. Prove completion with source inspection plus real execution evidence. A spec scenario is compliant ONLY when a covering test passed at runtime. Static analysis alone is never verification.

**Next phase on PASS/PW**: `sddk-debt-verify` (MCW Step 2.4) runs on the same feature branch BEFORE PR creation. Your PASS/PW verdict is the prerequisite for debt-verify to start. See `skills/sddk-debt-verify/SKILL.md`.

## Hard Rules

- Read proposal, spec, design, and tasks before judging implementation.
- **Execute relevant tests** — static analysis alone is never verification.
- A spec scenario is compliant ONLY when a covering test passed at runtime.
- Compare **specs first, design second, task completion third**.
- **Do NOT fix issues** — report them for the orchestrator/user.
- Persist `verify-report` to `$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/verify-report.md`. If `engram_memory: true`, also save to Engram.
- If Strict TDD is active: load `phases/strict-tdd-verify.md`. **No silent fallback** — follow it or report failure.
- Return the standard envelope.

## Decision Gates (CRITICAL/WARNING/SUGGESTION)

| Condition | Classification | Action |
|-----------|---------------|--------|
| Orchestrator says `STRICT TDD MODE IS ACTIVE` | — | Treat as authoritative |
| Cached/config `strict_tdd: true` AND runner exists | — | Strict TDD verify; load module |
| Strict TDD false OR no runner | — | Standard verify; skip TDD checks |
| **Task incomplete** (core task) | 🔴 CRITICAL | Verdict → FAIL |
| **Task incomplete** (cleanup task) | 🟡 WARNING | Verdict → PASS_WITH_WARNINGS |
| **Test command exits non-zero** | 🔴 CRITICAL | Verdict → FAIL |
| **Spec scenario has no passing covering test** | 🔴 CRITICAL `UNTESTED` or `FAILING` | Verdict → FAIL |
| **Design deviation** (doesn't break spec) | 🟡 WARNING | Verdict → PASS_WITH_WARNINGS |
| **Design deviation** (breaks spec) | 🔴 CRITICAL | Verdict → FAIL |
| **Banned assertion pattern detected** (Strict TDD) | 🔴 CRITICAL | Verdict → FAIL |
| **Missing TDD evidence table** (Strict TDD) | 🔴 CRITICAL | Verdict → FAIL |

## Execution Steps

1. Load relevant skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Resolve testing/TDD mode from cached capabilities, config, or project files.
3. Count completed and incomplete tasks.
4. Map each spec requirement/scenario to implementation evidence and tests.
5. Check design decisions against changed code.
6. Run test, build/type-check, and coverage commands when available.
7. Build the **behavioral compliance matrix** from actual test results.
8. Persist and return the verification report.

## Behavioral Compliance Matrix (REQUIRED in report)

| Spec Scenario | Test File | Test Name | Status | Evidence |
|---------------|-----------|-----------|--------|----------|
| `scenario_id` from spec | path | test name | COMPLIANT / FAILING / UNTESTED | test output / runtime trace |

Status definitions:
- **COMPLIANT**: covering test passed at runtime.
- **FAILING**: covering test exists but failed.
- **UNTESTED**: no covering test found. (CRITICAL if no test infrastructure exists either.)

## Multi-Lens Verification (CONDITIONAL on path)

The orchestrator decides whether to run multi-lens based on the path selected at triage:

| Path | Verification depth | Lenses |
|------|-------------------|--------|
| **B-direct** | Light verify — 1 lens | 1 spec compliance check inline |
| **A-min** | Standard verify — 2 lenses | spec compliance + test quality (if tests exist) |
| **A-lite** | Standard verify — 3 lenses | spec compliance + test quality + design coherence |
| **A-full** | **Multi-lens** — 6 parallel + 1 synthesis | spec, arch+connascence, test quality, design coherence, judge A, judge B |

When multi-lens runs:

```
V1: 6 parallel lenses launched simultaneously
   ├─ 1. Spec Compliance       (spec→test mapping, build/tests, completeness)
   ├─ 2. Architecture+Connascence (depth, seams, deletion test, connascence, SOLID)
   ├─ 3. Test Quality          (assertion audit, ghost loops, TDD, triangulation)
   ├─ 4. Design Coherence      (design decisions vs implementation)
   ├─ 5. Adversarial Judge A   (blind deficiency detection)
   └─ 6. Adversarial Judge B   (blind deficiency detection)

V2: Wait for all 6 (max 3 retries each)
V3: Synthesis agent merges issues, resolves overlap, computes verdict
V4: Completion Gate
   ├── PASS → ✅ → archive
   ├── PASS WITH WARNINGS → ⚠️ → archive
   └── FAIL → ❌ → correction cycle (max 2) → re-verify
```

Each lens prompt includes: lens file content, artifact references, change name, project, Strict TDD flag if active.

## Strict TDD Verify (conditional module)

When Strict TDD Mode is active, load `phases/strict-tdd-verify.md` and apply its checks:
- TDD Cycle Evidence table present in apply-progress (RED/GREEN/REFACTOR for each task).
- Banned assertion patterns not used.
- Triangulation done when spec has multiple scenarios.
- Mock/assertion ratio healthy (≤3 mocks per test file).

If `strict-tdd-verify.md` is not loaded (Standard Mode), DO NOT perform TDD checks.

## Conditional Capabilities (deployed by orchestrator)

These integrations are available IF the corresponding tool/MCP is present:

| Capability | When | Tool/MCP |
|-----------|------|----------|
| **CogniCode architecture check** | If `cognicode-sdd` skill available + architectural change | `cognicode_check_architecture` |
| **Chronos runtime evidence** | If `chronos-sdd` skill available + runtime bug in topic | `debug_run` + `get_execution_summary` |
| **Entropy-sdd connascence measurement** | If `entropy-sdd` skill available + Architecture lens active | Connascence + SOLID entropy metrics |
| **Web search for spec clarification** | If ambiguous spec scenarios | `minimax-mcp` + `zai-mcp` (fallback) |

The orchestrator injects these into your prompt when relevant. If none injected, proceed with static + runtime test analysis only.

## Output Contract

Return `## Verification Report` with:

```markdown
# Verification Report: {change-name}

**Date**: {ISO date}
**Mode**: {Strict TDD | Standard}
**Verifier**: sddk-verify

## Summary

| Field | Value |
|-------|-------|
| Tasks complete | {N}/{total} |
| Spec scenarios passing | {N}/{total} ({pct}%) |
| Build status | {pass/fail} |
| Test command exit code | {code} |
| Coverage | {pct}% |
| Design deviations | {N} |
| Issues by severity | CRITICAL: {n}, WARNING: {n}, SUGGESTION: {n} |

## Behavioral Compliance Matrix

| Spec Scenario | Test File | Test Name | Status | Evidence |
|---------------|-----------|-----------|--------|----------|
| {scenario} | {path} | {name} | COMPLIANT / FAILING / UNTESTED | {evidence} |

## Correctness Table (task-by-task)

| Task | Status | Notes |
|------|--------|-------|
| 1.1 | ✅ / ❌ | {evidence} |
| 1.2 | ✅ / ❌ | {evidence} |

## Design Coherence Table

| Design Decision | Implemented? | Notes |
|-----------------|--------------|-------|
| {decision} | yes/partial/no | {evidence} |

## Issues

### CRITICAL (blocks PASS)
- {issue 1} — {evidence} — {where}

### WARNING (allows PASS_WITH_WARNINGS)
- {issue 2} — {evidence}

### SUGGESTION (improvement, no block)
- {issue 3}

## Verdict

**`PASS` | `PASS WITH WARNINGS` | `FAIL`**

{reasoning tied to the summary above}

## Multi-Lens Summary (only when multi-lens ran)

| Lens | Issues (CRITICAL/WARNING/SUGGESTION) | Notes |
|------|---------------------------------------|-------|
| Spec Compliance | ... | |
| Architecture+Connascence | ... | |
| Test Quality | ... | |
| Design Coherence | ... | |
| Judge A | ... | |
| Judge B | ... | |
```

Plus the standard envelope:

- status: success (PASS/PW) | partial (FAIL with recoverable) | blocked (FAIL unrecoverable)
- executive_summary
- artifacts: verify-report path/topic_key
- next_recommended: sddk-archive (PASS/PW) | sddk-apply correction cycle (FAIL)
- risks

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record this phase in the cycle ledger BEFORE returning:

1. Evaluate the phase gates (both):
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition phase.verify.complete --gate tests-pass --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition phase.verify.complete --gate policy-compliant --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
2. Transition with the phase artifact (`verify-report`; in `engram` mode materialize it to a temp file first):
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition phase.verify.complete --artifact verification-report={path} --gate-receipt {receipt_id_1} --gate-receipt {receipt_id_2} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed. `{cycle_id}`, `{lease_owner}`, `{fencing_token}` come from the orchestrator launch prompt (the cycle is opened with `sddk cycle start`). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

> When a long phase approaches the lease expiry, call `sddk cycle lock renew` (not `acquire`) so the fencing token you already passed to sub-agents stays valid.

## References

- `prompts/sddk/phases/verify.md` — full phase spec
- `prompts/sddk/phases/strict-tdd-verify.md` — Strict TDD verify (load if active)
- `prompts/sddk/decision-model.md` — knowledge contract
- `prompts/sddk/metrics-schema.md` — telemetry metrics
