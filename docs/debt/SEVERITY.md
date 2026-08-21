# Severity Taxonomy

The intrinsic technical impact of a debt finding, independent of scheduling.

## critical
**Definition**: Blocks release OR causes data loss OR breaches security boundary.
**Examples**: SQL injection, RCE in production, unhandled panic on hot path, data corruption.
**Escalates when**: Found in any release-blocking gate.
**Response SLA**: Immediate (P0 priority expected, not enforced here).

## high
**Definition**: Degrades core functionality without workaround.
**Examples**: Major feature broken, performance regression > 50%, broken contract.
**Escalates when**: No workaround exists or workaround degrades other paths.
**Response SLA**: Within current or next cycle (P1 priority expected).

## medium
**Definition**: Degrades non-core functionality; workaround exists.
**Examples**: Sub-optimal UX, edge case bug, missing test coverage.
**Escalates when**: Workaround becomes unavailable or scope expands.
**Response SLA**: Next minor release (P2 priority expected).

## low
**Definition**: Cosmetic, structural, or speculative debt.
**Examples**: Naming inconsistency, unused import, dead branch, over-abstraction.
**Escalates when**: Accumulates into maintainability problems.
**Response SLA**: Opportunistic (P3 priority expected).

> See [ADR-0047](./../adr/ADR-0047-durable-debt-remediation.md) for the framework rationale.
