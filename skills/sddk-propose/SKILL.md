---
name: sddk-propose
description: "Trigger: sddk-new, sddk-propose. Create adaptive change proposal from exploration."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-propose`.

## Executor Override

If you ARE the `sddk-propose` sub-agent, continue. Create the proposal.

## Activation Contract

Take the exploration analysis (or direct user input) and produce a structured proposal document. The proposal is the CONTRACT between this phase and sddk-spec — its **Capabilities section** tells sddk-spec exactly which spec files to create or update.

## Hard Rules

- ALWAYS include the **Capabilities** section — it is the contract with sddk-spec.
- Research existing capabilities (e.g., `openspec/specs/`) BEFORE writing Capabilities — use correct existing names.
- Every proposal MUST have a **Rollback Plan** and **Success Criteria**.
- Use concrete file paths in Affected Areas.
- If existing proposal found, READ first and UPDATE.
- Apply any `rules.proposal` from `openspec/config.yaml`.
- **Size budget**: proposal MUST be under 450 words. Bullets and tables over prose.

## Capabilities Section Rules (the contract)

```
### New Capabilities
<!-- Each becomes a new openspec/specs/<name>/spec.md (full spec).
     Use kebab-case (e.g., user-auth, data-export). -->

### Modified Capabilities
<!-- Each becomes a delta spec. Existing requirements are CHANGING (not just implementation).
     Use existing spec names from openspec/specs/. -->
```

- If nothing changes at spec level (pure refactor, config), explicitly write "None" under both — don't leave placeholders.
- Use Existing Capability Names: research `openspec/specs/` first.

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Read exploration findings (if provided).
3. Define scope, approach, invariants, explicit unknowns.
4. **Write Capabilities section** (this is the contract with sddk-spec).
5. Identify knowledge gaps requiring escalation.
6. Persist to `{cycle-artifacts-dir}/proposal`.
7. Return envelope.

## Proposal Template (use this exact structure)

```markdown
# Proposal: {Change Title}

## Intent
{What problem are we solving? Why? Be specific about user need or technical debt.}

## Scope

### In Scope
- {Concrete deliverable 1}
- {Concrete deliverable 2}

### Out of Scope
- {What we're explicitly NOT doing}
- {Future work deferred}

## Capabilities

> This section is the CONTRACT between proposal and specs phases.
> Research `openspec/specs/` before filling this in.

### New Capabilities
<!-- Each becomes a new openspec/specs/<name>/spec.md. Kebab-case. -->
- `<capability-name>`: <brief description>

### Modified Capabilities
<!-- Existing capabilities whose REQUIREMENTS are changing. Each needs a delta spec. -->
- `<existing-capability-name>`: <what requirement is changing>

## Approach
{High-level technical approach. Reference recommended approach from exploration if available.}

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `path/to/area` | New/Modified/Removed | {What changes} |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| {Risk} | Low/Med/High | {Mitigation} |

## Rollback Plan
{How to revert. Be specific.}

## Dependencies
- {External dependency or prerequisite}

## Success Criteria
- [ ] {How do we know this change succeeded?}
- [ ] {Measurable outcome}
```

## Return Format

```markdown
**Status**: success
**Summary**: Proposal created for `{change-name}`. Defined scope, approach, and rollback plan.
**Artifacts**: Engram `{cycle-artifacts-dir}/proposal` | `{cycle-artifacts-dir}/proposal.md`
**Change**: {change-name}
**Capabilities**:
- New: {N} (each will become openspec/specs/<name>/spec.md)
- Modified: {M} (each will become a delta spec)
**Risk Level**: {Low/Medium/High}
**Next**: sddk-spec
**Risks**: {list or "None"}
```

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), register the proposal artifact in the cycle ledger BEFORE returning (proposal has no own workflow transition — it feeds the specify transition):

```
sddk artifact store --root . --scope . --file {proposal-file} --kind proposal --cycle {cycle_id} --producer sddk-kernel
sddk ledger verify --root . --scope .
```

In `engram` mode, materialize the proposal to a temp file first. A failed store is a BLOCKER: report it in the envelope and do not proceed. Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sdd-kernel/phases/propose.md` — full phase spec
- `prompts/sdd-kernel/decision-model.md` — context quality, path selection
- `skills/_shared/sddk-phase-common.md` — shared protocol