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

## Phase 10 — Phase 1 Completion and Remaining SHOULD Items

> Retroactive Phase 1 fixes identified during Phase 0–9 implementation audit (2026-08-19).

### Phase 1 Blockers (P1-BLOCK)

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| Remove `sddk-engine → sddk-storage` production dependency | DONE (P1-FIX-001, P1-FIX-002) | ADR-0021 | P1-FIX-001..006 |
| Move persistence orchestration out of CLI | DONE (d712fc7) | — | P1-FIX-004 |
| Create `sddk-testkit` crate with in-memory fakes | DONE (P1-TK-001..004) | ADR-0022 | P1-TK-001..007 |
| Composition root explicit (LedgerFactory port) | DONE (d712fc7) | ADR-0021 §2 | P1-FIX-002 |
| `sddk dev entropy` advisory command | DONE (0ca8360) | — | P1-ENTROPY |
| Architecture lints ratcheted to fail (ARCH003) | DONE | ADR-0021 §3 | P1-FIX-005 |
| ArtifactStore port (insert/get/list) implemented for Storage | DONE (d712fc7) | ADR-0021 §3 | P1-FIX-005 |

### Phase 2 MUST Completion (P2-BLOCK)

> Retroactive Phase 2 verification (2026-08-19). All items verified against existing implementation.

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| Versioned EventEnvelope (EventEnvelopeV1, schema_version=1) | DONE (pre-existing) | ADR-0023 | — |
| SQLite EventStore adapter (SqliteEventStore) | DONE (pre-existing) | ADR-0023 | — |
| Projection checkpoint/rebuild contract (Projection trait + rebuild fn) | DONE (pre-existing) | ADR-0023 | — |
| Emit CEP events for one bounded slice (phase transition events via event_bus) | DONE (pre-existing) | ADR-0023 | — |
| Event hashing/canonicalization tests (content_hash + chain_hash) | DONE (pre-existing) | ADR-0023 | — |
| Outcome-vs-error taxonomy (TransitionOutcome, GateOutcomeStatus) | DONE (pre-existing) | ADR-0023 | — |
| Stream hash chaining (chain_hash column + verify_stream_chain) | DONE (pre-existing) | ADR-0023 | — |
| Event schema compatibility fixtures (5 roundtrip/back-compat tests) | DONE (df8d820) | — | P2-SC-001 |
| Projection rebuild byte/semantic equivalence tests | DONE (rebuild_integration.rs) | ADR-0023 | — |

### Phase 2 SHOULD Completion (P2-SHOULD)

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| Event export JSONL for debugging/tooling | DONE (e197405) | ADR-0023 | P2-JL-001..006 |
| Stream hash chaining | DONE (chain_hash in events_v1) | ADR-0023 | — |

### Phase 3 MUST Completion (P3-MUST)

> Retroactive Phase 3 verification (2026-08-19).

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| Extract universal Evidence model from UAT concepts | DONE (pre-existing) | ADR-0016 | — |
| Proposal -> Policy -> Capability -> Verify -> Receipt flow | DONE (pre-existing) | ADR-0020 | — |
| Bind agent/behavior version hashes into receipts | DONE (pre-existing) | — | — |
| Human approval as first-class events (emit_approval_requested/granted/denied) | DONE (pre-existing) | — | — |
| Define redaction rules for evidence | DONE (3e95d64) | ADR-0024 | P3-RD-001..005 |

### Phase 5 SHOULD Pending (P5-SHOULD)

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| Graph structural diff | MISSING | — | P5-DIFF |
| Architecture/C4 mapping from Cognicode | MISSING | — | P5-C4 |

### Phase 7 SHOULD Pending (P7-SHOULD)

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| Semantic diff metrics | MISSING | — | P7-SEM |
| Model/prompt/policy A/B workflow | MISSING | — | P7-AB |
| Git worktree/branch experiment via capability gateway | MISSING | — | P7-WORKTREE |

### Phase 8 SHOULD Pending (P8-SHOULD)

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| High-performance WebGL renderer | MISSING | — | P8-WEBGL |
| tldraw-like editable canvas adapter | MISSING | — | P8-TLDRAW |
| Mermaid/PlantUML export | MISSING | — | P8-MERMAID |
| Fork side-by-side diff UX | MISSING | — | P8-FORK-DIFF |

### Phase 9 SHOULD Pending (P9-SHOULD)

| Item | Status | Spec | Issue |
|------|--------|------|-------|
| in-toto/Sigstore provenance mapping | MISSING | — | P9-SIGSTORE |
| Third-party pack authoring guide | MISSING | — | P9-PACK-GUIDE |

---

## Dependency summary

```text
P0 -> P1 -> P2 -> P3 -> P4 -> P5 -> P6 -> P7 -> P8 -> P9
                   \              \             /
                    +-- attestation+-- explorer

P10: Phase 1 completion + remaining SHOULD items (can run in parallel with P2-P9)
```

Some spikes may run earlier, but production implementation should respect authority and data-contract dependencies.

## ADR Index

| ADR | Title | Phase | Status |
|-----|-------|-------|--------|
| ADR-0021 | Phase 1 hexagonal architecture enforcement | P1 | SPEC |
| ADR-0022 | sddk-testkit: in-memory test fakes | P1 | SPEC |
| ADR-0023 | Event export JSONL | P2 | SPEC |
| ADR-0024 | Evidence redaction rules | P3 | SPEC |
