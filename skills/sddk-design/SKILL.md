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
- Apply any `rules.design` from `openspec/config.yaml`.
- If open questions BLOCK the design, say so clearly — don't guess.
- **Size budget**: design MUST be under 800 words. Decisions as tables (option | tradeoff | decision). Code snippets only for non-obvious patterns.

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Read proposal from `sddk/{change}/proposal`.
3. Read spec from `sddk/{change}/spec`.
4. Read the actual codebase for affected files, patterns, conventions.
5. Write design.md with the template below.
6. Identify ADRs to create (architectural decisions).
7. Persist to `sddk/{change}/design`.
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
**Location**: `openspec/changes/{change-name}/design.md` (openspec/hybrid) | Engram `sddk/{change-name}/design` (engram) | inline (none)

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

## References

- `prompts/sdd-kernel/phases/design.md` — full phase spec
- `prompts/sdd-kernel/decision-model.md` — knowledge contract
- `prompts/sdd-kernel/lens-registry.md` — available lenses
- `skills/_shared/sddk-phase-common.md` — shared protocol
- `prompts/sdd-kernel/adr-template.md` — ADR format