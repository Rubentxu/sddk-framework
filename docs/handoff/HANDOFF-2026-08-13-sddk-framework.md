# HANDOFF — sddk-framework — 2026-08-13

> **Cycle:** `sddk-2-0-phase0-doc-governance` (SDDK2-006)
> **Released as:** v1.9.9
> **HEAD:** `13f6028` (pre-cycle base) → final cycle-close SHA
> **Tag:** v1.9.9

## Drift carry-over (not resolved in this cycle)

| Drift | Location | Status |
|-------|----------|--------|
| `bootstrap.sh` SHARED_DIR → SDDK_FRAMEWORK_ROOT | `bootstrap.sh` L16 | Pending — cosmetic rename only |

## Last closed cycle

`sddk-2-0-phase0-doc-governance` (v1.9.9) — doc governance split.

## Current state (cargo test / clippy)

```
cargo test --workspace   ✓ green (215 tests, pre-cycle baseline)
cargo clippy --workspace ✓ 0 errors
```

## Recovery cheat sheet

```bash
# Verify workspace hygiene (no crate src/ modified)
git diff --name-only HEAD~3 -- 'crates/sddk-*/src/**'  # expect empty

# Check AGENTS.md LOC budget
wc -l AGENTS.md                                    # expect ≤150
wc -l docs/history/AGENTS-history.md               # expect ≤80
wc -l docs/handoff/HANDOFF-*-sddk-framework.md    # expect ≤100

# Rollback this cycle
git reset --hard <pre-cycle-SHA> && git tag -d v1.9.9
```

## What changed (3 commits)

1. `docs(agents): split AGENTS.md into stable/history/handoff surfaces (SDDK2-006)`
2. `docs(roadmap): renumber SDDK2-004/SDDK2-005 to SDDK2-006/SDDK2-007 (reconcile roadmap↔vault ID collision)`
3. `chore(release): bump to v1.9.9 (sddk-2-0-phase0-doc-governance)`

## Next cycle (suggested)

`sddk-2-0-phase0-baseline` (SDDK2-002) — dependency + entropy baseline.
