# ADR Template — Architecture Decision Record

An ADR captures a single significant architectural decision, its context, the options considered, and the consequences of the chosen path. ADRs are **immutable once accepted** — supersede, never edit.

## File Naming

`docs/adr/ADR-NNN-{kebab-case-title}.md`

Where:
- `NNN` is a zero-padded sequence number (e.g., `001`, `042`)
- `{kebab-case-title}` is short and descriptive
- Files are sorted by NNN, not by date

Example: `docs/adr/ADR-042-event-sourcing-decision.md`

## Lifecycle

| State | Meaning | Where |
|-------|---------|-------|
| `proposed` | Written, awaiting review | Marked in `Status` field, listed in `docs/adr/README.md` |
| `accepted` | Decision is binding, implemented successfully | `sdd-kernel-release` updates at Step 3 (update-adrs) |
| `challenged` | Implementation revealed problems — decision may need revision | `sdd-kernel-release` at Step 3, when incidences are logged |
| `superseded by ADR-NNN` | Replaced by a newer decision | Status updated, link to superseding ADR |
| `deprecated` | No longer relevant | `sdd-kernel-release` or orchestrator |

**The decision text is immutable** — to change a decision, write a new ADR that supersedes the old one. However, each ADR has an **Implementation Log** section (append-only) where the release agent records what actually happened during implementation: incidences, deviations, scope changes, or confirmation that it went smoothly.

## When to Write an ADR

The orchestrator's MCW (Step 1.7 and Step 2.1) **triggers an ADR** when any of these apply:

- New technology, library, or framework introduced
- Change to public API or contract (breaking or non-breaking)
- Change to data model or schema
- Change to build, test, or deployment architecture
- Adoption of a design pattern (Hexagonal, CQRS, Saga, etc.)
- Rejection of a "popular" choice (e.g., "we explicitly do NOT use X")
- Significant refactor that changes module boundaries
- Performance, security, or scalability trade-off

When in doubt: write one. ADRs are cheap; the cost of "why did we do this?" 6 months later is expensive.

## Template

```markdown
# ADR-NNN: <Title>

**Status:** proposed | accepted | superseded by ADR-NNN | deprecated
**Date:** YYYY-MM-DD
**Deciders:** <agent name>, <user name>
**Related:** ADR-NNN, ADR-NNN (link to related ADRs)
**Change:** <link to PR that introduced this decision>

## Context and Problem Statement

<Describe the forces at play, including:
- The technical, business, or political context
- The question or decision that needs to be made
- Any constraints (deadlines, existing systems, team skills)>

## Decision Drivers

<Numbered list of forces that influenced the decision:
1. Must work with existing CI/CD pipeline
2. Team has 2 weeks of Rust experience
3. Performance budget is 100ms p95
4. ...>

## Considered Options

### Option 1: <Name>

<Description of the option. Include:
- What it is
- How it works
- Pros
- Cons>

### Option 2: <Name>

<Same structure>

### Option 3: <Name>

<Same structure, optional>

## Decision Outcome

**Chosen option:** "<Option N>", because <justify based on the decision drivers>.

### Consequences

**Positive:**
- <Benefit 1>
- <Benefit 2>

**Negative:**
- <Cost 1 — what we give up>
- <Cost 2 — risks we accept>

**Risks:**
- <Risk 1 and mitigation>
- <Risk 2 and mitigation>

### Confirmation

<How will we know this decision was correct?
- Metrics to track
- Review date (e.g., "revisit in 6 months")
- What would trigger a supersession>

## Pros and Cons of the Options

<Detailed comparison. Use a table:

| Criterion | Option 1 | Option 2 | Option 3 |
|-----------|----------|----------|----------|
| Performance | <value> | <value> | <value> |
| Complexity | <value> | <value> | <value> |
| Team familiarity | <value> | <value> | <value> |
| Maintenance | <value> | <value> | <value> |

## Implementation Log (append-only — updated by `sdd-kernel-release` at Step 3)

This section is **append-only**. The release agent adds an entry after each cycle that touches this ADR. The original decision text above is never modified — only this log grows.

### Entry format

```markdown
### <YYYY-MM-DD> — Cycle <change-name> (PR #<N>, v<version>)

**Outcome:** accepted | challenged
**Verifier:** sdd-kernel-release

**What happened:**
<1-3 sentences. Did the implementation follow the decision? Were there deviations?>

**Incidences (if any):**
- <Incidence 1 — e.g., "Postgres migration took 3x longer than estimated due to SSL config issue">
- <Incidence 2 — e.g., "Read replica lag exceeded 500ms under load test, added connection pooling">

**Scope changes (if any):**
- <Change 1 — e.g., "Decision driver #2 (read replicas) deferred to next cycle; only primary deployed">

**Decision health:**
<One sentence: is this decision still sound, or does it need revision? If challenged, explain why.>
```

### When no issues occurred

```markdown
### 2026-06-15 — Cycle postgres-migration (PR #42, v1.2.0)

**Outcome:** accepted
**Verifier:** sdd-kernel-release

**What happened:**
Implementation followed the decision as written. PostgreSQL 16 deployed, migrations applied, read replicas configured. All decision drivers met.

**Decision health:**
Sound — no revision needed.
```

### When the decision is challenged

If the implementation revealed that the decision was wrong, incomplete, or based on faulty assumptions, the release agent sets the ADR Status to `challenged` and logs the specifics. A challenged ADR should trigger a new ADR (superseding) in the next cycle.

## More Information

<Links to:
- Code that implements this decision
- Tests that verify it
- Related ADRs
- External references (blog posts, papers, talks)>

## Note

<Optional. If superseded, link to the superseding ADR.>
```

## Example (ADR-001 from a real project)

```markdown
# ADR-001: Use PostgreSQL for primary persistence

**Status:** accepted
**Date:** 2026-06-01
**Deciders:** sdd-kernel-propose, rubentxu
**Related:** ADR-005 (Postgres migrations), ADR-010 (Read replicas)
**Change:** PR #42 (feat: postgres-primary-persistence)

## Context and Problem Statement

The application needs persistent storage for orders, users, and products. The current system uses SQLite, which works for single-node deployments but fails when:
- We need read replicas for analytics queries
- Multiple writers contend on the same database
- We need full-text search across products

## Decision Drivers

1. Multi-writer concurrency required (3+ app instances)
2. Read replicas needed for analytics workload
3. Team has Postgres experience from previous project
4. Migration budget: 1 week

## Considered Options

### Option 1: PostgreSQL 16

Industry-standard RDBMS. Mature, well-supported, JSON support for flexible schemas.

### Option 2: CockroachDB

Distributed SQL, scales horizontally without sharding. Newer, smaller community.

### Option 3: MongoDB

Document database, no fixed schema. Different mental model from current code.

## Decision Outcome

**Chosen option:** "PostgreSQL 16", because the team has direct experience, migration tooling is mature (pg_dump + custom scripts), and we don't need Cockroach's distributed features for the next 12 months.

### Consequences

**Positive:**
- Familiar tooling reduces onboarding time
- JSON support allows gradual schema evolution
- Read replicas available out-of-box

**Negative:**
- Single point of failure unless we add HA setup
- Slightly higher ops cost than SQLite

## Pros and Cons of the Options

| Criterion | PostgreSQL | CockroachDB | MongoDB |
|-----------|-----------|-------------|---------|
| Maturity | High | Medium | High |
| Multi-writer | Yes (with replicas) | Yes (built-in) | Yes |
| Team familiarity | High | Low | Medium |
| Migration cost | Low | High | High |

## More Information

- Implementation: `crates/persistence/src/postgres.rs`
- Migrations: `migrations/`
- Related: ADR-005 (Postgres migrations), ADR-010 (Read replicas)
```

## ADR Index (`docs/adr/README.md`)

Maintain an index file that lists all ADRs:

```markdown
# ADR Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [ADR-001](ADR-001-postgres-primary-persistence.md) | Use PostgreSQL for primary persistence | accepted | 2026-06-01 |
| [ADR-002](ADR-002-hexagonal-architecture.md) | Adopt Hexagonal Architecture | accepted | 2026-06-15 |
| [ADR-003](ADR-003-no-langchain.md) | Reject LangChain for LLM orchestration | accepted | 2026-07-01 |
| [ADR-004](ADR-004-supersedes-002.md) | Replace Hexagonal with simpler layered | superseded by ADR-004 | 2026-08-01 |
```

## Workflow Integration with MCW

| MCW Step | ADR Action |
|----------|-----------|
| Step 1.2 (propose) | If a decision is being made, mark the trigger — ADR will be created in Step 1.4 |
| Step 1.4 (spec/design) | Create ADR(s) for architectural decisions in `docs/adr/ADR-NNN-{title}.md` with Status: `proposed` |
| Step 1.7 (review budget guard) | Check if any pending ADRs need reviewer sign-off |
| Step 2.5 (archive) | Verify all ADRs created during this cycle are referenced in archive-report.md |
| **Step 3 (release — `update-adrs`)** | **For each ADR created or touched by this cycle: (1) update Status from `proposed` to `accepted` or `challenged`, (2) append Implementation Log entry with incidences/deviations/scope changes, (3) commit ADR updates to main with the release** |

## When NOT to Write an ADR

- Trivial implementation details (which JSON library to use in a single file)
- Decisions that are easily reversible (rename a variable)
- Pure bug fixes (no architectural change)
- Config changes (env vars, feature flags)

If the decision is "obvious" or "we'll change it next week", don't write an ADR. Use a code comment instead.

## Review Process

ADRs are reviewed as part of the PR that introduces the decision. Reviewers should check:

1. Are the **decision drivers** clearly stated?
2. Are the **options** actually alternatives (not straw men)?
3. Is the **chosen option** justified against the drivers?
4. Are the **consequences** (positive and negative) honest?
5. Is the **confirmation criterion** measurable?

An ADR that fails review must be revised, not accepted.