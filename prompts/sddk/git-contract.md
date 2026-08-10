# SDDK Git Contract v2

This contract defines how SDDK phases integrate with git operations and the
full change lifecycle. It is technology-agnostic.

This contract is the **single source of truth** for git operations across the SDDK pipeline. The orchestrator prompt references this file but does not duplicate its rules.

## Lifecycle Overview

```
PRE-FLIGHT: trunk sync + consolidation check
    ↓
PLAN: explore → propose → spec || design → tasks → branch
    ↓
BUILD: apply → verify → debt-verify (pre-PR) → archive
    ↓
CONSOLIDATE: push → PR → checks/approval → merge → tag → HTML report → close issue → knowledge graph → release lock
    ↓
RESET: trunk sync + cycle marker
```

## Quick Path

1. **Trunk base always.** Every cycle starts on a fresh `main` and ends by syncing back to it.
2. **Branch per change.** Every SDDK change lives on its own feature branch. No two changes share a branch.
3. **Conventional commits only.** `<type>(<scope>): <description>` validated by the commit checklist and regression tests.
4. **One commit = one logical unit.** Atomic, never broken.
5. **PR is the gate to main.** Never commit directly to main. Always go through a PR.
6. **Merge commit (`--no-ff`).** Never fast-forward, never rebase onto main.
7. **Never delete branches.** Feature branches live forever as historical record.
8. **Semver tag at milestones.** Every completed cycle → `v<major>.<minor>.<patch>` tag pushed.
9. **HTML report at end.** Every cycle ends with a self-contained HTML closing report.
10. **No two cycles open.** Previous cycle must be 100% consolidated before starting a new one.
11. **Zero intrusion.** SDDK durable knowledge lives in `{vault}` and cycle
    artifacts live in `{cycle-artifacts-dir}`. SDDK never creates framework
    metadata, knowledge, reports, ignores, or checkpoints in an adopted
    workspace. Existing product-owned documentation is read-only evidence.

## Persistence Boundary

| Data | Authoritative location |
|---|---|
| Durable project knowledge | `{vault}`, resolved by `sddk knowledge path` |
| Per-cycle working state and reports | `{cycle-artifacts-dir}` |
| Generated inventory/workflow docs | `$SDDK_DATA_DIR/projects/{project_id}/generated/` |
| Metrics and caches | XDG state/cache |
| Optional presentation copy | `/tmp` |

Engram is an optional mirror only when enabled by `sddk knowledge status`.
Failure of any external persistence channel is blocking; never fall back to
workspace-local files.

## Git-SDDK Phase Interleaving (Detailed)

```
sddk-init
    ↓
sddk-explore → explore-report.md
    ↓
sddk-propose → proposal.md
    ↓ (coherence check)
[sddk-spec || sddk-design] (PARALLEL) → spec.md, design.md
    ↓ (coherence check)
sddk-tasks → tasks.md
    ↓
git checkout -b <type>/<description>
git push -u origin <type>/<description>
    ↓
sddk-apply → atomic conventional commits on branch
    ↓ (coherence check)
sddk-verify → verify-report.md
    ↓ (correction cycle on FAIL)
sddk-debt-verify → debt-report.md (MANDATORY on A-*, n/a on B-direct — on feature branch, pre-PR)
    ↓ (remediation on same branch if FAIL — max 3 rounds, remediation_round incremented, NO separate branch/PR/release)
sddk-archive → archive-report.md
    ↓
git push origin <type>/<description>
    ↓
gh pr create --base main --head <type>/<description> \
  --title "<type>(<scope>): <description>" \
  --body-file <generated from artifacts>
    ↓
[wait for review approval]
    ↓ (after merge)
git checkout main && git pull origin main
    ↓
git tag -a v<major>.<minor>.<patch> -m "<type>: milestone — <description>"
git push origin v<major>.<minor>.<patch>
    ↓
[generate HTML closing report]
    ↓
[close tracking issue if any]
    ↓
[update knowledge graph and release lock]
    ↓
[commit cycle marker to main]
    ↓
NEXT CYCLE (must repeat Phase 0 pre-flight)
```

## Invariant Rules

### Rule 0 — Trunk is the only source of truth (HARD GATE)

`main` is the trunk. All other branches are ephemeral work areas. **No commit may reach `main` except via an approved PR that has passed `sddk-verify` (PASS or PW). Debt-verify (MANDATORY on A-*, n/a on B-direct) must also have passed (PASS or PW) before PR creation.**

**Pre-flight check (MCW Step 0.1)** — HARD GATE:
```bash
git fetch origin main
git checkout main
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || BLOCK
```

**Post-flight check (MCW Step 4.1)** — HARD GATE: same check, must pass before next cycle.

**Invariant**: trunk sync is enforced by checklist gate on `git push` (blocked if targeting `main`) and on `git checkout main && git pull` (result must match `origin/main`).

If the cycle ended with commits not on `main` (e.g., on a feature branch), the cycle is **NOT closed**. Step 0.2 gate will BLOCK the next cycle.

### Rule 1 — Branch per SDDK change

Every SDDK change lives in its own feature branch. Two distinct SDDK changes never share a branch.

**Branch naming**: `<type>/<description>`

| Type | When |
|------|------|
| `feat` | New user-visible or API functionality |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `chore` | Maintenance, dependencies, tooling, configuration |
| `refactor` | Code change without behavior change |
| `perf` | Performance improvement |
| `test` | Tests only |
| `ci` | CI/CD changes |
| `revert` | Reversion of a previous commit |

Description: kebab-case, max 72 chars, descriptive.

**Validation**: branch name validated against regex `^[a-z]+/[a-z0-9-]+$` by checklist gate on `git checkout -b`. Invalid names are rejected.

### Rule 2 — Branch creation and push

- **When**: Immediately after `sddk-tasks` completes and before `sddk-apply` starts.
- **Push**: Push to remote immediately after creation. Code must never live uncommitted between SDDK iterations.

### Rule 3 — Conventional Commits

**Format**:
```
<type>(<scope>): <short description>

[optional body]

[optional footer with references]
```

**Examples**:
```
feat(auth): add OAuth2 login flow
fix(api): handle null user in session lookup
docs(readme): update install instructions
chore(deps): bump axios to 1.6.0
```

**Validation**: the commit checklist validates every `git commit -m "<msg>"`. Bad messages block the operation. This is a declarative invariant until runtime enforcement is implemented.

### Rule 4 — Atomic commits

One commit = one logical unit of work. Do not bundle unrelated changes.

Every commit must:
- Build cleanly
- Pass all tests
- Be self-contained
- Reference an issue if applicable (via `Closes #N` in the footer)

Never commit broken code. If a partial change is unavoidable, use `chore(WIP):` commits sparingly.

### Rule 5 — PR is the only path to main

Main is protected:
- **No direct commits to main.** Always go through a PR.
- **No force pushes to main.** Ever.
- **No merge commits bypassing review.** PR must be approved before merge.
- **No fast-forward from feature branch.** Always use merge commit (`--no-ff`).

PR creation is part of the orchestrator workflow (see MCW Step 3.2 in orchestrator.md).

### Rule 6 — Merge via approved PR

```
gh pr create --base main --head <branch> --title "..." --body "..."
# wait for CI + review approval
gh pr merge <pr-number> --squash  # or --merge for --no-ff
```

The orchestrator must wait for the PR to reach `MERGED` state before proceeding to tag.

### Rule 7 — Never delete branches

Feature branches live in the remote as permanent historical record. GitHub preserves the PR↔branch link even after merge.

Do NOT run `git branch -d` or `git push origin --delete` on a feature branch.

### Rule 8 — Trunk sync at start AND end of every cycle

**At start (Phase 0 Step 0.1)**:
```
git checkout main && git pull origin main
```

**At end (Phase 4 Step 4.1)**:
```
git checkout main && git pull origin main
```

Never start a new change from a stale main. Never leave the workspace on a feature branch when the cycle is done.

### Rule 9 — Close before open (no overlapping cycles)

Do not start a new SDDK change until the previous one is:
- Merged to main
- Tagged with semver
- Reported in HTML
- Marked with semver tag pointing to a main commit (the tag itself is the cycle marker)

If blocked, record the blocker in the cycle manifest/incidence node and resolve it before starting a new cycle.

### Rule 10 — Semantic versioning at milestones

After each completed cycle (PR merged to main), create an annotated tag:

```
git tag -a v<major>.<minor>.<patch> \
  -m "<type>: milestone — <description>

Release: <change-name>
- <feature bullet>
- <fix bullet>

See {cycle-artifacts-dir}/archive-report.md for details."
git push origin v<major>.<minor>.<patch>
```

**Version bump rules**:

| Bump | When | Example |
|------|------|---------|
| `major` | Breaking change to public API or contract | `v1.0.0 → v2.0.0` |
| `minor` | New feature, non-breaking | `v1.0.0 → v1.1.0` |
| `patch` | Bug fix, chore, docs, refactor | `v1.0.0 → v1.0.1` |

First release of a new project starts at `v0.1.0`. Pre-1.0 versions follow the same rules (0.1.0 → 0.2.0 → 1.0.0).

### Rule 11 — HTML closing report at end of every cycle

After tag push, generate the HTML closing report (delegated to `sddk-archive` agent using `prompts/sddk/HTML-REPORT.md`):

```
xdg-open <report-path>
```

Path routing:
- Authority: `{cycle-artifacts-dir}/reports/cierre.html`
- Optional presentation copy: `/tmp/sddk-<change>-<YYYYMMDD>.html`

The orchestrator must verify the HTML file exists and is non-empty.

### Rule 12 — Issue closure

If the cycle was tracked by a GitHub issue:

```
gh issue close <issue-number> --comment "Completed in PR #<pr-number>. Released as v<version>."
```

### Rule 13 — Large PRs (>400 LOC changed)

Load `skill(name="chained-pr")` to split into reviewable stacked PRs. Each slice must be independently buildable and testable.

This rule is enforced by the Review Budget Guard (MCW Step 1.7) before launching apply.

### Rule 14 — Cycle completion: tag + F3 metrics

The semver tag (Rule 10) IS the cycle marker. No separate marker file.

Additionally, MCW Step 4.2 writes cycle metrics to:
- `$SDDK_STATE_DIR/metrics/{cycle_id}.jsonl` (local, machine-readable)
- Optional Engram observation `topic_key: sddk/cycle-metrics/{cycle_id}`
- `metrics/aggregate` rolling 7d/30d (F3 tuner consumes this)

If cycle had `verify_verdict=PASS` + `first_pass_success=true` + reusable decision, also save jurisprudence observation (topic_key: `jurisprudence/{category}`). See `prompts/sddk/decision-model.md` § Jurisprudence Schema.

The presence of a semver tag on a main commit = cycle closed. MCW Step 0.2 checks this to enforce "no overlapping cycles" via `git tag --points-at main`.

## Phase Agent Responsibilities

| Phase | Git responsibility |
|-------|--------------------|
| `sddk-tasks` | After producing tasks, the orchestrator creates the feature branch and pushes to remote before launching apply. |
| `sddk-apply` | Every completed task slice gets its own atomic conventional commit. Use the type table from Rule 3. Never commit broken code. |
| `sddk-verify` | Fix commits follow the same conventional format (`fix(<scope>): ...`). No skipping verify. |
| **`sddk-debt-verify`** (MANDATORY on A-*) | **Read-only** on the feature branch. Runs BEFORE PR creation. Emits `debt-report.md` and verdict. On FAIL with `re_iterate_from: apply`, triggers remediation on the SAME feature branch (increment `remediation_round`; max 3 rounds). Cluster agents never commit; only the remediation apply phase produces commits. |
| `sddk-archive` | After archiving, the orchestrator hands off to release for PR, merge, tag, report, knowledge graph update, and lock release. |

## Debt-Remediation Discipline (v3.6)

When `sddk-debt-verify` returns FAIL with `re_iterate_from: apply`, the remediation happens on the **SAME feature branch**, not a separate branch:

1. Increment `remediation_round` (starts at 1, max 3)
2. The orchestrator applies fixes directly on the feature branch via `sddk-apply`
3. Re-run `sddk-verify` on the fixed branch
4. Re-run `sddk-debt-verify` with the updated `remediation_round`
5. If round 3 fails: escalate to user with full debt report. Do NOT auto-merge.

**Key invariants**:
- Same feature branch and same `cycle_id`; never create an auxiliary remediation branch
- NO separate PR or release for remediation — it merges with the original feature branch's PR
- Remediation commits are part of the same PR to main
- `remediation_round` is tracked in the debt-report and launch plan

## Enforcement

| Rule | Mechanism |
|------|-----------|
| Branch name format | Checklist validates regex `^[a-z]+/[a-z0-9-]+$` on `git checkout -b` |
| Commit message format | Checklist validates conventional commit format on `git commit` |
| Force-push to main | Checklist blocks `git push --force` targeting main |
| Rebase onto main | Checklist blocks `git rebase` |
| Stash | Checklist blocks `git stash` |
| Reset | Checklist blocks `git reset` |
| Phase agent git writes | Checklist enforces no direct git writes by phase agents |
| Commit message secrets | Checklist rejects commits containing API keys |
| File size > 500KB | Checklist rejects oversized commits |
| Cycle consolidation | Orchestrator prompt Step 0.2 hard gate |
| Trunk sync | Orchestrator prompt Step 0.1 + Step 4.1 hard gates |
| Verify before PR | `verify-report.md` must exist with PASS/PW (always) |
| Debt-verify before PR (MANDATORY on A-*, n/a on B-direct) | `debt-report.md` must exist with PASS/PW on A-* paths (depth derived from path; no user opt-in; reversibility adjusts depth, never skip) |
| PR approval | Orchestrator prompt Step 3.3 hard gate |
| Tag push | Orchestrator prompt Step 3.5 hard gate |
| HTML report | Orchestrator prompt Step 3.6 hard gate |

## Anti-Patterns (FORBIDDEN)

| Anti-pattern | Consequence | Enforcement |
|---|---|---|
| Committing directly to main | Bypasses review | Checklist + branch protection |
| Force-pushing to main | Destroys history | Checklist + GitHub branch protection |
| Rebasing feature branches onto main | Loses review history | Checklist |
| Deleting branches after merge | Loses PR↔branch link | Manual flag in runbook |
| Starting new cycle without merging previous | Two cycles open, conflicts | MCW Step 0.2 |
| Skipping PR review | Unreviewed merge | MCW Step 3.3 |
| Merging without PR | Bypasses review | MCW Step 3.4 verifies commit came via PR |
| Skipping semver tag | Lost milestone marker | MCW Step 3.5 |
| Skipping debt-verify on A-* path | CRITICAL debt reaches main | MCW Step 2.4 mandatory gate |
| Remediation on a separate branch | Separate release cycle breaks trunk discipline | Debt-Remediation Discipline: same branch, same cycle_id |
| Skipping HTML report | No audit trail | MCW Step 3.6 |
| Skipping trunk sync | Stale main | MCW Step 0.1 + Step 4.1 |
| Mixing multiple SDDK changes in one branch | Confused scope | Branch naming enforcement |
| Large monolithic commits | Unreviewable | MCW Step 1.7 chained-pr |

## Recovery

If the cycle is interrupted mid-flight, the orchestrator should be able to resume:

1. Check `git tag --points-at main` → if last semver tag exists, cycle was complete; new cycle can start
2. Check the artifact registry for which phase artifact is the latest
3. Resume from the next pending phase
4. If apply was interrupted, use
   `{cycle-artifacts-dir}/apply-checkpoint.json`
5. If PR was created but not merged, check PR state and resume from there
6. If tag was not pushed, push it after merge
7. If HTML report was not generated, regenerate

The orchestrator prompt's "Dependency Graph" section describes how to resume from any phase.
