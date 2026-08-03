---
name: sddk-spec
description: "Trigger: sddk-spec. Write behavior specs from kernel proposals."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-spec`.

## Executor Override

If you ARE the `sddk-spec` sub-agent, continue. Write specs.

## Activation Contract

Take the proposal and produce **delta specs** — structured requirements and scenarios describing what is being ADDED, MODIFIED, or REMOVED from the system's behavior. Specs are the **source of truth** for what the implementation must satisfy.

## Hard Rules

- ALWAYS use Given/When/Then format for scenarios.
- ALWAYS use RFC 2119 keywords (MUST, SHALL, SHOULD, MAY) for requirement strength.
- Read the proposal's **Capabilities section** FIRST — it tells you exactly which spec files to create.
- Every requirement MUST have at least ONE scenario.
- Include both happy path AND edge case scenarios.
- Keep scenarios **TESTABLE** — someone should be able to write an automated test from each.
- DO NOT include implementation details in specs — specs describe WHAT, not HOW.
- **MODIFIED requirements MUST be the FULL block** — copy entire requirement + all scenarios from main spec, then edit. Partial MODIFIED blocks lose content at archive time.
- If adding new behavior WITHOUT changing existing → use ADDED, not MODIFIED.
- **Size budget**: spec MUST be under 650 words. Each scenario: 3-5 lines max.
- **KNOWLEDGE GRAPH (v3.5)**: Load `skill(name="knowledge-graph")`. For each requirement ADDED or MODIFIED, create a `REQ-{Slug}.md` node in the vault at `.sddk-knowledge/specs/{domain}/` with:
  - Properties: `type: requirement`, `title`, `slug`, `domain: "[[domain]]"`, `status: active`, `created`, `created_in_cycle: "[[CYC-date-slug]]"`, `decision_authority: "[[ADR-NNN]]"` (from design's ADR candidates), `rfc2119: MUST|SHALL|SHOULD|MAY`, `stale_after`
  - Body: requirement text, scenarios, traceability section with wikilinks to ADR and cycle
  - Read the template at `.sddk-knowledge/templates/requirement.md` before creating
  - Log the creation to `_log.md`

## Execution Steps

1. Load skills per `skills/_shared/sddk-phase-common.md` Section A. **Also load `knowledge-graph` skill.**
2. Read proposal — focus on **Capabilities section**.
3. Read design — extract **ADR candidates** (these link to requirements via `decision_authority`).
4. For each New Capability → write a full spec (delta) AND create REQ nodes in the vault.
5. For each Modified Capability → write a delta spec (ADDED/MODIFIED/REMOVED) AND update/create REQ nodes in the vault.
6. Use the **MODIFIED Requirements Workflow** for modified capabilities (CRITICAL).
6. Persist to `sddk/{change}/spec`.
7. Return envelope.

## MODIFIED Requirements Workflow (CRITICAL)

When writing a `## MODIFIED Requirements` section, follow EXACTLY:

```
1. Locate the requirement in openspec/specs/{domain}/spec.md
2. COPY the ENTIRE requirement block — from `### Requirement:` through ALL its scenarios
3. PASTE it under `## MODIFIED Requirements`
4. EDIT the copy to reflect the new behavior
5. Add "(Previously: {one-line summary of what changed})" under the requirement text

Why copy-full-then-edit?
→ The archive step REPLACES the requirement in main specs with your MODIFIED block
→ If your block is partial, the archive will lose scenarios you didn't copy
→ Common pitfall: only writing the changed scenario and losing the rest
→ If adding NEW behavior WITHOUT changing existing behavior, use ADDED instead
```

## RFC 2119 Keywords

| Keyword | Meaning |
|---------|---------|
| **MUST / SHALL** | Absolute requirement |
| **MUST NOT / SHALL NOT** | Absolute prohibition |
| **SHOULD** | Recommended, but exceptions may exist with justification |
| **SHOULD NOT** | Not recommended, but may be acceptable with justification |
| **MAY** | Optional |

## Delta Spec Format (for MODIFIED capabilities)

```markdown
# Delta for {Domain}

## ADDED Requirements

### Requirement: {Requirement Name}

{Description using RFC 2119 keywords: MUST, SHALL, SHOULD, MAY}

The system {MUST/SHALL/SHOULD} {do something specific}.

#### Scenario: {Happy path scenario}

- GIVEN {precondition}
- WHEN {action}
- THEN {expected outcome}
- AND {additional outcome, if any}

#### Scenario: {Edge case scenario}

- GIVEN {precondition}
- WHEN {action}
- THEN {expected outcome}

## MODIFIED Requirements

### Requirement: {Existing Requirement Name}

{Full updated requirement text — replaces the existing one entirely}
(Previously: {what it was before, in one line})

#### Scenario: {Unchanged scenario — keep if still valid}

- GIVEN {precondition}
- WHEN {action}
- THEN {outcome}

#### Scenario: {Updated or new scenario}

- GIVEN {updated precondition}
- WHEN {updated action}
- THEN {updated outcome}

## REMOVED Requirements

### Requirement: {Requirement Being Removed}

(Reason: {why this requirement is being deprecated/removed})
```

## Full Spec Format (for NEW capabilities)

```markdown
# {Domain} Specification

## Purpose
{High-level description of this spec's domain.}

## Requirements

### Requirement: {Name}

The system {MUST/SHALL/SHOULD} {behavior}.

#### Scenario: {Name}

- GIVEN {precondition}
- WHEN {action}
- THEN {outcome}
```

## Return Format

```markdown
## Specs Created

**Change**: {change-name}

### Specs Written

| Domain | Type | Requirements | Scenarios |
|--------|------|--------------|-----------|
| {domain} | Delta/New | {N added, M modified, K removed} | {total scenarios} |

### Coverage
- Happy paths: {covered/missing}
- Edge cases: {covered/missing}
- Error states: {covered/missing}

### Next Step
Ready for design (sddk-design). If design exists, ready for tasks (sddk-tasks).
```

## References

- `prompts/sdd-kernel/phases/spec.md` — full phase spec
- `prompts/sdd-kernel/decision-model.md` — knowledge contract
- `skills/_shared/sddk-phase-common.md` — shared protocol