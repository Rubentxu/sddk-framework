# HANDOFF — sddk-framework — 2026-08-22

> **Cycle:** `kernel-cycle-12-workflow-contract-reconciliation` (kernel)
> **Released as:** v1.36.3
> **HEAD:** `651eefd488191416043f22ad435c8c3da021f8c0` (17 xfail closed + release authority hardened)
> **Tag:** v1.36.3

## Cycle-11 (kernel-cycle-11-a-full-coherence-gate-ordering) — DONE

- Released: **v1.36.2** (`9de8dc3`), annotated tag v1.36.2 pushed to origin/main.
- 9 commits (b3e2637..9de8dc3: 8 apply + 1 bump), 1,067 cargo tests pass, 0 fail.
- Python: 276 PASS / 0 FAIL / 17 XFAIL (DEBT-CYCLE-11-PYTEST-CONTRACT-P1).
- 36/36 forward ACs COMPLIANT + 9/9 anti-ACs PASS (round 2 remediation).
- verify-report SHA-256 `b6882dc9acf30cdc393036470dba8903e3ee03a0edf7775b9e705acce4858f51` → PASS.
- debt-verify PASS: 7 low/P3 introduced + 1 high/P1 INC consolidated (INC-CYCLE-11-PYTEST-CONTRACT-P1).
- 0 Rust LOC. ~127 docs/prompts/python. Path: A-min.
- Bundle: ~/.local/share/sddk/framework/1.36.2/
- Archive: ~/.sddk-knowledge/sddk-framework/archive/2026-08-22-kernel-cycle-11-a-full-coherence-gate-ordering/

## Anchors (cycle-11 verified)

- A-1: Round 1 FAIL (8 critical C1-C8) → round 2 remediation (bd6c77b + c8aa615) → PASS.
- A-2: D1 recorded (remediation agent pushed directly to origin/main; no tag/transition effects).
- A-3: D2 recorded (remediation agent skipped phase.verify.remediate; reconciled in verify round 2).
- A-4: MANIFEST.sha256 regenerated in bd6c77b alongside YAML changes (cycle-10 discipline confirmed).
- A-5: 17 xfail markers exact match spec verbatim names (COHO-008-A1 anti-AC verified).
- A-6: Cycle-10 lesson applied: apply ran against clean HEAD throughout (0 fixup commits).

## What changed (cycle-11: 8 apply commits)

1. `223cf44` fix(prompts): reordenar coherence gates tras producers en MCW A-full (+~5 LOC)
2. `1e38207` fix(prompts): consistency checks for step renumbering in Quick Reference
3. `a307a1c` fix(prompts): re-number Quick Reference table A-full rows 1.3-1.6
4. `5ec6d8d` fix(workflow): add COHERENCE_GATES list to sddk-a-full.yaml (+12 LOC)
5. `e9f7533` test(workflow): xfail markers for 17 pre-existing failures (+25 LOC)
6. `142e3a6` test(workflow): REGRESSION L/O negative + E2E tests (round 1)
7. `bd6c77b` fix(prompts): depends_on explicitos en coherencias fase 2 A-full (+2 LOC)
8. `c8aa615` test(workflow): surface completa tests de coherencia — REGRESSION P/Q/R (round 2, +307 LOC)

Total: ~127 LOC docs/prompts/python. **0 Rust LOC.**

## Forward debt queue (cycle-12 candidates)

| # | Item | Severity | Priority | Source |
|---|------|----------|----------|--------|
| 1 | INC-CYCLE-11-PYTEST-CONTRACT-P1: 17 xfail, 5 regression clusters | high | P1 | cycle-11 apply |
| 2 | workflow.yaml ↔ MCW reconciliation (6 phases) | medium | P1 | spec §Out of scope |
| 3 | skills/sddk-verify/SKILL.md v2.2 → v3.4 (8 fails J) | medium | P1 | explore §3.4 |
| 4 | REGRESSION B: transition artifact refs &lt;15 (found 5) | medium | P1 | explore §3.3 |
| 5 | sddk-coherence/SKILL.md does not exist | low | P2 | explore §2.3.4 |
| 6 | A-lite YAML tasks step inconsistency | low | P2 | spec §Out of scope |
| 7 | B-direct branch-creation after execute | low | P2 | spec §Out of scope |
| 8 | Agent model frontmatter vs YAML model (9/11 disagree) | low | P2 | explore §3.1 |
| 9 | Coherence gates missing input_artifacts declarations | low | P2 | explore §3.2 |
| 10 | orchestrator.md missing scan→verify→import ordering | low | P2 | explore §3.5 |
| 11 | sddk-release/SKILL.md missing local authority contract | high | P1 | INC-CYCLE-11-PYTEST-CONTRACT-P1 row 4-6 |
| 12 | sddk-propose.md / sddk-debt-verify.md missing sddk artifact store ref | medium | P1 | INC-CYCLE-11-PYTEST-CONTRACT-P1 rows 1-2, 8-9 |

**P1 count: 6 items. P2 count: 6 items.**

## Cycle-12 (kernel-cycle-12-workflow-contract-reconciliation) — DONE

- Released: **v1.36.3** (`651eefd`), annotated tag v1.36.3 pushed to origin/main.
- 4 commits (d6cc8c8..651eefd: 3 apply + 1 bump), 1,067 cargo tests pass, 0 fail.
- Python: 296 PASS / 0 FAIL / 0 XFAIL (17 xfail closed: I-cluster artifact store x2, J-cluster sddk-verify x8, C-cluster release patterns x5, D-cluster knowledge pipeline x1 + 4 new positive assertions).
- verify-report SHA-256 `895bf7f78df370546a7f3a7c49600814205491e2e310e554fe3fc736d43ef984` → PASS_WITH_WARNINGS (14/14 scenarios + 11/11 anti-ACs COMPLIANT; 1 cosmetic WARNING W1: orphan DEBT label at test L1116).
- debt-verify PASS: 2 LOW/P3 introduced (FIND-0001 orphan label, FIND-0002 drift-prone release-authority duplication); INC-CYCLE-11-PYTEST-CONTRACT-P1 closed.
- 0 Rust LOC. ~50 docs/prompts/tests LOC. Path: A-min.
- Bundle: ~/.local/share/sddk/framework/1.36.3/
- Archive: ~/.sddk-knowledge/sddk-framework/archive/2026-08-22-kernel-cycle-12-workflow-contract-reconciliation/

## What changed (cycle-12: 3 apply commits)

1. `d6cc8c8` fix(agents): refs artifact store y autoridad release local (+~3 LOC)
2. `73fd14e` fix(prompts): sddk-verify v2.3, narrativa MCW y orden scan-verify-import (+~13 LOC)
3. `3257fff` test(workflow): cerrar 17 xfail — XFAIL vacio + umbral 15->5 + INC + MANIFEST (+~34 LOC)

Total: ~50 LOC docs/prompts/tests. **0 Rust LOC.**

## Debt introduced (cycle-12)

| ID | Title | Severity | Priority | Cluster |
|---|---|---|---|---|
| FIND-0001 | orphan DEBT label at `test_workflow_contract.py:1116` | low | P3 | CL-0013 |
| FIND-0002 | release-authority paraphrased duplication across 3 files | low | P3 | CL-0013 |

## Deviations (cycle-12)

- **Cancelled-apply**: Prior apply attempt discarded (contract-corrupting "tag optional" wording). Cycle-12 hardened all 3 release files explicitly: "annotated tag is mandatory and peels to verified SHA".
- **Stray handoff commit**: `docs/handoff/HANDOFF-2026-08-21-sddk-framework.md` was stashed during verify (not part of cycle scope).

## Forward debt queue (cycle-13+ candidates)

| # | Item | Severity | Priority | Status | Source |
|---|------|----------|----------|--------|--------|
| 1 | workflow.yaml ↔ MCW reconciliation (6 phases) | medium | P1 | open | cycle-12 §Out of scope |
| 2 | skills/sddk-verify/SKILL.md v2.3 → v3.4 (8 literal gaps) | medium | P1 | open | cycle-12 §Out of scope |
| 3 | REGRESSION B: transition artifact refs <15 (threshold now 5) | medium | P1 | open | cycle-12 §Out of scope |
| 4 | sddk-coherence/SKILL.md does NOT exist | low | P2 | open | cycle-12 §Out of scope |
| 5 | A-lite YAML tasks step inconsistency | low | P2 | open | cycle-12 §Out of scope |
| 6 | B-direct branch-creation timing (after execute) | low | P2 | open | cycle-12 §Out of scope |
| 7 | Agent model frontmatter vs YAML model (9/11 disagree) | low | P2 | open | cycle-11 explore §3.1 |
| 8 | Coherence gates missing `input_artifacts` declarations | low | P2 | open | cycle-11 explore §3.2 |
| 9 | Explore drift items not covered by WFCR (vocabulary, review/uat) | low | P2 | open | cycle-11 explore |
| 10 | cycle-11 FIND-0001..FIND-0007 (7 low/P3, still open) | low | P3 | open | cycle-11 debt |
| 11 | FIND-0001 (cycle-12): orphan DEBT label `test_workflow_contract.py:1116` | low | P3 | open | cycle-12 CL-0013 |
| 12 | FIND-0002 (cycle-12): release-authority drift-prone duplication (3 files) | low | P3 | open | cycle-12 CL-0013 |

**P1 count: 3 items. P2 count: 6 items. P3 count: 5 items.**

## Next candidates

| # | Candidate | Status | Notes |
|---|-----------|--------|-------|
| 1 | M1/M13b fork from roadmap | P1 | pending |
| 2 | P1 queue items (workflow.yaml↔MCW, sddk-verify v3.4, threshold B) | P1 | open |
| 3 | sddk-coherence/SKILL.md missing | P2 | cycle-12 §Out of scope |

## Cycle-10 (kernel-cycle-10-apply-discipline-loc-policy) — DONE

- Released: **v1.36.1** (`6f2c99f`), GitHub Release published.
- 4 commits (98904c2..6f2c99f), 1,067 tests pass, 0 fail.
- 17 ACs PASS (16 forward + 1 anti-AC), 0 FAIL.
- 3 cycle-9 debt findings CLOSED: DEBT-CYCLE-9-APPLY-DISCIPLINE + DEBT-CYCLE-9-LOC-OVERAGE + DEBT-CYCLE-9-MANIFEST-DISCIPLINE.
- Recovery cost: 1 of 4 commits = 25% (vs cycle-9 4 of 9 = 44%). Cycle-9 lesson applied successfully.
- New ADR: ADR-0048 (LOC budget policy reformulation — total-module-sum + 3 categories).
- New tool: tools/manifest.sh (regenerates MANIFEST.sha256).
- New rule: apply.md §Pre-commit Discipline (NON-NEGOTIABLE) + verify.md new mandatory gate.
- Bundle: ~/.local/share/sddk/framework/1.36.1/
- Archive: ~/.sddk-knowledge/sddk-framework/archive/2026-08-22-kernel-cycle-10-apply-discipline-loc-policy/

## Done (cycle-9, 2026-08-21)

**kernel-cycle-9-hardening-loc-refactor** — LOC budget hardening + Rust FSafe Any. 9 commits. Released as v1.36.0.

Archive: `~/.sddk-knowledge/sddk-framework/archive/2026-08-21-kernel-cycle-9-hardening-loc-refactor/`

| Metric | Value |
|--------|-------|
| LOC | 160 absorbed / 927 target = 17% |
| Commits | 9 (5 + 4 recovery) |
| Verdict | PASS |
| Debt | 2 forward entries for cycle-10 (apply discipline + LOC coverage) |

**Forward debt entries filed:**
- DEBT-CYCLE-9-APPLY-DISCIPLINE (apply ran against dirty working tree)
- DEBT-CYCLE-9-LOC-OVERAGE (per-file targets unachievable; 17% absorption)

## Done (cycle-7b, 2026-08-21)

**kernel-cycle-7b-durable-debt-runtime** — Runtime contract surface for ADR-0047 shipped (JSON Schema + INC template + agent vocabulary + workflow gates + prompt updates). Released as v1.36.0.

Archive: `~/.sddk-knowledge/sddk-framework/archive/2026-08-21-kernel-cycle-7b-durable-debt-runtime/`

## Cycle-8 (kernel-cycle-8-debt-runtime-implementation) — DONE

- Released: **v1.35.0** (`1ed973d`), GitHub Release published.
- 14 commits (a8f3f21..1ed973d), 1,066 tests pass, 0 fail.
- 60/65 ACs PASS, 3 PASS_WITH_NOTE, 1 DEFERRED, 0 FAIL.
- 10 debt findings (6 medium, 4 low); 1 forward entry for cycle-9.
- Forward: DEBT-CYCLE-8-LOC-OVERAGE → cycle-9 hardening.
- Bundle: ~/.local/share/sddk/framework/1.35.0/

## Current state (cargo test / clippy)

```
cargo test --workspace  ✓ green (all crates)
cargo clippy --workspace ✓ 0 errors
```

## What changed (cycle-10: 4 commits)

1. `98904c2` docs(prompts): anadir Pre-commit discipline a apply + verify (+25 LOC)
2. `7f1a5a4` docs(adr): nuevo ADR-0048 LOC budget policy reformulation (+67 LOC)
3. `a1e6938` chore(tools): anadir tools/manifest.sh + §MANIFEST en docs/RELEASING.md (+27 LOC shell)
4. `6f2c99f` chore(release): bump workspace version 1.36.0 → 1.36.1 (hygiene)

Total: ~134 LOC docs/prompts/shell. **0 Rust LOC.**

## Recovery cheat sheet

```bash
# Verify zero Rust LOC changes
git diff ab54b8e..HEAD -- '*.rs'  # expect empty

# Check manifest is consistent
tools/manifest.sh && git diff MANIFEST.sha256

# Verify apply discipline (cycle-10 new rule)
grep "Pre-commit Discipline" prompts/sddk/phases/apply.md
grep "commit's tree" prompts/sddk/phases/apply.md

# Check LOC policy ADR
grep "total-module-sum" docs/adr/ADR-0048-loc-budget-policy-reformulation.md
```

## Next candidates

| # | Candidate | Status | Notes |
|---|-----------|--------|-------|
| 1 | cycle-11: gate evaluator runtime | P1 | ADR-0047 §Gate wiring; deferred from cycle-7b |
| 2 | cycle-11: INC generator runtime | P2 | ADR-0047 §INC template |
| 3 | cycle-11: fingerprint generator runtime | P3 | ADR-0047 §Fingerprint |

**Deferred from cycle-9:** Manifest refresh discipline (REQ-K10-003) — now addressed in cycle-10.

## Anchors (cycle-10 verified)

- A-1: Cycle-9 lesson applied: apply phase ran against clean HEAD throughout (0 fixup commits)
- A-2: A-min path was correct for docs/prompts-only cycle (134 LOC, 0 Rust)
- A-3: tools/manifest.sh self-enforces manifest discipline for future cycles
- A-4: ADR-0048 cites cycle-9 160/927 = 17% as canonical worked example
