# LogSeq Vault Convention (shared across all SDD skills)

Replaces openspec-convention.md when artifact store mode is `logseq`.

## Vault Location

```
<your-logseq-vault-path>/
```

## Entity Page Names

All entities are LogSeq pages created via mcp-logseq API. Page names follow this convention:

| Entity | Page Name Format | Template |
|--------|-----------------|----------|
| Proyecto | `{project-name}` | Proyecto Template |
| SDD Change | `SDD Change: {change-name}` | SDD Change Template |
| SDD Explore | `Explore: {change-name}` | SDD Explore Template |
| SDD Proposal | `Proposal: {change-name}` | SDD Proposal Template |
| SDD Spec | `Spec: {change-name}` | SDD Spec Template |
| SDD Design | `Design: {change-name}` | SDD Design Template |
| SDD Tasks | `Tasks: {change-name}` | SDD Tasks Template |
| SDD Verify | `Verify: {change-name}` | SDD Verify Template |
| SDD Archive | `Archive: {change-name}` | SDD Archive Template |
| Slice | `Slice: {feature-name}` | Slice Template |
| Component | `Component: {module-name}` | Component Template |
| Decision | `Decision: {short-title}` | Decision Template |
| Term | `Term: {canonical-name}` | Term Template |
| Deepening Candidate | `Deepening: {module-name}` | Deepening Candidate Template |
| Cross-cutting Concern | `Cross-cutting: {concern-name}` | Cross-cutting Concern Template |
| Pattern | `Pattern: {pattern-name}` | Pattern Template |
| Recipe | `Recipe: {recipe-name}` | Recipe Template |
| Incidencia | `Incidencia: {title}` | Incidencia Template |

## Write Protocol

**ALWAYS use mcp-logseq API. NEVER write .md files directly.**

### Page Format (CRITICAL)

All properties MUST be children of the first block, indented with 2 spaces. Do NOT use the `properties` parameter of `create_page` — it creates flat text outside blocks.

```
CORRECT — properties in content:
mcp-logseq_create_page(
  title: "Component: handler.groovy",
  content: "- Component: handler.groovy\n  type:: [[Component]]\n  proyecto:: [[Proyecto: X]]\n  depth:: deep\n  - ## Informacion\n    - description here"
)

WRONG — properties parameter creates broken format:
mcp-logseq_create_page(
  title: "Component: handler.groovy",
  properties: { "type": "Component", "depth": "deep" }
)
```

The resulting markdown file must look like:
```markdown
- Component: handler.groovy
  type:: [[Component]]
  proyecto:: [[Proyecto: X]]
  depth:: deep
  - ## Informacion
    - description here
```

### Creating a Page from Template

```
1. mcp-logseq_get_page_content(page_name: "{Template Name}")
   → Read the template structure

2. mcp-logseq_create_page(
     title: "{Entity Type}: {name}",
     content: "(filled template content, replacing placeholders)"
   )
```

### Updating a Page

```
mcp-logseq_update_page(
  page_name: "{Entity Type}: {name}",
  content: "(updated content)",
  mode: "replace"  // or "append" for adding sections
)
```

### Updating a Specific Block

```
1. mcp-logseq_search(query: "text to find", include_blocks: true)
   → Get block_uuid

2. mcp-logseq_update_block(
     block_uuid: "{uuid}",
     content: "(updated block content)"
   )
```

### Deleting a Page

```
mcp-logseq_delete_page(page_name: "{Entity Type}: {name}")
```

## Read Protocol

### Reading a Full Page

```
mcp-logseq_get_page_content(
  page_name: "{Entity Type}: {name}",
  max_depth: -1
)
```

### Searching for Entities

```
mcp-logseq_search(query: "{search terms}", include_blocks: true, limit: 20)
```

### Querying by Property

```
mcp-logseq_query(
  query: "(and (property :type [[Slice]]) (property :status activo))",
  result_type: "pages_only"
)
```

### Finding Backlinks

```
mcp-logseq_get_page_backlinks(page_name: "{Entity Type}: {name}")
```

## Journal Protocol

**MANDATORY**: The journal is the live progress dashboard. Write entries in real-time as work happens.

### CRITICAL: The `properties` Parameter is BROKEN

> ⚠️ **NEVER use the `properties` parameter of `mcp-logseq_create_page`**
> It creates properties as flat text outside blocks, with broken `:::` syntax.
> Put ALL properties inside the `content` parameter as indented children of the first block.

### Responsibility

| Writer | When | Entry Type | Format |
|--------|------|------------|--------|
| Sub-agent | After creating EACH entity page | `CREAR` | `- CREAR [[Type: Name]] — description` |
| Sub-agent | After updating an entity page | `NOTA` | `- NOTA [[Type: Name]] — what changed` |
| Sub-agent | At end of phase | `DONE` | `- DONE {phase} — summary` |
| Orchestrator | After receiving phase result | `AVANZAR` | `- AVANZAR [[SDD Change: Name]] — fase X → fase Y` |

### The Mandatory Sequence (write-as-you-go)

For EACH page you create:

```
1. mcp-logseq_create_page(title: "{Type}: {name}", content: "...")
2. mcp-logseq_update_page(page_name: "dd-MM-yyyy", mode: "append", content: "- CREAR [[{Type}: {name}]] — {description}")
```

### Pattern: Write-as-you-go

After EACH page creation, immediately write a journal entry:
```
1. mcp-logseq_create_page(title: "Slice: X", content: "...")
2. mcp-logseq_update_page(page_name: "dd-MM-yyyy", mode: "append", content: "- CREAR [[Slice: X]] — ...")
3. mcp-logseq_create_page(title: "Component: Y", content: "...")
4. mcp-logseq_update_page(page_name: "dd-MM-yyyy", mode: "append", content: "- CREAR [[Component: Y]] — ...")
```

### Phase Completion Entry

At the END of each SDD phase, write a DONE summary:
```
mcp-logseq_update_page(
  page_name: "dd-MM-yyyy",
  mode: "append",
  content: "- DONE {phase-name} — {N} pages created, {summary}\n  - artifacts:: {count}\n  - sdd-change:: [[SDD Change: {name}]]"
)
```

### Journal date format
- Use `dd-MM-yyyy` (e.g., "25-05-2026")
- This comes from the vault's `config.edn` property `:journal/page-title-format`
```

### Action Prefixes

See [[Convencion Journal]] page in the vault for the full list:
- CREAR, AVANZAR, VALIDAR, DONE, INCIDENTE, SNAPSHOT
- DEEPENING, TERM, SLICE, COMPONENT, DECISION, NOTA

## DAG State

Unlike openspec (which uses `state.yaml`), DAG state lives as properties on the SDD Change page:

- `sdd-fase`:: current phase (explore/propose/spec/design/tasks/apply/verify/archive)
- `estado`:: active/completed/archived
- `validacion`:: draft/aprobado/rechazado

To recover state after compaction: read the SDD Change page properties.

## Artifact Topic Keys (Engram compatibility)

When mode is `logseq`, engram is still used for cross-session memory. Use these topic keys:

| Artifact | Engram Topic Key |
|----------|-----------------|
| Project context | `sdd-init/{project}` |
| SDD Change state | `sdd/{change-name}/state` |
| Apply progress | `sdd/{change-name}/apply-progress` |

All other artifacts live ONLY in LogSeq (not duplicated in engram).

## Mode Comparison

| Capability | `engram` | `openspec` | `logseq` | `hybrid` | `none` |
|------------|----------|------------|----------|----------|--------|
| Cross-session recovery | ✅ | ❌ | ✅ | ✅ | ❌ |
| Compaction survival | ✅ | ❌ | ✅ | ✅ | ❌ |
| Shareable with team | ❌ | ✅ | ✅ (git) | ✅ | ❌ |
| Graph traversal | ❌ | ❌ | ✅ | ✅ | ❌ |
| Journal timeline | ❌ | ❌ | ✅ | ✅ | ❌ |
| Queries by property | ❌ | ❌ | ✅ | ✅ | ❌ |
| Human validation flow | ❌ | ❌ | ✅ | ✅ | ❌ |
| Project files created | Never | Yes | No (LogSeq) | Yes | Never |

## Reading Artifacts by Phase

| Phase | Read | Write |
|-------|------|-------|
| sdd-init | — | `Proyecto: {name}`, initial Terms |
| sdd-explore | `Proyecto: {name}` | `Explore: {change}`, `Slice: *`, `Component: *`, `Term: *` |
| sdd-propose | `Explore: {change}` | `Proposal: {change}`, `Decision: *` |
| sdd-spec | `Proposal: {change}` | `Spec: {change}` |
| sdd-design | `Proposal: {change}`, `Spec: {change}` | `Design: {change}`, `Decision: *` |
| sdd-tasks | `Spec: {change}`, `Design: {change}` | `Tasks: {change}` |
| sdd-apply | `Tasks: {change}`, `Design: {change}` | Update `Tasks: {change}`, update `Component: *` |
| sdd-verify | `Spec: {change}`, `Tasks: {change}` | `Verify: {change}`, update `Component: *` |
| sdd-archive | All phase pages | `Archive: {change}`, update Proyecto, final snapshot |
