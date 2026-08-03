# ROADMAP Template — Living Project Roadmap

The ROADMAP is a **living document** that describes the current state, near-term milestones, and long-term vision of the project. It is also the **serialization lock** for SDDK cycles: only one cycle can be Active at a time (see § Serialization Rule below).

It is updated:

- **At the start of every SDDK cycle** (MCW Step 0.2 — lock acquisition): the new cycle is added to "Active Milestones" with `Status: in_progress`. This acquires the lock.
- **At the end of every SDDK cycle** (MCW Step 3.8 — lock release): the milestone is marked `completed` and moved to "Completed Milestones". This releases the lock.

## File Location

`docs/ROADMAP.md` (at the project root)

If the project uses a different name (`ROADMAP-INTEGRADO.md`, `docs/epics/ROADMAP.md`, etc.), the orchestrator should detect it during Phase 0 and use it consistently.

## Serialization Rule (One Cycle at a Time)

**Only one Active Milestone at a time.** If the "Active Milestones" section has any entry with `Status: in_progress`, a new SDDK cycle CANNOT start. The orchestrator's MCW Step 0.2 reads ROADMAP and BLOCKS if an active cycle exists.

| Status | Holds lock? | When to use |
|--------|-------------|-------------|
| `in_progress` | **YES** | Cycle is active. Set at MCW Step 0.2 (lock acquired). |
| `completed` | NO | Release succeeded. Set at MCW Step 3.8 by `sdd-kernel-release` (lock released). |
| `blocked` | NO | Release failed and user deferred. Entry moves to "Blocked Cycles" section. Does NOT hold lock — a new cycle can start. |
| `abandoned` | NO | User killed the cycle. Entry moves to "Abandoned Cycles" section. Does NOT hold lock. |

**Lock lifecycle:**
```
Step 0.2 (start):  ROADMAP[change] = in_progress   → LOCK ACQUIRED
Step 3.8 (end):    ROADMAP[change] = completed      → LOCK RELEASED
                   OR: ROADMAP[change] = blocked/abandoned → LOCK RELEASED (user decision)
```

The lock survives across sessions. If a session crashes mid-cycle, the next session's Step 0.2 sees the `in_progress` entry and directs the user to resume (`/sddk-release <change>`) rather than starting a new cycle.

## Lifecycle

| State | Meaning | Who updates |
|-------|---------|-------------|
| **draft** | Being created, not yet reviewed | sdd-kernel-propose agent |
| **current** | Approved, reflects active plan | Orchestrator after MCW Step 3.8 |
| **historical** | Replaced by newer version | Keep for archive, add link to newer version |

## When to Update

The orchestrator MUST update `docs/ROADMAP.md` at these MCW steps:

| MCW Step | Update Action | Lock effect |
|----------|---------------|-------------|
| **Step 0.2** (previous cycle closed) | READ ROADMAP. If any Active `in_progress` entry exists → BLOCK. If none → ADD new entry as `in_progress` to "Active Milestones" | **Acquires lock** |
| **Step 0.3** (knowledge coverage) | Confirm the cycle's milestone is correctly placed (read-only) | — |
| **Step 3.8** (consolidation, owned by release agent) | MOVE entry from "Active" to "Completed" with PR/tag/learnings | **Releases lock** |
| **User-initiated block/abandon** | MOVE entry to "Blocked Cycles" or "Abandoned Cycles" section | **Releases lock** |

If the ROADMAP is missing at Step 0.3, the orchestrator should create it (or escalate to user if too early in the project).

## Template

```markdown
# Project ROADMAP

> Living document. Updated at the start and end of every SDDK cycle.
> Last updated: YYYY-MM-DD by sdd-kernel-archive (cycle: <cycle-name>)

## Vision

<One paragraph: what is the project trying to become?>

## Current State

<Where the project is now. Be honest about gaps and limitations.>

## Active Milestones (In Progress)

### M-NNN: <Milestone name>

- **Cycle:** <SDDK change name>
- **Branch:** `<type>/<description>`
- **Status:** in_progress
- **Started:** YYYY-MM-DD
- **Target release:** v<version>
- **Goal:** <What this milestone achieves>
- **Scope:** <In / Out>
- **Tracking issue:** #<N> (or null)
- **Linked ADR(s):** ADR-NNN (or null)

## Planned Milestones (Next 90 days)

### M-NNN: <Milestone name>

- **Status:** planned
- **Priority:** P0 | P1 | P2 | P3
- **Target:** <vague date or version>
- **Goal:** <What this milestone achieves>
- **Depends on:** M-NNN (or "nothing")

<More planned milestones...>

## Backlog (Beyond 90 days)

| ID | Title | Priority | Effort | Notes |
|----|-------|----------|--------|-------|
| M-NNN | <title> | P2 | M | <notes> |

## Completed Milestones (Last 6 months)

### M-NNN: <Milestone name> — COMPLETED

- **Cycle:** <change-name>
- **PR:** <url>
- **Tag:** v<version>
- **Completed:** YYYY-MM-DD
- **HTML report:** <path>
- **Key learnings:**
  - <Learning 1>
  - <Learning 2>

## Blocked Cycles (lock released — deferred)

Cycles whose release failed and the user decided to defer. They do NOT hold the serialization lock — a new cycle can start. The feature branch may still exist (unmerged).

### M-NNN: <change-name> — BLOCKED

- **Cycle:** <change-name>
- **Branch:** `<type>/<description>` (unmerged)
- **Blocked since:** YYYY-MM-DD
- **Blocker:** <what prevented release — conflict, timeout, missing approval, etc.>
- **Recovery:** `/sddk-release <change-name>` resumes from the last uncompleted sub-step
- **Deferred reason:** <why the user chose to defer instead of resolving now>

## Abandoned Cycles (lock released — killed)

Cycles the user explicitly killed. They do NOT hold the serialization lock. The feature branch persists as historical record (never deleted per trunk-based policy).

### M-NNN: <change-name> — ABANDONED

- **Cycle:** <change-name>
- **Branch:** `<type>/<description>` (unmerged, kept as historical record)
- **Abandoned:** YYYY-MM-DD
- **Reason:** <why the user abandoned — superseded, no longer needed, wrong approach, etc.>

## Out of Scope (Explicitly Deferred)

<Things we considered and explicitly chose NOT to do, and why.>

## Update Log

- YYYY-MM-DD — sdd-kernel-archive — Updated after cycle <change-name> (PR #N, v<version>)
- YYYY-MM-DD — sdd-kernel-propose — Initial draft
- ...
```

## Sections Explained

### Vision

Short paragraph. What is the project trying to be? This should be stable across cycles; if it changes, that's a major decision (write an ADR).

### Current State

Honest assessment. What's working, what's not. This section is updated whenever the orchestrator's MCW detects a major change.

### Active Milestones (In Progress)

Milestones currently being worked on. Usually 1-3 at a time.

The format mirrors the MCW data:
- Cycle name (from `/sddk-new`)
- Branch (from MCW Step 1.8)
- Target release version (from MCW Step 3.5)
- Tracking issue (from MCW Step 3.7)
- Linked ADR (from MCW Step 1.4)

### Planned Milestones (Next 90 days)

Milestones in the next quarter. Each should have:
- Priority (P0 critical, P1 high, P2 medium, P3 low)
- Target (version or rough date)
- Dependencies

This section is updated at the START of every cycle (Step 0.3) when the orchestrator confirms the cycle's milestone placement.

### Backlog

Ideas without a target. Effort estimate (S/M/L) and priority.

### Completed Milestones (Last 6 months)

Historical record. Each completion adds:
- Cycle name
- PR link
- Tag version
- HTML report path
- Key learnings (2-3 bullets)

This section grows at MCW Step 3.8.

### Out of Scope (Explicitly Deferred)

Things we considered but rejected. This is important — it prevents re-litigating old decisions.

Example:
- **Microservices architecture** — Deferred indefinitely. Current monolith is sufficient for 10x current load.
- **GraphQL API** — Deferred until REST endpoints stabilize. Migration cost is high for unclear benefit.

### Update Log

Append-only history of who updated the ROADMAP and when. This is the audit trail.

## Integration with Cycle Closure

The semver tag (pushed to origin at Step 3.5) IS the cycle marker. `git tag --points-at main` shows the last completed cycle. No separate marker file.

Additionally, F3 metrics (`~/.local/share/opencode/sddk/metrics/{cycle_id}.jsonl` + Engram `cycle-metrics/{cycle_id}`) capture machine-readable closure data for jurisprudence aggregation.

The ROADMAP should reference this file for the latest cycle, but the marker file is the canonical source for "what was the last completed cycle". The ROADMAP is human-readable; the marker is machine-readable.

## Update Workflow (MCW integration)

### At Step 0.3 (read):

```bash
cat docs/ROADMAP.md | head -50  # See current state
grep "Active Milestone" docs/ROADMAP.md  # Find current cycle's entry
```

If the current cycle is NOT in the ROADMAP:
- Block: "Cycle `<change>` is not in the ROADMAP. Add it before proceeding or escalate."
- OR: Orchestrator adds it (auto-update), informing the user

### At Step 3.8 (write):

**v3.5: The ROADMAP lives in the knowledge graph vault, NOT in the project repo.** The `sdd-kernel-release` agent updates the milestone node at `{project}/~/.sddk-knowledge/{project}/milestones/M-NNN-{slug}.md` (status → completed, pr, tag, cycle). No `git add docs/ROADMAP.md` — that path doesn't exist anymore. See `skills/knowledge-graph/SKILL.md` for the vault write protocol.

## Anti-Patterns

- ❌ Forgetting to update the ROADMAP at the end of a cycle
- ❌ Stale "Active Milestones" section showing cycles that completed months ago
- ❌ No link to PR, tag, or HTML report from completed milestones
- ❌ ROADMAP diverges from actual project state (e.g., "planned" feature that's already in main)
- ❌ Update log missing or sparse

## Frequency

- **Read**: Every cycle (Step 0.3)
- **Write**: Every cycle that completes (Step 3.8)
- **Rewrite**: When project vision changes (rare; write an ADR first)