# OpenSpec File Convention (shared across all SDD skills)

## Non-intrusive Base (ADR-0011)

The framework never writes into the project's git repository. The base
directory for all openspec artifacts is the cycle artifact directory resolved
by the CLI (absolute path, outside the repo):

```
sddk cycle artifacts-dir --cycle {cycle_id} --root . --scope .
```

It prints e.g. `~/.local/share/sddk/projects/<project_id>/cycle-artifacts/<cycle_id>/`.
All paths below are relative to that base. If the CLI ledger channel is
inoperative (not adopted), fall back to a local temp dir and note it.

## Directory Structure

```
{cycle-artifacts-dir}/
├── config.yaml              <- Project-specific SDD config
├── specs/                   <- Source of truth (main specs)
│   └── {domain}/
│       └── spec.md
└── changes/                 <- Active changes
    ├── archive/             <- Completed changes (YYYY-MM-DD-{change-name}/)
    └── {change-name}/       <- Active change folder
        ├── state.yaml       <- DAG state (survives compaction)
        ├── exploration.md   <- (optional) from sdd-explore
        ├── proposal.md      <- from sdd-propose
        ├── specs/           <- from sdd-spec
        │   └── {domain}/
        │       └── spec.md  <- Delta spec
        ├── design.md        <- from sdd-design
        ├── tasks.md         <- from sdd-tasks (updated by sdd-apply)
        └── verify-report.md <- from sdd-verify
```

## Artifact File Paths

Relative to `{cycle-artifacts-dir}` (resolved via `sddk cycle artifacts-dir`):

| Skill | Creates / Reads | Path |
|-------|----------------|------|
| orchestrator | Creates/Updates | `changes/{change-name}/state.yaml` |
| sdd-init | Creates | `config.yaml`, `specs/`, `changes/`, `changes/archive/` |
| sdd-explore | Creates (optional) | `changes/{change-name}/exploration.md` |
| sdd-propose | Creates | `changes/{change-name}/proposal.md` |
| sdd-spec | Creates | `changes/{change-name}/specs/{domain}/spec.md` |
| sdd-design | Creates | `changes/{change-name}/design.md` |
| sdd-tasks | Creates | `changes/{change-name}/tasks.md` |
| sdd-apply | Updates | `changes/{change-name}/tasks.md` (marks `[x]`) |
| sdd-verify | Creates | `changes/{change-name}/verify-report.md` |
| sdd-archive | Moves | `changes/{change-name}/` → `changes/archive/YYYY-MM-DD-{change-name}/` |
| sdd-archive | Updates | `specs/{domain}/spec.md` (merges deltas into main specs) |

## Reading Artifacts

```
Proposal:   {cycle-artifacts-dir}/changes/{change-name}/proposal.md
Specs:      {cycle-artifacts-dir}/changes/{change-name}/specs/  (all domain subdirectories)
Design:     {cycle-artifacts-dir}/changes/{change-name}/design.md
Tasks:      {cycle-artifacts-dir}/changes/{change-name}/tasks.md
Verify:     {cycle-artifacts-dir}/changes/{change-name}/verify-report.md
Config:     {cycle-artifacts-dir}/config.yaml
Main specs: {cycle-artifacts-dir}/specs/{domain}/spec.md
```

## Writing Rules

- Always create the change directory before writing artifacts
- If a file already exists, READ it first and UPDATE it (don't overwrite blindly)
- If the change directory already exists with artifacts, the change is being CONTINUED
- Use `config.yaml` `rules` section for project-specific constraints per phase
- NEVER write outside `{cycle-artifacts-dir}` into the project repo (ADR-0011)

## Delta Spec Sections

Delta specs MAY include these sections:

```markdown
## ADDED Requirements
## MODIFIED Requirements
## REMOVED Requirements
## RENAMED Requirements
```

- `ADDED` appends new requirements to the main spec.
- `MODIFIED` replaces the full matching requirement block in the main spec. The delta MUST contain the entire updated requirement, including unchanged scenarios that must be preserved.
- `REMOVED` deletes the matching requirement from the main spec. Each removed requirement MUST include `(Reason: ...)` and SHOULD include `(Migration: ...)` when consumers or persisted behavior are affected.
- `RENAMED` changes a requirement heading/name without changing behavior unless the delta also includes a `MODIFIED` block for the new requirement. Each rename MUST state old and new names explicitly.

## Config File Reference

```yaml
# {cycle-artifacts-dir}/config.yaml
schema: spec-driven

context: |
  Tech stack: {detected}
  Architecture: {detected}
  Testing: {detected}
  Style: {detected}

rules:
  proposal:
    - Include rollback plan for risky changes
  specs:
    - Use Given/When/Then for scenarios
    - Use RFC 2119 keywords (MUST, SHALL, SHOULD, MAY)
  design:
    - Include sequence diagrams for complex flows
    - Document architecture decisions with rationale
  tasks:
    - Group by phase, use hierarchical numbering
    - Keep tasks completable in one session
  apply:
    guidelines:
      - Follow existing code patterns
    tdd: false           # Set to true to enable RED-GREEN-REFACTOR
    test_command: ""
  verify:
    test_command: ""
    build_command: ""
    coverage_threshold: 0
  archive:
    - Warn before merging destructive deltas
```

## Archive Structure

When archiving, the change folder moves to:
```
{cycle-artifacts-dir}/changes/archive/YYYY-MM-DD-{change-name}/
```

Use today's date in ISO format. The archive is an AUDIT TRAIL — never delete or modify archived changes.
