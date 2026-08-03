---
name: logseq-vault
description: >
  Protocol for reading and writing SDD artifacts to the LogSeq vault.
  Used by all SDD phase agents when artifact store mode is `logseq`.
  Provides unified access to mcp-logseq API for entity creation, updates,
  journal linking, and graph queries.
license: MIT
metadata:
  author: gentleman-programming
  version: "1.1"
---

## Purpose

You have access to the LogSeq vault as your persistence backend. This skill defines HOW to read and write artifacts. Follow this protocol exactly — do not write .md files directly or use filesystem operations for the vault.

## Vault Location

```
<your-logseq-vault-path>/
```

**Journal date format**: `dd-MM-yyyy` (e.g., "25-05-2026")
This comes from `config.edn` property `:journal/page-title-format "dd-MM-yyyy"`.

## MCP Tools Available

| Tool | When to Use |
|------|-------------|
| `mcp-logseq_create_page` | Creating a new entity page |
| `mcp-logseq_update_page` | Updating an existing page (append or replace) |
| `mcp-logseq_get_page_content` | Reading a page's full content |
| `mcp-logseq_search` | Finding pages or blocks by content |
| `mcp-logseq_query` | Querying by properties (DSL) |
| `mcp-logseq_get_page_backlinks` | Finding all pages that link to a page |
| `mcp-logseq_delete_page` | Removing a page |
| `mcp-logseq_update_block` | Updating a specific block by UUID |
| `mcp-logseq_insert_nested_block` | Adding a child block to an existing block |

## CRITICAL: The `properties` Parameter is BROKEN

> ⚠️ **NEVER use the `properties` parameter of `mcp-logseq_create_page`**
> It creates properties as flat text outside blocks, with broken `:::` syntax.
> This creates INVALID LogSeq pages that don't query properly.

**CORRECT**: Put ALL properties inside the `content` parameter as indented children of the first block.

```
❌ WRONG (creates broken pages):
mcp-logseq_create_page(
  title: "Component: handler.groovy",
  properties: { "type": "Component", "depth": "deep" }  // ← NEVER USE THIS
)

✅ CORRECT (properties in content):
mcp-logseq_create_page(
  title: "Component: handler.groovy",
  content: "- Component: handler.groovy\n  type:: [[Component]]\n  depth:: deep\n  - ## Informacion\n    - description"
)
```

The resulting markdown file looks like:
```markdown
- Component: handler.groovy
  type:: [[Component]]
  depth:: deep
  - ## Informacion
    - description
```

## Write Rules

### Rule 1: ALWAYS use mcp-logseq API — NEVER use `properties` param

Never write .md files to the vault directory. Always use the API tools above.

**IMPORTANT**: The `properties` parameter of `create_page` creates INVALID pages.
Put ALL properties in the `content` parameter as indented children of the first block.

### Rule 2: Journal in REAL-TIME — Write after EVERY page creation

The journal is the **live progress dashboard** for the human user and **context source** for AI agents.

**CRITICAL**: Write journal entries IMMEDIATELY after each action, not at the end of your phase.
If you batch journal entries, you will forget to write them. This is a deterministic outcome.

#### The mandatory sequence (do this in order):

```
Step 1: mcp-logseq_create_page(title: "Component: X", content: "...")
Step 2: mcp-logseq_update_page(page_name: "dd-MM-yyyy", mode: "append", content: "- CREAR [[Component: X]] — ...")
Step 3: mcp-logseq_create_page(title: "Slice: Y", content: "...")
Step 4: mcp-logseq_update_page(page_name: "dd-MM-yyyy", mode: "append", content: "- CREAR [[Slice: Y]] — ...")
...
Step N: At the END, write DONE entry
```

#### Journal entry prefixes:

| Prefix | When | Example |
|--------|------|---------|
| `CREAR` | After creating each page | `- CREAR [[Component: handler.groovy]] — métricas de profundidad analizadas` |
| `NOTA` | After updating a page | `- NOTA [[Component: handler.groovy]] — properties actualizadas` |
| `DONE` | End of phase | `- DONE explore — 17 slices, 5 components creados` |
| `AVANZAR` | Phase transition (orchestrator only) | `- AVANZAR [[SDD Change: X]] — explore completada → propose` |

#### Example with real date (today is 25-05-2026):

```
Step 1: mcp-logseq_create_page(
  title: "Component: handler.groovy",
  content: "- Component: handler.groovy\n  type:: [[Component]]\n  proyecto:: [[Proyecto: Shared Library Correos]]\n  depth:: deep\n  - ## Métricas\n    - lineas:: 809"
)

Step 2: mcp-logseq_update_page(
  page_name: "25-05-2026",
  mode: "append",
  content: "- CREAR [[Component: handler.groovy]] — métricas de profundidad analizadas\n  - proyecto:: [[Proyecto: Shared Library Correos]]\n  - sdd-change:: [[SDD Change: Refactor Handler]]"
)

Step 3: (next page creation...)
```

#### Phase DONE entry (write at the very end):

```
mcp-logseq_update_page(
  page_name: "25-05-2026",
  mode: "append",
  content: "- DONE explore — 17 slices, 5 components, 8 terms creados\n  - artifacts:: 6 pages\n  - sdd-change:: [[SDD Change: Refactor Handler]]"
)
```

### Rule 3: Follow the naming convention

Entity pages use the format `{Type}: {name}`:
- `SDD Change: add-dark-mode`
- `Slice: Deploy AWS Lambda`
- `Component: AwsDeploy`
- `Decision: Registry pattern for handler`
- `Term: evolutivo`
- `Explore: add-dark-mode`

### Rule 4: Use templates

When creating a page, read the template first, then fill in the values:

```
1. mcp-logseq_get_page_content(page_name: "{Type} Template")
2. Fill in placeholders (replace xx, <%today%>, etc.)
3. mcp-logseq_create_page(title: "{Type}: {name}", content: filled_content)
```

### Rule 5: Link entities with [[double brackets]]

Always use `[[Page Name]]` syntax to create links between entities:
- In SDD Change: `slice-link:: [[Slice: Deploy AWS]]`
- In Component: link to Slices that use it
- In Decision: link to Components and SDD Changes affected

### Rule 6: Properties go in the first block

Properties (key:: value) must ALL be children of the first block, indented with 2 spaces:

```
- Component: handler.groovy
  type:: [[Component]]
  proyecto:: [[Proyecto: Shared Library Correos]]
  depth:: deep
  cognicode-source:: manual
  - ## Informacion
    - description here
```

## Minimum Content Bar (what "complete" means)

Each entity page must meet this minimum:

| Entity Type | Minimum Content |
|-------------|----------------|
| `Component` | type, proyecto, ubicacion, depth, leverage, locality, deletion-test |
| `Slice` | type, proyecto, sdd-change, entry-point, slices-modificados |
| `Term` | type, canonical, definition, avoid |
| `Decision` | type, proyecto, alternatives (2+), trade-offs, eleccion |
| `SDD Change` | type, proyecto, estado, sdd-fase |
| `Explore` | type, proyecto, sdd-change, resumen (3+ líneas), metrics |
| `Proposal` | type, proyecto, sdd-change, intent (2+ líneas), approach |
| `Spec` | type, proyecto, sdd-change, requirements list (not just titles) |
| `Design` | type, proyecto, sdd-change, technical approach, code examples |
| `Tasks` | type, proyecto, sdd-change, task list with dependencies |

**If a page has less than this minimum, it is INCOMPLETE.**
Do not return from a sub-agent until all pages meet the minimum content bar.

## Error Handling

If mcp-logseq API fails:
1. Do NOT fall back to writing .md files
2. Report the error to the orchestrator
3. The orchestrator will decide whether to retry or switch modes
4. If the vault is inaccessible, use engram as fallback for critical state only

## Compact Rules

- **ALWAYS** use mcp-logseq API — never write .md directly to the vault
- **NEVER** use the `properties` parameter — put all properties in `content`
- **ALWAYS** write journal entries after EVERY page creation (not at the end)
- Journal date format is `dd-MM-yyyy` (e.g., "25-05-2026")
- Entity names use format `{Type}: {name}`
- Read templates before creating pages
- Link entities with `[[Page Name]]`
- Properties go in the first block after template declaration
- Minimum content bar: each page must have more than just titles
- If mcp-logseq fails, report to orchestrator — do not silently fall back
- Orchestrator validates journal entries after EACH sub-agent phase
