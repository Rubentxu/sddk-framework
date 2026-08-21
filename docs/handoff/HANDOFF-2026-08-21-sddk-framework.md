# HANDOFF — sddk-framework — 2026-08-21

> **Cycle:** `kernel-cycle-5-cross-cutting-debt` (kernel)
> **Released as:** v1.33.0
> **HEAD:** `120f3f1` (v1.32.0 base) → after 5 mechanical commits
> **Tag:** v1.33.0

## Current state (cargo test / clippy)

```
cargo test --workspace  ✓ green (all crates)
cargo clippy --workspace ✓ 0 errors (pre-existing event_envelope_golden.rs::unused_mut warning is lint-level, not blocking)
```

## What changed (5 commits)

1. `refactor(cli): consolidate uat_common::time::now_rfc3339 → sddk_domain::format (REQ-K5-001)` — 11 files, −64/+16 LOC
2. `feat(domain): extend assert_variant_count_eq! to 7 trimmed enums (REQ-K5-002)` — 5 files, +89 LOC
3. `docs(agents): trim AGENTS.md 229→≤100 LOC + extract docs/RELEASING.md + ARCHITECTURE-MODEL.md (REQ-K5-003)` — 3 files, −31/+107 LOC
4. `fix(domain): improve assert_variant_count_eq! diagnostic + close clippy drift + add variant_counts tests (REQ-K5-004)` — 4 files, +95/−9 LOC
5. `chore(release): bump to v1.33.0 (kernel-cycle-5-cross-cutting-debt) (REQ-K5-005)` — 4 files, +30 LOC

## Cycle-6 candidates

| # | Candidate | Location | Notes |
|---|-----------|----------|-------|
| — | **now_rfc3339_utc wrapper consolidation** | `format.rs`, 10 files | **DONE in cycle-6**: 19 sites migrated to Stack A, wrapper deleted, v1.34.0 |
| 1 | Local `now_rfc3339` style consolidation | `telemetry.rs:843`, `rules_cmd.rs:56`, `uat.rs:2323` | 4+3+12 sites; different impls (`OffsetDateTime::now_utc()`); not Hinnant drift; style goal |
| 2 | `ExpansionPermission::is_allowed` `#[deprecated]` removal | cycle-3 deprecation seam | Never used in cycle-4 or cycle-5 |
| 3 | WV-0027 `expires_at` clarification | `proposal.schema.json:91` (not `architecture-rules.yaml:92-107`) | Needs user intent on field vs waiver body |
| 4 | macOS / Windows musl targets | `scripts/install.sh` + `sddk-cli/Cargo.toml` | Toolchain proven; release-engineering scope |
| 5 | `compile_error!`-driven variant guard (alternative) | research spike | Macro jurisprudence locked; no parallel track |

## Recovery cheat sheet

```bash
# Verify LOC targets
wc -l AGENTS.md          # expect ≤100
wc -l docs/RELEASING.md  # expect ≥50

# Check zero now_rfc3339 orphan references
rg "crate::uat_common::time::now_rfc3339" crates/  # expect 0

# Verify variant guards (negative test)
# Edit Phase count 10→11 in cycle.rs → cargo check must fail

# Rollback this cycle
git revert <merge-sha> && git tag -d v1.33.0
```

## Anchors (apply-phase verified)

- D-1: Phase=10, CycleStatus=10 (not "likely 7" from user spec)
- D-2: 13 call sites in 9 files (not "~15 in 7")
- D-3: `sddk-domain` already a `sddk-cli` dep — no Cargo.toml edit for WU-K5-1
- D-5: PM-3 clippy fixes applied; pre-existing `event_envelope_golden.rs::unused_mut` not modified (not my responsibility)
