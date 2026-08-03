---
name: knowledge-graph
description: >
  Protocol for reading and writing the SDDK knowledge graph vault. Used by all SDDK phase agents
  to create, update, and query nodes (milestones, ADRs, requirements, cycles, incidences, terms).
  The vault lives INSIDE each project repo at `.sddk-knowledge/`, versionable with git.
  Follows OKF + Obsidian Properties conventions. Wikilinks [[like-this]] create the graph.
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  okf_version: "0.2"
---

# Knowledge Graph Protocol

You have access to the SDDK knowledge graph vault as the **single source of truth** for project knowledge. This skill defines HOW to read, write, and query nodes. Follow this protocol exactly.

## Vault Location

```
{project_root}/.sddk-knowledge/
```

Where `{project_root}` is the project workspace (from `git rev-parse --show-toplevel` or `pwd`). The vault **lives inside the project repo**, in a `.sddk-knowledge/` directory that is **versionable with git** (committed to the repo).

**CRITICAL**: The vault is per-project and per-repo. It is NOT in `$HOME`. Each project has its own `.sddk-knowledge/` directory, committed to that project's git history. This makes the knowledge portable — clone the project, you get its knowledge graph.

The **template** for the vault lives in the **SDDK framework repo** (`~/.sddk-shared/knowledge-template/` when installed locally, or `https://github.com/Rubentxu/sddk-framework/tree/main/knowledge-template` in the published repo). The first time `sddk-adopt` runs in a project, it copies the template into `.sddk-knowledge/`.

## Node Types

| Type | Directory | Naming | Created by |
|------|-----------|--------|------------|
| `milestone` | `milestones/` | `M-NNN-{slug}.md` | Orchestrator (Step 0.2) |
| `active_lock` | `milestones/` | `_active.md` | Orchestrator (Step 0.2) + Release (release-lock) |
| `adr` | `adrs/` | `ADR-NNN-{slug}.md` | sddk-spec / sddk-design (Step 1.4) |
| `requirement` | `specs/{domain}/` | `REQ-{Slug}.md` | sddk-spec (Step 1.4) |
| `cycle` | `cycles/` | `CYC-{date}-{slug}.md` | sdd-kernel-archive (Step 2.5) |
| `incidence` | `incidences/` | `INC-NNN-{slug}.md` | sdd-kernel-release (update-knowledge-graph) |
| `term` | `terms/` | `TERM-{Slug}.md` | sddk-explore / sddk-spec |

## Properties Convention (OKF + Obsidian)

All properties use **`snake_case`** (Obsidian Dataview compatibility). Property types:

| Type | YAML format | Example |
|------|-------------|---------|
| Text | `key: "value"` | `title: "Use JWT for Auth"` |
| Number | `key: 42` | `pr: 42` |
| Date | `key: 2026-08-03` | `created: 2026-08-03` |
| Checkbox | `key: true` | `verified: true` |
| List | `key: ["a", "b"]` | `affects_domains: ["[[auth]]"]` |
| Wikilink | `key: "[[node]]"` | `decision_authority: "[[ADR-003]]"` |
| Wikilink list | `key: ["[[n1]]", "[[n2]]"]` | `linked_adrs: ["[[ADR-003]]"]` |
| Null/empty | `key:` (no value) | `completed:` |

**Built-in properties** (Obsidian): `aliases` (list), `tags` (list).

## Read Rules

### Rule 1: Read from vault, never from external sources

```bash
# CORRECT — read from vault
cat .sddk-knowledge/adrs/ADR-003-jwt-auth.md

# WRONG — vault doesn't live in $HOME anymore
cat .sddk-knowledge/adrs/ADR-003.md
```

### Rule 2: Use grep for queries

```bash
# All accepted ADRs
grep -l "status: accepted" .sddk-knowledge/adrs/*.md

# All requirements in auth domain
ls .sddk-knowledge/specs/auth/REQ-*.md

# What ADRs affect auth?
grep -l "affects_domains:.*auth" .sddk-knowledge/adrs/*.md

# Is a cycle active?
cat .sddk-knowledge/milestones/_active.md
```

### Rule 3: Follow wikilinks for navigation

When you see `[[ADR-003-jwt-auth]]` in a node, open that file to continue the trace. Wikilinks are the graph edges.

## Write Rules

### Rule 1: Read the template first

Before creating any node, read the corresponding template from the SDDK framework:

```bash
# Template source (in the SDDK framework, NOT in the project repo)
cat ~/.sddk-shared/knowledge-template/templates/{type}.md
```

Fill in the placeholders. Do not invent properties not in the template.

### Rule 2: Create with complete properties

Every node MUST have at minimum: `type`, `title`, `slug`, `status`, `created`, `stale_after`. Domain-specific properties are MANDATORY (see templates).

### Rule 3: Use wikilinks for ALL cross-references

```yaml
# CORRECT
decision_authority: "[[ADR-003-jwt-auth]]"
affects_requirements: ["[[REQ-Session-Expiration]]", "[[REQ-Token-Refresh]]"]

# WRONG — plain text breaks the graph
decision_authority: "ADR-003"
affects_requirements: ["Session Expiration", "Token Refresh"]
```

### Rule 4: Changelog is append-only

Every node that evolves (ADR, Requirement, Milestone) has a `## Changelog` section. Add entries at the END, never edit existing ones:

```markdown
## Changelog (bi-temporal)

- 2026-08-03T10:00 | created | status=proposed | valid_from=2026-08-03 | valid_to=∞
- 2026-08-03T15:00 | status: proposed→accepted | cycle=[[CYC-007]] | valid_from=2026-08-03 | valid_to=∞
```

### Rule 5: Log to _log.md after every write

After creating or updating ANY node, append an entry to `.sddk-knowledge/_log.md`:

```bash
echo "- $(date -Iseconds) | {action} | {what} | [[{node}]]" >> .sddk-knowledge/_log.md
```

The `_log.md` file IS committed to git (provides cross-cycle audit trail).

### Rule 6: Staleness

Every node has `stale_after`. When you update a node, push `stale_after` forward:
- Milestone: +90 days from update
- ADR: +365 days from update
- Requirement: +90 days from update
- Incidence: +90 days from discovery

## Serialization Lock Protocol

### Acquiring the lock (orchestrator, Step 0.2)

```bash
LOCK_FILE=".sddk-knowledge/milestones/_active.md"

# Check if locked
if grep -q "LOCKED" "$LOCK_FILE" 2>/dev/null; then
    echo "BLOCK: Another cycle is active"
    grep "Milestone:" "$LOCK_FILE"
    exit 1
fi

# Acquire
cat > "$LOCK_FILE" << EOF
---
type: active_lock
milestone: "[[M-NNN-{slug}]]"
acquired: $(date +%Y-%m-%d)
---

# Active Cycle Lock

**Status:** LOCKED

**Milestone:** [[M-NNN-{slug}]]
**Acquired:** $(date +%Y-%m-%d)
**Branch:** \`{branch}\`
**Cycle:** [[CYC-{date}-{slug}]]
EOF
```

### Releasing the lock (release agent, release-lock step)

```bash
LOCK_FILE=".sddk-knowledge/milestones/_active.md"

# Release — reset to available state
cat > "$LOCK_FILE" << EOF
---
type: active_lock
milestone:
acquired:
---

# Active Cycle Lock

**Status:** AVAILABLE (no active cycle)
EOF
```

## Vault Initialization

If `.sddk-knowledge/` doesn't exist when the orchestrator needs it (Step 0.2), it means `sddk-adopt` was never run. Either:

1. Delegate to `sddk-adopt` (the adoption agent) which creates the full vault + plants SDDK working artifacts.
2. For a quick test-only setup, copy the template from the SDDK framework:

```bash
# The vault template lives in the SDDK framework (source of truth)
cp -r ~/.sddk-shared/knowledge-template/ .sddk-knowledge/
sed -i "s/{PROJECT_NAME}/$(basename "$(pwd)")/" .sddk-knowledge/_index.md
```

**Production path is option 1 (sddk-adopt).** Option 2 is only for ad-hoc tests.

## Versioning

The vault **IS committed to git** in the project repo. Every write produces a node + log entry + (optionally) a commit. The commit history of the project IS the timeline of its knowledge graph.

## Compact Rules

- Vault lives at `{project_root}/.sddk-knowledge/` — NEVER in `$HOME`, NEVER in `~/.sddk-knowledge/`
- Template source lives in the SDDK framework: `~/.sddk-shared/knowledge-template/`
- Properties use `snake_case`; values use wikilinks `[[]]`
- Every node has `type`, `title`, `slug`, `status`, `created`, `stale_after`
- Changelog is append-only (bi-temporal)
- Log every write to `.sddk-knowledge/_log.md`
- Serialization lock = `.sddk-knowledge/milestones/_active.md`
- The vault IS committed to the project repo (versionable, portable)
- Read templates from `~/.sddk-shared/knowledge-template/templates/` before creating nodes
- For adoption, delegate to `sddk-adopt` — it creates the vault AND plants SDDK working artifacts
