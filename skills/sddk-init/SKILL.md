---
name: sddk-init
description: "Trigger: sddk init. Detect SDDK context and testing capabilities without modifying the workspace."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.1"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill via `skill()`, delegate to
> `sddk-init`. Only the `sddk-init` executor continues below.

## Activation Contract

Detect the real stack, conventions, architecture, testing tools, and Strict TDD
mode. The adopted workspace is read-only input.

## Hard Rules

- Query `sddk adopt status`, `sddk knowledge status`, and `sddk knowledge path`
  before detection.
- Never write workspace docs, metadata, ignore files, workflows, registries,
  checkpoints, or caches.
- Never derive `project_id` or `{vault}` from a directory name.
- Store testing capabilities at
  `$SDDK_DATA_DIR/projects/{project_id}/testing-capabilities.yaml` and the init
  report at `{cycle-artifacts-dir}/init.md`.
- Engram is an optional mirror only when the resolved profile enables it. Its
  topic is `sddk/{project_id}/testing-capabilities`.

## Decision Gates

| Input | Action |
|---|---|
| Adoption/profile absent | Return partial and recommend `/sddk-adopt` |
| Explicit Strict TDD config | Use it |
| Test runner exists, no explicit config | `strict_tdd: true` |
| No test runner | `strict_tdd: false` with reason |

## Execution

1. Read any existing XDG capability file.
2. Inspect project and CI files as read-only evidence.
3. Detect test, coverage, lint, type-check, and format commands.
4. Persist the capability file and `{cycle-artifacts-dir}/init.md`.
5. Mirror to Engram only when enabled by `sddk knowledge status`.
6. Return the standard SDDK envelope with resolved paths and next step.

`sddk adopt apply --root . --scope .` is the only initialization fallback. It
registers the project and initializes external state without writing into the
workspace.

## References

- `agents/sddk-init.md`
- `skills/_shared/persistence-contract.md`
- `skills/_shared/sddk-phase-common.md`
