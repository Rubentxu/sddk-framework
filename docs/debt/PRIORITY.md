# Priority Taxonomy

When to remediate a debt finding, relative to other work in the cycle/release pipeline.

## P0
**Definition**: Drop everything. Fix now.
**Scheduling**: Blocks release; must be resolved before any release tag.
**Scope**: Single cycle, may extend multiple phases.

## P1
**Definition**: Fix in current or next cycle.
**Scheduling**: Sprint-level commitment.
**Scope**: Single cycle typically.

## P2
**Definition**: Fix in next minor release.
**Scheduling**: Release-level commitment.
**Scope**: May span multiple cycles if dependent.

## P3
**Definition**: Opportunistic. When convenient.
**Scheduling**: No commitment; picked up when low-risk.
**Scope**: Unlimited, may never happen.

> **Namespace note**: This priority taxonomy is distinct from UAT scenario priority (P0..P3 in `uat-plan.yaml`). They occupy different namespaces: UAT priority is feature-release scheduling; debt priority is remediation scheduling. See [ADR-0047](./../adr/ADR-0047-durable-debt-remediation.md) for the rationale.

> See [ADR-0047](./../adr/ADR-0047-durable-debt-remediation.md) for the framework rationale.
