# Product Backlog — Dynamic Workflow Refinement

## Epic DW — Dynamic Workflow Runtime
- WorkflowTemplate schema.
- WorkflowIR schema and hash/provenance.
- WorkflowCompiler service.
- WorkflowValidator service.
- Map/Join/Race/Loop operators.
- ExecutionGraphRevision.
- ExpansionProposal command/event lifecycle.
- graph budget/conflict/worktree guards.
- deterministic replay test.

## Epic SDD-A — Adaptive SDD
- ChangeContract schema.
- SHAPE capability and dynamic specialist selection.
- WorkGraph/WorkUnit model.
- BUILD worktree mapping.
- CONVERGE verdict/gap schema.
- adaptive verification router.
- proposal/spec/design/tasks/report projections.
- INTEGRATE behavior composition.

## Epic LAB — Workflow Laboratory
- WorkflowExperiment entity.
- A-full/adaptive comparable evaluation contract.
- fork/ablation runner.
- workflow metrics.
- handoff/read-use proxy.
- static Cockpit comparison views.
- promotion/shadow policy.

## Existing priority epics retained
- Hexagonal convergence/focused ports.
- Canonical events/ledger.
- OpenCode AgentHost.
- Provider failover/router.
- Context Capsules.
- Active Graph/Why.
- Static Cockpit.
- UAT extraction.
- Supply-chain provenance.

## Epic DEBT — Durable remediation
- `DebtReportV2` SDD-pack schema and canonical Rust validator.
- CAS-bound report plus evaluator-derived `DebtVerdict` and signed gate evidence.
- canonical `debt.*` events with idempotent operation IDs.
- rebuildable incidence, Active Graph and optional `INC-NNN` Markdown projections.
- tagged lifecycle operations for create/observe/reopen/reprioritize/resolve/fingerprint alias.
- governed accepted-risk, expiry, early resolution and emergency-plan override.
- deterministic P0-P3 queue with reason codes and versioned policy.
- immutable debt-plan input bound at workflow start.
- selected-debt ChangeContract invariant and bounded same-run convergence.
- read-only artifact inventory before any compaction proposal.

**Dependencies:** ADR-021/022/031/032/034/039, SPEC-023/027/031/034/035/038,
ADR-040 and SPEC-041. SDD-specific types remain pack-owned; no debt special case
enters the generic workflow kernel.

## Important sequencing
Dynamic graph execution belongs **before** trying to make the Supervisor smarter. The runtime must be able to validate and durably execute proposed strategies first.
