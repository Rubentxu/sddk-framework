# SDDK Document Catalog

This is the **index of all documents** that the SDDK pipeline produces, consumes, or maintains. Every document has a clear owner (which step in MCW creates/updates it) and a clear location.

## Documents Produced by SDDK (artefacts)

These are written by SDDK phases and consumed by downstream phases.

| Document | Location | Produced by | Consumed by | Format |
|----------|----------|-------------|-------------|--------|
| **explore-report** | `sddk/{change}/explore-report.md` (or engram: `sdd/{change}/explore`) | sdd-kernel-explore | sdd-kernel-propose, sdd-kernel-coherence | Markdown |
| **proposal** | `sddk/{change}/proposal.md` (or engram: `sdd/{change}/proposal`) | sdd-kernel-propose | sdd-kernel-spec, sdd-kernel-design | Markdown |
| **spec** | `sddk/{change}/spec.md` (or engram: `sdd/{change}/spec`) | sdd-kernel-spec | sdd-kernel-tasks | Markdown (Given/When/Then) |
| **design** | `sddk/{change}/design.md` (or engram: `sdd/{change}/design`) | sdd-kernel-design | sdd-kernel-tasks | Markdown |
| **tasks** | `sddk/{change}/tasks.md` (or engram: `sdd/{change}/tasks`) | sdd-kernel-tasks | sdd-kernel-apply | Markdown |
| **apply-progress** | `sddk/{change}/apply-progress.md` (or engram: `sdd/{change}/apply-progress`) | sdd-kernel-apply | sdd-kernel-verify, orchestrator | JSON + Markdown |
| **verify-report** | `sddk/{change}/verify-report.md` (or engram: `sdd/{change}/verify-report`) | sdd-kernel-verify | sdd-kernel-archive, orchestrator | Markdown |
| **archive-report** | `sddk/{change}/archive-report.md` (or engram: `sdd/{change}/archive-report`) | sdd-kernel-archive | HTML report, ROADMAP | Markdown |
| **coherence-report** | `sddk/{change}/coherence-{transition}.md` (transient) | sdd-kernel-coherence | orchestrator | Markdown |
| **HTML closing report** | `/tmp/sddk-{change}-{YYYYMMDD}.html` or `openspec/changes/{change}/reports/cierre.html` | sdd-kernel-archive | human reviewer | Self-contained HTML |

## Documents Maintained by SDDK (long-lived)

These are updated across many cycles and persist in the repo.

| Document | Location | Maintained by | When | Purpose |
|----------|----------|---------------|------|---------|
| **ROADMAP** | `docs/ROADMAP.md` | orchestrator (Step 0.3 read, Step 3.8 write) | Every cycle | Project vision + active/planned/completed milestones |
| **ADR index** | `docs/adr/README.md` | orchestrator (Step 1.4) | When ADRs are added/removed | Index of all ADRs |
| **ADRs** | `docs/adr/ADR-NNN-{title}.md` | sdd-kernel-spec/design (Step 1.4) | When architectural decisions are made | Immutable decision records |
| **Cycle marker** | semver tag on main | orchestrator (Step 3.5) | Every cycle end | `git tag --points-at main` returns last completed cycle |
| **CONTEXT** | `CONTEXT.md` | sdd-kernel-explore, sdd-kernel-coherence | When glossary changes | Domain language glossary |

## Documents Read by SDDK (inputs)

These are read by SDDK phases but not modified by them.

| Document | Location | Read by | When |
|----------|----------|---------|------|
| **Code** | `<repo>` | All phases | Always (for context) |
| **Tests** | `<repo>/tests` etc. | sdd-kernel-verify | Step 2.3 |
| **Config files** | `<repo>/package.json`, `Cargo.toml`, etc. | sdd-kernel-init | Step 0.3 |
| **Architecture docs** | `<repo>/docs/architecture/`, `ARCHITECTURE.md` | All phases | As needed |
| **Existing ADRs** | `<repo>/docs/adr/` | sdd-kernel-spec, sdd-kernel-design | Step 1.4 |
| **Existing ROADMAP** | `<repo>/docs/ROADMAP.md` | orchestrator | Step 0.3, Step 3.8 |
| **CONTEXT** | `<repo>/CONTEXT.md` | All phases | When language is ambiguous |
| **Tests config** | `sdd-init/{project}` (engram) | sdd-kernel-apply | Step 2.1 |

## Document Relationships

```
                    ROADMAP.md (vision, milestones)
                          ▲
                          │ Step 3.8 update
                          │
        ┌─────────────────┴─────────────────┐
        │                                   │
   ADR-NNN (decisions)              Cycle marker
   ADR-MMM (decisions)             (machine-readable)
        │                                   │
        │ Step 1.4 create                   │
        │                                   │
        ▼                                   │
   proposal.md ───► spec.md ───► design.md │
        │              │             │     │
        │              │             │     │
        └────────► tasks.md ────────┘     │
                    │                       │
                    ▼                       │
              apply-progress.md              │
                    │                       │
                    ▼                       │
              verify-report.md               │
                    │                       │
                    ▼                       │
              archive-report.md ────────────┘
                    │
                    ▼
              HTML report (human)
```

## Document Lifecycle (when each is created/updated)

```
                    PRE-FLIGHT         PLAN                    BUILD                CONSOLIDATE
                    ──────────         ────                    ─────                ───────────
ROADMAP             READ               -                       -                    UPDATE
ADR (new)           -                  CREATE                  -                    -
ADR (update)        -                  CREATE (supersession)   -                    -
explore-report      -                  CREATE                   -                    -
proposal            -                  CREATE                   -                    -
spec                -                  CREATE                   -                    -
design              -                  CREATE                   -                    -
tasks               -                  CREATE                   -                    -
apply-progress      -                  -                        CREATE/UPDATE         -
verify-report       -                  -                        CREATE                -
archive-report      -                  -                        -                    CREATE
HTML report         -                  -                        -                    CREATE
Cycle marker        -                  -                        -                    CREATE+COMMIT
CONTEXT.md          READ               UPDATE (if needed)      -                    UPDATE (if needed)
ADR index           READ               UPDATE (if new ADR)     -                    -
```

## Update Discipline

### For orchestrator:

- **Every cycle MUST update ROADMAP at Step 3.8** (hard gate)
- **Every cycle MUST write the cycle marker at Step 4.2** (hard gate)
- **ADRs are created at Step 1.4** when architectural decisions are made
- **CONTEXT.md is updated** when domain language changes (rare)

### For phase agents:

- Phase agents write their outputs to engram OR `sddk/{change}/` depending on artifact store mode
- Phase agents do NOT modify ROADMAP, ADRs, or CONTEXT.md directly — they propose updates and the orchestrator executes them
- Phase agents reference existing ADRs in their proposals/designs

## Naming Conventions

### Files in repo

```
docs/
├── ROADMAP.md
├── adr/
│   ├── README.md                    (index)
│   ├── ADR-001-{title}.md
│   ├── ADR-002-{title}.md
│   └── ...
├── architecture/
│   ├── overview.md
│   └── ...
├── CONTEXT.md                       (glossary)
└── ...

sddk/
└── {change}/
    ├── explore-report.md
    ├── proposal.md
    ├── spec.md
    ├── design.md
    ├── tasks.md
    ├── apply-progress.json           (machine-readable)
    ├── apply-progress.md             (human-readable)
    ├── verify-report.md
    └── archive-report.md

(cycle marker = semver tag on main, not a file)
```

### IDs

- ADR numbers: zero-padded `NNN`, sequential, never reused
- Cycle names: kebab-case, descriptive (`add-oauth2-login`, `fix-null-session-lookup`)
- Milestone IDs (ROADMAP): `M-NNN`, sequential within project

## Storage Modes

| Mode | Where artefacts go | When |
|------|-------------------|------|
| `engram` | Engram memory only (fast, no files) | Default if engram is available |
| `openspec` | `sddk/{change}/` files (traceable in repo) | When user wants repo-traceable artefacts |
| `hybrid` | Both (slow, redundant) | When user explicitly wants redundancy |
| `none` | Return results inline only | When user disables persistence |

The orchestrator's MCW must respect the chosen mode at every step.

## Cross-References

Every artefact should reference its neighbours:

- `proposal.md` references `ROADMAP.md` (which milestone this addresses)
- `spec.md` references `proposal.md` + relevant `ADR-NNN`
- `design.md` references `spec.md` + relevant `ADR-NNN`
- `tasks.md` references `design.md` + `spec.md` + `proposal.md`
- `verify-report.md` references `tasks.md` + `spec.md`
- `archive-report.md` references ALL preceding artefacts + `ROADMAP.md`
- `HTML report` includes ALL of the above as references
- `cycle marker` references `archive-report.md` + PR + tag

This is the **nervous system** of the project. If a document doesn't reference its neighbours, the chain is broken.

## Recovery

If a document is missing or out-of-date:

| Missing doc | Recovery action |
|-------------|------------------|
| ROADMAP.md | Create from template (Step 3.8 first time, or Step 0.3 if blocking) |
| ADR (missing) | sdd-kernel-spec/design cannot reference it; revert the change or write the ADR retroactively |
| ADR (out-of-date) | Write a new ADR that supersedes it |
| archive-report | Re-run sdd-kernel-archive |
| cycle marker | Re-run Step 4.2 (but only if previous cycle was actually complete) |
| HTML report | Re-generate via sdd-kernel-archive |

The orchestrator should detect missing documents at MCW Step 0.3 and either fix them or block.