# SDDK Verify Phase

## Role And Boundary

Prove that the exact cycle implementation satisfies its specifications with real, production-ready code. Verify is read-only and scoped to the changed code plus the runtime paths needed by the cycle.

Do not substitute task completion for evidence. Do not run `sddk-debt-verify`: that later phase audits broader technical debt.

## Required Inputs

- `path`: `B-direct | A-min | A-lite | A-full`
- `verify_role`: `coordinator | lens`; `lens_id` is required only for a lens invocation
- exact `base_commit` and `head_commit`, or a reproducible dirty-diff digest
- testing capabilities and project-local quality commands
- Strict TDD flag and runner, when active
- risk declarations from project standards and available cycle artifacts

Acceptance authority depends on path:

| Path | Required authority |
|---|---|
| B-direct | User request, selected skill contract, bug reproduction or jurisprudence claim, project invariants, and execution diff |
| A-min | Spec, tasks, apply evidence, and project invariants |
| A-lite | Proposal, spec, tasks, apply evidence, and project invariants |
| A-full | Proposal, spec, design, tasks, apply evidence, and project invariants |

Missing or contradictory authoritative input blocks verification; it never becomes a warning.

## Mandatory Gates

These gates run on every path. Adaptive lenses only add depth.

| Gate | Passing evidence | Failure |
|---|---|---|
| Subject identity | Base/head SHA, clean state or diff digest, CWD, timestamp | `blocked` + verdict `FAIL` |
| Behavioral compliance | Every required scenario has a passing test that reaches production logic | `FAIL` |
| Real implementation | No stub, placeholder, hardcoded value (data, path, secret, magic constant satisfying known examples), unreachable body, or production-wired mock / fake / in-memory adapter in the changed business path | `FAIL` |
| Documentation discipline | No comments in changed production paths reference issue numbers, PR IDs, task identifiers, user handles, cycle / phase pointers, or commit-history metadata; only language-standard docs (`///` in Rust, JSDoc in TS, docstrings in Python, doc comments in Go) explain the *what* and *why* | `FAIL` |
| Test strength | Assertions observe required outcomes; changed boundaries have real contract/integration evidence | `FAIL` |
| Regression and build | Fresh relevant tests and repository-required build/type/lint/regression checks pass | `FAIL`; infrastructure absence is `blocked` |
| Pre-commit discipline | Apply MUST run gates against commit's tree, not dirty working tree | `git status --porcelain` empty + explicit HEAD citation in verify report | blocking |
| Production readiness | Every readiness dimension is `PASS` or evidence-backed `N/A` | `FAIL` when applicable behavior is missing; unknown critical applicability is `blocked` |
| Design and SOLID | No concrete changed-scope violation breaks the approved design, substitutability, client contracts, dependency direction, or local changeability | `FAIL` if material; otherwise warning |
| Task completeness | Every required task, including planned hardening/refactor work, is complete | `FAIL`; only a pre-declared optional item with no required-path impact may warn |

## Procedure

### 1. Pin The Subject

Record base/head SHA and `git status`. The subject MUST be a clean commit tree — `git status --porcelain` MUST be empty. Evidence from another subject, cached summaries, or unidentifiable runs is invalid. If dirty, block until the working tree is cleaned and gates re-run against the verified clean HEAD.

### 2. Build The Behavioral Matrix

Map every requirement and scenario to implementation symbols, test file/name, command, and observed result. For B-direct, derive requirements only from its authority row above; do not invent a spec. Use:

- `COMPLIANT`: covering test passed and exercised production logic.
- `FAILING`: covering test ran and failed.
- `UNTESTED`: no covering executable test exists.
- `BLOCKED`: required evidence could not run or artifacts contradict.

Any required row other than `COMPLIANT` prevents PASS and PASS_WITH_WARNINGS.

### 3. Prove The Implementation Is Real

Inspect the changed production files, callers, adapters, and composition root.

1. Search the changed production diff for markers and empty primitives such as `TODO`, `FIXME`, `XXX`, `HACK`, `todo!`, `unimplemented!`, `NotImplemented`, empty/pass-only bodies, placeholder panics, and constant success responses.
2. Inspect every hit in context. Fail reachable placeholders or behavior required by the cycle; do not fail unrelated historical text outside the changed execution path.
3. Trace each scenario from entry point to the changed implementation. Fail dead, unwired, bypassed, or tests-only code.
4. Confirm mocks, stubs, fakes, in-memory adapters, and fixtures are confined to tests or an explicitly approved non-production profile. Changed external boundaries need a contract or integration test that executes the real adapter; if the real dependency cannot run locally, require its official emulator/sandbox plus a contract test and record the limitation.
5. Challenge suspicious hard-coded values or branches that satisfy only known examples. Require another scenario, negative control, RED evidence, or targeted mutation evidence.

### 3.b Prove The Documentation Discipline

The Code Quality Standards in `apply.md` §"Code Quality Standards" require that comments exist only to explain functionality. Comments whose only purpose is to point at issue trackers, task IDs, user handles, or commit history substitute meta-traceability for documentation and are a violation.

1. Scan the changed production diff for comment lines containing:
   - Issue / PR / ticket references: `#123`, `gh-456`, `JIRA-789`, `TODO #321`.
   - Task identifiers: `REQ-…`, `AC-…`, `T-…`, `Task 3`, `issue/1242`.
   - User handles / attribution: `@author`, `@contributor`, `Signed-off-by:`, `Reviewed-by:`, `Pair-programmed-with:`.
   - Cycle / phase pointers: `cycle-X`, `deferred to cycle-Y`, `recovered in #sha`.
   - Generic placeholders masquerading as documentation: `// see issue`, `// PR #`, `// tracked in `.
2. Inspect every hit in context. Comments that exist purely for traceability and do NOT document behavior are `FAIL`. Comments that pair valid documentation with a reference (e.g., `/// Compute the SHA-256… /// Spec: REQ-K8-001`) are `PASS` — the documentation is the point, the REQ is just an attachment.
3. For language-specific standard docs, sample a few public items and confirm the contract is documented in the language's idiom (`///` in Rust, JSDoc in TS, docstrings in Python, doc comments in Go). Missing standard documentation on a public item without a deferral reason is `WARNING`, not `FAIL` — the documentation-discipline gate is about absence of *bad* documentation, not about mandatory presence of *all* documentation.
4. Tests that legitimately need fixture data hardcoded (e.g., `let admin = User::new("admin")`) are exempt. The gate targets comment shape, not test data.

### 4. Execute Fresh Evidence

Run deterministic checks before semantic judgment:

1. Scenario-focused tests.
2. Tests for the changed package/module.
3. Repository-required regression suite and build/type/lint/format checks.
4. Risk-specific checks declared by spec, design, project standards, or testing capabilities.

For each command record CWD, exact command, timestamp, exit code, subject SHA/diff digest, and log path or concise output. An LLM lens cannot reinterpret a non-zero exit as success.

### 5. Judge Test Strength

- Reject tautologies, type/existence-only assertions used alone, ghost loops, snapshots with no relevant oracle, and tests that only assert mock calls when behavior is required.
- Require a test to fail when its covered behavior is broken. Accept persisted Strict TDD RED evidence, a mutation command with tool/output recorded, or an equivalent negative control tied to the same subject.
- Treat coverage as reachability evidence, not behavioral proof.
- If doubles isolate a changed boundary, require a contract or integration test that executes the real adapter or approved emulator/sandbox. A mock-only proof is insufficient.

### 6. Judge Production Quality And SOLID

Evaluate changed code against the approved design and existing project conventions. Report concrete evidence, not generic scores:

| Principle | Verify |
|---|---|
| SRP | The change does not mix unrelated reasons to change or policy with infrastructure. |
| OCP | The required extension does not force avoidable edits across stable modules. |
| LSP | Implementations preserve the declared input, output, error, and state contract. |
| ISP | Changed clients are not forced to depend on methods or data they do not use. |
| DIP | Policy depends toward the project's intended abstraction/boundary, not a new infrastructure detail. |

SOLID is not a demand for classes, interfaces, or layers. Fail only a concrete material violation in the changed scope. Run mandatory `entropy-sdd` Protocol D when configured, but use its estimates as supporting evidence rather than the sole verdict.

Evaluate every readiness dimension: errors/recovery, state/data integrity, resource cleanup, concurrency, migrations/compatibility, security, performance, and observability/deployability. Mark `N/A` only with changed-scope evidence. Security applies to changed external input, auth, authorization, secrets, or trust boundaries; migrations apply to persisted/schema changes; concurrency applies to async/shared state; performance applies to declared hot paths/SLOs; observability applies to services or operational failure modes. Unknown applicability in security, data integrity, or migration blocks verification.

An item is optional only when an authoritative artifact marked it optional before apply and it cannot affect a required scenario or mandatory production gate. Elevate it to required when source/runtime evidence shows a regression or dependency from a required path.

### 7. Run Path Lenses

Core gates above remain mandatory.

| Path | Lenses |
|---|---|
| B-direct | `direct-acceptance` inline |
| A-min | `spec-compliance`, `test-quality` |
| A-lite | `spec-compliance`, `test-quality`, `production-readiness` |
| A-full | `spec-compliance`, `architecture-connascence`, `test-quality`, `design-coherence`, `jd-judge-a`, `jd-judge-b` |

Production readiness remains a mandatory core gate even when no dedicated lens is configured. Lens focus:

| Lens | Focus |
|---|---|
| `direct-acceptance` | B-direct authority versus final behavior and diff |
| `spec-compliance` | Requirements/scenarios versus implementation and tests |
| `test-quality` | Oracle strength, negative controls, doubles, and regressions |
| `production-readiness` | Readiness matrix and concrete SOLID effects without a design artifact |
| `architecture-connascence` | A-full design boundaries, dependencies, connascence, and entropy evidence |
| `design-coherence` | A-full design decisions versus production implementation |
| `jd-judge-a`, `jd-judge-b` | Blind adversarial deficiency search |

The coordinator runs mandatory deterministic gates once. For A-* paths it launches all configured lenses in one parallel batch: use `sddk-verify` with `verify_role: lens` and one `lens_id` for non-judge lenses, and the exact `jd-judge-a` / `jd-judge-b` agents for judges. A lens never dispatches, persists, updates the ledger, or reruns supplied commands. The coordinator waits, deduplicates, synthesizes, persists, and alone decides the verdict.

Every lens receives the same subject identity, artifact paths, changed files, commands already run, Strict TDD mode, and one focus. Synthesis deduplicates findings but cannot downgrade deterministic failures or missing mandatory evidence.

A verify lens returns only:

```yaml
lens_id: string
status: pass | findings | blocked
findings: [{severity: CRITICAL|WARNING|SUGGESTION, claim: string, evidence: string, location: string}]
evidence_gaps: []
```

### 8. Decide

| Verdict | Exact condition |
|---|---|
| `PASS` | All mandatory gates pass with fresh evidence; no blocking finding remains. |
| `PASS_WITH_WARNINGS` | All mandatory gates pass; only optional, explicitly non-blocking improvements remain. |
| `FAIL` | Any mandatory gate fails, is untested, or cannot be proven. |

Use envelope `status: blocked` with verdict `FAIL` when infrastructure or contradictory authority prevents a decision. Use `status: partial` for a recoverable code/spec failure. Warnings never compensate for failure.

## Report Contract

Persist `{cycle-artifacts-dir}/verify-report.md` with:

```markdown
# Verification Report: {change-name}

## Subject
| Base | Head | Dirty diff digest | CWD | Verified at |

## Summary
| Verdict | Mode | Path | Required scenarios | Commands passed | Critical | Warnings |

## Behavioral Compliance
| Requirement / Scenario | Production Path | Test | Status | Evidence |

## Production Readiness
| Gate | Status: PASS/FAIL/BLOCKED/N/A | Evidence | Findings / N/A reason |

## Code Quality

| Standard | Status: PASS/FAIL/WARNING | Evidence | Findings |
|----------|--------------------------|----------|----------|
| Business code reality (no stub / mock / hardcoded satisfier in `src/` / `lib/` / `bin/`) | {status} | {grep + diff references} | {list of hits} |
| Documentation discipline (no issue / task / user / cycle refs in comments) | {status} | {grep + diff references} | {list of hits} |

## SOLID And Design
| Principle / Decision | Status | Concrete evidence | Impact |

## Commands
| Command | Exit | Subject | Evidence |

## Issues
### CRITICAL
### WARNING
### SUGGESTION

## Lens Summary
| Lens | Findings | Evidence gaps |

## Verdict
**PASS | PASS_WITH_WARNINGS | FAIL**
{reason tied to mandatory gates}
```

Return:

```yaml
status: success | partial | blocked
executive_summary: 1-3 sentences
artifacts: ["{cycle-artifacts-dir}/verify-report.md"]
verdict: PASS | PASS_WITH_WARNINGS | FAIL
subject: {base: sha, head: sha, diff_digest: sha256|null}
mandatory_gates:
  subject_identity: PASS|FAIL|BLOCKED|N/A
  behavioral_compliance: PASS|FAIL|BLOCKED|N/A
  real_implementation: PASS|FAIL|BLOCKED|N/A
  documentation_discipline: PASS|FAIL|BLOCKED|N/A
  test_strength: PASS|FAIL|BLOCKED|N/A
  regression_and_build: PASS|FAIL|BLOCKED|N/A
  production_readiness: PASS|FAIL|BLOCKED|N/A
  design_and_solid: PASS|FAIL|BLOCKED|N/A
  task_completeness: PASS|FAIL|BLOCKED|N/A
issues_by_severity: {critical: N, warning: N, suggestion: N}
unverified: []
next_recommended: sddk-debt-verify | sddk-apply correction cycle | resolve blocker
risks: []
context_quality: C0|C1|C2|C3
lenses_used: []
skill_resolution: paths-injected | fallback-registry | fallback-path | none
```

On A-* `PASS` or `PASS_WITH_WARNINGS`, next is `sddk-debt-verify`. On B-direct, follow its workflow transition. On `FAIL`, return to correction; never fix inside verify. The coordinator still records failed gate receipts and applies the path-specific verify transition so the CLI moves the cycle to `REMEDIATING/verify`; reporting failure without that ledger mutation is incomplete.

## Ledger Contract (Coordinator Only)

Inspect `sddk cycle status --root . --scope . --cycle {cycle_id} --format
json`. Require matching cycle/path, `status=OPEN`, and `phase=verify`, then select
the path transition:

| Path | Transition |
|---|---|
| A-full | `phase.verify.complete` |
| A-min | `phase.verify.complete.a-min` |
| A-lite | `phase.verify.complete.a-lite` |
| B-direct | `phase.verify.complete.b-direct` |

1. Require `git rev-parse HEAD == head_commit`; recompute report/log hashes.
2. Evaluate `tests-pass` and `policy-compliant` with evidence containing subject
   SHA, result, commands, report path, and report SHA-256. Boolean-only evidence
   is invalid.
3. Transition with `verification-report` and both returned receipt IDs. Append
   lease owner/token only when current cycle status contains a lease; otherwise
   omit both flags.
4. A passing verdict requires transition `outcome=succeeded`. A failure or
   blocked verification requires `outcome=failed`, `status=REMEDIATING`, and
   `phase=verify`.
5. Run `sddk ledger verify --root . --scope .` before returning.

Gate evaluation and transition are required for both pass and fail outcomes. A
CLI error blocks the phase. Renew an expiring live lease before gate evaluation.

## References

- `skills/sddk-verify/SKILL.md`
- `prompts/sddk/phases/strict-tdd-verify.md`
- `skills/_shared/sddk-phase-common.md`
- `docs/research/sddk-verify-agent-practices.md`
