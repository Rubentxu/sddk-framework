---
name: sddk-explore
description: "Trigger: sddk-explore, sddk-new. Investigate codebase and clarify problem taxonomy."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-explore`.

## Executor Override

If you ARE the `sddk-explore` sub-agent, continue. Investigate the topic.

## Activation Contract

Investigate the codebase and think through problems. Compare approaches. By default research and report back; only create `explore-report.md` when this exploration is tied to a named change.

## Hard Rules

- **DO NOT modify any existing code or files.** Read-only investigation.
- **ALWAYS read real code**, never guess about the codebase.
- **Keep analysis CONCISE** — orchestrator needs a summary, not a novel.
- **If you can't find enough information, say so clearly.**
- **If the request is too vague to explore, say what clarification is needed.**
- The ONLY file you MAY create is `explore-report.md` (when tied to a named change).
- Use context quality gates: C0 (unknown) requires full investigation, C3 (known) is minimal.
- Surface ambiguous domain language immediately.
- When code contradicts docs, surface the contradiction and pause.

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Resolve knowledge sources: roadmap/backlog, ADRs, architecture docs, ownership, learnings.
3. Run preflight checks.
4. Apply adaptive lenses (if any).
5. Execute mandatory protocols (persistence, context discipline, entropy envelope).
6. Return exploration findings.

### Investigation Method

```
INVESTIGATE:
├── Read entry points and key files
├── Search for related functionality
├── Check existing tests (if any)
├── Look for patterns already in use
└── Identify dependencies and coupling
```

### Approach Comparison (when multiple approaches viable)

| Approach | Pros | Cons | Complexity |
|----------|------|------|------------|
| Option A | ... | ... | Low/Med/High |
| Option B | ... | ... | Low/Med/High |

## Return Format (use this structure)

```markdown
## Exploration: {topic}

### Current State
{How the system works today relevant to this topic}

### Affected Areas
- `path/to/file.ext` — {why it's affected}
- `path/to/other.ext` — {why it's affected}

### Approaches
1. **{Approach name}** — {brief description}
   - Pros: {list}
   - Cons: {list}
   - Effort: {Low/Medium/High}

2. **{Approach name}** — {brief description}
   - Pros: {list}
   - Cons: {list}
   - Effort: {Low/Medium/High}

### Recommendation
{Your recommended approach and why}

### Risks
- {Risk 1}
- {Risk 2}

### Ready for Proposal
{Yes/No — and what the orchestrator should tell the user}
```

Plus the standard envelope:

- status: success | partial | blocked
- executive_summary
- context_quality: C0-C3
- taxonomy: dominant axes identified
- artifacts: any findings saved
- next_recommended
- risks

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record this phase in the cycle ledger BEFORE returning:

1. Evaluate the phase gate:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition phase.explore.complete --gate exploration-sufficient --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
2. Transition with the phase artifact (`explore-report.md`; in `engram` mode materialize it to a temp file first):
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition phase.explore.complete --artifact exploration-report={path} --gate-receipt {receipt_id} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed. `{cycle_id}`, `{lease_owner}`, `{fencing_token}` come from the orchestrator launch prompt (the cycle is opened with `sddk cycle start`). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sddk/phases/explore.md` — full phase spec
- `prompts/sddk/decision-model.md` — context quality + taxonomy
- `skills/_shared/sddk-phase-common.md` — shared protocol
