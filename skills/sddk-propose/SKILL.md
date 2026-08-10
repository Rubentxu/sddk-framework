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
- Every proposal MUST have a **Rollback Plan** and **Success Criteria**.
- Use concrete file paths in Affected Areas.
- If existing proposal found, READ first and UPDATE.
- **Size budget**: proposal MUST be under 450 words. Bullets and tables over prose.

## Capabilities Section Rules (the contract)

```
### New Capabilities
<!-- Each becomes a new full spec in XDG: $SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/specs/<name>/spec.md -->

### Modified Capabilities
<!-- Each becomes a delta spec. Existing requirements are CHANGING (not just implementation). -->
```

- If nothing changes at spec level (pure refactor, config), explicitly write "None" under both — don't leave placeholders.

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Read exploration findings (if provided).
3. Define scope, approach, invariants, explicit unknowns.
4. **Write Capabilities section** (this is the contract with sddk-spec).
5. Identify knowledge gaps requiring escalation.
6. Persist to `$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/proposal.md`
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

### New Capabilities
<!-- Each becomes a new full spec. Use kebab-case names. -->
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
**Artifacts**: `$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/proposal.md`
**Change**: {change-name}
**Capabilities**:
- New: {N}
- Modified: {M}
**Risk Level**: {Low/Medium/High}
**Next**: sddk-spec
**Risks**: {list or "None"}
```

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), register the proposal artifact in the cycle ledger BEFORE returning:

```
sddk artifact store --root . --scope . --file {proposal-file} --kind proposal --cycle {cycle_id} --producer sddk-kernel
sddk ledger verify --root . --scope .
```

A failed store is a BLOCKER: report it in the envelope and do not proceed.

## References

- `prompts/sddk/phases/propose.md` — full phase spec
- `prompts/sddk/decision-model.md` — context quality, path selection
- `skills/_shared/sddk-phase-common.md` — shared protocol
