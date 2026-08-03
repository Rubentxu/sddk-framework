---
name: knowledge-graph
description: >
  Protocol for reading and writing the SDDK knowledge graph vault. Used by all SDDK phase agents
  to create, update, and query nodes (milestones, ADRs, requirements, cycles, incidences, terms).
  The vault lives OUTSIDE the project repo, in ~/.sddk-knowledge/{project}/. Follows OKF +
  Obsidian Properties conventions. Wikilinks [[like-this]] create the graph.
license: MIT
metadata:
  author: gentleman-programming
  version: "1.0"
  okf_version: "0.2"
---

# Knowledge Graph Protocol

You have access to the SDDK knowledge graph vault as the **single source of truth** for project knowledge. This skill defines HOW to read, write, and query nodes. Follow this protocol exactly.

## Vault Location

```
~/.sddk-knowledge/{project}/
```

Where `{project}` is the workspace basename (from `git rev-parse --show-toplevel` or `pwd`). If the vault doesn't exist, initialize it from the template at `~/.sddk-shared/knowledge-template/` (see § Vault Initialization below).

**CRITICAL**: The vault is OUTSIDE the project git repo. NEVER write knowledge files into the project repo. The project repo contains ONLY product code. All documentation (ROADMAP, ADRs, specs, requirements, manifests) lives in the vault.

## Node Types

| Type | Directory | Naming | Created by |
|------|-----------|--------|------------|
| `milestone` | `milestones/` | `M-NNN-{slug}.md` | Orchestrator (Step 0.2) |
| `active_lock` | `milestones/` | `_active.md` | Orchestrator (Step 0.2) + Release (release-lock) |
| `adr` | `adrs/` | `ADR-NNN-{slug}.md` | sddk-spec / sddk-design (Step 1.4) |
| `requirement` | `specs/{domain}/` | `REQ-{Slug}.md` | sddk-spec (Step 1.4) |
| `cycle` | `cycles/` | `CYC-{date}-{slug}.md` | sdd-kernel-archive (Step 2.5) |
| `incidence` | `incidences/` | `INC-NNN-{slug}.md` | sdd-kernel-release (update-adrs) |
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

### Rule 1: Read from vault, never from project repo

```bash
# CORRECT — read from vault
cat ~/.sddk-knowledge/{project}/adrs/ADR-003-jwt-auth.md

# WRONG — docs don't live in the repo anymore
cat docs/adr/ADR-003.md  # ← this path is dead
```

### Rule 2: Use grep for queries

```bash
# All accepted ADRs
grep -l "status: accepted" ~/.sddk-knowledge/{project}/adrs/*.md

# All requirements in auth domain
ls ~/.sddk-knowledge/{project}/specs/auth/REQ-*.md

# What ADRs affect auth?
grep -l "affects_domains:.*auth" ~/.sddk-knowledge/{project}/adrs/*.md

# Is a cycle active?
cat ~/.sddk-knowledge/{project}/milestones/_active.md
```

### Rule 3: Follow wikilinks for navigation

When you see `[[ADR-003-jwt-auth]]` in a node, open that file to continue the trace. Wikilinks are the graph edges.

## Write Rules

### Rule 1: Read the template first

Before creating any node, read the corresponding template:

```bash
cat ~/.sddk-knowledge/{project}/templates/{type}.md
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

After creating or updating ANY node, append an entry to `_log.md`:

```bash
echo "- $(date -Iseconds) | {action} | {what} | [[{node}]]" >> ~/.sddk-knowledge/{project}/_log.md
```

### Rule 6: Staleness

Every node has `stale_after`. When you update a node, push `stale_after` forward:
- Milestone: +90 days from update
- ADR: +365 days from update
- Requirement: +90 days from update
- Incidence: +90 days from discovery

## Serialization Lock Protocol

### Acquiring the lock (orchestrator, Step 0.2)

```bash
LOCK_FILE=~/.sddk-knowledge/{project}/milestones/_active.md

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
LOCK_FILE=~/.sddk-knowledge/{project}/milestones/_active.md

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

If `~/.sddk-knowledge/{project}/` doesn't exist when the orchestrator needs it (Step 0.2):

```bash
# The vault template lives centralized in the SDDK framework
cp -r ~/.sddk-shared/knowledge-template/ ~/.sddk-knowledge/{project}/
# Replace {PROJECT_NAME} in _index.md
sed -i "s/{PROJECT_NAME}/{project}/" ~/.sddk-knowledge/{project}/_index.md
```

## Error Handling

If the vault directory is inaccessible or the disk is full:
1. Report the error to the orchestrator
2. Fall back to Engram-only persistence (persist node content as `mem_save` with topic `sddk-kg/{project}/{type}/{slug}`)
3. Do NOT silently skip the write — the knowledge graph is the source of truth

## Compact Rules

- Vault lives at `~/.sddk-knowledge/{project}/` — NEVER in the project repo
- Properties use `snake_case`; values use wikilinks `[[]]`
- Every node has `type`, `title`, `slug`, `status`, `created`, `stale_after`
- Changelog is append-only (bi-temporal)
- Log every write to `_log.md`
- Serialization lock = `milestones/_active.md`
- Read templates before creating nodes
- Initialize vault from `_template/` if it doesn't exist
