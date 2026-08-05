---
name: sddk-init
description: "Trigger: sddk init. Initialize SDDK context, testing capabilities, registry, and persistence."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill via the `skill()` tool, you are the ORCHESTRATOR — STOP. Do NOT execute inline. Delegate to the `sddk-init` sub-agent.

## Executor Override

If you ARE the `sddk-init` sub-agent, continue below. Do not delegate.

## Activation Contract

Run this phase when the orchestrator/user asks to initialize SDDK in a project. Detect the real stack, conventions, architecture, testing tools, and persistence mode. Never guess.

## Hard Rules

- Detect, don't guess. Inspect project files (`package.json`, `go.mod`, `pyproject.toml`, CI configs, lint/test config) before declaring stack.
- In `engram` mode, do **not** create `openspec/`.
- In `openspec` mode, follow `openspec-convention.md` and write file artifacts.
- In `hybrid` mode, write both openspec files and Engram observations.
- Always persist testing capabilities separately as `sddk/{project}/testing-capabilities`.
- Always build `.atl/skill-registry.md`; also save `skill-registry` to Engram when available.
- Use `capture_prompt: false` for automated SDDK saves.
- If `openspec/` already exists, report what exists and ask before updating it.

## Decision Gates

| Input | Action |
|---|---|
| `mode=engram` | Save context and capabilities to Engram only. |
| `mode=openspec` | Create/update openspec bootstrap files only. |
| `mode=hybrid` | Do both Engram and openspec persistence. |
| `mode=none` | Return detected context only; write no SDDK artifacts except registry if required. |
| strict TDD marker/config found | Use that value. |
| no marker/config but test runner exists | Default `strict_tdd: true`. |
| no test runner | Set `strict_tdd: false` and explain unavailable. |

## Testing Capability Detection (priority order)

Detect in this order:

1. **Cached capabilities** (from prior init): `mem_search("sddk/{project}/testing-capabilities")`
2. **openspec config**: read `openspec/config.yaml` `testing:` section
3. **Project files**:
   - JS/TS: `package.json` scripts + presence of `vitest`, `jest`, `mocha`, `playwright`
   - Python: `pyproject.toml` or `pytest.ini` or `setup.cfg`
   - Go: `go.mod` + `*_test.go` files
   - Rust: `Cargo.toml` `[dev-dependencies]` + `#[cfg(test)]`
4. **Fallback**: if nothing found, `strict_tdd: false`

What to capture:
- `test_runner.command` (e.g., `pnpm vitest run`, `pytest`, `go test ./...`)
- `test_layers`: [unit, integration, e2e] — which are available
- `coverage.command` (e.g., `pnpm vitest --coverage`, `pytest --cov`)
- `linter.command` (e.g., `eslint`, `ruff`, `golangci-lint`)
- `type_checker.command` (e.g., `tsc --noEmit`, `mypy`)
- `formatter.command` (e.g., `prettier`, `black`, `gofmt`)

## Execution Steps

1. Inspect project files — summarize stack/conventions.
2. Detect test runner, layers, coverage, linter, type checker, formatter (priority order above).
3. Resolve Strict TDD from agent marker, openspec config, detected runner fallback, or no-runner fallback.
4. Initialize persistence for the resolved mode.
5. Build `.atl/skill-registry.md` using the skill-registry scan rules.
6. Persist testing capabilities and project context.
7. Return envelope.

## Output Contract

Return `status`, `executive_summary`, `artifacts`, `next_recommended`, `risks`.

Include:
- **Project**: name
- **Stack**: detected languages/frameworks
- **Persistence mode**: resolved
- **Strict TDD**: `true | false` + reason
- **Testing capability table**: layer / command / available
- **Saved observation IDs/paths**: where things live
- **Registry path**: `.atl/skill-registry.md`
- **Next step**: `/sddk-explore` or `/sddk-new`

```markdown
**Status**: success
**Summary**: Initialized SDDK for project `{project}`. Detected stack, cached testing capabilities, built skill registry.
**Artifacts**: Engram `sddk/{project}/init` + `sddk/{project}/testing-capabilities` | `.atl/skill-registry.md`
**Stack**: {languages, frameworks, build tools}
**Strict TDD**: {true|false} ({reason})
**Test Runner**: {command}
**Test Layers**: {unit, integration, e2e}
**Next**: /sddk-explore or /sddk-new
**Risks**: None
```

## CLI Contract (sddk ledger)

When the project is NOT yet adopted, adopt it so the cycle ledger becomes operative:

```
sddk adopt apply --root . --scope .
```

`adopt apply` plants `workflow/workflow.yaml` (canonical, embedded in the binary) and registers the project in the ledger. Verify with `sddk cycle status --root . --scope .`. Init has no workflow transition — the ledger duty starts at the first phase (explore). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sdd-kernel/phases/init.md` — full phase spec
- `prompts/sdd-kernel/decision-model.md` — context quality, path selection, jurisprudence
- `skills/_shared/sddk-phase-common.md` — shared SDDK protocol