# SDDK Persistence Contract

This contract applies to every active SDDK agent, skill, and prompt.

## Directory Authorities

| Context | Authority | Write policy |
|---|---|---|
| Framework development repo | Current `sddk-framework/` checkout | Framework sources, commits, releases, and explicit `--in-repo` dogfooding only |
| Runtime bundle | `$SDDK_DATA_DIR/framework/<version>/` | Installed snapshot and editor links; never edit as source |
| Adopted workspace | Product repository | Read product code and existing product docs as evidence; SDDK writes nothing |
| Durable knowledge | `{vault}` from `sddk knowledge path` | Milestones, ADRs, requirements, cycles, incidences, and terms |
| Operational state | `$SDDK_DATA_DIR/projects/<project_id>/` | Receipt, CAS, `{cycle-artifacts-dir}`, generated output, and project operational data |
| Engram | Optional parallel memory | Mirror only when the knowledge profile enables it; never artifact authority |

The only optional SDDK configuration in an adopted workspace is
`.sddk-versions`. The developer owns it; SDDK never creates or edits it.

## Zero Intrusion

SDDK MUST NOT create or modify repository-local framework state in an adopted
workspace. This includes `docs/`, `CONTEXT.md`, `CONTEXT-MAP.md`, ROADMAPs,
ADRs, specs, `sddk/`, `.sddk/`, `.atl/`, `.ignore`, `.gitignore`, workflow
manifests, checkpoints, reports, or cycle artifacts.

Pre-existing product documentation may be read as evidence. It is read-only
input and never becomes SDDK authority. Generated documentation goes to
`$SDDK_DATA_DIR/projects/<project_id>/generated/`. `--in-repo` is reserved for
explicit dogfooding of the framework development repo, never an adopted
workspace.

## Resolve Once

At phase start, query both authorities:

```bash
sddk knowledge status --root . --scope . --format json
VAULT=$(sddk knowledge path --root . --scope .)
```

Never reconstruct `{vault}` or `<project_id>` from a directory basename, git
checkout name, environment guess, or hard-coded home path. The orchestrator
passes the CLI-resolved `{vault}`, `{project_id}`, and
`{cycle-artifacts-dir}` to phase executors.

## Artifact Routing

- Durable project knowledge is written under `{vault}`.
- Proposal, spec, design, tasks, apply progress, verification, debt, archive,
  release, and HTML reports are written under `{cycle-artifacts-dir}`.
- CLI transitions receive those XDG paths as artifact inputs.
- `/tmp` may hold a disposable presentation copy. It is never authoritative.
- Engram mirrors use `sddk/{change-name}/{artifact-type}` topic keys only when
  `sddk knowledge status` reports `engram_enabled: true`.

## CLI Ledger

When adopted, the orchestrator opens or resumes a cycle and passes its
`cycle_id` and `{cycle-artifacts-dir}` to every phase. Each phase stores or
transitions its XDG artifact, then verifies the ledger:

```bash
sddk cycle status --root . --scope . --cycle {cycle_id}
sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} \
  --transition {phase-transition} --gate {gate} --outcome passed \
  --evaluator sddk.cli --evidence '{"checked": true}'
sddk cycle transition --root . --scope . --cycle {cycle_id} \
  --transition {phase-transition} --artifact {artifact-name}={artifact-path} \
  --gate-receipt {receipt_id}
sddk ledger verify --root . --scope .
```

A failed artifact store, gate, transition, or ledger verification blocks the
phase. It never falls back to repository-local files or Engram-only state.

## Optional Engram Mirror

If and only if the resolved knowledge profile enables Engram, mirror the full
artifact after the XDG write:

```text
mem_save(
  title: "sddk/{change-name}/{artifact-type}",
  topic_key: "sddk/{change-name}/{artifact-type}",
  type: "architecture",
  project: "{project}",
  capture_prompt: false,
  content: "{full artifact markdown}"
)
```

Downstream phases always read `{cycle-artifacts-dir}` and `{vault}` first.
Engram is for recovery and search, not pipeline authority.
