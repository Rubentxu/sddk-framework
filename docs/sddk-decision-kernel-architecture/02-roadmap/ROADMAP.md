# ROADMAP — refined for Dynamic Workflows & SDD Adaptive

## Strategy
Prioritize deterministic foundations first, then dynamic workflow power, then Supervisor intelligence. Keep A-full operational as a reference until empirical comparison validates simplification.

## Phase 0 — Baseline & architecture ratchet
- dependency/crate map;
- baseline current SDD A-min/A-lite/A-full behavior and quality;
- inventory `Phase`/`CyclePath` coupling;
- baseline tokens/time/agent calls/handoffs where available;
- initial `sddk check-arch`.

**Exit:** architectural and workflow baselines are measurable.

## Phase 1 — Hexagonal convergence
Focused ports, composition root, remove `engine -> storage`, in-memory adapters, compatibility facade.

## Phase 2 — Canonical Event Ledger
Event schema/versioning, correlation/causation, journal projection, replay tests.

## Phase 3 — Workflow Runtime v2 core
- WorkflowTemplate/WorkflowIR;
- WorkflowRun/NodeRun/Attempt;
- Sequence/Parallel/Choice/Gate/Wait/SubWorkflow;
- pause/resume/retry/cancel;
- legacy SDD compiler.

**Exit:** current canonical workflows can run without hard-coded `Phase` in kernel.

## Phase 4 — Dynamic workflow engine **(raised priority)**
- Workflow Compiler/Validator;
- Map/dynamic fan-out;
- Join/Race;
- bounded Loop;
- ExecutionGraphRevision;
- expansion proposal/events;
- graph/node/depth/concurrency/budget guards;
- worktree conflict validation.

**Exit:** a discovery node can create N runtime work units after workflow start and replay reconstructs the same graph.

## Phase 5 — AgentHost + provider resilience
OpenCode event/control adapter, usage capture, failure classification, route health, circuit breakers, same-NodeRun failover.

## Phase 6 — Reactive behaviors + Supervisor
L0/L1/L2 reactions, dynamic workflow behaviors, typed OrchestratorSignals, cognitive replan, bounded sub-supervisors.

## Phase 7 — Context Compiler
Capsules, deltas, actual reads, staleness, negative knowledge, recovery context.

## Phase 8 — SDD Adaptive experimental
- ChangeContract;
- SHAPE/BUILD/CONVERGE/INTEGRATE;
- adaptive specialist activation;
- adaptive verification;
- legacy document projections.

**Exit:** `sdd-adaptive` completes representative simple and high-risk changes with all invariants/evidence.

## Phase 9 — Workflow Laboratory
- baseline A-full vs adaptive;
- fork/ablation;
- workflow metrics/handoff proxy;
- Cockpit experiment comparison;
- promotion policy/shadow rollout.

## Phase 10 — Active Graph + `sddk why`
Typed graph, dynamic graph revisions, causal queries, evidence/requirement edges, moldable views.

## Phase 11 — Static Cockpit
Overview, Journal, timeline, execution graph, provider health, usage, experiments, `build/open/watch`.

## Phase 12 — UAT bounded context / pack
Extract lifecycle, defects/retests/signoff/change impact; integrate UAT as convergence capability.

## Phase 13 — Multi-pack proof
SDD, UAT, Incident all on the same dynamic-capable runtime with no kernel domain special cases.

## Phase 14 — Supply chain, policy ratchets, production hardening
SBOM/provenance, artifact lifecycle, signed gates, performance/retention, migration cleanup.

## Promotion rule for `sdd-adaptive`
Do not make adaptive the default merely because it is cheaper. Require non-inferior quality/invariant coverage and bounded rollout evidence from Workflow Laboratory.
