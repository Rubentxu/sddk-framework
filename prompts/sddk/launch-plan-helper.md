# SDD Kernel Launch Plan Helper

## Schema

See `launch-plan.schema.json` for the complete JSON Schema definition.

## Quick Reference

Every launch plan MUST contain:

| Field | Required | Type | Values |
|--------|----------|------|--------|
| `phase` | Yes | string | sddk-init, sddk-explore, sddk-propose, sddk-spec, sddk-design, sddk-tasks, sddk-apply, sddk-verify, sddk-archive |
| `context_quality` | Yes | enum | C0, C1, C2, C3, unknown |
| `knowledge_coverage` | Yes | object | roadmap_backlog, work_items, architecture_adrs, ownership, learnings — each present/missing/stale |
| `taxonomy.dominant_axes` | Yes | array | domain_modeling, boundary_seam, coupling_connascence, api_contract, refactor_legacy, event_cqrs, testing, security_operations, socio_technical |
| `recommended_effort` | Yes | enum | skip, verify, deepen, recommend-lenses |
| `engram_memory` | No | boolean | Enable Engram as optional cross-session memory. Default: false |
| `with_knowledge` | No | boolean | Run knowledge pipeline (scan → verify → import) as preflight. Agents execute verify → scan and only import reviewed plan. Default: false |
| `knowledge_approved` | No | boolean | Explicit approval for quarantine candidates. Required for import step. When `with_knowledge: true` but `knowledge_approved: false`, import is skipped with "approval required". Default: false |
| `plan_version` | Yes | string | v1, v2, ... |

## Example: Minimal Valid Plan

```json
{
  "phase": "sddk-explore",
  "context_quality": "C1",
  "knowledge_coverage": {
    "roadmap_backlog": "missing",
    "work_items": "missing",
    "architecture_adrs": "present",
    "ownership": "present",
    "learnings": "missing"
  },
  "taxonomy": {
    "dominant_axes": ["boundary_seam", "coupling_connascence"],
    "evidence": "shallow modules in auth/ found via grep"
  },
  "recommended_effort": "deepen",
  "engram_memory": false,
  "plan_version": "v1"
}
```

## Example: Full Plan (apply phase)

```json
{
  "phase": "sddk-apply",
  "context_quality": "C2",
  "knowledge_coverage": {
    "roadmap_backlog": "present",
    "work_items": "present",
    "architecture_adrs": "present",
    "ownership": "present",
    "learnings": "present"
  },
  "taxonomy": {
    "dominant_axes": ["api_contract", "testing"],
    "evidence": "REST API change with new endpoint"
  },
  "domain_language": {
    "resolved": ["Order", "Invoice"],
    "unresolved": ["Shipment"]
  },
  "invariants": {
    "known": ["Order.total must equal sum of line items"],
    "explicit_unknowns": ["how to handle partial refunds"]
  },
  "recommended_effort": "verify",
  "lens_registry": "prompts/sddk/lens-registry.md",
  "skill_references": {
    "api_contract": "<your-skills-dir>/design-an-interface/SKILL.md"
  },
  "mandatory_protocols": ["persistence", "testing_capability", "review_budget"],
  "adaptive_lenses": [
    { "lens_id": "api-interface-contract", "status": "verified", "reason": "C2 + clear API change" }
  ],
  "skipped_lenses": [
    { "lens_id": "refactor-legacy-migration", "reason": "greenfield API, no legacy" }
  ],
  "escalations": ["none"],
  "engram_memory": false,
  "artifact_references": {
    "tasks": "{cycle-artifacts-dir}/my-change/tasks",
    "spec": "{cycle-artifacts-dir}/my-change/spec",
    "design": "{cycle-artifacts-dir}/my-change/design"
  },
  "git_checkpoints": {
    "branch": "feat/my-change",
    "branch_created": true,
    "pushed": true,
    "merge_target": "main",
    "semver_tag_planned": "v1.2.0"
  },
  "dev_cycle": {
    "build": "cargo build",
    "test": "cargo test",
    "lint": "cargo clippy",
    "format": "cargo fmt --check"
  },
  "plan_version": "v1"
}
```

## Versioning Rules

- Start with `v1` for each new change
- Increment when: scope changes materially, new lenses are added, different phase is targeted
- Never decrease version
- The version is stored in the artifact so downstream phases can detect stale plans

## Validation Checklist (Orchestrator)

Before injecting a launch plan into a phase prompt, verify:

- [ ] All required fields present
- [ ] `phase` matches the agent being launched
- [ ] `knowledge_coverage` reflects actual inventory
- [ ] `taxonomy.dominant_axes` has at least one entry if not skip
- [ ] `adaptive_lenses` match the recommended_effort gate
- [ ] `artifact_references` point to existing artifacts for this change
- [ ] `git_checkpoints` reflect actual git state for apply/verify/archive phases
- [ ] `plan_version` is incremented if this is a revised plan

## Anti-patterns

- `context_quality: unknown` without a blocking question → BLOCKED
- `recommended_effort: deepen` with no adaptive_lenses → add or explain
- `engram_memory: true` with no Engram MCP available → WARN (persistence degraded)
- Same `plan_version` for materially different plans → increment version
