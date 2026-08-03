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
- Artifact store mode

## Execution Steps

1. Read the tasks artifact (from `sddk/{change}/tasks`)
2. Read specs (`sddk/{change}/spec`)
3. Read design (`sddk/{change}/design`)
4. Read existing code patterns
5. Implement tasks
6. Update task status
7. Persist progress to `sddk/{change}/apply-progress`
8. Return envelope

Read `prompts/sdd-kernel/phases/apply.md` for the full phase spec.
Read `skills/_shared/sddk-phase-common.md` for common protocol.

## Return Format

- status: success | partial | blocked
- executive_summary: 1-3 sentences
- artifacts: keys written
- next_recommended: next phase
- risks: issues or "None"
- context_quality: C0-C3 level
