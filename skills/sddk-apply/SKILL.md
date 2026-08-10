---
name: sddk-apply
description: "Trigger: orchestrator launches sddk-apply for one or more change tasks."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "1.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-apply`.

## Executor Override

If you ARE the `sddk-apply` sub-agent, continue. Implement the assigned tasks.

## What You Receive

From the orchestrator:
- Change name
- Specific tasks to implement
- Cycle artifacts directory and knowledge profile

## Execution Steps

1. Read the tasks artifact (from `{cycle-artifacts-dir}/tasks`)
2. Read specs (`{cycle-artifacts-dir}/spec`)
3. Read design (`{cycle-artifacts-dir}/design`)
4. Read existing code patterns
5. Implement tasks
6. Update task status
7. Persist progress to `{cycle-artifacts-dir}/apply-progress`
8. Return envelope

Read `prompts/sddk/phases/apply.md` for the full phase spec.
Read `skills/_shared/sddk-phase-common.md` for common protocol.

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record this phase in the cycle ledger BEFORE returning:

1. Evaluate the phase gate:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition phase.build.complete --gate implementation-complete --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
2. Transition with the phase artifact (implementation receipt; in `engram` mode materialize it to a temp file first):
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition phase.build.complete --artifact implementation-receipt={path} --gate-receipt {receipt_id} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed. `{cycle_id}`, `{lease_owner}`, `{fencing_token}` come from the orchestrator launch prompt (the cycle is opened with `sddk cycle start`). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## Return Format

- status: success | partial | blocked
- executive_summary: 1-3 sentences
- artifacts: keys written
- next_recommended: next phase
- risks: issues or "None"
- context_quality: C0-C3 level
