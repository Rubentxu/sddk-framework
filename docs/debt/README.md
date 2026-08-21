# Debt Documentation

This directory contains the canonical contracts for the durable debt remediation framework (ADR-0047).

## Files

- **[SEVERITY.md](./SEVERITY.md)** — Severity taxonomy (`critical | high | medium | low`). Intrinsic technical impact, independent of scheduling.
- **[PRIORITY.md](./PRIORITY.md)** — Priority taxonomy (`P0 | P1 | P2 | P3`). Remediation scheduling, distinct from UAT priority namespace.
- **[debt-report.schema.json](./debt-report.schema.json)** (v1.0.0, draft-07) — JSON Schema for the per-cycle debt report.
- **[INCIDENCE-TEMPLATE.md](./INCIDENCE-TEMPLATE.md)** — Template for `INC-NNN-{slug}.md` cross-cycle records.

## Source of truth

- [ADR-0047 — Remediación durable y priorizada de deuda técnica](../adr/ADR-0047-durable-debt-remediation.md)

## Status

- cycle-7a: ratified status, severity+priority taxonomies published (this directory).
- Cycle-7a + 7b: runtime contracts live.
