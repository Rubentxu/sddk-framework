# SDD Kernel

This folder belongs to the advanced `orchestrator` SDD flow. It must not be used by `gentle-orchestrator` or the traditional `/sdd-*` commands.

## Boundary

Traditional SDD stays in the existing structure:
- `gentle-orchestrator`
- `commands/sdd-*.md`
- `prompts/sdd/*.md`
- `skills/sdd-*`
- `skills/_shared/sdd-phase-common.md`

Kernel SDD owns a separate structure:
- `orchestrator`
- `commands/sddk-*.md`
- `prompts/sdd-kernel/**`
- `sdd-kernel-*` phase agents
- future `skills/sdd-kernel-*` skills, if reusable skills are needed

Do not share executable phase prompts or shared runtime files between traditional SDD and kernel SDD. Shared conceptual references are acceptable only when they are read-only and explicitly imported by a flow.

## Design Goal

Kernel SDD starts from the traditional SDD sequence, but adds an explicit decision kernel:

```text
session gates
  -> context quality gate
  -> problem taxonomy
  -> mandatory protocols
  -> adaptive lenses
  -> escalation engine
  -> delivery gates
```

The objective is not to run more agents. The objective is to decide when extra context, entropy analysis, architecture lenses, or grilling are worth their cost.

## Non-Goals

- Do not mutate traditional SDD prompts.
- Do not modify `gentle-orchestrator` behavior.
- Do not put kernel rules in global `AGENTS.md`.
- Do not make `entropy-sdd`, `auto-grill`, or `grill-with-docs` globally mandatory.
- Do not use `CONTEXT.md` as a spec, scratch pad, or architecture report.

## Migration Order

1. Keep traditional SDD untouched and working.
2. Evolve kernel-only phase prompts and shared contracts under this folder.
3. Evolve kernel-only phase agents: `sdd-kernel-explore`, `sdd-kernel-propose`, etc.
4. Use `sddk-*` commands for explicit kernel flow entry points.
5. Compare both flows in real use before removing or deprecating anything.
