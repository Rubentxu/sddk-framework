# SDD Kernel Verify — Strict TDD Module

> **This module is loaded ONLY when Strict TDD Mode is enabled AND a test runner is available.**
> Loaded by `sddk-verify` after the orchestrator confirms Strict TDD is active (via `strict_tdd_mode: true` in launch plan, OR `STRICT TDD MODE IS ACTIVE` injection from orchestrator).
> If you are reading this, the orchestrator already verified both conditions. Follow every instruction.

## TDD Verification Philosophy

Verify is the quality gate. Under Strict TDD, verify checks not only spec compliance but also the **discipline** of the TDD cycle — because tests written in the wrong order produce false confidence.

A test that passes without exercising production logic is worse than no test — it gives false confidence. Verify's job is to catch this.

## The Four Strict TDD Checks

### Check 1 — TDD Cycle Evidence Table Presence

The apply-progress MUST contain a TDD Cycle Evidence table with one row per task:

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| 1.1 | `path/test.ext` | Unit | ✅ 5/5 | ✅ Written | ✅ Passed | ✅ 3 cases | ✅ Clean |
| 1.2 | `path/test.ext` | Integration | N/A (new) | ✅ Written | ✅ Passed | ➖ Single | ✅ Clean |

- **Missing table entirely** → 🔴 CRITICAL
- **Row present but missing columns** → 🔴 CRITICAL
- **"➖ None needed" or "Skip" with no justification** → 🟡 WARNING (verify if justified)

### Check 2 — Three Laws Compliance

| Law | How to verify |
|-----|---------------|
| **No production code before test** | For each task, check git log: does the test commit precede the implementation commit? Or does the diff in a single commit show test additions before production code? |
| **No more test than necessary** | Tests reference existing spec scenarios. No speculative tests for unimplemented behavior. |
| **No more code than necessary** | Implementation is minimal — no over-engineering, no speculative features. |

- **Violation** → 🔴 CRITICAL

### Check 3 — Banned Assertion Patterns

Scan test files for the following banned patterns:

```
TRIVIAL ASSERTIONS — test proves nothing:
  expect(true).toBe(true)              # Tautology
  expect(false).toBe(false)            # Tautology
  expect(1).toBe(1)                    # Tautology — no production code involved
  assert True                          # Always passes
  assert 1 == 1                        # Always passes

EMPTY COLLECTION ASSERTIONS without setup context:
  expect(result).toEqual([])           # ONLY valid with setup
  expect(result).toHaveLength(0)       # Same — why empty?
  assert len(result) == 0              # Same
  assert result == []                  # Same

TYPE-ONLY ASSERTIONS — proves existence, not behavior:
  expect(result).toBeDefined()         # Alone is useless
  expect(result).not.toBeNull()        # Alone is useless
  expect(typeof result).toBe('object') # Alone is useless
  assert result is not None            # Alone

GHOST LOOPS:
  for (const item of []) { ... }       # Loop body never executes

CSS CLASS ASSERTIONS:
  expect(element.className).toContain("text-xs")  # Implementation detail
```

Any of these (without proper triangulation context) → 🔴 CRITICAL.

**Allowed exceptions:**
- `expect(result).toEqual([])` is OK IF a companion test with different setup produces NON-EMPTY (triangulation).
- `expect(result).toBeDefined()` is OK IF followed by a value assertion in the same test.

### Check 4 — Mock/Assertion Ratio

For each test file, count mocks and assertions:

| Ratio | Classification |
|-------|---------------|
| ≤ 3 mocks per test file | ✅ Healthy — focused test |
| 4-6 mocks per test file | 🟡 WARNING — consider Extract-Before-Mock |
| 7+ mocks per test file | 🔴 CRITICAL — STOP, you're testing at wrong layer |

**Extract-Before-Mock Rule**: If the behavior under test is a data transformation, mapping, filtering, or conditional logic, EXTRACT it to a pure function and test directly with zero mocks.

```
❌ BAD: 15 mocks to test a one-line status conversion
vi.mock("next/navigation", ...);
vi.mock("next/link", ...);
// ... 12 more mocks ...
render(<StatusCell row={mutedRow} />);
expect(screen.getByText("FAIL")).toBeInTheDocument();

✅ GOOD: extract and test the logic directly
export function resolveDisplayStatus(status: string, isMuted: boolean): string {
  return status === "MUTED" ? "FAIL" : status;
}
expect(resolveDisplayStatus("MUTED", true)).toBe("FAIL");
expect(resolveDisplayStatus("PASS", false)).toBe("PASS");
```

## Triangulation Verification

For each task, check if spec has multiple scenarios:

| Spec scenarios for task | Required triangulation |
|------------------------|------------------------|
| 1 scenario | "➖ Single" OK in evidence |
| 2+ scenarios | Each scenario MUST have a test. Missing → 🔴 CRITICAL. |

If a task shows "Triangulation skipped: {reason}" → 🟡 WARNING (verify reason is valid).

## Safety Net Verification

For tasks that modify existing files (not new files):
- Was the baseline test run BEFORE the changes? Evidence required.
- Did the baseline pass? (If pre-existing failure was reported, that's the correct behavior.)
- If no Safety Net was run for an existing-file modification → 🟡 WARNING.

## Pure Function Verification (when claimed)

If apply-progress claims "Pure functions created: {N}", verify by:
- Check that extracted functions have no side effects (no global state, no I/O).
- Check tests don't mock those functions (they should be testable directly).
- 🔴 CRITICAL if claimed but violated.

## Strict TDD Verify Report Section

Add this section to the standard verify report when Strict TDD is active:

```markdown
## Strict TDD Compliance

### TDD Cycle Evidence
- Table present: {yes/no}
- Tasks with complete evidence: {N}/{total}
- Tasks with missing columns: {N}

### Three Laws Compliance
- Law 1 (test before code): {compliant/violations: N}
- Law 2 (minimal test): {compliant/violations: N}
- Law 3 (minimal code): {compliant/violations: N}

### Assertion Quality
- Banned patterns found: {N}
- Files with critical mock ratios (≥7): {N}

### Triangulation
- Tasks with spec scenarios and matching tests: {N}/{total}
- Missing triangulation: {N}

### Safety Net
- Tasks with Safety Net run: {N}/{applicable}
- Missing Safety Net: {N}

### Strict TDD Issues
🔴 CRITICAL: {list}
🟡 WARNING: {list}
```

## Rules

- **NEVER relax these checks because "the tests pass"** — passing tests with banned patterns is a CRITICAL finding.
- **NEVER fix code in verify** — report only.
- **NEVER silently downgrade to Standard Mode** — if Strict TDD is active, follow it.
- If a test runner execution fails for INFRASTRUCTURE reasons (not test failures), report as "Blocked" and continue to next task.

## References

- `phases/apply-strict-tdd.md` — apply side (where tests are written)
- `phases/verify.md` — standard verify
- `prompts/sddk/decision-model.md` — strict_tdd_mode in launch plan
