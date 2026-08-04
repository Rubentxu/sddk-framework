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
- If `openspec/changes/archive/` doesn't exist, create it.
- Apply any `rules.archive` from `openspec/config.yaml`.

## Delta Spec Sync (the core operation)

For each delta spec in `openspec/changes/{change-name}/specs/`:

### If Main Spec Exists (`openspec/specs/{domain}/spec.md`)

```
FOR EACH SECTION in delta spec:
├── ADDED Requirements     → Append to main spec's Requirements section
├── MODIFIED Requirements  → Replace the matching requirement in main spec
│                             (match by Requirement name; preserve all OTHER requirements)
└── REMOVED Requirements   → Delete the matching requirement from main spec
```

**Match by Requirement name** (e.g., `### Requirement: Session Expiration`). Preserve all OTHER requirements that aren't in the delta. Maintain proper Markdown formatting and heading hierarchy.

**Why copy-full-then-edit (recall from sddk-spec):**
- The archive step REPLACES the requirement in main specs with the MODIFIED block
- If the MODIFIED block is partial (only the changed scenario), the archive loses other scenarios
- Common pitfall: only writing the changed scenario and losing the rest

### If Main Spec Does NOT Exist

The delta spec IS a full spec (not a delta). Copy it directly:

```bash
openspec/changes/{change-name}/specs/{domain}/spec.md
  → openspec/specs/{domain}/spec.md
```

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A.
2. Verify passing `verify-report` exists (verdict ∈ {PASS, PASS_WITH_WARNINGS}).
3. **Sync delta specs to main specs** (above).
4. **Move to archive** (Step 3 below).
5. Verify archive completeness (Step 4).
6. Persist archive report.
7. Return envelope with the **release-handoff envelope**: `{ "ready_for_release": true, "change": "{name}", "branch": "{type}/{description}", "merge_policy": "{auto|guided|strict|null}" }`. The orchestrator MUST launch `sdd-kernel-release` on the next tick — no opt-in, no user prompt. See orchestrator.md § "Release Is Mandatory Post-Archive (v3.3, no opt-out)".

### Step 3 — Move to Archive

**IF mode is `engram`:** Skip filesystem sync and move — artifacts live in Engram. The archive report in Engram serves as the audit trail.

**IF mode is `none`:** Skip — no filesystem operations.

**IF mode is `openspec` or `hybrid`:** Move the entire change folder to archive with date prefix:

```
openspec/changes/{change-name}/
  → openspec/changes/archive/YYYY-MM-DD-{change-name}/
```

Use today's date in ISO format.

### Step 4 — Verify Archive

**IF mode is `openspec` or `hybrid`:** Confirm:
- [ ] Main specs updated correctly
- [ ] Change folder moved to archive
- [ ] Archive contains all artifacts (proposal, specs, design, tasks, verify-report)
- [ ] Active changes directory no longer has this change

**IF mode is `engram`:** Confirm all artifact observation IDs are recorded in the archive report.

**IF mode is `none`:** Skip verification — no persisted artifacts.

### Step 5 — Persist Archive Report (MANDATORY)

Follow `skills/_shared/sddk-phase-common.md` Section C:
- artifact: `archive-report`
- topic_key: `sddk/{change-name}/archive-report`
- type: `architecture`

Include in the archive report:
- **All observation IDs / paths** for traceability.
- **Knowledge impact**: which specs became stale, which ADRs were superseded.
- **Entropy trend** (when `entropy-sdd` available): delta from pre-cycle to post-cycle.
- **Jurisprudence candidate**: if cycle had `verify_verdict=PASS` + `first_pass_success=true` + reusable decision, flag for F3 jurisprudence save.
- **Roadmap update**: change moves from "Active" to "Completed" in `docs/ROADMAP.md`.

## Return Format

```markdown
## Change Archived

**Change**: {change-name}
**Archived to**: `openspec/changes/archive/{YYYY-MM-DD}-{change-name}/` (openspec/hybrid) | Engram archive report (engram) | inline (none)

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
- `openspec/specs/{domain}/spec.md`

### Knowledge Impact
- Specs made stale: {list}
- ADRs superseded: {list}
- Jurisprudence candidate: {yes/no, topic_key if yes}

### Archive Complete — Release Required
The change has been planned, implemented, verified, and archived. The cycle remains open until mandatory release completes.
```

## References

- `prompts/sdd-kernel/phases/archive.md` — full phase spec
- `prompts/sdd-kernel/decision-model.md` — knowledge contract, jurisprudence schema
- `prompts/sdd-kernel/metrics-schema.md` — aggregate metrics
