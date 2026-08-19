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

## Important sequencing
Dynamic graph execution belongs **before** trying to make the Supervisor smarter. The runtime must be able to validate and durably execute proposed strategies first.
