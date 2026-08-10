---
name: sddk-tasks
description: "Trigger: sddk-tasks. Break down specs and designs into implementation tasks."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-tasks`.

## Executor Override

If you ARE the `sddk-tasks` sub-agent, continue. Create tasks.

## Activation Contract

Take the proposal + spec + design and produce a `tasks.md` with concrete, actionable implementation steps organized by phase. The **Review Workload Forecast** is part of the artifact — downstream guards (sddk-apply, orchestrator's MCW Step 1.7) match its exact plain-text lines literally.

## Hard Rules

- ALWAYS reference concrete file paths in tasks.
- Tasks MUST be ordered by dependency — Phase 1 tasks shouldn't depend on Phase 2.
- Each task should be completable in ONE session (if too big, split).
- Use hierarchical numbering: 1.1, 1.2, 2.1, 2.2, etc.
- NEVER include vague tasks like "implement feature" or "add tests".
- Follow the established project conventions and patterns.
- If project uses TDD, integrate test-first tasks: RED → GREEN → REFACTOR.
- **Size budget**: tasks MUST be under 530 words. Each task: 1-2 lines max.
- **Review workload guard**: ALWAYS include the Review Workload Forecast with exact plain-text lines.

## Task Writing Rules (anti-patterns)

| Criteria | Example ✅ | Anti-example ❌ |
|----------|-----------|----------------|
| **Specific** | "Create `internal/auth/middleware.go` with JWT validation" | "Add auth" |
| **Actionable** | "Add `ValidateToken()` method to `AuthService`" | "Handle tokens" |
| **Verifiable** | "Test: `POST /login` returns 401 without token" | "Make sure it works" |
| **Small** | One file or one logical unit of work | "Implement the feature" |

## Review Workload Forecast (MANDATORY — exact plain-text contract)

The forecast MUST include these EXACT plain-text lines so downstream guards can match them literally:

```text
Decision needed before apply: Yes|No
Chained PRs recommended: Yes|No
Chain strategy: stacked-to-main|feature-branch-chain|size-exception|pending
400-line budget risk: Low|Medium|High
```

Plus the readable table (for human eyes). Both must be present.

### Forecast Algorithm

Estimate whether implementation is likely to exceed the **400 changed-line review budget** (`additions + deletions`).

Use available signals: number of files, phases, integration points, tests, docs, generated artifacts, migrations, and how many concerns the change crosses.

If the estimate is **High** or likely above 400 lines:

1. Mark `Chained PRs recommended` as `Yes`.
2. Split tasks into **work units** that can become chained or stacked PRs.
3. Each suggested PR must have clear start, finish, verification, autonomous scope.
4. **Ask the user which chain strategy to use**:
   - **Stacked PRs to main** — each PR merges to main in order. Fast iteration.
   - **Feature Branch Chain** — feature/tracker branch accumulates; PR #1 targets tracker, later PRs target immediate previous. Only tracker merges to main.
   - **size:exception** — single PR with maintainer approval. Best for generated code, migrations, vendor diffs.
5. Set `Decision needed before apply` from delivery strategy:
   - `ask-on-risk`: `Yes`
   - `auto-chain`: `No`
   - `single-pr`: `Yes`
   - `exception-ok`: `No`

### Work Units (when chained/stacked)

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | <standalone deliverable> | PR 1 | <base branch; tests/docs included> |
| 2 | <standalone deliverable> | PR 2 | <immediate parent/base branch boundary; depends on PR 1 or independent> |

For `feature-branch-chain`, work units SHOULD name the intended base boundary: PR #1 base = feature/tracker branch; PR #2 base = PR #1 branch; PR #3 base = PR #2 branch.

## Phase Organization

```
Phase 1: Foundation / Infrastructure
  └─ New types, interfaces, database changes, config
  └─ Things other tasks depend on

Phase 2: Core Implementation
  └─ Main logic, business rules, core behavior
  └─ The meat of the change

Phase 3: Integration / Wiring
  └─ Connect components, routes, UI wiring

Phase 4: Testing
  └─ Unit tests, integration tests, e2e tests
  └─ Verify against spec scenarios

Phase 5: Cleanup (if needed)
  └─ Documentation, remove dead code, polish
```

## Tasks Template

```markdown
# Tasks: {Change Title}

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | <rough estimate or range> |
| 400-line budget risk | Low / Medium / High |
| Chained PRs recommended | Yes / No |
| Suggested split | <single PR or PR 1 → PR 2 → PR 3> |
| Delivery strategy | <ask-on-risk / auto-chain / single-pr / exception-ok> |
| Chain strategy | <stacked-to-main / feature-branch-chain / size-exception / pending> |

Decision needed before apply: <Yes|No>
Chained PRs recommended: <Yes|No>
Chain strategy: <stacked-to-main|feature-branch-chain|size-exception|pending>
400-line budget risk: <Low|Medium|High>

### Suggested Work Units
(if Chained PRs recommended = Yes)

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | <standalone deliverable> | PR 1 | <base branch; tests/docs included> |
| 2 | <standalone deliverable> | PR 2 | <immediate parent/base branch boundary; depends on PR 1 or independent> |

## Phase 1: {Phase Name} (e.g., Infrastructure / Foundation)

- [ ] 1.1 {Concrete action — what file, what change}
- [ ] 1.2 {Concrete action}
- [ ] 1.3 {Concrete action}

## Phase 2: {Phase Name} (e.g., Core Implementation)

- [ ] 2.1 {Concrete action}
- [ ] 2.2 {Concrete action}
- [ ] 2.3 {Concrete action}
- [ ] 2.4 {Concrete action}

## Phase 3: {Phase Name} (e.g., Testing / Verification)

- [ ] 3.1 {Write tests for ...}
- [ ] 3.2 {Write tests for ...}
- [ ] 3.3 {Verify integration between ...}

## Phase 4: {Phase Name} (e.g., Cleanup / Documentation)

- [ ] 4.1 {Update docs/comments}
- [ ] 4.2 {Remove temporary code}
```

## TDD Task Integration

When Strict TDD Mode is active, integrate test-first tasks:

```
Phase 2: Core Implementation
- [ ] 2.1 RED: Write failing test for {behavior} — {test file path}
- [ ] 2.2 GREEN: Implement minimum code to pass — {production file path}
- [ ] 2.3 REFACTOR: Clean up — {production file path}
```

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Read proposal, spec, design.
3. Analyze design → identify files, dependencies, testing requirements.
4. Estimate review workload (400-line budget).
5. Write tasks.md with template above (including EXACT plain-text forecast lines).
6. If chained PRs recommended: write Work Units table.
7. Persist to `{cycle-artifacts-dir}/tasks`.
8. Return envelope.

## Return Format

```markdown
## Tasks Created

**Change**: {change-name}
**Location**: `$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/tasks.md`

### Breakdown

| Phase | Tasks | Focus |
|-------|-------|-------|
| Phase 1 | {N} | {Phase name} |
| Phase 2 | {N} | {Phase name} |
| Phase 3 | {N} | {Phase name} |
| Total | {N} | |

### Implementation Order
{Brief description of recommended order and why}

### Review Workload Forecast
- Estimated changed lines: {estimate or range}
- 400-line budget risk: {Low | Medium | High}
- Chained PRs recommended: {Yes | No}
- Delivery strategy: {ask-on-risk | auto-chain | single-pr | exception-ok}
- Decision needed before apply: {Yes | No}
- Chain strategy: {stacked-to-main | feature-branch-chain | size-exception | pending}
- Suggested work-unit PR split: {brief list or "Not needed"}

### Next Step
{Ready for implementation (sddk-apply) OR ask the user whether to use chained PRs before sddk-apply.}
```

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record this phase in the cycle ledger BEFORE returning:

1. Evaluate the phase gate:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition phase.plan.complete --gate plan-executable --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
2. Transition with the phase artifact (`tasks.md`; in `engram` mode materialize it to a temp file first):
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition phase.plan.complete --artifact implementation-plan={path} --gate-receipt {receipt_id} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed. `{cycle_id}`, `{lease_owner}`, `{fencing_token}` come from the orchestrator launch prompt (the cycle is opened with `sddk cycle start`). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sddk/phases/tasks.md` — full phase spec
- `prompts/sddk/mcw.md` Step 1.7 (Review Budget Guard)
- `skills/_shared/sddk-phase-common.md` — shared protocol
