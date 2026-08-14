---
name: sddk-design
description: "Trigger: sddk-design. Create adaptive technical designs from proposals."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-design`.

## Executor Override

If you ARE the `sddk-design` sub-agent, continue. Create the design.

## Activation Contract

Take the proposal + spec and produce a design document. The design captures **HOW** the change will be implemented — architecture decisions, data flow, file changes, and technical rationale.

## Hard Rules

- ALWAYS read the actual codebase before designing — never guess.
- Every decision MUST have a rationale (the "why").
- Include concrete file paths, not abstract descriptions.
- Use the project's ACTUAL patterns and conventions, not generic best practices.
- If the codebase uses a pattern different from what you'd recommend, note it but FOLLOW the existing pattern unless the change specifically addresses it.
- Keep ASCII diagrams simple — clarity over beauty.
- Follow the design patterns established in the project.
- If open questions BLOCK the design, say so clearly — don't guess.
- **Size budget**: design MUST be under 800 words. Decisions as tables (option | tradeoff | decision). Code snippets only for non-obvious patterns.

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Read proposal from `{cycle-artifacts-dir}/proposal`.
3. Read spec from `{cycle-artifacts-dir}/spec`.
4. Read the actual codebase for affected files, patterns, conventions.
5. Write design.md with the template below.
6. Identify ADRs to create (architectural decisions).
7. Persist to `{cycle-artifacts-dir}/design`.
8. Return envelope.

## Design Template (use this exact structure)

```markdown
# Design: {Change Title}

## Technical Approach

{Concise description of the overall technical strategy.
How does this map to the proposal's approach? Reference specs.}

## Architecture Decisions

### Decision: {Decision Title}

**Choice**: {What we chose}
**Alternatives considered**: {What we rejected}
**Rationale**: {Why this choice over alternatives}

### Decision: {Decision Title}

**Choice**: {What we chose}
**Alternatives considered**: {What we rejected}
**Rationale**: {Why this choice over alternatives}

## Data Flow

{Describe how data moves through the system for this change.
Use ASCII diagrams when helpful.}

    Component A ──→ Component B ──→ Component C
         │                              │
         └──────── Store ───────────────┘

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `path/to/new-file.ext` | Create | {What this file does} |
| `path/to/existing.ext` | Modify | {What changes and why} |
| `path/to/old-file.ext` | Delete | {Why it's being removed} |

## Interfaces / Contracts

{Define any new interfaces, API contracts, type definitions, or data structures.
Use code blocks with the project's language.}

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | {What} | {How} |
| Integration | {What} | {How} |
| E2E | {What} | {How} |

## Migration / Rollout

{If this change requires data migration, feature flags, or phased rollout, describe the plan.
If not applicable, state "No migration required."}

## Open Questions

- [ ] {Any unresolved technical question}
- [ ] {Any decision that needs team input}
```

## ADR Candidates

While writing the design, flag decisions that meet ALL three ADR criteria:
- Hard to reverse
- Surprising without context
- Result of a real trade-off

List them in a `## ADR Candidates` section. The orchestrator creates the actual ADR files in Step 1.4 of the MCW.

## Conditional Capabilities (deployed by orchestrator when justified)

These are NOT loaded by default. The orchestrator injects them based on the launch plan's adaptive_lenses and context_quality:

| Capability | When to inject | Skill/integration |
|------------|----------------|-------------------|
| **CogniCode index** (entry points, hot paths, architecture check) | When `taxonomy` includes `coupling_connascence` or `boundary_seam`, OR `context_quality ≤ C2` | `cognicode-sdd` skill |
| **Chronos runtime evidence** | When `taxonomy` includes runtime bug / perf / race condition | `chronos-sdd` skill |
| **Web search multi-provider** | When proposal references external APIs / libraries / RFCs | `minimax-mcp` + `zai-mcp` skills |
| **Entropy-sdd heuristics** | When `recommended_effort ≥ deepen` OR `context_quality ≤ C2` | `entropy-sdd` skill |
| **Domain-modeling lens** | When `taxonomy` axis `domain_modeling` is active | `auto-grill-loop` if ambiguity |

If none apply, proceed without them. Token economy is a feature.

## Return Format

```markdown
## Design Created

**Change**: {change-name}
**Location**: `$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/design.md`

### Summary
- **Approach**: {one-line technical approach}
- **Key Decisions**: {N decisions documented}
- **Files Affected**: {N new, M modified, K deleted}
- **Testing Strategy**: {unit/integration/e2e coverage planned}
- **ADR Candidates**: {N architectural decisions flagged}

### Open Questions
{List any unresolved questions, or "None"}

### Next Step
Ready for tasks (sddk-tasks).
```

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record this phase in the cycle ledger BEFORE returning:

1. Evaluate the phase gate:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition phase.design.complete --gate architecture-consistent --outcome passed --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
2. Transition with the phase artifact (`design.md`; in `engram` mode materialize it to a temp file first):
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition phase.design.complete --artifact design={path} --gate-receipt {receipt_id} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed. `{cycle_id}`, `{lease_owner}`, `{fencing_token}` come from the orchestrator launch prompt (the cycle is opened with `sddk cycle start`). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sddk/phases/design.md` — full phase spec
- `prompts/sddk/decision-model.md` — knowledge contract
- `prompts/sddk/lens-registry.md` — available lenses
- `skills/_shared/sddk-phase-common.md` — shared protocol
- `prompts/sddk/adr-template.md` — ADR format
