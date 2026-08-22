# HANDOFF — sddk-framework — 2026-08-22

> **Cycle:** `kernel-cycle-10-apply-discipline-loc-policy` (kernel)
> **Released as:** v1.36.1
> **HEAD:** `6f2c99f` (apply discipline + LOC policy + manifest discipline)
> **Tag:** v1.36.1

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
