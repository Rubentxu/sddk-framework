---
name: sddk-archive
description: "Trigger: sddk-archive. Archive completed kernel change and sync delta specs."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-archive`.

## Executor Override

If you ARE the `sddk-archive` sub-agent, continue. Archive the change.

## Activation Contract

Merge delta specs into the main specs (source of truth), then move the change folder to archive. You complete the SDD cycle. The **delta spec sync** is the critical operation — without it, the main specs never reflect the new behavior and the system loses the audit trail of what changed.

## Hard Rules

- **NEVER archive a change with CRITICAL issues** in its verification report.
- **ALWAYS sync delta specs BEFORE moving to archive.**
- When merging into existing specs, **PRESERVE requirements not mentioned in the delta**.
- Use ISO date format (`YYYY-MM-DD`) for archive folder prefix.
- If the merge would be destructive (removing large sections), **WARN the orchestrator and ask for confirmation**.
- The archive is an **AUDIT TRAIL** — never delete or modify archived changes.

## Delta Spec Sync (the core operation)

For each delta spec in `$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/specs/`:

### If Main Spec Exists

Sync the delta spec into the main spec:
- **ADDED Requirements** → Append to main spec's Requirements section
- **MODIFIED Requirements** → Replace the matching requirement (match by Requirement name; preserve all OTHER requirements)
- **REMOVED Requirements** → Delete the matching requirement from main spec

**Match by Requirement name** (e.g., `### Requirement: Session Expiration`). Preserve all OTHER requirements that aren't in the delta. Maintain proper Markdown formatting and heading hierarchy.

**Why copy-full-then-edit (recall from sddk-spec):**
- The archive step REPLACES the requirement in main specs with the MODIFIED block
- If the MODIFIED block is partial (only the changed scenario), the archive loses other scenarios
- Common pitfall: only writing the changed scenario and losing the rest

### If Main Spec Does NOT Exist

The delta spec IS a full spec (not a delta). Copy it directly to `$SDDK_DATA_DIR/projects/{project_id}/specs/{domain}/spec.md`.

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Verify passing `verify-report` exists (verdict ∈ {PASS, PASS_WITH_WARNINGS}).
3. **Sync delta specs to main specs** (above).
4. **Move to archive** at `$SDDK_DATA_DIR/projects/{project_id}/archive/YYYY-MM-DD-{change_name}/`
5. Verify archive completeness.
6. Persist archive report.
7. Return envelope with the **release-handoff envelope**: `{ "ready_for_release": true, "change": "{name}", "branch": "{type}/{description}", "merge_policy": "{auto|guided|strict|null}" }`. The orchestrator MUST launch `sddk-release` on the next tick — no opt-in, no user prompt.

### Step 3 — Move to Archive

Move the entire change folder to archive:
```
$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/
  → $SDDK_DATA_DIR/projects/{project_id}/archive/YYYY-MM-DD-{change_name}/
```

### Step 4 — Verify Archive

Confirm all artifact paths are recorded in the archive report.

### Step 5 — Persist Archive Report (MANDATORY)

Follow `skills/_shared/sddk-phase-common.md` Section C:
- artifact: `archive-report`
- topic_key: `sddk/{change_name}/archive-report`
- type: `architecture`

Include in the archive report:
- **All observation IDs / paths** for traceability.
- **Knowledge impact**: which specs became stale, which ADRs were superseded.
- **Entropy trend** (when `entropy-sdd` available): delta from pre-cycle to post-cycle.
- **Jurisprudence candidate**: if cycle had `verify_verdict=PASS` + `first_pass_success=true` + reusable decision, flag for F3 jurisprudence save.

## Return Format

```markdown
## Change Archived

**Change**: {change-name}
**Archived to**: `$SDDK_DATA_DIR/projects/{project_id}/archive/YYYY-MM-DD-{change_name}/`

### Specs Synced

| Domain | Action | Details |
|--------|--------|---------|
| {domain} | Created/Updated | {N added, M modified, K removed requirements} |

### Archive Contents
- proposal.md ✅
- specs/ ✅
- design.md ✅
- tasks.md ✅ ({N}/{N} tasks complete)
- verify-report.md ✅ (verdict: {PASS|PASS_WITH_WARNINGS})
- archive-report.md ✅

### Source of Truth Updated
The following specs now reflect the new behavior:
- `$SDDK_DATA_DIR/projects/{project_id}/specs/{domain}/spec.md`

### Knowledge Impact
- Specs made stale: {list}
- ADRs superseded: {list}
- Jurisprudence candidate: {yes/no, topic_key if yes}

### Archive Complete — Release Required
The change has been planned, implemented, verified, and archived. The cycle remains open until mandatory release completes.
```

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record the archive in the cycle ledger BEFORE returning:

1. Evaluate the archive gate:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition archive.complete --gate ledger-valid --outcome passed --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
2. Transition with the archive manifest:
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition archive.complete --artifact archive-manifest={path} --gate-receipt {receipt_id} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed.

## References

- `prompts/sddk/phases/archive.md` — full phase spec
- `prompts/sddk/decision-model.md` — knowledge contract, jurisprudence schema
- `prompts/sddk/metrics-schema.md` — aggregate metrics
