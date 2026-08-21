# Implementation Backlog — Ordered

## Milestone M0 — Ratchet
- [ ] Add architecture dependency snapshot.
- [ ] Add allowlisted current violations.
- [ ] `check-arch` rule framework.
- [ ] Add contract tests around current CLI/workflow behavior.

## M1 — Ports & composition
- [ ] Introduce `EventAppender`, `EventReader`.
- [ ] Split workflow persistence ports.
- [ ] Split evidence/context ports.
- [ ] Move concrete storage construction out of app core.
- [ ] Remove production `engine -> storage` dependency.
- [ ] In-memory test adapters.

## M2 — Event foundation
- [ ] Event schema/version registry.
- [ ] Canonical event validator.
- [ ] Correlation/causation helpers.
- [ ] Subscription/reaction dispatcher.
- [ ] Journal projection.

## M3 — Workflow v2
- [ ] Definition schema/parser.
- [ ] WorkflowRun state machine.
- [ ] NodeRun state machine.
- [ ] Attempt model.
- [ ] Scheduler.
- [ ] parallel/join.
- [ ] wait-for-event.
- [ ] retry/timeout/cancel.
- [ ] legacy SDD compiler.

## M4 — OpenCode adapter
- [ ] Host capabilities.
- [ ] Event normalization.
- [ ] usage capture.
- [ ] execute turn.
- [ ] context injection.
- [ ] abort/resume.
- [ ] compatibility tests.

## M5 — Failover/router
- [ ] Failure classifier.
- [ ] Route candidates.
- [ ] Health projection.
- [ ] Circuit breaker behavior.
- [ ] retry policy.
- [ ] route explainability.
- [ ] quota failover acceptance test.

## M6 — Behaviors/supervisor
- [ ] Reaction level classifier.
- [ ] Behavior idempotency.
- [ ] OrchestratorSignal schema.
- [ ] SupervisorDecision schema.
- [ ] cognitive host invocation.
- [ ] delegation policy.

## M7 — Context
- [ ] ContextCapsule schema.
- [ ] selectors.
- [ ] actual-read events.
- [ ] staleness projection.
- [ ] negative knowledge.
- [ ] recovery deltas.

## M8 — Graph/Why
- [ ] typed graph builder.
- [ ] provenance edges.
- [ ] causal queries.
- [ ] rebuild test.
- [ ] `sddk why`.

## M9 — Cockpit
- [ ] snapshot schema.
- [ ] static renderer.
- [ ] overview.
- [ ] journal.
- [ ] timeline.
- [ ] provider health.
- [ ] causal lens.
- [ ] `build/open/watch`.

## M10 — UAT extraction
- [ ] domain split.
- [ ] repositories/ports.
- [ ] campaign/run/defect/retest/signoff.
- [ ] workflow definitions.
- [ ] change-impact integration.

## M11 — Multi-pack proof
- [ ] SDD pack.
- [ ] UAT pack.
- [ ] Incident pack.
- [ ] no kernel domain special-casing audit.

## M12 — Evaluation/forks
- [ ] fork metadata.
- [ ] isolated worktrees.
- [ ] outcome comparison.
- [ ] golden capability fixtures.
- [ ] routing shadow mode.

## M13 — Supply chain
- [ ] SBOM/provenance object types.
- [ ] artifact lifecycle projection.
- [ ] release gate policies.

## M13b — Durable debt remediation

Depends on M2, M3/M3b, M8, M11b and the M13 artifact primitives.

- [ ] Add SDD-pack `DebtReportV2`, tagged proposals and canonical serialization fixtures.
- [ ] Implement `sdd.debt.validate/evaluate` in Rust; callers cannot provide verdicts.
- [ ] Store `debt-report` in CAS and sign report/subject/baseline/policy/evaluator bindings in gate evidence.
- [ ] Register versioned `debt.*` event payloads and idempotent operation IDs.
- [ ] Build replayable incidence, queue, Active Graph and optional Markdown projections.
- [ ] Implement governed risk acceptance, expiry, early resolution and emergency override receipts.
- [ ] Implement deterministic P0-P3 policy with reason codes and deferral budgets.
- [ ] Add `sddk debt validate/evaluate/queue/plan/accept-risk/why` host commands.
- [ ] Bind immutable debt-plan input in Workflow Runtime v2; add an optional legacy cycle-start bridge.
- [ ] Enforce selected debt through ChangeContract and bounded CONVERGE remediation.
- [ ] Add three-run, reopen, alias, expiry, P0, stale-plan and projection-replay acceptance fixtures.
- [ ] Add read-only `sddk artifact inventory`; do not add deletion or compaction.

## M3b — Dynamic workflow core (insert immediately after Workflow v2)

- [ ] WorkflowTemplate vs WorkflowIR contracts.
- [ ] Workflow Compiler and Validator.
- [ ] ExecutionGraphRevision + digest.
- [ ] Map/dynamic fan-out.
- [ ] Join/Race.
- [ ] bounded Loop/Convergence.
- [ ] ExpansionProposal validation/events.
- [ ] graph node/depth/concurrency/budget guards.
- [ ] worktree collision checks.

## M11b — SDD Adaptive & Workflow Laboratory

- [ ] ChangeContract.
- [ ] SHAPE dynamic specialist selection.
- [ ] BUILD WorkGraph/WorkUnits.
- [ ] CONVERGE adaptive verification/remediation.
- [ ] legacy document projections.
- [ ] A-full baseline fixtures.
- [ ] WorkflowExperiment + ablation runner.
- [ ] Cockpit comparison view.
