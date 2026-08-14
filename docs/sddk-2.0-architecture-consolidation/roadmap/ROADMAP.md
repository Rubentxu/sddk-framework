# SDDK 2.0 Architecture Consolidation Roadmap

**Baseline:** v1.9.1 / `eb5117e6cd4366ceb205a5b2dde4195aa396d32f`  
**Strategy:** consolidation before expansion  
**Planning model:** MUST / SHOULD / DEFERRED

## Operating rule for this roadmap

During phases 0–4, do not add a new top-level product domain. New ideas are captured in `DEFERRED_IDEAS.md` with a revisit trigger. Existing UAT, SDD, testing, research, docs and other capabilities may be refactored/migrated.

## Phase 0 — Freeze, baseline and guardrails

**Objective:** create a safe refactoring envelope before moving architecture.

### MUST

- Pin current behavior with regression/CLI contract tests.
- Record current crate dependency graph and large-module baseline.
- Introduce architecture rule registry (`ARCH001..`).
- Split `AGENTS.md` normative content from historical/handoff content (SDDK2-006).
- Establish `ROADMAP.md`, ADR index and deferred-idea governance.
- Add `sddk dev entropy` in advisory mode or, minimally, generate its baseline metrics.

### Exit criteria

- Current v1.9.1 command surface has a compatibility fixture.
- No new architecture rule initially fails without being baselined/waived explicitly.
- Historical docs are preserved but no longer treated as live status.

## Phase 1 — Hexagonal seam and thin application boundary

**Objective:** make architecture enforceable by Cargo/dependency direction.

### MUST

- Introduce or formalize `sddk-app` use-case/port boundary.
- Remove direct `sddk-engine -> sddk-storage` dependency.
- Move persistence orchestration out of CLI.
- Define EventStore, ArtifactStore and core repository ports at inward boundaries.
- Add architecture lints as advisory, then ratchet selected rules to fail.

### SHOULD

- Reduce constructor/service-locator complexity with explicit composition root.
- Move cross-cutting test setup into `sddk-testkit` builders.

### Exit criteria

- `ARCH001` and `ARCH002` pass without waiver.
- Core application tests can run with fake/in-memory ports.
- CLI contains no direct SQL.

## Phase 2 — Common Event Protocol and ledger-first write path

**Objective:** create the common causal substrate.

### MUST

- Implement versioned EventEnvelope.
- Add SQLite EventStore adapter.
- Add projection checkpoint/rebuild contract.
- Emit CEP events for one bounded slice end-to-end (recommended: capability or new UAT execution events).
- Add event hashing/canonicalization tests.
- Add outcome-vs-error taxonomy.

### SHOULD

- Add stream hash chaining.
- Add event export JSONL for debugging/tooling.

### Exit criteria

- A projection can be deleted and rebuilt byte/semantically equivalent from the ledger.
- Event schema compatibility fixtures exist.

## Phase 3 — Evidence, proposal and governed side effects

**Objective:** unify assurance and authority.

### MUST

- Extract universal Evidence model from UAT concepts.
- Implement Proposal -> Policy -> Capability -> Verify -> Receipt flow for at least one governed capability.
- Bind agent/behavior version hashes into receipts.
- Add human approval as first-class events.
- Define redaction rules for evidence.

### SHOULD

- Prototype `sddk dev check --attest` receipt generation.

### Exit criteria

- A denied proposal provably causes no external effect.
- A successful capability has evidence + postcondition + receipt lineage.

## Phase 4 — Real packs and UAT extraction

**Objective:** prove the small-core/pack architecture with the most complex existing vertical.

### MUST

- Introduce Pack Manifest v2 (`requires`, `integrates_with`, `provides`, `conflicts_with`).
- Implement pack registry/load/verify/disable lifecycle.
- Extract UAT use cases/domain boundaries behind `sddk-pack-uat` or equivalent module boundary.
- Preserve v1.9 guided runner commands through compatibility facade.
- Move UAT evidence references to universal Evidence model.
- Add pack conformance fixtures.

### SHOULD

- Extract one bridge pack (recommended Cognicode mapping) as a second architecture proof.

### Exit criteria

- Core can run without UAT pack loaded.
- UAT pack passes current guided runner/release acceptance tests.
- Optional integrations degrade gracefully.

## Phase 5 — Reactive knowledge/evidence graph

**Objective:** make engineering state queryable and reactive without compromising kernel authority.

### MUST

- Implement GraphProjection from CEP events.
- Implement GraphStore port + local adapter.
- Add bounded GraphView.
- Add deterministic pattern matching for core high-value patterns.
- Add proposal-only Behavior runtime.
- Add at least two relation behaviors (`verifies`, `depends_on` or `governs`).
- Add `sddk graph why` and rebuild support.

### SHOULD

- Add graph structural diff.
- Add architecture/C4 mapping from Cognicode or repository facts.

### Exit criteria

- Graph state is rebuildable from ledger.
- Reactive behavior cannot directly acquire a governed capability.
- A code/requirement change can generate a deterministic stale/verification proposal.

## Phase 6 — Staleness, context reads and semantic assurance

**Objective:** make decisions explainable and freshness-aware.

### MUST

- Generalize staleness state and causal paths.
- Implement context-read tracing in opt-in/bounded mode.
- Add impact queries.
- Integrate critical UAT acceptance staleness with release policy.

### SHOULD

- Add documentation/ADR staleness advisory rules.
- Add `graph why-stale` UX.

### Exit criteria

- User can explain why an artifact/decision is stale through a causal path.
- User can inspect what artifacts/evidence an agent execution read without chain-of-thought storage.

## Phase 7 — Fork, replay, diff and controlled experiments

**Objective:** support counterfactual engineering and agent evaluation.

### MUST

- Implement frame identifiers consistently.
- Implement ledger fork from event/sequence.
- Implement reconstruct replay and deterministic strict replay.
- Implement recorded LLM/tool response cache for forks/evaluation.
- Implement structural diff.
- Implement fail-closed state promotion.

### SHOULD

- Semantic diff metrics.
- Model/prompt/policy A/B workflow.
- Git worktree/branch experiment integration through capability gateway.

### Exit criteria

- Same shared prefix yields identical reconstructed state.
- A fork can compare two agent/policy variants without replaying shared nondeterministic I/O.

## Phase 8 — Moldable Explorer

**Objective:** expose the ledger/graph as an engineering instrument, not only raw tables.

### MUST

- Graph + timeline/trace primary views.
- Declarative view descriptor contract.
- Architecture/C4, Verification, Evidence, UAT, Agent and Release views.
- Progressive disclosure for large graphs.
- Provenance panel and `why` navigation.

### SHOULD

- High-performance WebGL renderer.
- tldraw-like editable canvas adapter.
- Mermaid/PlantUML export.
- Fork side-by-side diff UX.

### Exit criteria

- Same entity can be opened in multiple task-specific views without duplicating domain data.

## Phase 9 — Quality ratchets, release channels and ecosystem hardening

**Objective:** make the new architecture maintainable.

### MUST

- Expand golden dataset to 30–50 cases.
- Ratchet entropy/architecture rules using measured baselines.
- Signed local gate receipt verification in protected remote flow.
- Define `stable/candidate/edge/dev` channel metadata and promotion rules.
- Generate/validate deterministic docs/inventories from machine-readable sources.

### SHOULD

- Standard provenance mapping to in-toto/Sigstore.
- Second GraphStore adapter only if metrics justify it.
- Third-party pack authoring guide.

### Exit criteria

- Architecture regressions are caught automatically.
- Stable release can be traced to signed gate evidence and immutable artifacts.

## Dependency summary

```text
P0 -> P1 -> P2 -> P3 -> P4 -> P5 -> P6 -> P7 -> P8 -> P9
                   \              \             /
                    +-- attestation+-- explorer
```

Some spikes may run earlier, but production implementation should respect authority and data-contract dependencies.
