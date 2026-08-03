# SDD Kernel Decision Model v2

Single source of truth for kernel SDD decisions. Replaces `decision-kernel.md`, `context-discipline.md`, and `knowledge-contract.md` (those files are now deprecated; keep them only for back-compat reads).

## Priority Order

The orchestrator runs these gates in order; each can short-circuit.

| # | Gate | Action if missing/blocked |
|---|------|---------------------------|
| 1 | Workspace + execution mode resolved | Ask or assume `auto` |
| 2 | Init context loaded (`sddk/{project}/init`) | Run `sddk-init` first |
| 3 | Triage: classify context quality C0-C3 | Drives path selection (see below) |
| 4 | Jurisprudence lookup (`mem_search` goal_pattern) | If hit → bias path toward prior successful pattern |
| 5 | Path selection (B-direct / A-min / A-lite / A-full) | Drives phase sequence |
| 6 | Lens selection (only if path ≥ A-min) | Use `lens-registry.md` |
| 7 | Lateral thinking config (F3 always on; F1/F4 opt-in) | Default: F3 only |
| 8 | Launch plan produced and validated | Required before each delegation |
| 9 | Pre-flight gates (artifact exists + approved + schema valid) | Block if any fail |
| 10 | Delivery gates (testing capability, review budget) | Block if exceeded |

## Context Quality

| Level | Signal | Recommended effort |
|-------|--------|--------------------|
| C0 | Vague request, no paths, no current behavior, no constraints | `deepen` (one blocking question) |
| C1 | Intent clear, but affected areas, invariants, ownership, or risks missing | `deepen` + selected lenses |
| C2 | Problem, state, areas, constraints, and risks clear | `verify` only (reuses context) |
| C3 | Exploration, specs, ADRs, tests, paths, and invariants explicit | `skip` (lightweight validation) |

## Path Selection (drives phase sequence + coherence gates)

```
B-direct  if: (C3 + jurisprudence_hit) OR user says "just do it" / "fix it"
A-min     if: C2 + scope simple (single apply phase, no architectural fork)
A-lite    if: C1 (default for bounded work)
A-full    if: C0 OR architectural change OR new domain
```

| Path | Phase sequence | Coherence gates | HTML report | Tag |
|------|----------------|-----------------|-------------|-----|
| B-direct | load skill → execute → light verify | 0 | no | patch |
| A-min | spec → apply → verify | 0 (skip if spec simple) | only on minor/major tag | yes |
| A-lite | propose → spec → apply → verify | 1 (apply→verify) | yes | yes |
| A-full | explore → propose → spec\|\|design → tasks → apply → verify → archive | 3 (propose→spec, spec+design→tasks, apply→verify) | yes | yes |

Jurisprudence hits can shorten any path by one phase when the prior cycle ended in PASS with `first_pass_success=true`.

## Knowledge Layers

| Layer | Purpose | Authority |
|-------|---------|-----------|
| Kernel workflow state | Coordinate the change lifecycle | Procedural, current-run only |
| Durable project knowledge | What the project believes is binding | Canonical when fresh |
| Engram memory | Episodic observations, learnings, jurisprudence | Recoverable, never canonical alone |

## Source Hierarchy

`code/tests > specs/tasks > ADRs/architecture docs > CONTEXT glossary > archive reports > Engram memory > chat claims`

Rules:
1. Fresher contradictory evidence wins only if provenance is explicit.
2. Memory can suggest a path; only durable artifacts or verified code/runtime evidence can bind a decision.

## Knowledge States

| State | Allowed Usage |
|-------|---------------|
| `proposed` | Exploration only |
| `trusted` | Routing and implementation input |
| `stale` | Advisory only (must recheck) |
| `superseded` | Historical context only |
| `contradicted` | Escalation trigger |

Never delete knowledge. Supersede or mark stale/contradicted instead.

## Jurisprudence Schema

When a cycle closes with PASS + first_pass_success=true + a reusable decision (ADR, lens, atajo), persist as Engram observation:

```
topic_key: jurisprudence/{category}
title: "{goal_pattern} — {path_that_worked}"
type: jurisprudence
content:
  goal_pattern: "{normalized goal}"
  stack_match: [lang, framework, ...]
  context_quality_typical: C0|C1|C2|C3
  path_that_worked: B-direct|A-min|A-lite|A-full
  lenses_that_mattered: [lens_id, ...]
  typical_duration_hours: float
  typical_cost_usd: float
  correction_cycles_typical: int
  key_learnings: "1-3 sentences"
  reusable: bool
```

At cycle start: `mem_search` goal_pattern → if hit, bias toward `path_that_worked` and `lenses_that_mattered`.

## Authority Matrix (compact)

| Question | Primary Source | Fallback | Never |
|----------|----------------|----------|-------|
| What problem? | roadmap/backlog, approved proposal | recent archive | chat alone |
| What's in scope? | work items, proposal, specs | launch plan | memory alone |
| What must remain true? | specs, ADRs, tests | verified learnings | agent guess |
| Why this design? | ADRs, architecture docs | archive report | implementation alone |
| What happened before? | archive reports, Engram | session summaries | user recollection |

## ADR Threshold

Write an ADR only when all three are true: hard to reverse, surprising without context, real trade-off. Otherwise keep in spec or comment.

## Retrieval Preflight (before routing a meaningful phase)

1. Project init: `sddk/{project}/init`
2. Change-local artifacts
3. Recent archive reports for same change/bounded context
4. Relevant ADRs, architecture docs, specs, tasks
5. Jurisprudence hits + prior verify failures

If a missing class blocks confidence, record the gap. Do not compensate with bigger prompts.

## Context Discipline (CONTEXT.md)

`CONTEXT.md` is a glossary only. Contains canonical terms, tight definitions, avoided synonyms, relationships, flagged ambiguities. Does NOT contain: implementation details, requirements, scenarios, architecture, invariants (unless needed to define a term).

If code and context disagree → surface contradiction, treat as escalation.

## Anti-Patterns

- Treating Engram memory as canonical truth
- Re-explaining whole project in each prompt instead of retrieving
- Editing accepted decisions in place (supersede instead)
- Verify findings die in a report without updating knowledge state
- Inferring ownership from code shape when explicit records exist
- Running full SDDK for a C3 bug fix (use B-direct)
- Coherence check at every transition regardless of context quality
- HTML report for a patch-level tag